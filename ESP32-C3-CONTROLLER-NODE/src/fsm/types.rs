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
    ScheduledDose {
        dose_a_ml: f32,
        dose_b_ml: f32,
        pwm_percent: u32,
    },
}

// ---------------------------------------------------------------------------
// SystemState – trạng thái FSM chính
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub enum SystemState {
    SystemBooting,
    ManualMode,
    DosingCycleComplete,
    Cooldown {
        finish_time: u64,
    },
    Monitoring,
    EmergencyStop(String),
    SystemFault(String),
    SensorCalibration {
        step: String,
        finish_time: u64,
    },
    WaterRefilling {
        target_level: f32,
        start_time: u64,
    },
    WaterDraining {
        target_level: f32,
        start_time: u64,
    },
    StartingOsakaPump {
        finish_time: u64,
        pending_action: PendingDose,
    },
    DosingPumpA {
        next_toggle_time: u64,
        dose_target_ml: f32,
        delivered_ml_est: f32,
        dose_b_ml: f32,
        pulse_on: bool,
        pulse_count: u32,
        max_pulse_count: u32,
        pulse_on_ms: u64,
        pulse_off_ms: u64,
        pwm_percent: u32,
        active_capacity_ml_per_sec: f32,
        target_ec: f32,
        start_ec: f32,
        start_ph: f32,
    },
    WaitingBetweenDose {
        finish_time: u64,
        dose_b_ml: f32,
        target_ec: f32,
        start_ec: f32,
        start_ph: f32,
        dose_a_ml_reported: f32,
    },
    DosingPumpB {
        next_toggle_time: u64,
        dose_target_ml: f32,
        delivered_ml_est: f32,
        pulse_on: bool,
        pulse_count: u32,
        max_pulse_count: u32,
        pulse_on_ms: u64,
        pulse_off_ms: u64,
        pwm_percent: u32,
        active_capacity_ml_per_sec: f32,
        target_ec: f32,
        start_ec: f32,
        start_ph: f32,
        dose_a_ml_reported: f32,
    },
    DosingPH {
        next_toggle_time: u64,
        is_up: bool,
        dose_target_ml: f32,
        delivered_ml_est: f32,
        pulse_on: bool,
        pulse_count: u32,
        max_pulse_count: u32,
        pulse_on_ms: u64,
        pulse_off_ms: u64,
        pwm_percent: u32,
        active_capacity_ml_per_sec: f32,
        target_ph: f32,
        start_ec: f32,
        start_ph: f32,
    },
    ActiveMixing {
        finish_time: u64,
    },
    Stabilizing {
        finish_time: u64,
    },
}

impl SystemState {
    pub fn to_payload_string(&self) -> String {
        match self {
            SystemState::SystemBooting => "SystemBooting".to_string(),
            SystemState::ManualMode => "ManualMode".to_string(),
            SystemState::DosingCycleComplete => "DosingCycleComplete".to_string(),
            SystemState::Cooldown { finish_time } => format!("Cooldown:{}", finish_time),
            SystemState::Monitoring => "Monitoring".to_string(),
            SystemState::EmergencyStop(reason) => format!("EmergencyStop:{}", reason),
            SystemState::SystemFault(reason) => format!("SystemFault:{}", reason),
            SystemState::SensorCalibration { step, .. } => format!("SensorCalibration:{}", step),
            SystemState::WaterRefilling { .. } => "WaterRefilling".to_string(),
            SystemState::WaterDraining { .. } => "WaterDraining".to_string(),
            SystemState::DosingPumpA { .. } => "DosingPumpA".to_string(),
            SystemState::WaitingBetweenDose { .. } => "WaitingBetweenDose".to_string(),
            SystemState::DosingPumpB { .. } => "DosingPumpB".to_string(),
            SystemState::DosingPH { .. } => "DosingPH".to_string(),
            SystemState::StartingOsakaPump { .. } => "StartingOsakaPump".to_string(),
            SystemState::ActiveMixing { .. } => "ActiveMixing".to_string(),
            SystemState::Stabilizing { .. } => "Stabilizing".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// PendingCalibrationSample – dữ liệu chờ cập nhật EMA sau mỗi chu kỳ bơm
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct PendingCalibrationSample {
    pub start_ec: f32,
    pub start_ph: f32,
    pub pump_a_ml: f32,
    pub pump_b_ml: f32,
    pub ph_up_ml: f32,
    pub ph_down_ml: f32,
    pub active_mixing_start_ms: u64,
    pub active_mixing_finish_ms: u64,
    pub stabilizing_start_ms: Option<u64>,
    pub stabilizing_finish_ms: Option<u64>,
    pub invalid_by_noise: bool,
    pub invalid_by_water_change: bool,
}
