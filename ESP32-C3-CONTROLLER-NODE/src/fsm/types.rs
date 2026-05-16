use crate::mqtt::SensorData;
use std::sync::{Arc, RwLock};

pub type SharedSensorData = Arc<RwLock<SensorData>>;

// ---------------------------------------------------------------------------
// PendingDose – mô tả hành động bơm sẽ thực hiện sau khi Osaka khởi động xong
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub enum PendingDose {
    EC {
        dose_ml: f32,
        target_ec: f32,
        pwm_percent: u32,
    },
    PH {
        is_up: bool,
        dose_ml: f32,
        target_ph: f32,
        pwm_percent: u32,
    },
}

// ---------------------------------------------------------------------------
// PendingCalibrationSample – dữ liệu chờ cập nhật EMA sau mỗi chu kỳ bơm
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct PendingCalibrationSample {
    pub cycle_id: String, // UUID cho chu trình
    pub trigger: String,  // "auto_ec", "auto_ph", "scheduled"
    pub start_ec: f32,
    pub start_ph: f32,
    pub start_water_level: f32,
    pub target_ec: f32,
    pub target_ph: f32,
    pub dose_a_ml: f32,       // Đổi tên từ pump_a_ml để rõ nghĩa
    pub dose_b_ml: f32,       // Đổi tên từ pump_b_ml
    pub dose_ph_up_ml: f32,   // Đổi tên từ ph_up_ml
    pub dose_ph_down_ml: f32, // Đổi tên từ ph_down_ml
    pub post_mixing_ec: f32,
    pub post_mixing_ph: f32,
    pub start_ms: u64, // Thời gian bắt đầu chu trình (thay cho active_mixing_start_ms)
    pub active_mixing_finish_ms: u64,
    pub stabilizing_start_ms: Option<u64>,
    pub stabilizing_finish_ms: Option<u64>,
    pub invalid_by_noise: bool,
    pub invalid_by_water_change: bool,
}
