//! Tests for SensorCalibration phase lifecycle and timeout

#![allow(clippy::field_reassign_with_default)]

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors};

use hydragrow_controller_core::core::actors::dosing_actor::{
    DosingSubState, PulseJob, PumpTarget,
};
use hydragrow_controller_core::core::actors::water_actor::{WaterJob, WaterSubState};
use hydragrow_controller_core::core::fsm::types::PendingCalibrationSample;
use hydragrow_controller_core::core::fsm::{context::SystemContext, orchestrator};
use hydragrow_shared::fsm::SystemPhase;

fn dummy_calibration_sample() -> PendingCalibrationSample {
    PendingCalibrationSample {
        cycle_id: "test_cycle".to_string(),
        trigger: "test".to_string(),
        start_ec: 1.2,
        start_ph: 6.0,
        start_water_level: 20.0,
        start_temp: 24.0,
        target_ec: 1.5,
        target_ph: 6.2,
        dose_a_ml: 1.0,
        dose_b_ml: 1.0,
        dose_ph_up_ml: 0.0,
        dose_ph_down_ml: 0.0,
        water_in_sec: 0.0,
        water_out_sec: 0.0,
        post_mixing_ec: 0.0,
        post_mixing_ph: 0.0,
        start_ms: 0,
        active_mixing_finish_ms: 0,
        stabilizing_start_ms: None,
        stabilizing_finish_ms: None,
        invalid_by_noise: false,
        invalid_by_water_change: false,
    }
}

#[test]
fn sensor_calibration_timeout_leaves_calibration_safely() {
    let config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = SystemContext::default();

    ctx.phase = SystemPhase::SensorCalibration;
    ctx.phase_finish_ms = Some(1000);
    ctx.calibration.pending_sample = Some(dummy_calibration_sample());

    let now_ms = 1001u64;
    let uptime_ms = 1001u64;
    let sensor_last_update_ms = 1001u64;

    let mut result = orchestrator::tick(
        now_ms,
        uptime_ms,
        &config,
        &sensors,
        sensor_last_update_ms,
        &mut ctx,
    );
    ctx.apply_delta(&mut result.delta);

    assert_eq!(
        ctx.phase,
        SystemPhase::Monitoring,
        "SensorCalibration must transition to Monitoring on timeout"
    );
    assert_eq!(ctx.phase_finish_ms, None, "phase_finish_ms must be cleared");
    assert!(
        ctx.calibration.pending_sample.is_none(),
        "Pending calibration sample must be cleared on calibration exit"
    );
}

#[test]
fn sensor_calibration_cleanup_clears_peripherals_actors_and_ownership() {
    let mut ctx = SystemContext::default();

    ctx.dosing.sub_state = DosingSubState::PumpingA(PulseJob {
        pump: PumpTarget::NutrientA { dose_b_ml: 5.0 },
        target_ml: 5.0,
        delivered_ml: 1.0,
        pulse_on: true,
        pulse_count: 1,
        max_pulses: 5,
        on_ms: 1000,
        off_ms: 1000,
        pwm: 80,
        ml_per_sec: 1.0,
        next_toggle_ms: 5000,
    });
    ctx.water.sub_state = WaterSubState::Filling {
        job: WaterJob {
            trigger: "test".to_string(),
            target_level: 20.0,
            start_level: 15.0,
            start_ms: 1000,
        },
    };
    ctx.peripherals.osaka_pwm = 80;
    ctx.peripherals.pump_status.osaka_pwm = Some(80);
    ctx.peripherals.pump_status.mist_valve = true;
    ctx.peripherals.pump_status.mix_valve = true;
    ctx.peripherals.pump_status.water_pump_in = true;
    ctx.peripherals.pump_status.water_pump_out = true;
    ctx.peripherals.misting_started_by_dosing = true;
    ctx.peripherals.mix_valve_started_by_dosing = true;
    ctx.calibration.pending_sample = Some(dummy_calibration_sample());

    // Reset actors, ownership, and pending calibration state
    ctx.reset_active_actors_and_ownership();

    assert_eq!(ctx.dosing.sub_state, DosingSubState::Idle);
    assert_eq!(ctx.water.sub_state, WaterSubState::Idle);
    assert!(!ctx.peripherals.misting_started_by_dosing);
    assert!(!ctx.peripherals.mix_valve_started_by_dosing);
    assert!(
        ctx.calibration.pending_sample.is_none(),
        "Pending calibration sample must be cleared on reset_active_actors_and_ownership"
    );
}
