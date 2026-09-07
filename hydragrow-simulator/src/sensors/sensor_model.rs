use crate::plant::tank::Tank;
use hydragrow_shared::SensorData;
use rand::Rng;
use rand::rngs::StdRng;

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

/// Box-Muller transform. Returns 0.0 without touching `rng` when `std_dev <= 0.0`,
/// so `NoiseConfig::none()` stays bit-exact identical to the raw tank reading.
fn gaussian_sample(rng: &mut StdRng, std_dev: f32) -> f32 {
    if std_dev <= 0.0 {
        return 0.0;
    }
    let u1: f32 = rng.gen_range(f32::EPSILON..1.0);
    let u2: f32 = rng.gen_range(0.0..1.0);
    let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
    z0 * std_dev
}

pub fn read_sensor(tank: &Tank, config: &NoiseConfig, rng: &mut StdRng) -> SensorData {
    let ec_noise = gaussian_sample(rng, config.ec_noise_std_dev);
    let ph_noise = gaussian_sample(rng, config.ph_noise_std_dev);

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
    use rand::SeedableRng;
    use rand::rngs::StdRng;

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
        let mut rng = StdRng::seed_from_u64(cfg.seed);
        let sensor = read_sensor(&tank, &cfg, &mut rng);
        assert_eq!(sensor.ec, 1.5);
        assert_eq!(sensor.ph, 6.2);
    }

    #[test]
    fn test_sensor_noise_perturbs_reading_and_is_seed_reproducible() {
        let tank = Tank {
            volume_l: 10.0,
            ec: 1.5,
            ph: 6.2,
            temp: 24.5,
            water_level: 40.0,
        };
        let cfg = NoiseConfig {
            ec_noise_std_dev: 0.05,
            ph_noise_std_dev: 0.02,
            seed: 42,
        };

        let mut rng_a = StdRng::seed_from_u64(cfg.seed);
        let sensor_a = read_sensor(&tank, &cfg, &mut rng_a);

        let mut rng_b = StdRng::seed_from_u64(cfg.seed);
        let sensor_b = read_sensor(&tank, &cfg, &mut rng_b);

        assert_eq!(
            sensor_a.ec, sensor_b.ec,
            "same seed must reproduce identical noise for a fresh reading"
        );
        assert_ne!(
            sensor_a.ec, tank.ec,
            "non-zero std dev must perturb the raw tank EC reading"
        );
        assert_ne!(
            sensor_a.ph, tank.ph,
            "non-zero std dev must perturb the raw tank pH reading"
        );
    }
}
