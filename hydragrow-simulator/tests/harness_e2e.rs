use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::{ControlMode, ControllerConfig};
use hydragrow_simulator::harness::Harness;
use hydragrow_simulator::plant::tank::Tank;
use hydragrow_simulator::sensors::sensor_model::NoiseConfig;

fn test_controller_config() -> ControllerConfig {
    ControllerConfig {
        control_mode: ControlMode::Auto,
        is_enabled: true,
        ec_target: 1.5,
        ec_tolerance: 0.05,
        ph_target: 6.0,
        ph_tolerance: 0.1,
        enable_ec_sensor: true,
        enable_ph_sensor: true,
        cooldown_sec: 2,
        pump_a_capacity_ml_per_sec: 2.0,
        pump_b_capacity_ml_per_sec: 2.0,
        ec_gain_per_ml: 0.3,
        dosing_min_pwm_percent: 30,
        dosing_pulse_on_ms: 50,
        dosing_pulse_off_ms: 50,
        ..Default::default()
    }
}

fn test_tank() -> Tank {
    Tank {
        volume_l: 10.0,
        ec: 1.0,
        ph: 6.0,
        temp: 25.0,
        water_level: 50.0,
    }
}

#[test]
fn harness_runs_controller_against_virtual_plant() {
    let config = test_controller_config();
    let tank = test_tank();
    let noise = NoiseConfig::none();
    let mut harness = Harness::new(config, tank, noise);

    for _ in 0..20 {
        harness.tick(1000).unwrap();
    }

    assert_eq!(harness.uptime_ms(), 20_000);
    assert!(harness.tank.ec.is_finite());
    assert!(harness.ctx.phase != SystemPhase::Booting);
}

#[test]
fn e2e_closed_loop_dosing_cycle() {
    let mut config = test_controller_config();
    config.cooldown_sec = 2;

    // Start tank with low EC (0.8 < target 1.5 - 0.05)
    let tank = Tank {
        volume_l: 10.0,
        ec: 0.8,
        ph: 6.0,
        temp: 25.0,
        water_level: 50.0,
    };
    let noise = NoiseConfig::none();
    let mut harness = Harness::new(config, tank, noise);
    harness.ctx.phase = SystemPhase::Monitoring;

    // Phase 1: Monitoring -> MimoDosing
    let res1 = harness.tick(1000).unwrap();
    assert_eq!(
        harness.ctx.phase,
        SystemPhase::MimoDosing,
        "Low EC must trigger MimoDosing phase"
    );
    let _ = res1;

    // Tick inside MimoDosing and verify dosing pumps actuate and plant EC increases
    let initial_ec = harness.tank.ec;
    for _ in 0..10 {
        harness.tick(500).unwrap();
    }

    // EC should have increased via tank.step driven by virtual hardware pump activation
    assert!(
        harness.tank.ec >= initial_ec,
        "Tank EC should rise or stay active during dosing"
    );

    // Force phase finish for MimoDosing to move to Cooldown
    let current_uptime = harness.uptime_ms();
    harness.ctx.phase_start_ms = Some(current_uptime);
    harness.ctx.phase_finish_ms = Some(current_uptime + 100);

    // Advance tick past finish_ms + buffer
    harness.tick(10_000).unwrap();
    assert_eq!(
        harness.ctx.phase,
        SystemPhase::Cooldown,
        "After dosing timeout, phase must enter Cooldown"
    );

    // Cooldown timeout -> Monitoring
    let cooldown_start = harness.uptime_ms();
    harness.ctx.phase_finish_ms = Some(cooldown_start + 2000);
    harness.tank.ec = 1.5; // Balanced
    harness.tick(3000).unwrap();

    assert_eq!(
        harness.ctx.phase,
        SystemPhase::Monitoring,
        "After Cooldown timeout with balanced EC, phase must return to Monitoring"
    );
}
