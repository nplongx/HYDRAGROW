use serde::{Deserialize, Serialize};

use crate::{AlertMessage, DosingReportPayload, PumpStatus, SensorData};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatusPayload {
    pub device_id: String,
    pub is_online: bool,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum AppEvent {
    SensorUpdate(SensorData),
    DeviceStatus(DeviceStatusPayload),
    FsmTransition(FsmTransitionPayload),
    DosingReport(DosingReportPayload),
    SystemAlert(AlertMessage),
    WaterEvent(WaterEventPayload),
    CalibrationUpdate(CalibrationUpdatePayload),
}
