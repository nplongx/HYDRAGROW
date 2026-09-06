use crate::plant::tank::Tank;
use hydragrow_shared::SensorData;

#[derive(Debug, Clone, Default)]
pub struct NoiseConfig {
    pub ec_noise_std_dev: f32,
    pub ph_noise_std_dev: f32,
}

impl NoiseConfig {
    pub fn none() -> Self {
        Self {
            ec_noise_std_dev: 0.0,
            ph_noise_std_dev: 0.0,
        }
    }
}

pub fn read_sensor(tank: &Tank, _config: &NoiseConfig) -> SensorData {
    let ec_noise = 0.0;
    let ph_noise = 0.0;

    SensorData {
        device_id: "sim-dev".to_string(),
        ec: tank.ec + ec_noise,
        ph: tank.ph + ph_noise,
        temp: tank.temp,
        water_level: tank.water_level,
        pump_status: hydragrow_shared::PumpStatus::default(),
        time: "".to_string(),
        controller_received_ms: None,
        rssi: None,
        free_heap: None,
        err_ec: None,
        err_ph: None,
        err_temp: None,
        err_water: None,
        is_continuous: None,
        ph_voltage_mv: None,
        uptime: None,
        ec_received_ms: None,
        ph_received_ms: None,
        temp_received_ms: None,
        water_received_ms: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plant::tank::Tank;

    #[test]
    fn test_sensor_read_no_noise() {
        let tank = Tank {
            volume_l: 10.0,
            ec: 1.5,
            ph: 6.2,
            temp: 24.5,
            water_level: 40.0,
        };
        let cfg = NoiseConfig::none();
        let sensor = read_sensor(&tank, &cfg);
        assert_eq!(sensor.ec, 1.5);
        assert_eq!(sensor.ph, 6.2);
    }
}
