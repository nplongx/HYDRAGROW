//! Regression tests for resetting active actors and peripheral ownership on terminal boundaries:
//! - Sensor fault / timeout
//! - SensorCalibration entry
//! - ManualMode disable / transition

#![allow(clippy::field_reassign_with_default)]

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors};

use hydragrow_controller_core::core::actors::dosing_actor::{DosingSubState, PulseJob, PumpTarget};
use hydragrow_controller_core::core::actors::water_actor::{WaterJob, WaterSubState};
use hydragrow_controller_core::core::fsm::{context::SystemContext, orchestrator};
use hydragrow_shared::ControlMode;
use hydragrow_shared::fsm::{FaultCode, SystemPhase};

fn dummy_pulse_job() -> PulseJob {
    PulseJob {
        pump: PumpTarget::NutrientA { dose_b_ml: 10.0 },
        target_ml: 10.0,
        delivered_ml: 2.0,
        pulse_on: true,
        pulse_count: 1,
        max_pulses: 5,
        on_ms: 1000,
        off_ms: 1000,
        pwm: 80,
        ml_per_sec: 1.0,
        next_toggle_ms: 5000,
    }
}

fn dummy_water_job() -> WaterJob {
    WaterJob {
        trigger: "test_refill".to_string(),
        target_level: 20.0,
        start_level: 15.0,
        start_ms: 1000,
    }
}

#[test]
fn sensor_fault_timeout_resets_active_dosing_actor_and_ownership() {
    let config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = SystemContext::default();

    ctx.phase = SystemPhase::MimoDosing;
    ctx.dosing.sub_state = DosingSubState::PumpingA(dummy_pulse_job());
    ctx.peripherals.misting_started_by_dosing = true;
    ctx.peripherals.mix_valve_started_by_dosing = true;

    // Simulate sensor timeout (> 90s)
    let uptime_ms = 100_000u64;
    let sensor_last_update_ms = 0u64;

    let mut result = orchestrator::tick(
        100_000,
        uptime_ms,
        &config,
        &sensors,
        sensor_last_update_ms,
        &mut ctx,
    );
    ctx.apply_delta(&mut result.delta);

    assert_eq!(
        ctx.phase,
        SystemPhase::Fault(FaultCode::SensorTimeout),
        "FSM must enter Fault(SensorTimeout)"
    );
    assert_eq!(
        ctx.dosing.sub_state,
        DosingSubState::Idle,
        "DosingActor must be reset to Idle on fault boundary"
    );
    assert!(
        !ctx.peripherals.misting_started_by_dosing,
        "misting_started_by_dosing ownership must be cleared"
    );
    assert!(
        !ctx.peripherals.mix_valve_started_by_dosing,
        "mix_valve_started_by_dosing ownership must be cleared"
    );
}

#[test]
fn calibration_entry_resets_active_dosing_and_water_actors_and_ownership() {
    let mut ctx = SystemContext::default();

    ctx.phase = SystemPhase::MimoDosing;
    ctx.dosing.sub_state = DosingSubState::PumpingA(dummy_pulse_job());
    ctx.water.sub_state = WaterSubState::Filling {
        job: dummy_water_job(),
    };
    ctx.peripherals.misting_started_by_dosing = true;
    ctx.peripherals.mix_valve_started_by_dosing = true;
    ctx.peripherals.is_misting_active = true;
    ctx.peripherals.is_scheduled_mixing_active = true;

    ctx.reset_active_actors_and_ownership();

    assert_eq!(
        ctx.dosing.sub_state,
        DosingSubState::Idle,
        "DosingActor must be reset to Idle"
    );
    assert_eq!(
        ctx.water.sub_state,
        WaterSubState::Idle,
        "WaterActor must be reset to Idle"
    );
    assert!(!ctx.peripherals.misting_started_by_dosing);
    assert!(!ctx.peripherals.mix_valve_started_by_dosing);
    assert!(!ctx.peripherals.is_misting_active);
    assert!(!ctx.peripherals.is_scheduled_mixing_active);
}

#[test]
fn manual_mode_transition_resets_active_actors_and_ownership() {
    let mut config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = SystemContext::default();

    ctx.phase = SystemPhase::MimoDosing;
    ctx.dosing.sub_state = DosingSubState::PumpingA(dummy_pulse_job());
    ctx.peripherals.misting_started_by_dosing = true;
    ctx.peripherals.mix_valve_started_by_dosing = true;

    // Switch config to Manual
    config.control_mode = ControlMode::Manual;

    let mut result = orchestrator::tick(1000, 1000, &config, &sensors, 1000, &mut ctx);
    ctx.apply_delta(&mut result.delta);

    assert_eq!(ctx.phase, SystemPhase::ManualMode);
    assert_eq!(
        ctx.dosing.sub_state,
        DosingSubState::Idle,
        "DosingActor must be reset to Idle on transition to ManualMode"
    );
    assert!(
        !ctx.peripherals.misting_started_by_dosing,
        "Ownership must be cleared on transition to ManualMode"
    );

    // Re-enable Auto: Must not resume previous job
    config.control_mode = ControlMode::Auto;
    let mut result2 = orchestrator::tick(2000, 2000, &config, &sensors, 2000, &mut ctx);
    ctx.apply_delta(&mut result2.delta);

    assert_eq!(ctx.phase, SystemPhase::Monitoring);
    assert_eq!(
        ctx.dosing.sub_state,
        DosingSubState::Idle,
        "DosingActor must remain Idle after re-enabling Auto"
    );
}
