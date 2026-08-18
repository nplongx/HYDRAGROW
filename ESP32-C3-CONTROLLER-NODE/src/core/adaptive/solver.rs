// src/fsm/solver.rs
//! SolverStrategy — Trừu tượng hóa thuật toán tính ControlVector.
//! Cold path dùng hằng số config tĩnh.
//! Warm path dùng Moore-Penrose giả nghịch đảo.

use hydragrow_shared::{ControllerConfig, SensorData};

use crate::core::{adaptive::matrix::{ControlVector, StateDeltaVector}, fsm::SystemContext, optimizer::apply_safety_guardrails};

/// Kết quả từ solver: control vector và target để pass vào DosingActor.
#[derive(Debug, Clone)]
pub enum SolveResult {
    Execute {
        control: ControlVector,
        target_ec: f32,
        target_ph: f32,
        pwm: u32,
    },
    Idle,
}

/// Interface chung cho tất cả solver strategies.
pub trait SolverStrategy {
    fn solve(
        &self,
        sensors: &SensorData,
        config: &ControllerConfig,
        ctx: &SystemContext,
    ) -> SolveResult;
}

/// Helper DTO chứa các giá trị sai lệch an toàn (đã áp dụng tolerance & sensor enable flag).
#[derive(Debug, Clone, Default)]
struct SafeStateDeltas {
    ec: f32,
    ph: f32,
    water: f32,
    temp: f32,
}

impl SafeStateDeltas {
    fn compute(sensors: &SensorData, config: &ControllerConfig, ctx: &SystemContext) -> Self {
        let ec_delta = (config.ec_target - sensors.ec).max(0.0);
        let ph_delta = config.ph_target - sensors.ph;
        let water_delta = config.water_level_target - sensors.water_level;
        let temp_delta = config.misting_temp_threshold - sensors.temp;

        let ec_tolerance = effective_ec_tolerance(config, ctx);
        let ph_tolerance = effective_ph_tolerance(config, ctx);

        let ec = if config.enable_ec_sensor && ec_delta.abs() > ec_tolerance {
            ec_delta
        } else {
            0.0
        };

        let ph = if config.enable_ph_sensor && ph_delta.abs() > ph_tolerance {
            ph_delta
        } else {
            0.0
        };

        let water = if config.enable_water_level_sensor
            && water_delta.abs() > config.water_level_tolerance
        {
            water_delta
        } else {
            0.0
        };

        let temp = if config.enable_temp_sensor && temp_delta < 0.0 {
            temp_delta
        } else {
            0.0
        };

        Self {
            ec,
            ph,
            water,
            temp,
        }
    }

    fn is_empty(&self) -> bool {
        self.ec == 0.0 && self.ph == 0.0 && self.water == 0.0 && self.temp == 0.0
    }
}

/// Cold path: Học máy chưa ấm, dùng hằng số config tĩnh làm dự phòng.
pub struct ColdPathSolver;

impl SolverStrategy for ColdPathSolver {
    fn solve(
        &self,
        sensors: &SensorData,
        config: &ControllerConfig,
        ctx: &SystemContext,
    ) -> SolveResult {
        let deltas = SafeStateDeltas::compute(sensors, config, ctx);

        if deltas.ec == 0.0 && deltas.ph == 0.0 && deltas.water == 0.0 {
            return SolveResult::Idle;
        }

        let mut control = ControlVector::default();

        // 1. Tính toán châm phân (Nutrient A/B)
        if config.enable_ec_sensor && deltas.ec > 0.0 {
            let gain = ctx
                .tuner
                .gain_learner
                .effective_ec_gain(config.ec_gain_per_ml)
                .max(0.0001);

            let (ec_step, _) = extract_step_ratios(ctx);
            let ml = (deltas.ec / gain * ec_step).clamp(0.0, config.max_dose_per_cycle);

            control.nutrient_a_ml = ml;
            control.nutrient_b_ml = ml;
        }

        // 2. Tính toán điều chỉnh pH (pH Up / Down)
        if config.enable_ph_sensor && deltas.ph.abs() > 0.0 {
            let is_up = deltas.ph > 0.0;
            let gain = if is_up {
                ctx.tuner
                    .gain_learner
                    .effective_ph_up_gain(config.ph_shift_up_per_ml)
            } else {
                ctx.tuner
                    .gain_learner
                    .effective_ph_down_gain(config.ph_shift_down_per_ml)
            }
            .max(0.0001);

            let (_, ph_step) = extract_step_ratios(ctx);
            let ml = (deltas.ph.abs() / gain * ph_step).clamp(0.0, config.max_dose_per_cycle);

            if is_up {
                control.ph_up_ml = ml;
            } else {
                control.ph_down_ml = ml;
            }
        }

        // 3. Tính toán xả / cấp nước
        if config.enable_water_level_sensor {
            if deltas.water > 0.0 {
                control.water_in_sec =
                    (deltas.water / 0.1).clamp(0.0, config.max_refill_duration_sec as f32);
            } else if deltas.water < 0.0 && config.auto_drain_overflow {
                control.water_out_sec =
                    (deltas.water.abs() / 0.1).clamp(0.0, config.max_drain_duration_sec as f32);
            }
        }

        finalize_solve_result(control, config)
    }
}

/// Warm path: Ma trận đã hội tụ, dùng Moore-Penrose giả nghịch đảo.
pub struct WarmPathSolver;

impl SolverStrategy for WarmPathSolver {
    fn solve(
        &self,
        sensors: &SensorData,
        config: &ControllerConfig,
        ctx: &SystemContext,
    ) -> SolveResult {
        let deltas = SafeStateDeltas::compute(sensors, config, ctx);

        if deltas.is_empty() {
            return SolveResult::Idle;
        }

        let target_error = StateDeltaVector {
            ec_delta: deltas.ec,
            ph_delta: deltas.ph,
            water_level_delta: deltas.water,
            temp_delta: deltas.temp,
        };

        let mut control = match ctx.tuner.interaction_matrix.solve(&target_error) {
            Some(c) => c,
            None => return SolveResult::Idle,
        };

        // 1. Áp dụng step ratio từ AutoTuner & clamp giới hạn
        let (ec_step, ph_step) = extract_step_ratios(ctx);

        control.nutrient_a_ml = (control.nutrient_a_ml * ec_step).min(config.max_dose_per_cycle);
        control.nutrient_b_ml = (control.nutrient_b_ml * ec_step).min(config.max_dose_per_cycle);
        control.ph_up_ml = (control.ph_up_ml * ph_step).min(config.max_dose_per_cycle);
        control.ph_down_ml = (control.ph_down_ml * ph_step).min(config.max_dose_per_cycle);
        control.water_in_sec = control
            .water_in_sec
            .min(config.max_refill_duration_sec as f32);
        control.water_out_sec = control
            .water_out_sec
            .min(config.max_drain_duration_sec as f32);

        // 2. Tắt các kênh không bật cảm biến tương ứng
        if !config.enable_ec_sensor {
            control.nutrient_a_ml = 0.0;
            control.nutrient_b_ml = 0.0;
        }
        if !config.enable_ph_sensor {
            control.ph_up_ml = 0.0;
            control.ph_down_ml = 0.0;
        }
        if !config.enable_water_level_sensor {
            control.water_in_sec = 0.0;
            control.water_out_sec = 0.0;
        }

        // 3. Safety guardrails
        apply_safety_guardrails(
            &mut control,
            sensors.ec,
            sensors.ph,
            sensors.water_level,
            config,
        );

        // 4. Kiểm tra xem sau guardrails có lệnh nào được thực thi không
        if is_control_zero(&control) {
            return SolveResult::Idle;
        }

        finalize_solve_result(control, config)
    }
}

/// Factory: chọn solver phù hợp dựa vào trạng thái matrix của ctx.
pub fn select_solver(ctx: &SystemContext) -> &'static dyn SolverStrategy {
    if ctx.tuner.matrix_is_warm {
        &WarmPathSolver
    } else {
        &ColdPathSolver
    }
}

// --- Internal Helper Functions ---

#[inline]
fn extract_step_ratios(ctx: &SystemContext) -> (f32, f32) {
    if ctx.tuner.is_locked() {
        (ctx.tuner.best_ec_ratio, ctx.tuner.best_ph_ratio)
    } else {
        (ctx.tuner.active_ec_ratio(), ctx.tuner.adaptive_ph_ratio)
    }
}

#[inline]
fn is_control_zero(control: &ControlVector) -> bool {
    control.nutrient_a_ml == 0.0
        && control.nutrient_b_ml == 0.0
        && control.ph_up_ml == 0.0
        && control.ph_down_ml == 0.0
        && control.water_in_sec == 0.0
        && control.water_out_sec == 0.0
        && control.misting_sec == 0.0
}

#[inline]
fn finalize_solve_result(control: ControlVector, config: &ControllerConfig) -> SolveResult {
    SolveResult::Execute {
        control,
        target_ec: config.ec_target,
        target_ph: config.ph_target,
        pwm: config.dosing_pwm_percent as u32,
    }
}

fn effective_ec_tolerance(config: &ControllerConfig, ctx: &SystemContext) -> f32 {
    ctx.tuner.effective_ec_tolerance(config.ec_tolerance)
}

fn effective_ph_tolerance(config: &ControllerConfig, ctx: &SystemContext) -> f32 {
    ctx.tuner.effective_ph_tolerance(config.ph_tolerance)
}

#[cfg(test)]
mod tests {
    use hydragrow_shared::{PumpStatus, SensorData};

    fn sensor(ec: f32, ph: f32, water_level: f32) -> SensorData {
        SensorData {
            device_id: "device_001".to_string(),
            ec: ec,
            ph,
            temp: 25.0,
            water_level,
            pump_status: PumpStatus::default(),
            time: "2026-05-29T00:00:00Z".to_string(),
            controller_received_ms: Some(1_000),
            rssi: None,
            free_heap: None,
            uptime: None,
            err_water: None,
            err_temp: None,
            err_ph: None,
            err_ec: None,
            is_continuous: None,
            ph_voltage_mv: None,
        }
    }

    #[test]
    fn cold_solver_uses_adaptive_ec_tolerance_to_skip_small_chatter_dose() {
        let mut config = ControllerConfig {
            enable_ec_sensor: true,
            enable_ph_sensor: false,
            enable_water_level_sensor: false,
            ec_target: 1.20,
            ec_tolerance: 0.05,
            ..ControllerConfig::default()
        };
        config.control_mode = hydragrow_shared::ControlMode::Auto;

        let sensors = sensor(1.13, 6.0, 20.0);
        let mut ctx = SystemContext::default();
        ctx.tuner.state = TunerState::Exploring;
        ctx.tuner.ec_tracker.oscillation = 1.0;

        let result = ColdPathSolver.solve(&sensors, &config, &ctx);

        assert!(matches!(result, SolveResult::Idle));
    }

    #[test]
    fn adaptive_solver_tolerance_keeps_configured_tolerance_when_stable() {
        let tolerance = adaptive_solver_tolerance(0.05, TunerState::Stable, 0.0);

        assert!((tolerance - 0.05).abs() < f32::EPSILON);
    }
}