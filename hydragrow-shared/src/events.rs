// hydragrow-shared/src/events.rs — version mới
use crate::telemetry::cycle::{DosingCycleEvent, WaterCycleEvent};
use crate::telemetry::health::DeviceHealthSnapshot;
use crate::telemetry::transition::FsmTransitionEvent;
use crate::{AlertMessage, DosingReportPayload, PumpStatus, SensorData};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatusPayload {
    pub device_id: String,
    pub is_online: bool,
}

/// Legacy — giữ lại để không break backend handler cũ
/// Dùng AppEvent::FsmTransition cho code mới
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsmTransitionPayload {
    pub device_id: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pump_status: Option<PumpStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterEventPayload {
    pub device_id: String,
    pub trigger: String,
    pub level_before: f32,
    pub level_after: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationUpdatePayload {
    pub device_id: String,
    pub parameter: String,
    pub new_value: f32,
}

/// AppEvent enum — internal bus giữa MQTT handler và WebSocket broadcaster
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum AppEvent {
    // --- Legacy variants (giữ lại để không break existing handler) ---
    SensorUpdate(SensorData),
    DeviceStatus(DeviceStatusPayload),
    /// Legacy — dùng FsmTransition (typed) cho code mới
    FsmTransitionLegacy(FsmTransitionPayload),
    /// Legacy dosing report — dùng DosingCycle cho code mới
    DosingReport(DosingReportPayload),
    SystemAlert(AlertMessage),
    WaterEvent(WaterEventPayload),
    CalibrationUpdate(CalibrationUpdatePayload),

    // --- New typed variants ---
    /// FSM phase transition với đầy đủ context
    FsmTransition(FsmTransitionEvent),
    /// Chu kỳ MIMO hoàn chỉnh
    DosingCycle(DosingCycleEvent),
    /// Chu kỳ cấp/xả nước hoàn chỉnh
    WaterCycle(WaterCycleEvent),
    /// Snapshot sức khỏe thiết bị
    HealthSnapshot(DeviceHealthSnapshot),
}

