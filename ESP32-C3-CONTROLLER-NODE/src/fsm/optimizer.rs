use crate::fsm::matrix::*;
use crate::fsm::utils::soft_deadband_scale;
use hydragrow_shared::ControllerConfig;

#[derive(Debug, Clone, Copy, Default)]
pub struct OptimizationResult {
    pub dose: f32,
    pub predicted_response: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DoseOptimizer;

impl DoseOptimizer {
    #[allow(clippy::too_many_arguments)]
    pub fn solve(
        &self,
        config: &ControllerConfig,
        target_delta_ec: f32,
        target_delta_ph: f32,
        ec_matrix: EcMatrix,
        ph_matrix: PhMatrix,
        coupling: CouplingMatrix,
        max_dose: f32,
    ) -> (OptimizationResult, OptimizationResult, OptimizationResult) {
        let ec_gain = ec_matrix.g_ec_a.max(0.0001);
        let step_ratio_ec = ec_matrix.step_ratio_ec.max(0.0);
        let dose_a_ml = (target_delta_ec / ec_gain * step_ratio_ec).clamp(0.0, max_dose);

        let pump_a_power = config.pump_a_capacity_ml_per_sec.max(0.0001);
        let pump_b_power = config.pump_b_capacity_ml_per_sec.max(0.0001);
        let pump_ratio_ab = pump_b_power / pump_a_power;
        let dose_b_ml = (dose_a_ml * pump_ratio_ab).clamp(0.0, max_dose);

        let predicted_ec = (dose_a_ml * ec_gain).max(0.0);

        let coupling_ph_delta = (dose_a_ml * coupling.ph_per_ml_a) + (dose_b_ml * coupling.ph_per_ml_b);
        let residual_ph_delta = target_delta_ph - coupling_ph_delta;

        let ph_gain = ph_matrix.g_ph_x.max(0.0001);
        let step_ratio_ph = ph_matrix.step_ratio_ph.max(0.0);
        let ph_tolerance = ph_matrix.ph_tolerance.max(0.0);
        let db_scale = apply_deadband(residual_ph_delta.abs(), ph_tolerance);
        let dose_ph_ml = (residual_ph_delta.abs() / ph_gain * step_ratio_ph * db_scale).clamp(0.0, max_dose);

        let conf_ec = confidence_from_delta(target_delta_ec, predicted_ec);
        let predicted_ph = dose_ph_ml * ph_gain;
        let conf_ph = confidence_from_delta(residual_ph_delta.abs(), predicted_ph);

        (
            OptimizationResult {
                dose: dose_a_ml,
                predicted_response: predicted_ec,
                confidence: conf_ec,
            },
            OptimizationResult {
                dose: dose_b_ml,
                predicted_response: dose_b_ml * ec_gain,
                confidence: conf_ec,
            },
            OptimizationResult {
                dose: dose_ph_ml,
                predicted_response: predicted_ph,
                confidence: conf_ph,
            },
        )
    }
}

pub fn apply_deadband(delta: f32, tolerance: f32) -> f32 {
    soft_deadband_scale(delta.max(0.0), tolerance.max(0.0))
}

fn confidence_from_delta(target: f32, predicted: f32) -> f32 {
    let denom = target.abs().max(0.0001);
    (1.0 - ((target - predicted).abs() / denom)).clamp(0.0, 1.0)
}
