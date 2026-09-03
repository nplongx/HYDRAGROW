use hydragrow_shared::ControllerConfig;
use hydragrow_simulator::harness::Harness;
use hydragrow_simulator::plant::tank::Tank;
use hydragrow_simulator::sensors::sensor_model::NoiseConfig;

#[test]
fn test_dosing_cycle_snapshot() {
    let config = ControllerConfig {
        ec_gain_per_ml: 0.5,
        pump_a_capacity_ml_per_sec: 2.0,
        ..Default::default()
    };

    let tank = Tank {
        volume_l: 10.0,
        ec: 1.0,
        ph: 6.0,
        temp: 25.0,
        water_level: 50.0,
    };
    let noise = NoiseConfig::none();
    let mut harness = Harness::new(config, tank, noise);

    let mut history = vec![];
    for _ in 0..10 {
        harness.hw.pump_a.on = true;
        harness.hw.pump_a.pwm_percent = 100;
        harness.tick(1000); // 1 second per tick
        history.push(harness.tank.ec);
    }

    insta::assert_json_snapshot!(history);
}
