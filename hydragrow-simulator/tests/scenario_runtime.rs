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
