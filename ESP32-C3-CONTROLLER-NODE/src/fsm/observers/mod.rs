//! Observer layer — các subsystem subscribe OrchestratorEvent stream.
//!
//! DESIGN: Không dùng dyn Trait để tránh heap allocation và vtable overhead trên embedded.
//! Thay vào đó dùng concrete structs được gom trong ObserverSet.
//! Mỗi observer nhận `&OrchestratorEvent` + `&SystemContext` — read-only access.
//! Observer KHÔNG được mutate ctx, KHÔNG được gọi hardware trực tiếp.
//! Observer chỉ được gọi `mqtt_tx.send()` và `dosing_report_tx.send()`.

pub mod dosing_analytics;
pub mod fault_alarm;
pub mod mqtt_telemetry;
pub mod system_log;
pub use dosing_analytics::DosingAnalyticsObserver;
pub use fault_alarm::FaultAlarmObserver;
pub use mqtt_telemetry::MqttTelemetryObserver;
pub use system_log::SystemLogObserver;

/// Context chung được pass vào mỗi observer khi notify.
/// Immutable view — observer không được phép mutate bất cứ thứ gì ở đây.
pub struct ObserverContext<'a> {
    pub ctx: &'a crate::fsm::system_context::SystemContext,
    pub config: &'a hydragrow_shared::ControllerConfig,
    pub now_ms: u64,
    pub mqtt_tx: &'a std::sync::mpsc::Sender<String>,
    pub dosing_report_tx: &'a std::sync::mpsc::Sender<String>,
}
