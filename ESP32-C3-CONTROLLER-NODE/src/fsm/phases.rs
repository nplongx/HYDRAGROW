#[derive(Debug, Clone, PartialEq)]
pub enum SystemPhase {
    Booting,
    Monitoring,
    ManualMode,
    WaterRefilling,
    WaterDraining,
    DosingEC,
    DosingPH,
    ActiveMixing,
    Stabilizing,
    Cooldown,
    SensorCalibration { step: String },
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
            SystemPhase::DosingEC => "DosingEC",
            SystemPhase::DosingPH => "DosingPH",
            SystemPhase::ActiveMixing => "ActiveMixing",
            SystemPhase::Stabilizing => "Stabilizing",
            SystemPhase::Cooldown => "Cooldown",
            SystemPhase::SensorCalibration { .. } => "SensorCalibration",
            SystemPhase::Fault(_) => "Fault",
            SystemPhase::EmergencyStop(_) => "EmergencyStop",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FaultCode {
    EcDosingFailed,
    PhDosingFailed,
    WaterRefillFailed,
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

#[allow(dead_code)]
pub fn map_fault_code(reason: &str) -> FaultCode {
    if reason.contains("EC_DOSING_FAILED") {
        FaultCode::EcDosingFailed
    } else if reason.contains("PH_DOSING_FAILED") {
        FaultCode::PhDosingFailed
    } else if reason.contains("WATER_REFILL_FAILED") {
        FaultCode::WaterRefillFailed
    } else if reason.contains("TOO_MANY_REFILLS") {
        FaultCode::TooManyRefills
    } else if reason.contains("TOO_MANY_DRAINS") {
        FaultCode::TooManyDrains
    } else if reason.contains("MAX_HOURLY_DOSE_PH") {
        FaultCode::MaxHourlyDosePh
    } else {
        FaultCode::MaxHourlyDoseEc
    }
}

