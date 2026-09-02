use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::scenario::format::FaultEventKind;
use hydragrow_shared::SensorData;
use std::collections::HashMap;

#[derive(Default)]
pub struct Injector {
    pub active_faults: Vec<FaultEventKind>,
    pub frozen_samples: HashMap<String, f32>,
}

impl Injector {
    pub fn new() -> Self {
        Self {
            active_faults: vec![],
            frozen_samples: HashMap::new(),
        }
    }

    pub fn add_active_fault(&mut self, fault: FaultEventKind) {
        self.active_faults.push(fault);
    }

    pub fn apply_hardware_faults(&self, hw: &mut VirtualHardwareState) {
        for fault in &self.active_faults {
            match fault {
                FaultEventKind::PumpStuckOn { pump } => match pump.to_ascii_uppercase().as_str() {
                    "PUMP_A" => hw.pump_a.on = true,
                    "PUMP_B" => hw.pump_b.on = true,
                    "PUMP_PH_UP" | "PH_UP" => hw.pump_ph_up.on = true,
                    "PUMP_PH_DOWN" | "PH_DOWN" => hw.pump_ph_down.on = true,
                    "WATER_PUMP_IN" | "WATER_IN" => hw.water_pump_in.on = true,
                    "WATER_PUMP_OUT" | "WATER_OUT" => hw.water_pump_out.on = true,
                    _ => {}
                },
                FaultEventKind::PumpStuckOff { pump } => match pump.to_ascii_uppercase().as_str() {
                    "PUMP_A" => hw.pump_a.on = false,
                    "PUMP_B" => hw.pump_b.on = false,
                    "PUMP_PH_UP" | "PH_UP" => hw.pump_ph_up.on = false,
                    "PUMP_PH_DOWN" | "PH_DOWN" => hw.pump_ph_down.on = false,
                    "WATER_PUMP_IN" | "WATER_IN" => hw.water_pump_in.on = false,
                    "WATER_PUMP_OUT" | "WATER_OUT" => hw.water_pump_out.on = false,
                    _ => {}
                },
                _ => {} // Sensor faults handled separately
            }
        }
    }

    pub fn apply_sensor_faults(&mut self, sensor_data: &mut SensorData) {
        for fault in &self.active_faults {
            if let FaultEventKind::SensorFrozen { sensor } = fault {
                let key = sensor.to_ascii_uppercase();
                match key.as_str() {
                    "EC" => {
                        let frozen_val = *self
                            .frozen_samples
                            .entry("EC".to_string())
                            .or_insert(sensor_data.ec);
                        sensor_data.ec = frozen_val;
                    }
                    "PH" => {
                        let frozen_val = *self
                            .frozen_samples
                            .entry("PH".to_string())
                            .or_insert(sensor_data.ph);
                        sensor_data.ph = frozen_val;
                    }
                    "TEMP" => {
                        let frozen_val = *self
                            .frozen_samples
                            .entry("TEMP".to_string())
                            .or_insert(sensor_data.temp);
                        sensor_data.temp = frozen_val;
                    }
                    "WATER_LEVEL" | "WATER" => {
                        let frozen_val = *self
                            .frozen_samples
                            .entry("WATER_LEVEL".to_string())
                            .or_insert(sensor_data.water_level);
                        sensor_data.water_level = frozen_val;
                    }
                    _ => {}
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
        injector.add_active_fault(FaultEventKind::PumpStuckOn {
            pump: "PUMP_B".to_string(),
        });

        injector.apply_hardware_faults(&mut hw);
        assert!(hw.pump_a.on);
        assert!(hw.pump_b.on);
    }
}
