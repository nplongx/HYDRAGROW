use std::sync::{Arc, RwLock};

use hydragrow_shared::SensorData;

pub type SharedSensorData = Arc<RwLock<SensorData>>;

// ---------------------------------------------------------------------------
// PendingCalibrationSample – dữ liệu chờ cập nhật EMA sau mỗi chu kỳ bơm
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct PendingCalibrationSample {
    pub cycle_id: String,
    pub trigger: String,
    pub start_ec: f32,
    pub start_ph: f32,
    pub start_water_level: f32,
    pub start_temp: f32,
    pub target_ec: f32,
    pub target_ph: f32,
    pub dose_a_ml: f32,
    pub dose_b_ml: f32,
    pub dose_ph_up_ml: f32,
    pub dose_ph_down_ml: f32,
    pub water_in_sec: f32,
    pub water_out_sec: f32,
    pub post_mixing_ec: f32,
    pub post_mixing_ph: f32,
    pub start_ms: u64,
    pub active_mixing_finish_ms: u64,
    pub stabilizing_start_ms: Option<u64>,
    pub stabilizing_finish_ms: Option<u64>,
    pub invalid_by_noise: bool,
    pub invalid_by_water_change: bool,
}
