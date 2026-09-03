use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::event_dispatcher::apply_event;
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

    pub fn dispatch(&mut self, event: &OrchestratorEvent, hw: &mut VirtualHardwareState) {
        apply_event(hw, event);
        if let Some(bridge) = &mut self.mqtt_bridge {
            let _ = bridge.publish_event(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_controller_core::core::fsm::events::{DosingPumpTarget, OrchestratorEvent};

    #[test]
    fn sim_dispatcher_updates_hardware_state() {
        let mut dispatcher = SimDispatcher::new();
        let mut hw = VirtualHardwareState::default();

        dispatcher.dispatch(
            &OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: true,
                pwm_percent: 50,
            },
            &mut hw,
        );

        assert!(hw.pump_a.on);
        assert_eq!(hw.pump_a.pwm_percent, 50);
    }
}
