//! Tests for explicit WaterActor result semantics and duration propagation

#![allow(clippy::field_reassign_with_default)]

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors};

use hydragrow_controller_core::core::actors::water_actor::{WaterJob, WaterSubState};
use hydragrow_controller_core::core::fsm::tick_result::CalibrationDelta;
use hydragrow_controller_core::core::fsm::{context::SystemContext, orchestrator};
use hydragrow_shared::fsm::SystemPhase;

#[test]
fn water_failure_in_mimo_does_not_advance_to_active_mixing() {
    let config = auto_config();
    let mut sensors = balanced_sensors();
    sensors.water_level = 15.0;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::MimoDosing;
    ctx.phase_start_ms = Some(1000);
    ctx.phase_finish_ms = Some(500_000);
    ctx.peripherals.pump_status.water_pump_in = true;
    ctx.peripherals.water_pump_started_uptime_ms = Some(1000);

    let max_duration_ms = config.max_refill_duration_sec as u64 * 1000;
    ctx.water.sub_state = WaterSubState::Filling {
        job: WaterJob {
            trigger: "test_refill".to_string(),
            target_level: 20.0,
            start_level: 15.0,
            start_ms: 1000,
            max_duration_sec: None,
        },
    };

    let failure_uptime_ms = 1000 + max_duration_ms + 1000;
    let result = orchestrator::tick(
        failure_uptime_ms,
        failure_uptime_ms,
        &config,
        &sensors,
        failure_uptime_ms,
        &mut ctx,
    );

    assert_ne!(
        result.delta.phase,
        Some(SystemPhase::ActiveMixing),
        "Failed water refill must never advance to ActiveMixing"
    );
}

#[test]
fn water_actual_duration_is_propagated_to_calibration_sample() {
    let config = auto_config();
    let mut sensors = balanced_sensors();
    sensors.water_level = 20.0;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::MimoDosing;
    ctx.phase_start_ms = Some(10_000);
    ctx.phase_finish_ms = Some(500_000);
    ctx.peripherals.pump_status.water_pump_in = true;
    ctx.peripherals.water_pump_started_uptime_ms = Some(10_000);

    ctx.water.sub_state = WaterSubState::Filling {
        job: WaterJob {
            trigger: "test_refill".to_string(),
            target_level: 20.0,
            start_level: 15.0,
            start_ms: 10_000,
            max_duration_sec: None,
        },
    };

    let current_uptime = 25_000u64;
    let result = orchestrator::tick(
        current_uptime,
        current_uptime,
        &config,
        &sensors,
        current_uptime,
        &mut ctx,
    );

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::ActiveMixing),
        "Successful water refill must transition to ActiveMixing"
    );

    if let Some(CalibrationDelta::Start(sample)) = result.delta.calibration {
        assert_eq!(
            sample.water_in_sec, 15.0,
            "Calibration sample must record actual elapsed water duration (15s), not configured maximum ({})",
            config.max_refill_duration_sec
        );
    } else {
        panic!("Expected CalibrationDelta::Start in result");
    }
}

#[test]
fn cycle_complete_with_active_water_records_actual_elapsed_seconds() {
    let config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::MimoDosing;
    ctx.phase_start_ms = Some(10_000);
    ctx.peripherals.pump_status.water_pump_in = true;
    ctx.peripherals.water_pump_started_uptime_ms = Some(10_000);

    ctx.dosing.sub_state =
        hydragrow_controller_core::core::actors::dosing_actor::DosingSubState::PumpingPH(
            hydragrow_controller_core::core::actors::dosing_actor::PulseJob {
                pump: hydragrow_controller_core::core::actors::dosing_actor::PumpTarget::PhUp,
                target_ml: 1.0,
                delivered_ml: 1.0,
                pulse_on: true,
                pulse_count: 1,
                max_pulses: 5,
                on_ms: 100,
                off_ms: 100,
                pwm: 80,
                ml_per_sec: 1.0,
                next_toggle_ms: 100,
            },
        );

    let current_uptime = 18_000u64;
    let result = orchestrator::tick(
        current_uptime,
        current_uptime,
        &config,
        &sensors,
        current_uptime,
        &mut ctx,
    );

    assert_eq!(result.delta.phase, Some(SystemPhase::ActiveMixing));
    if let Some(CalibrationDelta::Start(sample)) = result.delta.calibration {
        assert_eq!(
            sample.water_in_sec, 8.0,
            "CycleComplete must record actual elapsed duration (8s), not max_refill_duration_sec ({})",
            config.max_refill_duration_sec
        );
    } else {
        panic!("Expected CalibrationDelta::Start in result");
    }
}

#[test]
fn water_watchdog_timeout_aborts_actor_and_prevents_subsequent_on() {
    let mut config = auto_config();
    config.max_refill_duration_sec = 10;
    let mut sensors = balanced_sensors();
    sensors.water_level = 10.0;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::MimoDosing;
    ctx.phase_start_ms = Some(1000);
    ctx.phase_finish_ms = Some(100_000);
    ctx.peripherals.pump_status.water_pump_in = true;
    ctx.peripherals.water_pump_started_uptime_ms = Some(1000);

    ctx.water.sub_state = WaterSubState::Filling {
        job: WaterJob {
            trigger: "test_refill".to_string(),
            target_level: 25.0,
            start_level: 10.0,
            start_ms: 1000,
            max_duration_sec: Some(10),
        },
    };

    // Tick 1: trigger watchdog timeout (elapsed 11s >= 10s)
    let t1 = 12_000u64;
    let mut r1 = orchestrator::tick(t1, t1, &config, &sensors, t1, &mut ctx);
    ctx.apply_delta(&mut r1.delta);

    // Actor must be aborted/idle after watchdog timeout
    assert_eq!(
        ctx.water.sub_state,
        WaterSubState::Idle,
        "Water actor must be Idle after watchdog timeout"
    );
    assert!(
        !ctx.peripherals.pump_status.water_pump_in,
        "Shadow water_pump_in must be false"
    );

    // Tick 2: Subsequent tick must NOT re-emit SetWaterPump(In)
    let t2 = 13_000u64;
    let r2 = orchestrator::tick(t2, t2, &config, &sensors, t2, &mut ctx);
    assert!(
        !r2.events.iter().any(|e| matches!(
            e,
            hydragrow_controller_core::core::fsm::events::OrchestratorEvent::SetWaterPump {
                direction: hydragrow_controller_core::WaterDirection::In
            }
        )),
        "Tick after timeout must NOT emit SetWaterPump(In)"
    );
}

#[test]
fn water_refill_phase_timeout_faults_with_water_refill_failed() {
    let mut config = auto_config();
    config.max_refill_duration_sec = 5;
    let mut sensors = balanced_sensors();
    sensors.water_level = 10.0;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::WaterRefilling;
    ctx.water
        .start_fill_with_duration(1000, 25.0, &sensors, "test", Some(5));

    // Tick at 1000 + 5001ms -> times out
    let t = 6001u64;
    let r = orchestrator::tick(t, t, &config, &sensors, t, &mut ctx);
    assert_eq!(
        r.delta.phase,
        Some(SystemPhase::Fault(
            hydragrow_shared::fsm::FaultCode::WaterRefillFailed
        )),
        "WaterRefilling timeout must transition to Fault(WaterRefillFailed)"
    );
}

#[test]
fn water_drain_phase_timeout_faults_with_water_drain_failed() {
    let mut config = auto_config();
    config.max_drain_duration_sec = 5;
    let mut sensors = balanced_sensors();
    sensors.water_level = 30.0;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::WaterDraining;
    ctx.water
        .start_drain_with_duration(1000, 10.0, &sensors, "test", Some(5));

    // Tick at 1000 + 5001ms -> times out
    let t = 6001u64;
    let r = orchestrator::tick(t, t, &config, &sensors, t, &mut ctx);
    assert_eq!(
        r.delta.phase,
        Some(SystemPhase::Fault(
            hydragrow_shared::fsm::FaultCode::WaterDrainFailed
        )),
        "WaterDraining timeout must transition to Fault(WaterDrainFailed)"
    );
}
