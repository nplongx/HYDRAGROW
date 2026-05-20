use serde::{Deserialize, Serialize};

use crate::PumpStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
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
    SensorCalibration,
    Fault,
    EmergencyStop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FaultCode { EcDosingFailed, PhDosingFailed, WaterRefillFailed, TooManyRefills, TooManyDrains, MaxHourlyDoseEc, MaxHourlyDosePh, SensorTimeout, EcStagnant, PhOscillating, WaterLevelCritical }
impl FaultCode {
    pub fn as_str(&self) -> &'static str { match self { Self::EcDosingFailed=>"EC_DOSING_FAILED",Self::PhDosingFailed=>"PH_DOSING_FAILED",Self::WaterRefillFailed=>"WATER_REFILL_FAILED",Self::TooManyRefills=>"TOO_MANY_REFILLS",Self::TooManyDrains=>"TOO_MANY_DRAINS",Self::MaxHourlyDoseEc=>"MAX_HOURLY_DOSE_EC",Self::MaxHourlyDosePh=>"MAX_HOURLY_DOSE_PH",Self::SensorTimeout=>"SENSOR_TIMEOUT",Self::EcStagnant=>"EC_STAGNANT",Self::PhOscillating=>"PH_OSCILLATING",Self::WaterLevelCritical=>"WATER_LEVEL_CRITICAL" } }
    pub fn from_str(s: &str) -> Option<Self> { Some(match s {"EC_DOSING_FAILED"=>Self::EcDosingFailed,"PH_DOSING_FAILED"=>Self::PhDosingFailed,"WATER_REFILL_FAILED"=>Self::WaterRefillFailed,"TOO_MANY_REFILLS"=>Self::TooManyRefills,"TOO_MANY_DRAINS"=>Self::TooManyDrains,"MAX_HOURLY_DOSE_EC"=>Self::MaxHourlyDoseEc,"MAX_HOURLY_DOSE_PH"=>Self::MaxHourlyDosePh,"SENSOR_TIMEOUT"=>Self::SensorTimeout,"EC_STAGNANT"=>Self::EcStagnant,"PH_OSCILLATING"=>Self::PhOscillating,"WATER_LEVEL_CRITICAL"=>Self::WaterLevelCritical,_=>return None}) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsmStatePayload { pub online: bool, pub current_state: String, pub pump_status: PumpStatus, pub budgets: FsmBudgets }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FsmBudgets { pub ec_ml: f32, pub ph_ml: f32, pub refill_count: u32, pub drain_count: u32 }
