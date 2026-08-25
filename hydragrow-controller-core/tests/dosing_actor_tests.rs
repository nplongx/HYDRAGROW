#![allow(clippy::field_reassign_with_default)]

//! Tests cho DosingActor: pulse sequencing, delivery tracking, idle detection

mod helpers;
use helpers::fixtures::auto_config;

use hydragrow_controller_core::core::actors::dosing_actor::{DosingActor, DosingEvent};
use hydragrow_controller_core::core::adaptive::matrix::ControlVector;
use hydragrow_shared::SensorData;

fn make_sensors() -> SensorData {
    SensorData {
        device_id: "test".to_string(),
        ec: 1.2,
        ph: 5.9,
        temp: 25.0,
        water_level: 20.0,
        pump_status: Default::default(),
        time: "2026-08-25T10:00:00Z".to_string(),
        controller_received_ms: None,
        rssi: None,
        free_heap: None,
        uptime: None,
        err_water: None,
        err_temp: None,
        err_ec: None,
        err_ph: None,
        is_continuous: None,
        ph_voltage_mv: None,
    }
}

// Test 1: DosingActor ban đầu là idle
#[test]
fn dosing_actor_starts_idle() {
    let actor = DosingActor::new();
    assert!(actor.is_idle(), "DosingActor mới tạo phải idle");
}

// Test 2: start_matrix_cycle với dose 0 → vẫn idle sau tick
#[test]
fn dosing_cycle_with_zero_dose_stays_idle() {
    let mut actor = DosingActor::new();
    let config = auto_config();
    let sensors = make_sensors();

    let control = ControlVector::default(); // tất cả 0

    actor.start_matrix_cycle(
        1000, // uptime_ms
        &control, 1.5, // target_ec
        6.0, // target_ph
        80,  // pwm
        &config, &sensors,
    );

    // Sau cycle với 0ml, vẫn idle
    assert!(actor.is_idle(), "Zero dose không nên bắt đầu pumping state");
}

// Test 3: Dosing cycle với dose > 0 → không idle ngay sau start
#[test]
fn dosing_cycle_with_nonzero_dose_not_idle_immediately() {
    let mut actor = DosingActor::new();
    let config = auto_config();
    let sensors = make_sensors();

    let mut control = ControlVector::default();
    control.nutrient_a_ml = 2.0;

    actor.start_matrix_cycle(1000, &control, 1.5, 6.0, 80, &config, &sensors);

    // Ngay sau start, có thể đang soft start (không idle)
    // Tick một lần
    let (event, _hw_events) = actor.tick(1200, &config); // 200ms sau start

    // Không được immediately done
    assert_ne!(
        event,
        DosingEvent::CycleComplete {
            dose_a_ml: 0.0,
            dose_b_ml: 0.0,
            ph_up_ml: 0.0,
            ph_down_ml: 0.0,
        },
        "Cycle vừa bắt đầu không thể hoàn thành ngay"
    );
}

// Test 4: Tick nhiều lần đến khi done (test với dose nhỏ)
#[test]
fn dosing_cycle_eventually_completes() {
    let mut actor = DosingActor::new();
    let mut config = auto_config();
    config.soft_start_duration = 0; // skip soft start cho test nhanh
    config.dosing_pulse_on_ms = 50;
    config.dosing_pulse_off_ms = 50;
    config.pump_a_capacity_ml_per_sec = 2.0;

    let sensors = make_sensors();

    let mut control = ControlVector::default();
    control.nutrient_a_ml = 0.2; // dose nhỏ
    actor.start_matrix_cycle(0, &control, 1.5, 6.0, 80, &config, &sensors);

    let mut completed = false;
    let mut current_ms = 0u64;

    for _ in 0..1000 {
        current_ms += 100;
        let (event, _) = actor.tick(current_ms, &config);
        if matches!(event, DosingEvent::CycleComplete { .. }) {
            completed = true;
            break;
        }
    }

    assert!(completed, "Dosing cycle phải hoàn thành sau đủ số tick");
}

// Test 5: Tick khi idle → trả về Pending, không crash
#[test]
fn tick_when_idle_returns_pending_safely() {
    let mut actor = DosingActor::new();
    let config = auto_config();

    let (event, hw_events) = actor.tick(1000, &config);

    assert_eq!(
        event,
        DosingEvent::Pending,
        "Tick khi idle phải trả về Pending"
    );
    assert!(
        hw_events.is_empty(),
        "Tick khi idle không emit hardware events"
    );
}
