use serde::{Deserialize, Serialize};

use crate::PumpStatus;
//
// use crate::PumpStatus;
//
#[derive(Debug, Clone, PartialEq, Deserialize)]
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
            SystemPhase::SensorCalibration { .. } => "SensorCalibration",
            SystemPhase::Fault(_) => "Fault",
            SystemPhase::EmergencyStop(_) => "EmergencyStop",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsmStatePayload {
    pub online: bool,
    pub current_state: String,
    pub pump_status: PumpStatus,
    pub budgets: FsmBudgets,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FsmBudgets {
    pub ec_ml: f32,
    pub ph_ml: f32,
    pub refill_count: u32,
    pub drain_count: u32,
}
