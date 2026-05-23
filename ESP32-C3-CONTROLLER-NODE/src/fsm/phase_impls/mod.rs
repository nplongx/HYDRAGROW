// src/fsm/phase_impls/mod.rs
pub mod active_mixing;
pub mod cooldown;
pub mod mimo_dosing;
pub mod monitoring;
pub mod stabilizing;

pub use active_mixing::ActiveMixingPhase;
pub use cooldown::CooldownPhase;
use hydragrow_shared::fsm::FaultCode;
pub use mimo_dosing::MimoDosingPhase;
pub use monitoring::MonitoringPhase;
pub use stabilizing::StabilizingPhase;
pub mod water_phases;
pub use water_phases::{WaterDrainingPhase, WaterRefillingPhase};

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
