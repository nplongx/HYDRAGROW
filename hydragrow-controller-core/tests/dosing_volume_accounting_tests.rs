//! Regression tests for delivered-volume accounting and Pump B start semantics

#![allow(clippy::field_reassign_with_default)]

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors};

use hydragrow_controller_core::core::actors::dosing_actor::{
    DosingActor, DosingEvent, DosingSubState,
};
use hydragrow_controller_core::core::adaptive::matrix::ControlVector;
use hydragrow_controller_core::core::fsm::events::{DosingPumpTarget, OrchestratorEvent};

#[test]
fn pump_a_on_event_does_not_increment_delivered_ml_before_pulse_finishes() {
    let mut actor = DosingActor::new();
    let mut config = auto_config();
    config.soft_start_duration = 0;
    config.dosing_min_dose_ml = 5.0;
    config.dosing_pulse_on_ms = 100;
    config.dosing_pulse_off_ms = 100;
    config.pump_a_capacity_ml_per_sec = 1.0;

    let sensors = balanced_sensors();
    let mut control = ControlVector::default();
    control.nutrient_a_ml = 1.0;

    let _ = actor.start_matrix_cycle(0, &control, 1.5, 6.0, 80, &config, &sensors);

    // First tick: completes SoftStarting (duration 0)
    let (event, _) = actor.tick(0, &config);
    assert_eq!(event, DosingEvent::SoftStartDone);

    // Second tick: starts PumpingA and emits ON event
    let (_event, hw_events) = actor.tick(0, &config);
    let has_pump_a_on = hw_events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: true,
                ..
            }
        )
    });
    assert!(has_pump_a_on, "Must emit SetDosingPump A on=true");

    // Delivered volume MUST still be 0.0 while pump is on / in-flight!
    if let DosingSubState::PumpingA(ref job) = actor.sub_state {
        assert_eq!(
            job.delivered_ml, 0.0,
            "delivered_ml must NOT be incremented upon ON event before pulse completes"
        );
    } else {
        panic!("Actor should be in PumpingA state");
    }

    // Second tick at on_ms: pulse completes, turning pump OFF
    let (_event, hw_events) = actor.tick(100, &config);
    let has_pump_a_off = hw_events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: false,
                ..
            }
        )
    });
    assert!(has_pump_a_off, "Must emit SetDosingPump A on=false");

    if let DosingSubState::PumpingA(ref job) = actor.sub_state {
        assert!(
            job.delivered_ml > 0.0,
            "delivered_ml must be credited after pulse completes"
        );
        assert!((job.delivered_ml - job.ml_per_sec * 0.1).abs() < 1e-3);
    } else {
        panic!("Actor should remain in PumpingA state for next pulse");
    }
}

#[test]
fn pump_ph_on_event_does_not_increment_delivered_ml_before_pulse_finishes() {
    let mut actor = DosingActor::new();
    let mut config = auto_config();
    config.soft_start_duration = 0;
    config.dosing_min_dose_ml = 5.0;
    config.dosing_pulse_on_ms = 100;
    config.dosing_pulse_off_ms = 100;
    config.pump_ph_down_capacity_ml_per_sec = 1.0;

    let sensors = balanced_sensors();
    let mut control = ControlVector::default();
    control.ph_down_ml = 1.0;

    let _ = actor.start_matrix_cycle(0, &control, 1.5, 6.0, 80, &config, &sensors);

    // First tick: completes SoftStarting (duration 0)
    let (event, _) = actor.tick(0, &config);
    assert_eq!(event, DosingEvent::SoftStartDone);

    // Second tick: starts PumpingPH and emits ON event
    let (_event, hw_events) = actor.tick(0, &config);
    let has_ph_on = hw_events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhDown,
                on: true,
                ..
            }
        )
    });
    assert!(has_ph_on, "Must emit SetDosingPump PhDown on=true");

    // Delivered volume MUST still be 0.0 while pump is on / in-flight!
    if let DosingSubState::PumpingPH(ref job) = actor.sub_state {
        assert_eq!(
            job.delivered_ml, 0.0,
            "delivered_ml must NOT be incremented upon ON event before pulse completes"
        );
    } else {
        panic!("Actor should be in PumpingPH state");
    }

    // Second tick at on_ms: pulse completes, turning pump OFF
    let (_event, hw_events) = actor.tick(100, &config);
    let has_ph_off = hw_events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhDown,
                on: false,
                ..
            }
        )
    });
    assert!(has_ph_off, "Must emit SetDosingPump PhDown on=false");

    if let DosingSubState::PumpingPH(ref job) = actor.sub_state {
        assert!(
            job.delivered_ml > 0.0,
            "delivered_ml must be credited after pulse completes"
        );
        assert!((job.delivered_ml - job.ml_per_sec * 0.1).abs() < 1e-3);
    } else {
        panic!("Actor should remain in PumpingPH state for next pulse");
    }
}

#[test]
fn pump_b_direct_start_keeps_delivered_ml_zero_during_on_pulse() {
    let mut actor = DosingActor::new();
    let mut config = auto_config();
    config.soft_start_duration = 0;
    config.dosing_pulse_on_ms = 100;
    config.dosing_pulse_off_ms = 100;
    config.pump_b_capacity_ml_per_sec = 1.0;

    let sensors = balanced_sensors();
    let mut control = ControlVector::default();
    control.nutrient_a_ml = 0.0;
    control.nutrient_b_ml = 1.0;

    let _ = actor.start_matrix_cycle(0, &control, 1.5, 6.0, 80, &config, &sensors);

    // First tick: completes SoftStarting (duration 0)
    let (event, _) = actor.tick(0, &config);
    assert_eq!(event, DosingEvent::SoftStartDone);

    // Second tick: starts PumpingB and emits ON event
    let (_event, hw_events) = actor.tick(0, &config);
    let has_pump_b_on = hw_events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientB,
                on: true,
                ..
            }
        )
    });
    assert!(has_pump_b_on, "Must emit SetDosingPump B on=true");

    if let DosingSubState::PumpingB(ref job) = actor.sub_state {
        assert_eq!(
            job.delivered_ml, 0.0,
            "delivered_ml must remain 0.0 while pump B pulse is in flight"
        );
    } else {
        panic!("Actor should be in PumpingB state");
    }

    // Hardware fault / cancel resets actor cleanly
    actor.reset();
    assert!(actor.is_idle());
}
