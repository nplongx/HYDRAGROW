use hydragrow_shared::{PumpStatus, SensorData};
use hydragrow_simulator::faults::injector::Injector;
use hydragrow_simulator::scenario::engine::ScenarioEngine;
use hydragrow_simulator::scenario::format::{FaultEvent, FaultEventKind, InitialTank, Scenario};

fn sample_tank() -> InitialTank {
    InitialTank {
        volume_l: 10.0,
        ec: 1.2,
        ph: 6.0,
        temp: 22.0,
        water_level: 50.0,
    }
}

fn sample_sensor(ec: f32) -> SensorData {
    SensorData {
        device_id: "test_device".into(),
        ec,
        ph: 6.0,
        temp: 22.0,
        water_level: 50.0,
        pump_status: PumpStatus::default(),
        time: "2026-03-31T00:00:00Z".into(),
        controller_received_ms: None,
        rssi: None,
        free_heap: None,
        uptime: None,
        err_water: None,
        err_temp: None,
        err_ph: None,
        err_ec: None,
        is_continuous: None,
        ph_voltage_mv: None,
        ec_received_ms: None,
        ph_received_ms: None,
        temp_received_ms: None,
        water_received_ms: None,
    }
}

#[test]
fn a_fault_activates_once_when_simulated_time_crosses_at_ms() {
    let scenario = Scenario {
        initial_tank: sample_tank(),
        faults: vec![FaultEvent {
            at_ms: 5000,
            kind: FaultEventKind::PumpStuckOn {
                pump: "PUMP_A".into(),
            },
        }],
    };
    let mut engine = ScenarioEngine::new(scenario);
    assert!(engine.activate_between(0, 1000).is_empty());
    assert_eq!(engine.activate_between(4000, 5000).len(), 1);
    assert!(engine.activate_between(5000, 6000).is_empty());
}

#[test]
fn sensor_frozen_fault_reuses_the_activation_sample() {
    let mut injector = Injector::new();
    injector.add_active_fault(FaultEventKind::SensorFrozen {
        sensor: "EC".into(),
    });
    let mut first = sample_sensor(1.2);
    injector.apply_sensor_faults(&mut first);
    let frozen = first.ec;
    let mut later = sample_sensor(1.8);
    injector.apply_sensor_faults(&mut later);
    assert_eq!(later.ec, frozen);
}

#[test]
fn simulator_sensor_timeout_fault_stops_all_actuators_and_recovers() {
    use hydragrow_controller_core::core::fsm::orchestrator;
    use hydragrow_shared::ControllerConfig;
    use hydragrow_shared::fsm::{FaultCode, SystemPhase};
    use hydragrow_simulator::actuators::virtual_hw::VirtualHardwareState;
    use hydragrow_simulator::dispatcher::SimDispatcher;

    let config = ControllerConfig {
        ec_target: 1.5,
        ..Default::default()
    };
    let mut ctx = hydragrow_controller_core::core::fsm::context::SystemContext {
        phase: SystemPhase::Monitoring,
        ..Default::default()
    };
    let mut hw = VirtualHardwareState::default();
    hw.pump_a.on = true;
    hw.pump_a.pwm_percent = 50;
    hw.water_pump_in.on = true;

    let mut dispatcher = SimDispatcher::new();
    let sensor = sample_sensor(1.5);

    // Timeout: 100s elapsed since last sensor update
    let uptime_ms = 100_000u64;
    let sensor_last_update_ms = 0u64;
    let mut result = orchestrator::tick(
        1_700_000_000_000 + uptime_ms,
        uptime_ms,
        &config,
        &sensor,
        sensor_last_update_ms,
        &mut ctx,
    );
    ctx.apply_delta(&mut result.delta);
    for event in &result.events {
        dispatcher.dispatch(event, &mut hw);
    }

    assert_eq!(ctx.phase, SystemPhase::Fault(FaultCode::SensorTimeout));
    // Verify virtual hardware state reflects all off
    assert!(!hw.pump_a.on);
    assert_eq!(hw.pump_a.pwm_percent, 0);
    assert!(!hw.water_pump_in.on);

    // Recovery: fresh sensor update received
    let fresh_sensor_ms = uptime_ms + 1000;
    let mut recover_result = orchestrator::tick(
        1_700_000_000_000 + fresh_sensor_ms,
        fresh_sensor_ms,
        &config,
        &sensor,
        fresh_sensor_ms,
        &mut ctx,
    );
    ctx.apply_delta(&mut recover_result.delta);
    for event in &recover_result.events {
        dispatcher.dispatch(event, &mut hw);
    }

    assert_eq!(ctx.phase, SystemPhase::Monitoring);
}

#[test]
fn required_sensor_fault_scenarios_preserve_explicit_error_semantics() {
    let config = hydragrow_shared::ControllerConfig::default();
    for name in ["sensor_unavailable.json", "invalid_sensor_data.json"] {
        let path = format!("src/scenario/library/{name}");
        let mut harness =
            hydragrow_simulator::harness::Harness::from_scenario(config.clone(), path).unwrap();
        harness.tick(1000).unwrap();
        assert_eq!(harness.last_sensor.err_ec, Some(true), "scenario={name}");
        assert_ne!(harness.last_sensor.ec, 0.0, "scenario={name}");
    }
}

#[test]
fn actuator_fault_scenarios_keep_fault_effect_in_actual_hardware_state() {
    for (scenario_path, desired_on, expected_on, expected_pwm) in [
        (
            "src/scenario/library/actuator_stuck_on.json",
            false,
            true,
            100,
        ),
        (
            "src/scenario/library/actuator_stuck_off.json",
            true,
            false,
            100,
        ),
    ] {
        let scenario = hydragrow_simulator::scenario::format::load_scenario(std::path::Path::new(
            scenario_path,
        ))
        .unwrap();
        let mut engine = hydragrow_simulator::scenario::engine::ScenarioEngine::new(scenario);
        let faults = engine.activate_between(0, 1000);
        let mut injector = hydragrow_simulator::faults::injector::Injector::new();
        for fault in faults {
            injector.add_active_fault(fault);
        }
        let mut hw = hydragrow_simulator::actuators::virtual_hw::VirtualHardwareState::default();
        hw.pump_a.desired_on = desired_on;
        hw.pump_a.desired_pwm_percent = 100;
        hw.pump_a.on = desired_on;
        hw.pump_a.pwm_percent = 100;
        injector.apply_hardware_faults(&mut hw);
        assert_eq!(hw.pump_a.on, expected_on, "scenario={scenario_path}");
        assert_eq!(
            hw.pump_a.pwm_percent, expected_pwm,
            "scenario={scenario_path}"
        );
    }
}

#[test]
fn actuator_delayed_keeps_actual_old_until_exact_delay_boundary() {
    use hydragrow_simulator::actuators::virtual_hw::VirtualHardwareState;
    let mut injector = Injector::new();
    injector.add_active_fault_at(
        FaultEventKind::ActuatorDelayed {
            pump: "PUMP_A".into(),
            delay_ms: 500,
        },
        1000,
    );
    let mut hw = VirtualHardwareState::default();
    hw.pump_a.apply_command(false, 0);
    hw.pump_a.desired_on = true;
    hw.pump_a.desired_pwm_percent = 80;

    injector.apply_hardware_faults_at(&mut hw, 1499);
    assert!(hw.pump_a.desired_on);
    assert!(!hw.pump_a.on);
    assert_eq!(hw.pump_a.pwm_percent, 0);

    injector.apply_hardware_faults_at(&mut hw, 1500);
    assert!(hw.pump_a.on);
    assert_eq!(hw.pump_a.pwm_percent, 80);
    assert_eq!(hw.pump_a.desired_pwm_percent, 80);
}

#[test]
fn actuator_delayed_supports_water_pumps() {
    let mut injector = Injector::new();
    injector.add_active_fault_at(
        FaultEventKind::ActuatorDelayed {
            pump: "WATER_PUMP_IN".into(),
            delay_ms: 500,
        },
        1000,
    );
    let mut hw = hydragrow_simulator::actuators::virtual_hw::VirtualHardwareState::default();
    hw.water_pump_in.apply_command(false, 0);
    hw.water_pump_in.desired_on = true;
    hw.water_pump_in.desired_pwm_percent = 100;

    injector.apply_hardware_faults_at(&mut hw, 1499);
    assert!(!hw.water_pump_in.on);

    injector.apply_hardware_faults_at(&mut hw, 1500);
    assert!(hw.water_pump_in.on);
}

#[test]
fn lifecycle_faults_have_observable_harness_effects() {
    use hydragrow_shared::ControllerConfig;
    use hydragrow_simulator::harness::Harness;
    use hydragrow_simulator::plant::tank::Tank;
    use hydragrow_simulator::sensors::sensor_model::NoiseConfig;

    let config = ControllerConfig::default();
    let mut boot_loop =
        Harness::from_scenario(config.clone(), "src/scenario/library/boot_loop.json").unwrap();
    boot_loop.tick(1000).unwrap();
    assert_eq!(
        boot_loop.controller.lifecycle,
        hydragrow_simulator::controller::ControllerLifecycle::Degraded
    );
    assert!(!boot_loop.hw.pump_a.on);

    let mut config_loss =
        Harness::from_scenario(config, "src/scenario/library/configuration_loss.json").unwrap();
    config_loss.tick(1000).unwrap();
    assert!(!config_loss.configuration_available);
    assert_eq!(
        config_loss.controller.lifecycle,
        hydragrow_simulator::controller::ControllerLifecycle::Degraded
    );
    assert!(!config_loss.hw.pump_a.on);

    let mut restart = Harness::new(
        ControllerConfig::default(),
        Tank::default(),
        NoiseConfig::none(),
    );
    let old_boot_id = restart.controller.boot_id;
    let scenario = Scenario {
        initial_tank: sample_tank(),
        faults: vec![FaultEvent {
            at_ms: 1000,
            kind: FaultEventKind::ControllerRestart,
        }],
    };
    restart.scenario_engine = Some(ScenarioEngine::new(scenario));
    restart.tick(1000).unwrap();
    assert_eq!(restart.controller.boot_id, old_boot_id + 1);
    assert_eq!(restart.uptime_ms(), 0);

    let mut clock = Harness::from_scenario(
        ControllerConfig::default(),
        "src/scenario/library/clock_jump.json",
    )
    .unwrap();
    let wall_before = clock.clock.now_ms();
    clock.tick(1000).unwrap();
    assert_eq!(clock.clock.now_ms(), wall_before + 1000 - 5000);
    assert_eq!(clock.uptime_ms(), 1000);
}

#[test]
fn required_phase_six_scenario_files_are_executable_inputs() {
    for name in [
        "mqtt_reconnect.json",
        "stale_telemetry.json",
        "configuration_sync.json",
        "configuration_sync_restart.json",
        "command_timeout.json",
        "command_retry.json",
        "duplicate_command.json",
    ] {
        let path = std::path::Path::new("src/scenario/library").join(name);
        let scenario = hydragrow_simulator::scenario::format::load_scenario(&path)
            .unwrap_or_else(|e| panic!("scenario {name} invalid: {e}"));
        assert!(
            !scenario.faults.is_empty(),
            "scenario {name} has no executable actions/faults"
        );
    }
}

#[test]
fn controller_restart_resets_runtime_and_changes_boot_id() {
    let mut harness = hydragrow_simulator::harness::Harness::new(
        hydragrow_shared::ControllerConfig::default(),
        hydragrow_simulator::plant::tank::Tank::default(),
        hydragrow_simulator::sensors::sensor_model::NoiseConfig::none(),
    );
    harness.tick(1000).unwrap();
    let old_boot_id = harness.controller.boot_id;
    assert_eq!(harness.uptime_ms(), 1000);

    harness.controller_restart();

    assert_eq!(harness.controller.boot_id, old_boot_id + 1);
    assert_eq!(
        harness.controller.lifecycle,
        hydragrow_simulator::controller::ControllerLifecycle::Boot
    );
    assert_eq!(harness.uptime_ms(), 0);
    assert!(!harness.hw.pump_a.on);
    assert!(!harness.hw.pump_b.on);
    assert!(!harness.hw.pump_ph_up.on);
    assert!(!harness.hw.pump_ph_down.on);
    assert!(!harness.hw.water_pump_in.on);
    assert!(!harness.hw.water_pump_out.on);
    assert!(!harness.hw.mist_valve);
    assert!(!harness.hw.mix_valve);
    assert_eq!(harness.hw.osaka_pwm_percent, 0);
}

#[test]
fn communication_and_timing_faults_are_deterministically_exposed_by_injector() {
    let scenario = Scenario {
        initial_tank: sample_tank(),
        faults: vec![
            FaultEvent {
                at_ms: 1,
                kind: FaultEventKind::MqttDisconnect,
            },
            FaultEvent {
                at_ms: 1,
                kind: FaultEventKind::MqttDrop,
            },
            FaultEvent {
                at_ms: 1,
                kind: FaultEventKind::MqttDuplicate,
            },
            FaultEvent {
                at_ms: 1,
                kind: FaultEventKind::MqttDelay { delay_ms: 25 },
            },
            FaultEvent {
                at_ms: 1,
                kind: FaultEventKind::TelemetryPause,
            },
            FaultEvent {
                at_ms: 1,
                kind: FaultEventKind::TelemetryDelay { delay_ms: 50 },
            },
            FaultEvent {
                at_ms: 1,
                kind: FaultEventKind::TelemetryOutOfOrder,
            },
            FaultEvent {
                at_ms: 1,
                kind: FaultEventKind::BootLoop,
            },
            FaultEvent {
                at_ms: 1,
                kind: FaultEventKind::ConfigurationLoss,
            },
            FaultEvent {
                at_ms: 1,
                kind: FaultEventKind::ClockJump { delta_ms: -100 },
            },
        ],
    };
    let mut engine = ScenarioEngine::new(scenario);
    let faults = engine.activate_between(0, 1);
    let mut injector = Injector::new();
    for fault in faults {
        injector.add_active_fault_at(fault, 1);
    }

    assert!(injector.mqtt_disconnected());
    assert!(injector.mqtt_drop());
    assert!(injector.mqtt_duplicate());
    assert_eq!(injector.mqtt_delay_ms(), Some(25));
    assert!(injector.telemetry_paused());
    assert_eq!(injector.telemetry_delay_ms(), Some(50));
    assert!(injector.telemetry_out_of_order());
    assert!(injector.boot_loop());
    assert!(injector.configuration_lost());
    assert_eq!(injector.clock_jump_ms(), Some(-100));
}

#[test]
fn mqtt_reconnect_clears_disconnect_fault() {
    let mut injector = Injector::new();
    injector.add_active_fault(FaultEventKind::MqttDisconnect);
    assert!(injector.mqtt_disconnected());
    injector.add_active_fault(FaultEventKind::MqttReconnect);
    assert!(!injector.mqtt_disconnected());
}

#[test]
fn virtual_clock_jump_changes_wall_clock_without_changing_uptime() {
    let mut clock = hydragrow_simulator::clock::VirtualClock::new(1_000);
    clock.advance(500);
    clock.jump_wall_clock(-200);
    assert_eq!(clock.now_ms(), 1_300);
    assert_eq!(clock.uptime_ms(), 500);
}

#[test]
fn required_fault_library_files_load_and_validate() {
    for name in [
        "mqtt_disconnect.json",
        "mqtt_delay.json",
        "mqtt_drop.json",
        "mqtt_duplicate.json",
        "actuator_delayed.json",
        "boot_loop.json",
        "telemetry_pause.json",
        "configuration_loss.json",
        "clock_jump.json",
        "telemetry_delay.json",
        "out_of_order_telemetry.json",
    ] {
        let path = std::path::Path::new("src/scenario/library").join(name);
        hydragrow_simulator::scenario::format::load_scenario(&path)
            .unwrap_or_else(|error| panic!("failed to load {name}: {error}"));
    }
}
