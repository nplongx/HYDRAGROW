use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::scenario::format::InitialTank;
use hydragrow_controller_core::test_support::{calculate_ec_change, calculate_ph_change};
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
    pub fn from_initial(initial: &InitialTank) -> Self {
        Self {
            volume_l: initial.volume_l,
            ec: initial.ec,
            ph: initial.ph,
            temp: initial.temp,
            water_level: initial.water_level,
        }
    }

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

        // Water level rate is derived from ControllerConfig (tank_height / max duration),
        // not a fabricated constant, per module-rules: plant model must read coefficients
        // from ControllerConfig rather than inventing parallel ones. `volume_l` is
        // intentionally left unchanged here: there is no existing config field converting
        // a height/level reading to liters, so adding one would be inventing a new
        // coefficient rather than using an existing source of truth.
        let tank_height = config.tank_height as f32;
        if actuators.water_pump_in.on && config.max_refill_duration_sec > 0 {
            let refill_rate_per_sec = tank_height / config.max_refill_duration_sec as f32;
            self.water_level = (self.water_level + refill_rate_per_sec * dt_sec).min(tank_height);
        }
        if actuators.water_pump_out.on && config.max_drain_duration_sec > 0 {
            let drain_rate_per_sec = tank_height / config.max_drain_duration_sec as f32;
            self.water_level = (self.water_level - drain_rate_per_sec * dt_sec).max(0.0);
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

    #[test]
    fn test_tank_step_water_level_refill_uses_config_rate_and_clamps() {
        let mut tank = Tank {
            volume_l: 10.0,
            ec: 1.0,
            ph: 6.0,
            temp: 25.0,
            water_level: 90.0,
        };
        let config = ControllerConfig {
            tank_height: 100,
            max_refill_duration_sec: 50,
            ..Default::default()
        };
        let hw = VirtualHardwareState {
            water_pump_in: VirtualPump {
                on: true,
                pwm_percent: 100,
            },
            ..Default::default()
        };
        // rate = tank_height / max_refill_duration_sec = 100/50 = 2.0 units/sec.
        // Over 10s that is +20, which would overshoot tank_height(100) from 90 -> must clamp.
        tank.step(10_000, &hw, &config);
        assert_eq!(
            tank.water_level, 100.0,
            "water level must clamp at tank_height"
        );
    }

    #[test]
    fn test_tank_step_water_level_drain_uses_config_rate_and_clamps_to_zero() {
        let mut tank = Tank {
            volume_l: 10.0,
            ec: 1.0,
            ph: 6.0,
            temp: 25.0,
            water_level: 5.0,
        };
        let config = ControllerConfig {
            tank_height: 100,
            max_drain_duration_sec: 20,
            ..Default::default()
        };
        let hw = VirtualHardwareState {
            water_pump_out: VirtualPump {
                on: true,
                pwm_percent: 100,
            },
            ..Default::default()
        };
        // rate = tank_height / max_drain_duration_sec = 100/20 = 5.0 units/sec.
        // Over 3s that is -15, which would undershoot 0 from 5 -> must clamp.
        tank.step(3_000, &hw, &config);
        assert_eq!(tank.water_level, 0.0, "water level must clamp at zero");
    }
}
