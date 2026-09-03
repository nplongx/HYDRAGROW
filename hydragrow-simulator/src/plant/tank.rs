use crate::actuators::virtual_hw::VirtualHardwareState;
use hydragrow_shared::ControllerConfig;

#[derive(Debug, Clone, Default)]
pub struct Tank {
    pub volume_l: f32,
    pub ec: f32,
    pub ph: f32,
    pub temp: f32,
    pub water_level: f32,
}

impl Tank {
    /// Advances the tank simulation state by `dt_ms` milliseconds.
    ///
    /// This is a first-order linear model driven by configuration fields.
    pub fn step(
        &mut self,
        dt_ms: u64,
        actuators: &VirtualHardwareState,
        config: &ControllerConfig,
    ) {
        let dt_sec = dt_ms as f32 / 1000.0;
        let max_height = if config.tank_height > 0 {
            config.tank_height as f32
        } else {
            config.water_level_max.max(100.0)
        };

        // Preserve liters per cm ratio if level/volume are non-zero, otherwise default
        let liters_per_cm = if self.water_level > 0.0 && self.volume_l > 0.0 {
            self.volume_l / self.water_level
        } else {
            0.2
        };

        let old_volume = self.volume_l;

        // 1. Water pumps (Refill and Drain)
        let mut level_change = 0.0;
        if actuators.water_pump_in.on {
            let pwm_factor = if actuators.water_pump_in.pwm > 0 {
                actuators.water_pump_in.pwm as f32 / 100.0
            } else {
                1.0
            };
            level_change += 1.0 * dt_sec * pwm_factor;
        }
        if actuators.water_pump_out.on {
            let pwm_factor = if actuators.water_pump_out.pwm > 0 {
                actuators.water_pump_out.pwm as f32 / 100.0
            } else {
                1.0
            };
            level_change -= 1.0 * dt_sec * pwm_factor;
        }

        let new_level = (self.water_level + level_change).clamp(0.0, max_height);
        self.water_level = new_level;
        self.volume_l = (self.water_level * liters_per_cm).max(0.0);

        // Dilute EC / pH when clean water is added (refill)
        if level_change > 0.0 && self.volume_l > old_volume && old_volume > 0.0 {
            self.ec = (self.ec * old_volume) / self.volume_l;
            self.ph = (self.ph * old_volume + 7.0 * (self.volume_l - old_volume)) / self.volume_l;
        }

        // 2. Dosing pumps (Nutrient A, Nutrient B, pH Up, pH Down)
        let mut ec_change = 0.0;
        let mut ph_change = 0.0;

        let current_vol = self.volume_l.max(f32::EPSILON);

        if actuators.pump_a.on {
            let flow = config.pump_a_capacity_ml_per_sec
                * dt_sec
                * (actuators.pump_a.pwm_percent as f32 / 100.0);
            ec_change += calculate_ec_change(flow, self.volume_l, config);
        }
        if actuators.pump_b.on {
            let flow = config.pump_b_capacity_ml_per_sec
                * dt_sec
                * (actuators.pump_b.pwm_percent as f32 / 100.0);
            ec_change += calculate_ec_change(flow, self.volume_l, config);
        }
        if actuators.pump_ph_up.on {
            let flow = config.pump_ph_up_capacity_ml_per_sec
                * dt_sec
                * (actuators.pump_ph_up.pwm_percent as f32 / 100.0);
            ph_change += calculate_ph_change(flow, self.volume_l, true, config);
        }
        if actuators.pump_ph_down.on {
            let flow = config.pump_ph_down_capacity_ml_per_sec
                * dt_sec
                * (actuators.pump_ph_down.pwm_percent as f32 / 100.0);
            ph_change += calculate_ph_change(flow, self.volume_l, false, config);
        }

        self.ec += ec_change;
        self.ph += ph_change;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actuators::virtual_hw::{VirtualHardwareState, VirtualPump};
    use hydragrow_shared::ControllerConfig;

    #[test]
    fn test_tank_step_dosing_ec() {
        let mut tank = Tank {
            volume_l: 10.0,
            ec: 1.0,
            ph: 6.0,
            temp: 25.0,
            water_level: 50.0,
        };
        let config = ControllerConfig {
            ec_gain_per_ml: 0.5,
            pump_a_capacity_ml_per_sec: 2.0,
            ..Default::default()
        };

        let hw = VirtualHardwareState {
            pump_a: VirtualPump {
                on: true,
                pwm_percent: 100,
            },
            ..Default::default()
        };

        tank.step(1000, &hw, &config);

        assert_eq!(tank.ec, 1.1);
    }
}
