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
