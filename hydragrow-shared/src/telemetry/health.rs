// hydragrow-shared/src/telemetry/health.rs
use crate::hestia::HestiaAssessment;
use serde::{Deserialize, Serialize};

/// Độ tự tin của từng trục Kalman (0.0 - 1.0)
/// Ánh xạ với 8 cột của InteractionMatrix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KalmanConfidence {
    pub nutrient_a: f32,
    pub nutrient_b: f32,
    pub ph_up: f32,
    pub ph_down: f32,
    pub water_in: f32,
    pub water_out: f32,
    pub osaka_mixing: f32,
    pub misting: f32,
}

/// Snapshot sức khỏe thiết bị tổng hợp
/// Topic: `AGITECH/{device_id}/controller/status`
/// Gửi mỗi 10 giây (hoặc khi force_sync)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHealthSnapshot {
    pub device_id: String,
    pub free_heap: u32,
    pub uptime_sec: u64,
    pub rssi: i8,
    /// Điểm sức khỏe tổng hợp từ LocalHealthAndDiagnostic (0-100)
    pub health_score_percent: u32,
    /// Display string của FSM phase hiện tại (để backward compat với frontend cũ)
    pub fsm_state_display: String,
    pub log_drop_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kalman_confidence: Option<KalmanConfidence>,
    pub matrix_update_count: u32,
    pub matrix_is_warm: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hestia: Option<HestiaAssessment>,
    pub timestamp_ms: u64,
}
