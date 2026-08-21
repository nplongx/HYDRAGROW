// src/core/adaptive/solver.rs
use hydragrow_shared::{ControllerConfig, SensorData};

use crate::core::{
    adaptive::matrix::{ControlVector, StateDeltaVector},
    fsm::SystemContext,
    optimizer::apply_safety_guardrails,
};

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

pub trait SolverStrategy {
    fn solve(
        &self,
        sensors: &SensorData,
        config: &ControllerConfig,
        ctx: &SystemContext,
    ) -> SolveResult;
}

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

/// Helper để phân bổ liều EC tuân thủ tuyệt đối tỉ lệ A:B từ Recipe
fn apply_ab_ratio(
    target_ec_delta: f32,
    gain_a: f32,
    gain_b: f32,
    ratio_a: f32,
    ratio_b: f32,
    step_a: f32,
    step_b: f32,
) -> (f32, f32) {
    let safe_ratio_a = ratio_a.max(0.0);
    let safe_ratio_b = ratio_b.max(0.0);

    // Fallback về 1:1 nếu công thức cấu hình lỗi (cả 2 = 0)
    let (r_a, r_b) = if safe_ratio_a == 0.0 && safe_ratio_b == 0.0 {
        (1.0, 1.0)
    } else {
        (safe_ratio_a, safe_ratio_b)
    };

    // Bắt buộc dùng chung một step_ratio (chọn min để hội tụ an toàn) để không phá vỡ tỉ lệ
    let unified_step = step_a.min(step_b);
    let combined_gain = r_a * gain_a + r_b * gain_b;

    if combined_gain <= 0.0 {
        return (0.0, 0.0);
    }

    let base_u = target_ec_delta / combined_gain;
    (base_u * r_a * unified_step, base_u * r_b * unified_step)
}

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

        if config.enable_ec_sensor && deltas.ec > 0.0 {
            let gain_a = ctx
                .tuner
                .gain_learner
                .effective_ec_a_gain(config.ec_gain_per_ml)
                .max(0.0001);
            let gain_b = ctx
                .tuner
                .gain_learner
                .effective_ec_b_gain(config.ec_gain_per_ml)
                .max(0.0001);
            let (ec_a_step, ec_b_step, _, _) = extract_step_ratios(ctx);

            let (dose_a, dose_b) = apply_ab_ratio(
                deltas.ec,
                gain_a,
                gain_b,
                config.nutrient_a_ratio,
                config.nutrient_b_ratio,
                ec_a_step,
                ec_b_step,
            );
            control.nutrient_a_ml = dose_a;
            control.nutrient_b_ml = dose_b;
        }

        if config.enable_ph_sensor && deltas.ph.abs() > 0.0 {
            let is_up = deltas.ph > 0.0;
            let (_, _, ph_up_step, ph_down_step) = extract_step_ratios(ctx);

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

        if config.enable_water_level_sensor {
            if deltas.water > 0.0 {
                control.water_in_sec =
                    (deltas.water / 0.1).clamp(0.0, config.max_refill_duration_sec as f32);
            } else if deltas.water < 0.0 && config.auto_drain_overflow {
                control.water_out_sec =
                    (deltas.water.abs() / 0.1).clamp(0.0, config.max_drain_duration_sec as f32);
            }
        }

        apply_safety_guardrails(
            &mut control,
            sensors.ec,
            sensors.ph,
            sensors.water_level,
            config,
            ctx.tuner
                .gain_learner
                .effective_ec_a_gain(config.ec_gain_per_ml),
            ctx.tuner
                .gain_learner
                .effective_ec_b_gain(config.ec_gain_per_ml),
        );

        if is_control_zero(&control) {
            return SolveResult::Idle;
        }

        finalize_solve_result(control, config)
    }
}

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

        let (ec_a_step, ec_b_step, ph_up_step, ph_down_step) = extract_step_ratios(ctx);

        // [VÁ BUG]: Khóa cứng output của Ma trận MIMO để tuân thủ tỉ lệ A:B
        if config.enable_ec_sensor {
            let gain_a = ctx
                .tuner
                .gain_learner
                .effective_ec_a_gain(config.ec_gain_per_ml);
            let gain_b = ctx
                .tuner
                .gain_learner
                .effective_ec_b_gain(config.ec_gain_per_ml);

            // 1. Tính tổng EC mà Solver thực sự muốn tăng
            let raw_intended_ec = control.nutrient_a_ml * gain_a + control.nutrient_b_ml * gain_b;

            if raw_intended_ec > 0.0 {
                // 2. Phân bổ lại tổng EC đó theo đúng tỉ lệ recipe
                let (dose_a, dose_b) = apply_ab_ratio(
                    raw_intended_ec,
                    gain_a,
                    gain_b,
                    config.nutrient_a_ratio,
                    config.nutrient_b_ratio,
                    ec_a_step,
                    ec_b_step,
                );
                control.nutrient_a_ml = dose_a;
                control.nutrient_b_ml = dose_b;
            } else {
                control.nutrient_a_ml = 0.0;
                control.nutrient_b_ml = 0.0;
            }
        } else {
            control.nutrient_a_ml = 0.0;
            control.nutrient_b_ml = 0.0;
        }

        // pH giữ nguyên việc nhân step
        control.ph_up_ml *= ph_up_step;
        control.ph_down_ml *= ph_down_step;

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

        apply_safety_guardrails(
            &mut control,
            sensors.ec,
            sensors.ph,
            sensors.water_level,
            config,
            ctx.tuner
                .gain_learner
                .effective_ec_a_gain(config.ec_gain_per_ml),
            ctx.tuner
                .gain_learner
                .effective_ec_b_gain(config.ec_gain_per_ml),
        );

        if is_control_zero(&control) {
            return SolveResult::Idle;
        }

        finalize_solve_result(control, config)
    }
}

pub fn select_solver(ctx: &SystemContext) -> &'static dyn SolverStrategy {
    if ctx.tuner.matrix_is_warm {
        &WarmPathSolver
    } else {
        &ColdPathSolver
    }
}

#[inline]
fn extract_step_ratios(ctx: &SystemContext) -> (f32, f32, f32, f32) {
    if ctx.tuner.is_locked() {
        (
            ctx.tuner.best_ec_a_ratio,
            ctx.tuner.best_ec_b_ratio,
            ctx.tuner.best_ph_up_ratio,
            ctx.tuner.best_ph_down_ratio,
        )
    } else {
        (
            ctx.tuner.active_ec_a_ratio(),
            ctx.tuner.active_ec_b_ratio(),
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
