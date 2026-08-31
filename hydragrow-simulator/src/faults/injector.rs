use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::scenario::format::FaultEventKind;
use hydragrow_shared::SensorData;

#[derive(Default)]
pub struct Injector {
    pub active_faults: Vec<FaultEventKind>,
}

impl Injector {
    pub fn new() -> Self {
        Self {
            active_faults: vec![],
        }
    }

    pub fn add_active_fault(&mut self, fault: FaultEventKind) {
        self.active_faults.push(fault);
    }

    pub fn apply_hardware_faults(&self, hw: &mut VirtualHardwareState) {
        for fault in &self.active_faults {
            match fault {
                FaultEventKind::PumpStuckOn { pump } if pump == "PUMP_A" => {
                    hw.pump_a.on = true;
                }
                FaultEventKind::PumpStuckOff { pump } if pump == "PUMP_A" => {
                    hw.pump_a.on = false;
                }
                _ => {} // Sensor faults handled separately
            }
        }
    }

    #[allow(clippy::collapsible_if)]
    pub fn apply_sensor_faults(&self, _sensor: &mut SensorData) {
        for fault in &self.active_faults {
            if let FaultEventKind::SensorFrozen { sensor: s } = fault {
                if s == "EC" {
                    // The scenario states we want to simulate EcStagnant.
                    // Usually EcStagnant happens if EC doesn't change after pumping.
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actuators::virtual_hw::VirtualHardwareState;

    #[test]
    fn test_injector_pump_stuck() {
        let mut hw = VirtualHardwareState::default();
        let mut injector = Injector::new();
        injector.add_active_fault(FaultEventKind::PumpStuckOn {
            pump: "PUMP_A".to_string(),
        });

        injector.apply_hardware_faults(&mut hw);
        assert!(hw.pump_a.on); // Forced on despite default false
    }
}
