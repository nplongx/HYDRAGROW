use crate::actuators::virtual_hw::VirtualHardwareState;
use hydragrow_controller_core::core::fsm::{OrchestratorEvent, DosingPumpTarget};
use hydragrow_controller_core::WaterDirection;

pub struct SimDispatcher;

impl SimDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch(&mut self, event: &OrchestratorEvent, hw: &mut VirtualHardwareState) {
        match event {
            OrchestratorEvent::SetDosingPump { pump, on, pwm_percent } => {
                let pwm = *pwm_percent as u8;
                match pump {
                    DosingPumpTarget::NutrientA => {
                        hw.pump_a.on = *on;
                        hw.pump_a.pwm = pwm;
                    }
                    DosingPumpTarget::NutrientB => {
                        hw.pump_b.on = *on;
                        hw.pump_b.pwm = pwm;
                    }
                    DosingPumpTarget::PhUp => {
                        hw.pump_ph_up.on = *on;
                        hw.pump_ph_up.pwm = pwm;
                    }
                    DosingPumpTarget::PhDown => {
                        hw.pump_ph_down.on = *on;
                        hw.pump_ph_down.pwm = pwm;
                    }
                }
            }
            OrchestratorEvent::SetWaterPump { direction } => {
                match direction {
                    WaterDirection::In => {
                        hw.water_pump_in.on = true;
                        hw.water_pump_in.pwm = 100;
                        hw.water_pump_out.on = false;
                        hw.water_pump_out.pwm = 0;
                    }
                    WaterDirection::Out => {
                        hw.water_pump_out.on = true;
                        hw.water_pump_out.pwm = 100;
                        hw.water_pump_in.on = false;
                        hw.water_pump_in.pwm = 0;
                    }
                    WaterDirection::Stop => {
                        hw.water_pump_in.on = false;
                        hw.water_pump_in.pwm = 0;
                        hw.water_pump_out.on = false;
                        hw.water_pump_out.pwm = 0;
                    }
                }
            }
            OrchestratorEvent::SetMistValve { on } => {
                hw.mist_valve = *on;
            }
            OrchestratorEvent::SetOsakaPump { pwm_percent } => {
                hw.osaka_pwm = *pwm_percent as u8;
            }
            _ => {
                // Ignore other events for now
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_controller_core::core::fsm::{OrchestratorEvent, DosingPumpTarget};
    use crate::actuators::virtual_hw::VirtualHardwareState;

    #[test]
    fn test_dispatcher_pump_update() {
        let mut hw = VirtualHardwareState::default();
        let mut dispatcher = SimDispatcher::new();
        dispatcher.dispatch(&OrchestratorEvent::SetDosingPump { pump: DosingPumpTarget::NutrientA, on: true, pwm_percent: 50 }, &mut hw);
        assert_eq!(hw.pump_a.on, true);
        assert_eq!(hw.pump_a.pwm, 50);
    }
}
