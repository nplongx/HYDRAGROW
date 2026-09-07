use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::event_dispatcher::apply_event;
use hydragrow_controller_core::core::fsm::events::OrchestratorEvent;

#[derive(Default)]
pub struct SimDispatcher;

impl SimDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch(&mut self, event: &OrchestratorEvent, hw: &mut VirtualHardwareState) {
        apply_event(hw, event);
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
