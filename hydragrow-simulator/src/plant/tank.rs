use crate::actuators::virtual_hw::VirtualHardwareState;
use hydragrow_controller_core::test_support::{calculate_ec_change, calculate_ph_change};
use hydragrow_shared::ControllerConfig;

#[derive(Debug, Clone)]
pub struct Tank {
    pub volume_l: f32,
    pub ec: f32,
    pub ph: f32,
    pub temp: f32,
    pub water_level: f32,
}

impl Tank {
    pub fn step(
        &mut self,
        dt_ms: u64,
        actuators: &VirtualHardwareState,
        config: &ControllerConfig,
    ) {
        let dt_sec = dt_ms as f32 / 1000.0;

        let mut ec_change = 0.0;
        let mut ph_change = 0.0;

        if actuators.pump_a.on {
            let flow =
                config.pump_a_capacity_ml_per_sec * dt_sec * (actuators.pump_a.pwm as f32 / 100.0);
            ec_change += calculate_ec_change(flow, self.volume_l, config);
        }
        if actuators.pump_b.on {
            let flow =
                config.pump_b_capacity_ml_per_sec * dt_sec * (actuators.pump_b.pwm as f32 / 100.0);
            ec_change += calculate_ec_change(flow, self.volume_l, config);
        }
        if actuators.pump_ph_up.on {
            let flow = config.pump_ph_up_capacity_ml_per_sec
                * dt_sec
                * (actuators.pump_ph_up.pwm as f32 / 100.0);
            ph_change += calculate_ph_change(flow, self.volume_l, true, config);
        }
        if actuators.pump_ph_down.on {
            let flow = config.pump_ph_down_capacity_ml_per_sec
                * dt_sec
                * (actuators.pump_ph_down.pwm as f32 / 100.0);
            ph_change += calculate_ph_change(flow, self.volume_l, false, config);
        }

        self.ec += ec_change;
        self.ph += ph_change;

        if actuators.water_pump_in.on {
            self.water_level += dt_sec;
        }
        if actuators.water_pump_out.on {
            self.water_level -= dt_sec;
        }
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
        let mut config = ControllerConfig::default();
        config.ec_gain_per_ml = 0.5;
        config.pump_a_capacity_ml_per_sec = 2.0;

        let mut hw = VirtualHardwareState::default();
        hw.pump_a = VirtualPump { on: true, pwm: 100 };

        tank.step(1000, &hw, &config);

        assert_eq!(tank.ec, 1.1);
    }
}
