use crate::telemetry::mqtt_bridge::MqttBridge;
use hydragrow_controller_core::core::fsm::events::OrchestratorEvent;

pub struct SimDispatcher {
    pub mqtt_bridge: Option<MqttBridge>,
}

impl Default for SimDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl SimDispatcher {
    pub fn new() -> Self {
        Self { mqtt_bridge: None }
    }

    pub fn dispatch(&mut self, event: &OrchestratorEvent) {
        if let Some(bridge) = &mut self.mqtt_bridge {
            bridge.publish_event(event);
        }
    }
}