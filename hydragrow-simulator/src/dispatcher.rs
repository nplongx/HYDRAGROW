use crate::actuators::virtual_hw::VirtualHardwareState;
use hydragrow_controller_core::core::fsm::OrchestratorEvent;
use hydragrow_controller_core::core::fsm::events::DosingPumpTarget;
use hydragrow_controller_core::WaterDirection;

pub struct SimDispatcher;

impl SimDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch(&mut self, event: &OrchestratorEvent, hw: &mut VirtualHardwareState) {
        match event {
            OrchestratorEvent::SetDosingPump { pump, on, pwm_percent } => {
                match pump {
                    DosingPumpTarget::NutrientA => { hw.pump_a.on = *on; hw.pump_a.pwm = *pwm_percent as u8; }
                    DosingPumpTarget::NutrientB => { hw.pump_b.on = *on; hw.pump_b.pwm = *pwm_percent as u8; }
                    DosingPumpTarget::PhUp => { hw.pump_ph_up.on = *on; hw.pump_ph_up.pwm = *pwm_percent as u8; }
                    DosingPumpTarget::PhDown => { hw.pump_ph_down.on = *on; hw.pump_ph_down.pwm = *pwm_percent as u8; }
                }
            }
            OrchestratorEvent::SetWaterPump { direction } => {
                match direction {
                    WaterDirection::In => { hw.water_pump_in.on = true; hw.water_pump_in.pwm = 100; hw.water_pump_out.on = false; hw.water_pump_out.pwm = 0; }
                    WaterDirection::Out => { hw.water_pump_in.on = false; hw.water_pump_in.pwm = 0; hw.water_pump_out.on = true; hw.water_pump_out.pwm = 100; }
                    WaterDirection::Stop => { hw.water_pump_in.on = false; hw.water_pump_in.pwm = 0; hw.water_pump_out.on = false; hw.water_pump_out.pwm = 0; }
                }
            }
            OrchestratorEvent::SetMistValve { on } => {
                hw.mist_valve = *on;
            }
            OrchestratorEvent::SetMixValve { .. } => {}
            OrchestratorEvent::SetOsakaPump { pwm_percent } => {
                hw.osaka_pwm = *pwm_percent as u8;
            }
            _ => {}
        }
    }
}
