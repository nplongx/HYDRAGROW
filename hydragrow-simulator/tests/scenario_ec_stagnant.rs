use hydragrow_shared::ControllerConfig;
use hydragrow_simulator::harness::Harness;
use hydragrow_simulator::plant::tank::Tank;
use hydragrow_simulator::scenario::format::Scenario;
use hydragrow_simulator::sensors::sensor_model::NoiseConfig;
use std::fs;

#[test]
fn test_ec_stagnant_scenario() {
    let json = fs::read_to_string("src/scenario/library/ec_stagnant.json").unwrap();
    let scenario: Scenario = serde_json::from_str(&json).unwrap();

    let config = ControllerConfig {
        ec_target: 1.5,
        max_ec_delta: 0.5,
        dosing_min_pwm_percent: 50,
        ..Default::default()
    };

    let tank = Tank {
        volume_l: scenario.initial_tank.volume_l,
        ec: scenario.initial_tank.ec,
        ph: scenario.initial_tank.ph,
        temp: scenario.initial_tank.temp,
        water_level: scenario.initial_tank.water_level,
    };
    let noise = NoiseConfig::default();
    let mut harness = Harness::new(config, tank, noise);

    let delta_ms = 1000;

    for _ in 0..15 {
        let next_time = harness.uptime_ms() + delta_ms;
        for fault in &scenario.faults {
            if fault.at_ms <= next_time && fault.at_ms > harness.uptime_ms() {
                harness.injector.add_active_fault(fault.kind.clone());
            }
        }
        harness.tick(delta_ms);
    }

    assert!(harness.hw.pump_a.on);
    assert_eq!(harness.uptime_ms(), 15_000);
}
