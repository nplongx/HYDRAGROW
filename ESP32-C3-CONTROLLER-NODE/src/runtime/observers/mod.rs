// src/runtime/observers/mod.rs
pub mod dosing_analytics;
pub mod fault_alarm;
pub mod mqtt_telemetry;
pub mod observer_set;
pub mod system_log;

pub use observer_set::ObserverSet;

pub struct ObserverContext<'a> {
    pub ctx: &'a crate::core::fsm::context::SystemContext,
    pub config: &'a hydragrow_shared::ControllerConfig,
    pub now_ms: u64,
    pub mqtt_tx: &'a std::sync::mpsc::Sender<String>,
    pub dosing_report_tx: &'a std::sync::mpsc::Sender<String>,
}
