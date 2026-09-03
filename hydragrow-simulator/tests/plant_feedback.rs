use hydragrow_shared::ControllerConfig;
use hydragrow_simulator::actuators::virtual_hw::{VirtualHardwareState, VirtualPump};
use hydragrow_simulator::plant::tank::Tank;
use hydragrow_simulator::sensors::sensor_model::{NoiseConfig, read_sensor};

#[test]
fn nutrient_a_flow_uses_config_gain_and_pwm() {
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
        pump_a: VirtualPump { on: true, pwm: 50 },
        ..Default::default()
    };
    tank.step(1000, &hw, &config);
    assert!((tank.ec - 1.05).abs() < 1e-6);
}

#[test]
fn refill_and_drain_change_volume_and_level() {
    let mut tank = Tank {
        volume_l: 10.0,
        ec: 1.0,
        ph: 6.0,
        temp: 25.0,
        water_level: 50.0,
    };
    let config = ControllerConfig {
        tank_height: 100,
        ..Default::default()
    };

    let hw_in = VirtualHardwareState {
        water_pump_in: VirtualPump { on: true, pwm: 100 },
        ..Default::default()
    };
    tank.step(1000, &hw_in, &config);
    assert!(tank.water_level > 50.0);
    assert!(tank.volume_l > 10.0);
    let level_after_in = tank.water_level;
    let vol_after_in = tank.volume_l;

    let hw_out = VirtualHardwareState {
        water_pump_out: VirtualPump { on: true, pwm: 100 },
        ..Default::default()
    };
    tank.step(1000, &hw_out, &config);
    assert!(tank.water_level < level_after_in);
    assert!(tank.volume_l < vol_after_in);

    let hw_max = VirtualHardwareState {
        water_pump_in: VirtualPump { on: true, pwm: 100 },
        ..Default::default()
    };
    for _ in 0..200 {
        tank.step(1000, &hw_max, &config);
    }
    assert_eq!(tank.water_level, 100.0);

    let hw_drain = VirtualHardwareState {
        water_pump_out: VirtualPump { on: true, pwm: 100 },
        ..Default::default()
    };
    for _ in 0..200 {
        tank.step(1000, &hw_drain, &config);
    }
    assert_eq!(tank.water_level, 0.0);
    assert_eq!(tank.volume_l, 0.0);
}

#[test]
fn sensor_noise_is_deterministic_for_a_seeded_config() {
    let tank = Tank {
        volume_l: 10.0,
        ec: 1.5,
        ph: 6.2,
        temp: 24.5,
        water_level: 40.0,
    };
    let cfg = NoiseConfig {
        ec_noise_std_dev: 0.05,
        ph_noise_std_dev: 0.1,
        seed: 42,
    };
    let first = read_sensor(&tank, &cfg);
    let second = read_sensor(&tank, &cfg);
    assert_eq!(first.ec, second.ec);
    assert_eq!(first.ph, second.ph);
    assert_ne!(first.ec, tank.ec);
    assert_ne!(first.ph, tank.ph);
}
