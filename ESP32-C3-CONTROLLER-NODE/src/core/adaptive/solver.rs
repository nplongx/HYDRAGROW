// src/core/adaptive/solver.rs
//! SolverStrategy — Trừu tượng hóa thuật toán tính ControlVector.
//! Cold path dùng hằng số config tĩnh kết hợp AutoTuner Step Ratio.
//! Warm path dùng Moore-Penrose giả nghịch đảo từ ma trận tương tác MIMO.

use hydragrow_shared::{ControllerConfig, SensorData};

use crate::core::{
    adaptive::matrix::{ControlVector, StateDeltaVector},
    fsm::SystemContext,
    optimizer::apply_safety_guardrails,
};

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

/// Cold path: Học máy chưa ấm, dùng hằng số config tĩnh kết hợp AutoTuner Step Ratio.
pub struct ColdPathSolver;

impl SolverStrategy for ColdPathSolver {
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

        let mut control = ControlVector::default();

        // 1. Tính toán châm dinh dưỡng (Nutrient A/B) — Chia đều 50/50 cho 2 bơm
        if config.enable_ec_sensor && deltas.ec > 0.0 {
            let gain = ctx
                .tuner
                .gain_learner
                .effective_ec_gain(config.ec_gain_per_ml)
                .max(0.0001);

            let (ec_step, _, _) = extract_step_ratios(ctx);
            // Tổng thể tích phân cần nạp vào bể
            let total_nutrient_ml = deltas.ec / gain * ec_step;
            // Chia đều cho từng bình A và B
            let per_pump_ml = (total_nutrient_ml / 2.0).clamp(0.0, config.max_dose_per_cycle);

            control.nutrient_a_ml = per_pump_ml;
            control.nutrient_b_ml = per_pump_ml;
        }

        // 2. Tính toán điều chỉnh pH (pH Up / Down) tách biệt độc lập
        if config.enable_ph_sensor && deltas.ph.abs() > 0.0 {
            let is_up = deltas.ph > 0.0;
            let (_, ph_up_step, ph_down_step) = extract_step_ratios(ctx);

            if is_up {
                let gain = ctx
                    .tuner
                    .gain_learner
                    .effective_ph_up_gain(config.ph_shift_up_per_ml)
                    .max(0.0001);

                control.ph_up_ml =
                    (deltas.ph / gain * ph_up_step).clamp(0.0, config.max_dose_per_cycle);
            } else {
                let gain = ctx
                    .tuner
                    .gain_learner
                    .effective_ph_down_gain(config.ph_shift_down_per_ml)
                    .max(0.0001);

                control.ph_down_ml =
                    (deltas.ph.abs() / gain * ph_down_step).clamp(0.0, config.max_dose_per_cycle);
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

        // 4. Áp dụng Safety Guardrails kiểm tra độc tính, tràn bồn và sốc môi trường
        apply_safety_guardrails(
            &mut control,
            sensors.ec,
            sensors.ph,
            sensors.water_level,
            config,
        );

        if is_control_zero(&control) {
            return SolveResult::Idle;
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

        // 1. Áp dụng step ratio từ AutoTuner
        let (ec_step, ph_up_step, ph_down_step) = extract_step_ratios(ctx);
        control.nutrient_a_ml *= ec_step;
        control.nutrient_b_ml *= ec_step;
        control.ph_up_ml *= ph_up_step;
        control.ph_down_ml *= ph_down_step;

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
fn extract_step_ratios(ctx: &SystemContext) -> (f32, f32, f32) {
    if ctx.tuner.is_locked() {
        (
            ctx.tuner.best_ec_ratio,
            ctx.tuner.best_ph_up_ratio,
            ctx.tuner.best_ph_down_ratio,
        )
    } else {
        (
            ctx.tuner.active_ec_ratio(),
            ctx.tuner.active_ph_up_ratio(),
            ctx.tuner.active_ph_down_ratio(),
        )
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
    use super::*;
    use hydragrow_shared::{PumpStatus, SensorData};

    fn sensor(ec: f32, ph: f32, water_level: f32) -> SensorData {
        SensorData {
            device_id: "device_001".to_string(),
            ec,
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
    fn cold_solver_splits_nutrient_equally_between_a_and_b() {
        let mut config = ControllerConfig {
            enable_ec_sensor: true,
            enable_ph_sensor: false,
            enable_water_level_sensor: false,
            ec_target: 1.60,
            ec_tolerance: 0.05,
            ec_gain_per_ml: 0.02,
            max_dose_per_cycle: 50.0,
            ..ControllerConfig::default()
        };
        config.control_mode = hydragrow_shared::ControlMode::Auto;

        let sensors = sensor(1.20, 6.0, 20.0);
        let mut ctx = SystemContext::default();
        ctx.tuner.adaptive_ec_ratio = 0.40;

        let result = ColdPathSolver.solve(&sensors, &config, &ctx);
        match result {
            SolveResult::Execute { control, .. } => {
                // delta = 0.40, gain = 0.02 -> total = 20ml * 0.40 = 8.0ml -> per pump = 4.0ml
                assert!((control.nutrient_a_ml - 4.0).abs() < 1e-3);
                assert!((control.nutrient_b_ml - 4.0).abs() < 1e-3);
            }
            _ => panic!("Expected Execute result"),
        }
    }
}
