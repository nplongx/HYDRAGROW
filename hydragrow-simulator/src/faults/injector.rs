use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::scenario::format::FaultEventKind;
use hydragrow_shared::SensorData;
use std::collections::HashMap;

#[derive(Default)]
pub struct Injector {
    pub active_faults: Vec<FaultEventKind>,
    pub frozen_samples: HashMap<String, f32>,
    activated_at_ms: Vec<u64>,
    delayed_actual: HashMap<String, (bool, u8)>,
}

impl Injector {
    pub fn new() -> Self {
        Self {
            active_faults: vec![],
            frozen_samples: HashMap::new(),
            activated_at_ms: vec![],
            delayed_actual: HashMap::new(),
        }
    }

    pub fn add_active_fault(&mut self, fault: FaultEventKind) {
        self.add_active_fault_at(fault, 0);
    }

    pub fn add_active_fault_at(&mut self, fault: FaultEventKind, uptime_ms: u64) {
        self.active_faults.push(fault);
        self.activated_at_ms.push(uptime_ms);
    }

    pub fn apply_hardware_faults(&mut self, hw: &mut VirtualHardwareState) {
        self.apply_hardware_faults_at(hw, u64::MAX);
    }

    pub fn apply_hardware_faults_at(&mut self, hw: &mut VirtualHardwareState, uptime_ms: u64) {
        for (index, fault) in self.active_faults.iter().enumerate() {
            match fault {
                FaultEventKind::PumpStuckOn { pump } => match pump.to_ascii_uppercase().as_str() {
                    // A relay stuck closed runs the pump: forcing only `on = true`
                    // while leaving `pwm_percent = 0` would deliver zero flow, which is
                    // physically incoherent and invisible to the plant model
                    // (`Tank::step` scales flow by `pwm_percent / 100`).
                    "PUMP_A" => {
                        hw.pump_a.on = true;
                        hw.pump_a.pwm_percent = 100;
                    }
                    "PUMP_B" => {
                        hw.pump_b.on = true;
                        hw.pump_b.pwm_percent = 100;
                    }
                    "PUMP_PH_UP" | "PH_UP" => {
                        hw.pump_ph_up.on = true;
                        hw.pump_ph_up.pwm_percent = 100;
                    }
                    "PUMP_PH_DOWN" | "PH_DOWN" => {
                        hw.pump_ph_down.on = true;
                        hw.pump_ph_down.pwm_percent = 100;
                    }
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
                FaultEventKind::ActuatorDelayed { pump, delay_ms } => {
                    let activated_at = self.activated_at_ms.get(index).copied().unwrap_or(0);
                    let Some(target) = delayed_pump_mut(hw, pump) else {
                        continue;
                    };
                    let key = pump.to_ascii_uppercase();
                    if target.desired_on != target.on
                        || target.desired_pwm_percent != target.pwm_percent
                    {
                        self.delayed_actual
                            .entry(key.clone())
                            .or_insert((target.on, target.pwm_percent));
                    }
                    if uptime_ms.saturating_sub(activated_at) < *delay_ms {
                        if let Some((on, pwm)) = self.delayed_actual.get(&key).copied() {
                            target.on = on;
                            target.pwm_percent = pwm;
                        }
                    } else {
                        target.on = target.desired_on;
                        target.pwm_percent = target.desired_pwm_percent;
                        self.delayed_actual.remove(&key);
                    }
                }
                _ => {} // Sensor faults handled separately
            }
        }
    }

    pub fn apply_sensor_faults(&mut self, sensor_data: &mut SensorData) {
        for fault in &self.active_faults {
            match fault {
                FaultEventKind::SensorFrozen { sensor } => {
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
                FaultEventKind::SensorMissing { sensor } => {
                    match sensor.to_ascii_uppercase().as_str() {
                        "EC" => sensor_data.err_ec = Some(true),
                        "PH" => sensor_data.err_ph = Some(true),
                        "TEMP" => sensor_data.err_temp = Some(true),
                        "WATER_LEVEL" | "WATER" => sensor_data.err_water = Some(true),
                        _ => {}
                    }
                }
                FaultEventKind::SensorInvalid { sensor } => {
                    match sensor.to_ascii_uppercase().as_str() {
                        "EC" => sensor_data.err_ec = Some(true),
                        "PH" => sensor_data.err_ph = Some(true),
                        "TEMP" => sensor_data.err_temp = Some(true),
                        "WATER_LEVEL" | "WATER" => sensor_data.err_water = Some(true),
                        _ => {}
                    }
                }
                FaultEventKind::SensorOutlier { sensor, multiplier } => {
                    match sensor.to_ascii_uppercase().as_str() {
                        "EC" => sensor_data.ec *= *multiplier,
                        "PH" => sensor_data.ph *= *multiplier,
                        "TEMP" => sensor_data.temp *= *multiplier,
                        "WATER_LEVEL" | "WATER" => sensor_data.water_level *= *multiplier,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

fn delayed_pump_mut<'a>(
    hw: &'a mut VirtualHardwareState,
    pump: &str,
) -> Option<&'a mut crate::actuators::virtual_hw::VirtualPump> {
    match pump.to_ascii_uppercase().as_str() {
        "PUMP_A" => Some(&mut hw.pump_a),
        "PUMP_B" => Some(&mut hw.pump_b),
        "PUMP_PH_UP" | "PH_UP" => Some(&mut hw.pump_ph_up),
        "PUMP_PH_DOWN" | "PH_DOWN" => Some(&mut hw.pump_ph_down),
        "WATER_PUMP_IN" | "WATER_IN" => Some(&mut hw.water_pump_in),
        "WATER_PUMP_OUT" | "WATER_OUT" => Some(&mut hw.water_pump_out),
        _ => None,
    }
}

impl Injector {
    pub fn mqtt_disconnected(&self) -> bool {
        let disconnected = self
            .active_faults
            .iter()
            .any(|f| matches!(f, FaultEventKind::MqttDisconnect));
        disconnected
            && !self
                .active_faults
                .iter()
                .any(|f| matches!(f, FaultEventKind::MqttReconnect))
    }

    pub fn mqtt_drop(&self) -> bool {
        self.active_faults
            .iter()
            .any(|f| matches!(f, FaultEventKind::MqttDrop))
    }

    pub fn mqtt_duplicate(&self) -> bool {
        self.active_faults
            .iter()
            .any(|f| matches!(f, FaultEventKind::MqttDuplicate))
    }

    pub fn mqtt_delay_ms(&self) -> Option<u64> {
        self.active_faults.iter().find_map(|f| match f {
            FaultEventKind::MqttDelay { delay_ms } => Some(*delay_ms),
            _ => None,
        })
    }

    pub fn telemetry_paused(&self) -> bool {
        self.active_faults
            .iter()
            .any(|f| matches!(f, FaultEventKind::TelemetryPause))
    }

    pub fn telemetry_delay_ms(&self) -> Option<u64> {
        self.active_faults.iter().find_map(|f| match f {
            FaultEventKind::TelemetryDelay { delay_ms } => Some(*delay_ms),
            _ => None,
        })
    }

    pub fn telemetry_out_of_order(&self) -> bool {
        self.active_faults
            .iter()
            .any(|f| matches!(f, FaultEventKind::TelemetryOutOfOrder))
    }

    pub fn boot_loop(&self) -> bool {
        self.active_faults
            .iter()
            .any(|f| matches!(f, FaultEventKind::BootLoop))
    }

    pub fn configuration_lost(&self) -> bool {
        self.active_faults
            .iter()
            .any(|f| matches!(f, FaultEventKind::ConfigurationLoss))
    }

    pub fn clock_jump_ms(&self) -> Option<i64> {
        self.active_faults.iter().find_map(|f| match f {
            FaultEventKind::ClockJump { delta_ms } => Some(*delta_ms),
            _ => None,
        })
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

    #[test]
    fn missing_sensor_is_explicit_and_does_not_fabricate_zero() {
        let mut sensor = SensorData {
            ec: 1.7,
            ..Default::default()
        };
        let mut injector = Injector::new();
        injector.add_active_fault(FaultEventKind::SensorMissing {
            sensor: "EC".into(),
        });
        injector.apply_sensor_faults(&mut sensor);
        assert_eq!(sensor.ec, 1.7);
        assert_eq!(sensor.err_ec, Some(true));
    }

    #[test]
    fn sensor_outlier_is_deterministic() {
        let mut sensor = SensorData {
            ph: 6.0,
            ..Default::default()
        };
        let mut injector = Injector::new();
        injector.add_active_fault(FaultEventKind::SensorOutlier {
            sensor: "PH".into(),
            multiplier: 2.0,
        });
        injector.apply_sensor_faults(&mut sensor);
        assert_eq!(sensor.ph, 12.0);
    }
}
