// src/fsm/solver.rs
//! SolverStrategy — Trừu tượng hóa thuật toán tính ControlVector.
//! Cold path dùng hằng số config tĩnh.
//! Warm path dùng Moore-Penrose giả nghịch đảo.

use hydragrow_shared::{ControllerConfig, SensorData};
use crate::fsm::matrix::{ControlVector, StateDeltaVector};
use crate::fsm::optimizer::apply_safety_guardrails;
use crate::fsm::system_context::SystemContext;

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

/// Cold path: Học máy chưa ấm, dùng hằng số config tĩnh làm dự phòng.
pub struct ColdPathSolver;

impl SolverStrategy for ColdPathSolver {
    fn solve(
        &self,
        sensors: &SensorData,
        config: &ControllerConfig,
        ctx: &SystemContext,
    ) -> SolveResult {
        let ec_val = sensors.ec;
        let ph_val = sensors.ph;
        let w_level = sensors.water_level;

        let ec_delta = (config.ec_target - ec_val).max(0.0);
        let ph_delta = config.ph_target - ph_val;
        let water_delta = config.water_level_target - w_level;

        let safe_ec_delta = if config.enable_ec_sensor && ec_delta.abs() > config.ec_tolerance {
            ec_delta
        } else {
            0.0
        };
        let safe_ph_delta = if config.enable_ph_sensor && ph_delta.abs() > config.ph_tolerance {
            ph_delta
        } else {
            0.0
        };
        let safe_water_delta = if config.enable_water_level_sensor
            && water_delta.abs() > config.water_level_tolerance
        {
            water_delta
        } else {
            0.0
        };

        if safe_ec_delta == 0.0 && safe_ph_delta == 0.0 && safe_water_delta == 0.0 {
            return SolveResult::Idle;
        }

        let mut control = ControlVector::default();

        if config.enable_ec_sensor && safe_ec_delta > 0.0 {
            let gain = ctx
                .tuner
                .gain_learner
                .effective_ec_gain(config.ec_gain_per_ml)
                .max(0.0001);
            let step_ratio = if ctx.tuner.is_locked() {
                ctx.tuner.best_ec_ratio
            } else {
                ctx.tuner.active_ec_ratio()
            };
            let ml = (safe_ec_delta / gain * step_ratio).clamp(0.0, config.max_dose_per_cycle);
            control.nutrient_a_ml = ml;
            control.nutrient_b_ml = ml;
        }

        if config.enable_ph_sensor && safe_ph_delta.abs() > 0.0 {
            let is_up = safe_ph_delta > 0.0;
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
            let step_ratio = if ctx.tuner.is_locked() {
                ctx.tuner.best_ph_ratio
            } else {
                ctx.tuner.adaptive_ph_ratio
            };
            let ml =
                (safe_ph_delta.abs() / gain * step_ratio).clamp(0.0, config.max_dose_per_cycle);
            if is_up {
                control.ph_up_ml = ml;
            } else {
                control.ph_down_ml = ml;
            }
        }

        if config.enable_water_level_sensor && safe_water_delta > 0.0 {
            control.water_in_sec =
                (safe_water_delta / 0.1).clamp(0.0, config.max_refill_duration_sec as f32);
        } else if config.enable_water_level_sensor
            && safe_water_delta < 0.0
            && config.auto_drain_overflow
        {
            control.water_out_sec = (safe_water_delta.abs() / 0.1)
                .clamp(0.0, config.max_drain_duration_sec as f32);
        }

        SolveResult::Execute {
            control,
            target_ec: config.ec_target,
            target_ph: config.ph_target,
            pwm: config.dosing_pwm_percent as u32,
        }
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
        let ec_val = sensors.ec;
        let ph_val = sensors.ph;
        let w_level = sensors.water_level;
        let temp_val = sensors.temp;

        let ec_delta = (config.ec_target - ec_val).max(0.0);
        let ph_delta = config.ph_target - ph_val;
        let water_delta = config.water_level_target - w_level;
        let temp_delta = config.misting_temp_threshold - temp_val;

        let safe_ec_delta = if config.enable_ec_sensor && ec_delta.abs() > config.ec_tolerance {
            ec_delta
        } else {
            0.0
        };
        let safe_ph_delta = if config.enable_ph_sensor && ph_delta.abs() > config.ph_tolerance {
            ph_delta
        } else {
            0.0
        };
        let safe_water_delta = if config.enable_water_level_sensor
            && water_delta.abs() > config.water_level_tolerance
        {
            water_delta
        } else {
            0.0
        };
        let safe_temp_delta = if config.enable_temp_sensor && temp_delta < 0.0 {
            temp_delta
        } else {
            0.0
        };

        if safe_ec_delta == 0.0
            && safe_ph_delta == 0.0
            && safe_water_delta == 0.0
            && safe_temp_delta == 0.0
        {
            return SolveResult::Idle;
        }

        let target_error = StateDeltaVector {
            ec_delta: safe_ec_delta,
            ph_delta: safe_ph_delta,
            water_level_delta: safe_water_delta,
            temp_delta: safe_temp_delta,
        };

        let mut control = match ctx.tuner.interaction_matrix.solve(&target_error) {
            Some(c) => c,
            None => return SolveResult::Idle,
        };

        // Áp dụng step ratio từ AutoTuner
        let ec_step = if ctx.tuner.is_locked() {
            ctx.tuner.best_ec_ratio
        } else {
            ctx.tuner.active_ec_ratio()
        };
        let ph_step = if ctx.tuner.is_locked() {
            ctx.tuner.best_ph_ratio
        } else {
            ctx.tuner.adaptive_ph_ratio
        };

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

        // Tắt kênh không có cảm biến
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

        // Safety guardrails
        apply_safety_guardrails(&mut control, ec_val, ph_val, w_level, config);

        // Nếu sau guardrails tất cả về 0 → Idle
        if control.nutrient_a_ml == 0.0
            && control.nutrient_b_ml == 0.0
            && control.ph_up_ml == 0.0
            && control.ph_down_ml == 0.0
            && control.water_in_sec == 0.0
            && control.water_out_sec == 0.0
            && control.misting_sec == 0.0
        {
            return SolveResult::Idle;
        }

        SolveResult::Execute {
            control,
            target_ec: config.ec_target,
            target_ph: config.ph_target,
            pwm: config.dosing_pwm_percent as u32,
        }
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
