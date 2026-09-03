use crate::plant::tank::Tank;
use hydragrow_shared::SensorData;

#[derive(Debug, Clone, Default)]
pub struct NoiseConfig {
    pub ec_noise_std_dev: f32,
    pub ph_noise_std_dev: f32,
    pub seed: u64,
}

impl NoiseConfig {
    pub fn none() -> Self {
        Self {
            ec_noise_std_dev: 0.0,
            ph_noise_std_dev: 0.0,
            seed: 0,
        }
    }
}

/// Simple deterministic pseudo-random generator mapping (seed, index) to [-1.0, 1.0]
fn pseudo_random_f32(seed: u64, index: u32) -> f32 {
    let mut state = seed.wrapping_add((index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    if state == 0 {
        state = 0x85EB_CA6B;
    }
    state ^= state >> 30;
    state = state.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    state ^= state >> 27;
    state = state.wrapping_mul(0x94D0_49BB_1331_11EB);
    state ^= state >> 31;
    let val = (state & 0x00FF_FFFF) as f32 / 16777216.0;
    val * 2.0 - 1.0
}

pub fn read_sensor(tank: &Tank, config: &NoiseConfig) -> SensorData {
    let ec_noise = if config.ec_noise_std_dev > 0.0 {
        pseudo_random_f32(config.seed, 0) * config.ec_noise_std_dev
    } else {
        0.0
    };

    let ph_noise = if config.ph_noise_std_dev > 0.0 {
        pseudo_random_f32(config.seed, 1) * config.ph_noise_std_dev
    } else {
        0.0
    };

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
