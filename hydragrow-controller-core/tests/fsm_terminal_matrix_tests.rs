//! Comprehensive FSM Terminal Transition Regression Matrix (Invariant I1)
//!
//! Verifies that regardless of the initial phase:
//! (Monitoring, MimoDosing, ActiveMixing, Stabilizing, Cooldown, WaterRefilling, WaterDraining, SensorCalibration)
//! transitioning to a terminal condition (Fault, EmergencyStop, ManualMode, Automation Disabled)
//! strictly guarantees:
//! 1. All shadow pump states are false / 0.
//! 2. All active actors (DosingActor, WaterActor) are aborted and reset to Idle.
//! 3. Peripheral ownership flags are cleared.
//! 4. Hardware all-off events are emitted.

#![allow(clippy::field_reassign_with_default)]

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors};

use hydragrow_controller_core::WaterDirection;
use hydragrow_controller_core::core::actors::dosing_actor::{DosingSubState, PulseJob, PumpTarget};
use hydragrow_controller_core::core::actors::water_actor::{WaterJob, WaterSubState};
use hydragrow_controller_core::core::fsm::events::{DosingPumpTarget, OrchestratorEvent};
use hydragrow_controller_core::core::fsm::tick_result::TickResult;
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
        trigger: "matrix_test_job".to_string(),
        target_level: 20.0,
        start_level: 15.0,
        start_ms: 1000,
        max_duration_sec: Some(60),
    }
}

fn setup_active_context(phase: SystemPhase) -> SystemContext {
    let mut ctx = SystemContext::default();
    ctx.phase = phase;

    // Simulate active actors
    ctx.dosing.sub_state = DosingSubState::PumpingA(dummy_pulse_job());
    ctx.water.sub_state = WaterSubState::Filling {
        job: dummy_water_job(),
    };

    // Simulate active peripheral states and ownerships
    ctx.peripherals.misting_started_by_dosing = true;
    ctx.peripherals.mix_valve_started_by_dosing = true;
    ctx.peripherals.is_misting_active = true;
    ctx.peripherals.is_scheduled_mixing_active = true;
    ctx.peripherals.osaka_pwm = 80;

    let p = &mut ctx.peripherals.pump_status;
    p.pump_a = true;
    p.pump_b = true;
    p.ph_up = false;
    p.ph_down = false;
    p.water_pump_in = true;
    p.water_pump_out = false;
    p.osaka_pump = true;
    p.mist_valve = true;
    p.mix_valve = true;

    ctx
}

fn assert_invariant_i1(
    phase_name: &str,
    trigger_name: &str,
    ctx: &SystemContext,
    result: &TickResult,
) {
    let prefix = format!("[{} -> {}]", phase_name, trigger_name);

    // 1. Both actors must be Idle
    assert!(ctx.dosing.is_idle(), "{} DosingActor must be Idle", prefix);
    assert!(ctx.water.is_idle(), "{} WaterActor must be Idle", prefix);

    // 2. Peripheral ownership and state flags must be cleared
    assert!(
        !ctx.peripherals.misting_started_by_dosing,
        "{} misting_started_by_dosing must be false",
        prefix
    );
    assert!(
        !ctx.peripherals.mix_valve_started_by_dosing,
        "{} mix_valve_started_by_dosing must be false",
        prefix
    );
    assert!(
        !ctx.peripherals.is_misting_active,
        "{} is_misting_active must be false",
        prefix
    );
    assert!(
        !ctx.peripherals.is_scheduled_mixing_active,
        "{} is_scheduled_mixing_active must be false",
        prefix
    );

    // 3. Shadow pumps and actuators must all be OFF / 0
    let p = &ctx.peripherals.pump_status;
    assert!(!p.pump_a, "{} shadow pump_a must be false", prefix);
    assert!(!p.pump_b, "{} shadow pump_b must be false", prefix);
    assert!(!p.ph_up, "{} shadow ph_up must be false", prefix);
    assert!(!p.ph_down, "{} shadow ph_down must be false", prefix);
    assert!(
        !p.water_pump_in,
        "{} shadow water_pump_in must be false",
        prefix
    );
    assert!(
        !p.water_pump_out,
        "{} shadow water_pump_out must be false",
        prefix
    );
    assert!(!p.osaka_pump, "{} shadow osaka_pump must be false", prefix);
    assert!(!p.mist_valve, "{} shadow mist_valve must be false", prefix);
    assert!(!p.mix_valve, "{} shadow mix_valve must be false", prefix);
    assert_eq!(
        ctx.peripherals.osaka_pwm, 0,
        "{} osaka_pwm must be 0",
        prefix
    );

    // 4. Hardware all-off events must be emitted
    let has_water_stop = result.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop
            }
        )
    });
    let has_mist_off = result
        .events
        .iter()
        .any(|e| matches!(e, OrchestratorEvent::SetMistValve { on: false }));
    let has_mix_off = result
        .events
        .iter()
        .any(|e| matches!(e, OrchestratorEvent::SetMixValve { on: false }));
    let has_osaka_off = result
        .events
        .iter()
        .any(|e| matches!(e, OrchestratorEvent::SetOsakaPump { pwm_percent: 0 }));

    assert!(has_water_stop, "{} Must emit SetWaterPump(Stop)", prefix);
    assert!(has_mist_off, "{} Must emit SetMistValve(false)", prefix);
    assert!(has_mix_off, "{} Must emit SetMixValve(false)", prefix);
    assert!(has_osaka_off, "{} Must emit SetOsakaPump(0)", prefix);

    let has_pump_a_off = result.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: false,
                ..
            }
        )
    });
    let has_pump_b_off = result.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientB,
                on: false,
                ..
            }
        )
    });
    assert!(
        has_pump_a_off,
        "{} Must emit SetDosingPump(NutrientA, OFF)",
        prefix
    );
    assert!(
        has_pump_b_off,
        "{} Must emit SetDosingPump(NutrientB, OFF)",
        prefix
    );
}

// -----------------------------------------------------------------------------------------
// Matrix test execution helpers
// -----------------------------------------------------------------------------------------

fn run_sensor_timeout_case(initial_phase: SystemPhase, phase_name: &str) {
    let config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = setup_active_context(initial_phase);

    // Stale sensor update (> 90s)
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
        "[{}] Must transition to Fault(SensorTimeout)",
        phase_name
    );
    assert_invariant_i1(phase_name, "SensorTimeout", &ctx, &result);
}

fn run_manual_mode_case(initial_phase: SystemPhase, phase_name: &str) {
    let mut config = auto_config();
    config.control_mode = ControlMode::Manual;
    let sensors = balanced_sensors();
    let mut ctx = setup_active_context(initial_phase);

    let uptime_ms = 10_000u64;
    let mut result = orchestrator::tick(10_000, uptime_ms, &config, &sensors, uptime_ms, &mut ctx);
    ctx.apply_delta(&mut result.delta);

    assert_eq!(
        ctx.phase,
        SystemPhase::ManualMode,
        "[{}] Must transition to ManualMode",
        phase_name
    );
    assert_invariant_i1(phase_name, "ManualMode", &ctx, &result);
}

fn run_disabled_controller_case(initial_phase: SystemPhase, phase_name: &str) {
    let mut config = auto_config();
    config.is_enabled = false;
    let sensors = balanced_sensors();
    let mut ctx = setup_active_context(initial_phase);

    let uptime_ms = 10_000u64;
    let mut result = orchestrator::tick(10_000, uptime_ms, &config, &sensors, uptime_ms, &mut ctx);
    ctx.apply_delta(&mut result.delta);

    assert_eq!(
        ctx.phase,
        SystemPhase::ManualMode,
        "[{}] Disabled controller must transition to ManualMode",
        phase_name
    );
    assert_invariant_i1(phase_name, "ControllerDisabled", &ctx, &result);
}

// -----------------------------------------------------------------------------------------
// Matrix test cases for each Phase x Trigger
// -----------------------------------------------------------------------------------------

#[test]
fn matrix_01_monitoring_sensor_timeout() {
    run_sensor_timeout_case(SystemPhase::Monitoring, "Monitoring");
}

#[test]
fn matrix_02_monitoring_manual_mode() {
    run_manual_mode_case(SystemPhase::Monitoring, "Monitoring");
}

#[test]
fn matrix_03_monitoring_controller_disabled() {
    run_disabled_controller_case(SystemPhase::Monitoring, "Monitoring");
}

#[test]
fn matrix_04_mimo_dosing_sensor_timeout() {
    run_sensor_timeout_case(SystemPhase::MimoDosing, "MimoDosing");
}

#[test]
fn matrix_05_mimo_dosing_manual_mode() {
    run_manual_mode_case(SystemPhase::MimoDosing, "MimoDosing");
}

#[test]
fn matrix_06_mimo_dosing_controller_disabled() {
    run_disabled_controller_case(SystemPhase::MimoDosing, "MimoDosing");
}

#[test]
fn matrix_07_active_mixing_sensor_timeout() {
    run_sensor_timeout_case(SystemPhase::ActiveMixing, "ActiveMixing");
}

#[test]
fn matrix_08_active_mixing_manual_mode() {
    run_manual_mode_case(SystemPhase::ActiveMixing, "ActiveMixing");
}

#[test]
fn matrix_09_stabilizing_sensor_timeout() {
    run_sensor_timeout_case(SystemPhase::Stabilizing, "Stabilizing");
}

#[test]
fn matrix_10_stabilizing_manual_mode() {
    run_manual_mode_case(SystemPhase::Stabilizing, "Stabilizing");
}

#[test]
fn matrix_11_cooldown_sensor_timeout() {
    run_sensor_timeout_case(SystemPhase::Cooldown, "Cooldown");
}

#[test]
fn matrix_12_cooldown_manual_mode() {
    run_manual_mode_case(SystemPhase::Cooldown, "Cooldown");
}

#[test]
fn matrix_13_water_refilling_sensor_timeout() {
    run_sensor_timeout_case(SystemPhase::WaterRefilling, "WaterRefilling");
}

#[test]
fn matrix_14_water_refilling_manual_mode() {
    run_manual_mode_case(SystemPhase::WaterRefilling, "WaterRefilling");
}

#[test]
fn matrix_15_water_draining_sensor_timeout() {
    run_sensor_timeout_case(SystemPhase::WaterDraining, "WaterDraining");
}

#[test]
fn matrix_16_water_draining_manual_mode() {
    run_manual_mode_case(SystemPhase::WaterDraining, "WaterDraining");
}

#[test]
fn matrix_17_sensor_calibration_sensor_timeout() {
    run_sensor_timeout_case(SystemPhase::SensorCalibration, "SensorCalibration");
}

#[test]
fn matrix_18_sensor_calibration_timeout_restores_monitoring_and_aborts_actors() {
    let config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = setup_active_context(SystemPhase::SensorCalibration);
    ctx.phase_finish_ms = Some(10_000);

    let uptime_ms = 10_001u64; // Exceeded phase_finish_ms
    let mut result = orchestrator::tick(10_001, uptime_ms, &config, &sensors, uptime_ms, &mut ctx);
    ctx.apply_delta(&mut result.delta);

    assert_eq!(
        ctx.phase,
        SystemPhase::Monitoring,
        "SensorCalibration timeout must return to Monitoring"
    );
    assert!(ctx.dosing.is_idle(), "DosingActor must be Idle");
    assert!(ctx.water.is_idle(), "WaterActor must be Idle");
    assert!(!ctx.peripherals.misting_started_by_dosing);
    assert!(!ctx.peripherals.mix_valve_started_by_dosing);
}
