use crate::fsm::utils::soft_deadband_scale;

pub fn apply_deadband(delta: f32, tolerance: f32) -> f32 {
    soft_deadband_scale(delta.max(0.0), tolerance.max(0.0))
}

pub fn confidence_from_error_ratio(target: f32, predicted: f32) -> f32 {
    let denom = target.abs().max(0.0001);
    (1.0 - ((target - predicted).abs() / denom)).clamp(0.0, 1.0)
}
