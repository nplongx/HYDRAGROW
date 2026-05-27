// hydragrow-shared/src/telemetry/cycle.rs
use serde::{Deserialize, Serialize};

/// Snapshot cảm biến tại một thời điểm trong chu kỳ (pre / post-mixing / post-stable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DosingPhaseSnapshot {
    pub ec: f32,
    pub ph: f32,
    pub water_level: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temp: Option<f32>,
}

/// Khối lượng thực tế đã bơm trong chu kỳ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DosingDoseRecord {
    pub pump_a_ml: f32,
    pub pump_b_ml: f32,
    pub ph_up_ml: f32,
    pub ph_down_ml: f32,
    pub water_in_sec: f32,
    pub water_out_sec: f32,
}

/// Kết quả chu kỳ — được backend dùng để tính success rate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum CycleOutcome {
    /// EC và pH đều đạt target trong tolerance
    Success,
    /// Chỉ một trong hai đạt
    PartialSuccess { ec_reached: bool, ph_reached: bool },
    /// Timeout cứng — phase bị ép thoát
    Timeout,
    /// Lỗi phần cứng trong khi châm
    HardwareFault { fault_code: String },
}

/// Thông số học từ Kalman filter (optional — chỉ có khi firmware bật adaptive learning)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KalmanLearningData {
    pub ec_gain_before: f32,
    pub ec_gain_after: f32,
    pub ph_up_gain_before: f32,
    pub ph_up_gain_after: f32,
    pub ph_down_gain_before: f32,
    pub ph_down_gain_after: f32,
    pub matrix_update_count: u32,
    pub matrix_is_warm: bool,
    pub adaptive_mixing_sec: u32,
    pub adaptive_stabilize_sec: u32,
}

/// Canonical record cho một chu kỳ MIMO hoàn chỉnh
/// Topic: `AGITECH/{device_id}/dosing_report`
/// Thay thế `DosingReportPayload` cũ — chứa nhiều context hơn và có computed properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DosingCycleEvent {
    pub cycle_id: String,
    pub device_id: String,
    /// Nguồn kích hoạt: "auto_mimo", "scheduled", "manual", "water_only"
    pub trigger: String,
    pub pre: DosingPhaseSnapshot,
    pub post_mixing: DosingPhaseSnapshot,
    pub post_stable: DosingPhaseSnapshot,
    pub target_ec: f32,
    pub target_ph: f32,
    pub dose: DosingDoseRecord,
    pub outcome: CycleOutcome,
    /// Thời gian tổng từ khi bắt đầu dosing đến khi phase Stabilizing kết thúc (ms)
    pub duration_ms: u64,
    /// Thời gian thực tế ở ActiveMixing (ms)
    pub mixing_duration_ms: u64,
    /// Thời gian thực tế ở Stabilizing (ms)
    pub stabilize_duration_ms: u64,
    pub timestamp_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kalman: Option<KalmanLearningData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season_id: Option<String>,
}

impl DosingCycleEvent {
    /// Sai số EC tại thời điểm bão hòa = target - post_stable.ec
    pub fn error_ec(&self) -> f32 {
        self.target_ec - self.post_stable.ec
    }

    /// Sai số pH tại thời điểm bão hòa = target - post_stable.ph
    pub fn error_ph(&self) -> f32 {
        self.target_ph - self.post_stable.ph
    }

    /// Biến thiên EC = post_stable.ec - pre.ec
    pub fn delta_ec(&self) -> f32 {
        self.post_stable.ec - self.pre.ec
    }

    /// Biến thiên pH = post_stable.ph - pre.ph
    pub fn delta_ph(&self) -> f32 {
        self.post_stable.ph - self.pre.ph
    }

    /// Tổng ml dinh dưỡng đã châm
    pub fn total_nutrient_ml(&self) -> f32 {
        self.dose.pump_a_ml + self.dose.pump_b_ml
    }

    /// Tổng ml hóa chất pH đã châm
    pub fn total_ph_ml(&self) -> f32 {
        self.dose.ph_up_ml + self.dose.ph_down_ml
    }
}

/// Hướng bơm nước
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WaterDirection {
    In,
    Out,
}

/// Canonical record cho một chu kỳ cấp/xả nước
/// Topic: `AGITECH/{device_id}/water_event`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterCycleEvent {
    pub cycle_id: String,
    pub device_id: String,
    pub direction: WaterDirection,
    pub level_before: f32,
    pub level_after: f32,
    pub target_level: f32,
    pub duration_sec: u64,
    pub success: bool,
    /// "auto_refill", "scheduled_change", "dilute", "manual"
    pub trigger: String,
    pub timestamp_ms: u64,
}
