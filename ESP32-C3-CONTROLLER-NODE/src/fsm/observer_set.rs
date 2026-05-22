//! ObserverSet — Container gom tất cả observers.
//! Gọi notify_all() sau mỗi event được dispatch để fan-out tới mọi subscriber.
//!
//! DESIGN: Struct-of-observers thay vì Vec<dyn Observer> — zero heap allocation,
//! compiler có thể inline từng on_event() call vì concrete types.

use crate::fsm::events::OrchestratorEvent;
use crate::fsm::observers::ObserverContext;
use crate::fsm::observers::{
    DosingAnalyticsObserver, FaultAlarmObserver, MqttTelemetryObserver, SystemLogObserver,
};

pub struct ObserverSet {
    pub mqtt_telemetry: MqttTelemetryObserver,
    pub system_log: SystemLogObserver,
    pub dosing_analytics: DosingAnalyticsObserver,
    pub fault_alarm: FaultAlarmObserver,
}

impl ObserverSet {
    pub fn new() -> Self {
        Self {
            mqtt_telemetry: MqttTelemetryObserver::new(),
            system_log: SystemLogObserver::new(),
            dosing_analytics: DosingAnalyticsObserver::new(),
            fault_alarm: FaultAlarmObserver::new(),
        }
    }

    /// Fan-out event tới tất cả observers.
    /// Thứ tự: system_log → fault_alarm → mqtt_telemetry → dosing_analytics
    /// (log và alarm trước để capture ngay cả khi mqtt bị drop)
    pub fn notify_all(&mut self, event: &OrchestratorEvent, oc: &ObserverContext<'_>) {
        self.system_log.on_event(event, oc);
        self.fault_alarm.on_event(event, oc);
        self.mqtt_telemetry.on_event(event, oc);
        self.dosing_analytics.on_event(event, oc);
    }
}
