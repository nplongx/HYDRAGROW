// src/core/fsm/phases/mod.rs
pub mod active_mixing;
pub mod cooldown;
pub mod mimo_dosing;
pub mod monitoring;
pub mod stabilizing;
pub mod water_phases;

pub use active_mixing::ActiveMixingPhase;
pub use cooldown::CooldownPhase;
pub use mimo_dosing::MimoDosingPhase;
pub use monitoring::MonitoringPhase;
pub use stabilizing::StabilizingPhase;
pub use water_phases::{WaterDrainingPhase, WaterRefillingPhase};
