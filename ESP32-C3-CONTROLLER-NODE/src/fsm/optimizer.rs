use crate::fsm::matrix::InteractionMatrix;
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
        // Thay thế 3 ma trận rời rạc bằng ma trận tương tác hệ thống chính xác của bạn
        interaction_matrix: &InteractionMatrix,
        // Đưa step_ratio và tolerance trực tiếp từ cấu trúc điều khiển của bạn vào
        step_ratio_ec: f32,
        step_ratio_ph: f32,
        max_dose: f32,
    ) -> (OptimizationResult, OptimizationResult, OptimizationResult) {
        // 1. Trích xuất gain EC từ ma trận tương tác (Hàng 0, Cột 0 cho Pump A)
        let ec_gain = interaction_matrix.get(0, 0).max(0.0001);
        let step_ratio_ec = step_ratio_ec.max(0.0);
        let dose_a_ml = (target_delta_ec / ec_gain * step_ratio_ec).clamp(0.0, max_dose);

        // 2. Tính toán liều lượng cho giếng B dựa theo tỷ lệ công suất thiết kế bơm
        let pump_a_power = config.pump_a_capacity_ml_per_sec.max(0.0001);
        let pump_b_power = config.pump_b_capacity_ml_per_sec.max(0.0001);
        let pump_ratio_ab = pump_b_power / pump_a_power;
        let dose_b_ml = (dose_a_ml * pump_ratio_ab).clamp(0.0, max_dose);

        let predicted_ec = (dose_a_ml * ec_gain).max(0.0);

        // 3. Tính toán độ lệch pH sinh ra (Coupling effect) do việc châm dinh dưỡng A và B gây ra
        // Hàng 1 (pH) - Cột 0 (tác động của A), Cột 1 (tác động của B)
        let coupling_ph_delta =
            (dose_a_ml * interaction_matrix.get(1, 0)) + (dose_b_ml * interaction_matrix.get(1, 1));

        // Tính toán lượng pH delta còn lại cần phải xử lý sau khi đã bị ảnh hưởng chéo
        let residual_ph_delta = target_delta_ph - coupling_ph_delta;

        // 4. Tính toán liều lượng chất điều chỉnh pH (Hàng 1, Cột 2)
        let ph_gain = interaction_matrix.get(1, 2).abs().max(0.0001);
        let step_ratio_ph = step_ratio_ph.max(0.0);

        // Sử dụng cấu hình sai số từ ControllerConfig làm vùng chết (deadband)
        let db_scale = apply_deadband(residual_ph_delta.abs(), config.ph_tolerance);
        let dose_ph_ml =
            (residual_ph_delta.abs() / ph_gain * step_ratio_ph * db_scale).clamp(0.0, max_dose);

        // 5. Đo lường mức độ tin cậy của thuật toán dự báo
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
                predicted_response: dose_b_ml * interaction_matrix.get(0, 1).max(0.0001),
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

