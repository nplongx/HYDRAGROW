// hydragrow-shared/src/fsm.rs — version mới hoàn chỉnh
use crate::PumpStatus;
use serde::{Deserialize, Serialize};

/// Phase của FSM — có Serialize/Deserialize để gửi qua MQTT và lưu DB
/// Serde sẽ serialize "Monitoring" -> "Monitoring", "Fault(EcDosingFailed)" -> {"Fault":"EC_DOSING_FAILED"}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SystemPhase {
    Booting,
    Monitoring,
    ManualMode,
    WaterRefilling,
    WaterDraining,
    MimoDosing,
    ActiveMixing,
    Stabilizing,
    Cooldown,
    SensorCalibration,
    Fault(FaultCode),
    EmergencyStop(String),
}

impl SystemPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            SystemPhase::Booting => "Booting",
            SystemPhase::Monitoring => "Monitoring",
            SystemPhase::ManualMode => "ManualMode",
            SystemPhase::WaterRefilling => "WaterRefilling",
            SystemPhase::WaterDraining => "WaterDraining",
            SystemPhase::MimoDosing => "MimoDosing",
            SystemPhase::ActiveMixing => "ActiveMixing",
            SystemPhase::Stabilizing => "Stabilizing",
            SystemPhase::Cooldown => "Cooldown",
            SystemPhase::SensorCalibration => "SensorCalibration",
            SystemPhase::Fault(_) => "Fault",
            SystemPhase::EmergencyStop(_) => "EmergencyStop",
        }
    }

    /// Trả về true nếu phase hoạt động cần Osaka pump chạy
    pub fn requires_mixing(&self) -> bool {
        matches!(
            self,
            Self::MimoDosing | Self::ActiveMixing | Self::Stabilizing
        )
    }

    /// Trả về true nếu là phase lỗi cần dừng toàn bộ actuator
    pub fn is_fault(&self) -> bool {
        matches!(self, Self::Fault(_) | Self::EmergencyStop(_))
    }

    /// Lấy fault code nếu đang ở trạng thái Fault
    pub fn fault_code(&self) -> Option<&FaultCode> {
        match self {
            Self::Fault(code) => Some(code),
            _ => None,
        }
    }
}

impl core::fmt::Display for SystemPhase {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Fault(code) => write!(f, "Fault:{}", code.as_str()),
            Self::EmergencyStop(reason) => write!(f, "EmergencyStop:{}", reason),
            other => write!(f, "{}", other.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FaultCode {
    EcDosingFailed,
    PhDosingFailed,
    WaterRefillFailed,
    WaterDrainFailed,
    TooManyRefills,
    TooManyDrains,
    MaxHourlyDoseEc,
    MaxHourlyDosePh,
    SensorTimeout,
    EcStagnant,
    PhOscillating,
    WaterLevelCritical,
}

impl FaultCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EcDosingFailed => "EC_DOSING_FAILED",
            Self::PhDosingFailed => "PH_DOSING_FAILED",
            Self::WaterRefillFailed => "WATER_REFILL_FAILED",
            Self::WaterDrainFailed => "WATER_DRAIN_FAILED",
            Self::TooManyRefills => "TOO_MANY_REFILLS",
            Self::TooManyDrains => "TOO_MANY_DRAINS",
            Self::MaxHourlyDoseEc => "MAX_HOURLY_DOSE_EC",
            Self::MaxHourlyDosePh => "MAX_HOURLY_DOSE_PH",
            Self::SensorTimeout => "SENSOR_TIMEOUT",
            Self::EcStagnant => "EC_STAGNANT",
            Self::PhOscillating => "PH_OSCILLATING",
            Self::WaterLevelCritical => "WATER_LEVEL_CRITICAL",
        }
    }
}

impl core::fmt::Display for FaultCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Snapshot trạng thái FSM — type-safe, gửi qua MQTT topic `fsm/state`
/// Thay thế `FsmStatePayload` cũ với `current_state: String`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsmSnapshot {
    pub online: bool,
    /// Phase hiện tại — đã typed, backend có thể match trực tiếp
    pub current_phase: SystemPhase,
    /// Phase ngay trước đó — để frontend có thể hiện animation transition
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_phase: Option<SystemPhase>,
    pub pump_status: PumpStatus,
    pub budgets: FsmBudgets,
    /// Dữ liệu chẩn đoán từ `LocalHealthAndDiagnostic` (optional để backward compat)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<FsmDiagnostics>,
}

/// Thông tin chẩn đoán edge AI nhúng trong snapshot FSM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsmDiagnostics {
    pub health_score_percent: u32,
    pub ec_pump_streak: u32,
    pub ph_pump_streak: u32,
    pub water_hydraulics_streak: u32,
    pub adaptive_mixing_sec: u32,
    pub adaptive_stabilize_sec: u32,
    /// Số lần log bị drop do channel đầy
    #[serde(default)]
    pub log_drop_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FsmBudgets {
    pub ec_ml: f32,
    pub ph_ml: f32,
    pub refill_count: u32,
    pub drain_count: u32,
}

// /// Legacy struct — giữ lại để backward compat với code backend cũ
// /// Dùng `FsmSnapshot` cho code mới
// #[deprecated(note = "Use FsmSnapshot instead — current_state is now typed as SystemPhase")]
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct FsmStatePayload {
//     pub online: bool,
//     pub current_state: String,
//     pub pump_status: PumpStatus,
//     pub budgets: FsmBudgets,
// }
