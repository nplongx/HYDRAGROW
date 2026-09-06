//! Task 16 — Cross-Layer FSM Regression Scenarios
//!
//! Each test pins a specific multi-step interaction that crosses FSM, actor,
//! and safety layers.  These are regression guardrails — failing any of these
//! means a systemic contract is broken.

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors};

use hydragrow_controller_core::core::actors::water_actor::WaterSubState;
use hydragrow_controller_core::core::adaptive::matrix::ControlVector;
use hydragrow_controller_core::core::fsm::{
    context::SystemContext, events::OrchestratorEvent, orchestrator,
};
use hydragrow_shared::ControlMode;
use hydragrow_shared::fsm::{FaultCode, SystemPhase};

// ---------------------------------------------------------------------------
// Scenario 1: Sensor timeout recovery — no stale actor resumes after fault
// ---------------------------------------------------------------------------

/// Sequence:
///   Monitoring → active dosing initiated →
///   sensor timeout → Fault(SensorTimeout) / all-off →
///   fresh sensor → Monitoring (actor must be idle, not resumed mid-cycle)
#[test]
fn sensor_timeout_stops_actor_and_recovery_starts_clean() {
    let config = auto_config();
    let sensors_balanced = balanced_sensors();

    let mut ctx = SystemContext {
        phase: SystemPhase::Monitoring,
        ..SystemContext::default()
    };

    // Inject an active dosing cycle directly on the actor
    let control = ControlVector {
        nutrient_a_ml: 1.0,
        ..ControlVector::default()
    };
    let _ = ctx
        .dosing
        .start_matrix_cycle(1000, &control, 1.5, 6.0, 80, &config, &sensors_balanced);
    assert!(
        !ctx.dosing.is_idle(),
        "Pre-condition: dosing actor must be active"
    );

    // Trigger sensor timeout (uptime=100_000, sensor_last_ms=0 → 100s gap)
    let fault_result =
        orchestrator::tick(100_000, 100_000, &config, &sensors_balanced, 0, &mut ctx);
    ctx.apply_delta(&mut { fault_result.delta });

    assert_eq!(
        ctx.phase,
        SystemPhase::Fault(FaultCode::SensorTimeout),
        "Sensor timeout must transition to Fault(SensorTimeout)"
    );
    assert!(
        ctx.dosing.is_idle(),
        "Dosing actor must be reset to idle on sensor timeout fault"
    );

    // Recovery tick with fresh sensor data
    let recovery_result = orchestrator::tick(
        101_000,
        101_000,
        &config,
        &sensors_balanced,
        100_500, // fresh sensor
        &mut ctx,
    );
    ctx.apply_delta(&mut { recovery_result.delta });

    assert_eq!(
        ctx.phase,
        SystemPhase::Monitoring,
        "After fresh sensor, must recover to Monitoring"
    );
    assert!(
        ctx.dosing.is_idle(),
        "After recovery, dosing actor must not have resumed the old in-flight job"
    );
}

// ---------------------------------------------------------------------------
// Scenario 3: Water timeout — no success accounting, actor reset
// ---------------------------------------------------------------------------

/// A water fill that times out must leave the actor idle (not stuck in Filling)
/// and must not fault to SensorTimeout.
#[test]
fn water_fill_timeout_resets_actor_to_idle() {
    let mut config = auto_config();
    config.max_refill_duration_sec = 1; // 1-second timeout for fast test
    config.enable_water_level_sensor = true;
    config.water_level_target = 25.0;
    config.water_level_min = 5.0;

    // Sensors: water level below min → triggers refill
    let mut sensors = balanced_sensors();
    sensors.water_level = 3.0;

    let mut ctx = SystemContext {
        phase: SystemPhase::Monitoring,
        ..SystemContext::default()
    };

    // Tick 1: trigger refill (water is low)
    let t1 = 100_000u64;
    let r1 = orchestrator::tick(t1, t1, &config, &sensors, t1, &mut ctx);
    ctx.apply_delta(&mut { r1.delta });

    // Tick 2: simulate timeout (2.1s > max_refill_duration_sec=1s; water level unchanged)
    let t2 = t1 + 2_100;
    let r2 = orchestrator::tick(t2, t2, &config, &sensors, t2, &mut ctx);
    ctx.apply_delta(&mut { r2.delta });

    // Actor must be idle after timeout
    assert!(
        matches!(ctx.water.sub_state, WaterSubState::Idle),
        "Water actor must be idle after fill timeout"
    );

    // Must not have faulted into SensorTimeout
    assert_ne!(
        ctx.phase,
        SystemPhase::Fault(FaultCode::SensorTimeout),
        "Water timeout must not cause SensorTimeout fault"
    );
}

// ---------------------------------------------------------------------------
// Scenario 6: Partial sensor packet — EC error flag gates dosing
// ---------------------------------------------------------------------------

/// When err_ec is set in the sensor packet, the controller must suppress EC
/// dosing — no NutrientA/B budget must be committed.
#[test]
fn ec_error_flag_prevents_ec_dose_budget_commit() {
    let config = auto_config();

    // Create a sensor packet with EC error flag set
    let mut sensors_with_ec_error = balanced_sensors();
    sensors_with_ec_error.err_ec = Some(true);
    sensors_with_ec_error.ec = 0.0; // Invalid reading

    let mut ctx = SystemContext {
        phase: SystemPhase::Monitoring,
        ..SystemContext::default()
    };

    // Tick with error flag
    let t1 = 100_000u64;
    let r1 = orchestrator::tick(t1, t1, &config, &sensors_with_ec_error, t1, &mut ctx);
    ctx.apply_delta(&mut { r1.delta });

    // EC error must gate EC dosing — no budget committed for NutrientA
    let ec_dose_after_error = ctx.safety.get_hourly_dose("NutrientA", t1 / 1000);
    assert_eq!(
        ec_dose_after_error, 0.0,
        "EC budget must not be committed when err_ec flag is set"
    );

    // Recovery with fresh packet (no error flags)
    let t2 = t1 + 1000;
    let sensors_fresh = balanced_sensors();
    let _r2 = orchestrator::tick(t2, t2, &config, &sensors_fresh, t2, &mut ctx);

    // After fresh packet, no spurious fault
    assert_ne!(
        ctx.phase,
        SystemPhase::Fault(FaultCode::SensorTimeout),
        "Fresh sensor packet must not trigger SensorTimeout"
    );
}

// ---------------------------------------------------------------------------
// Scenario 5: Manual mode abort — actors fully reset on entry
// ---------------------------------------------------------------------------

/// Switching from Auto to Manual when dosing is active must:
///   1. Transition to ManualMode
///   2. Reset dosing actor to idle
///   3. Emit dosing pump OFF events
#[test]
fn manual_mode_entry_aborts_dosing_actor_and_emits_off() {
    let mut config = auto_config();
    let sensors = balanced_sensors();

    let mut ctx = SystemContext {
        phase: SystemPhase::MimoDosing,
        ..SystemContext::default()
    };

    // Inject active dosing
    let control = ControlVector {
        nutrient_a_ml: 2.0,
        ..ControlVector::default()
    };
    let _ = ctx
        .dosing
        .start_matrix_cycle(1000, &control, 1.5, 6.0, 80, &config, &sensors);
    assert!(
        !ctx.dosing.is_idle(),
        "Pre-condition: dosing must be active"
    );

    // Switch to manual mode
    config.control_mode = ControlMode::Manual;

    let result = orchestrator::tick(2000, 2000, &config, &sensors, 2000, &mut ctx);
    ctx.apply_delta(&mut { result.delta });

    assert_eq!(
        ctx.phase,
        SystemPhase::ManualMode,
        "Must transition to ManualMode"
    );
    assert!(
        ctx.dosing.is_idle(),
        "Dosing actor must be idle after manual mode entry"
    );

    // Must emit off events for dosing pumps
    assert!(
        result
            .events
            .iter()
            .any(|e| matches!(e, OrchestratorEvent::SetDosingPump { on: false, .. })),
        "Manual mode entry must emit dosing pump OFF events"
    );
}

// ---------------------------------------------------------------------------
// Scenario: Auto re-enable from Manual — starts in Monitoring with clean state
// ---------------------------------------------------------------------------

/// After switching from Manual back to Auto, the system must enter Monitoring
/// with idle actors — no state from Manual period carried forward.
#[test]
fn auto_reenable_from_manual_starts_in_monitoring_with_clean_actors() {
    let mut config = auto_config();
    config.control_mode = ControlMode::Manual;
    let sensors = balanced_sensors();

    let mut ctx = SystemContext {
        phase: SystemPhase::ManualMode,
        ..SystemContext::default()
    };

    // Tick in Manual (should stay ManualMode)
    let r1 = orchestrator::tick(1000, 1000, &config, &sensors, 1000, &mut ctx);
    ctx.apply_delta(&mut { r1.delta });
    assert_eq!(ctx.phase, SystemPhase::ManualMode);

    // Switch back to Auto
    config.control_mode = ControlMode::Auto;
    let r2 = orchestrator::tick(2000, 2000, &config, &sensors, 2000, &mut ctx);
    ctx.apply_delta(&mut { r2.delta });

    assert_eq!(
        ctx.phase,
        SystemPhase::Monitoring,
        "Auto re-enable from Manual must enter Monitoring"
    );
    assert!(
        ctx.dosing.is_idle(),
        "Dosing actor must be idle after Auto re-enable"
    );
    assert!(
        matches!(ctx.water.sub_state, WaterSubState::Idle),
        "Water actor must be idle after Auto re-enable"
    );
}

// ---------------------------------------------------------------------------
// Dispatcher path: emergency shutdown clears all hardware and faults
// ---------------------------------------------------------------------------

/// emergency_shutdown = true must immediately:
///   1. Transition to Fault(EmergencyStop)
///   2. Reset all actors
///   3. Emit all-off events
#[test]
fn emergency_shutdown_faults_and_emits_all_off() {
    let mut config = auto_config();
    let sensors = balanced_sensors();

    let mut ctx = SystemContext {
        phase: SystemPhase::MimoDosing,
        ..SystemContext::default()
    };

    // Inject active dosing
    let control = ControlVector {
        nutrient_a_ml: 2.0,
        ..ControlVector::default()
    };
    let _ = ctx
        .dosing
        .start_matrix_cycle(1000, &control, 1.5, 6.0, 80, &config, &sensors);
    ctx.peripherals.pump_status.pump_a = true;

    // Trigger emergency shutdown
    config.emergency_shutdown = true;
    let result = orchestrator::tick(2000, 2000, &config, &sensors, 2000, &mut ctx);
    ctx.apply_delta(&mut { result.delta });

    assert_eq!(
        ctx.phase,
        SystemPhase::Fault(FaultCode::EmergencyStop),
        "emergency_shutdown must fault to EmergencyStop"
    );
    assert!(
        ctx.dosing.is_idle(),
        "Dosing actor must be idle after emergency shutdown"
    );
    assert!(
        result
            .events
            .iter()
            .any(|e| matches!(e, OrchestratorEvent::SetDosingPump { on: false, .. })),
        "Emergency shutdown must emit dosing pump OFF events"
    );
}

// ---------------------------------------------------------------------------
// Scenario 7: Mimo hard-timeout must abort dosing execution and turn off pumps
// ---------------------------------------------------------------------------

#[test]
fn mimo_dosing_hard_timeout_shuts_down_all_actuators_and_aborts_actor() {
    let config = auto_config();
    let sensors = balanced_sensors();

    let mut ctx = SystemContext {
        phase: SystemPhase::MimoDosing,
        phase_start_ms: Some(1000),
        phase_finish_ms: Some(10_000),
        ..SystemContext::default()
    };

    let control = ControlVector {
        nutrient_a_ml: 10.0,
        ..ControlVector::default()
    };
    let _ = ctx
        .dosing
        .start_matrix_cycle(1000, &control, 1.5, 6.0, 80, &config, &sensors);
    ctx.peripherals.pump_status.pump_a = true;

    // Trigger hard timeout (uptime >= 10_000 + 5_000 -> 15_001)
    let hard_timeout_ms = 15_001u64;
    let mut result = orchestrator::tick(
        hard_timeout_ms,
        hard_timeout_ms,
        &config,
        &sensors,
        hard_timeout_ms,
        &mut ctx,
    );

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::Cooldown),
        "Hard timeout must transition to Cooldown"
    );

    // Must emit OFF event for NutrientA
    assert!(
        result.events.iter().any(|e| matches!(
            e,
            OrchestratorEvent::SetDosingPump {
                pump: hydragrow_controller_core::core::fsm::DosingPumpTarget::NutrientA,
                on: false,
                ..
            }
        )),
        "Hard timeout must emit SetDosingPump(NutrientA, OFF)"
    );

    // Must NOT contain any ON events
    assert!(
        !result
            .events
            .iter()
            .any(|e| matches!(e, OrchestratorEvent::SetDosingPump { on: true, .. })),
        "Hard timeout must not emit any dosing ON events"
    );

    // Apply delta and assert actor is idle and shadow state is cleared
    ctx.apply_delta(&mut result.delta);
    assert!(
        ctx.dosing.is_idle(),
        "Dosing actor must be Idle after hard timeout"
    );
    assert!(
        !ctx.peripherals.pump_status.pump_a,
        "Shadow pump_a must be false after hard timeout"
    );
}

// ---------------------------------------------------------------------------
// Scenario 8: Invariant I2 — Water IN/OUT mutual exclusion
// ---------------------------------------------------------------------------

#[test]
fn water_in_and_out_mutual_exclusion_enforced() {
    use hydragrow_controller_core::core::fsm::tick_result::{ContextDelta, PeripheralDelta};

    // 1. Setting water_pump_out when water_pump_in was true must clear water_pump_in to false
    let mut base_delta = ContextDelta {
        peripherals: Some(PeripheralDelta {
            water_pump_in: Some(true),
            ..PeripheralDelta::default()
        }),
        ..ContextDelta::default()
    };

    let addition_delta = ContextDelta {
        peripherals: Some(PeripheralDelta {
            water_pump_out: Some(true),
            ..PeripheralDelta::default()
        }),
        ..ContextDelta::default()
    };

    base_delta.merge_from(addition_delta);
    let peri = base_delta.peripherals.expect("peripherals must exist");
    assert_eq!(peri.water_pump_out, Some(true));
    assert_eq!(
        peri.water_pump_in,
        Some(false),
        "Setting water_pump_out to true must force water_pump_in to Some(false)"
    );

    // 2. Conflicting simultaneous true in addition must reject both
    let mut delta2 = ContextDelta::default();
    let conflict = ContextDelta {
        peripherals: Some(PeripheralDelta {
            water_pump_in: Some(true),
            water_pump_out: Some(true),
            ..PeripheralDelta::default()
        }),
        ..ContextDelta::default()
    };
    delta2.merge_from(conflict);
    let peri2 = delta2.peripherals.expect("peripherals must exist");
    assert!(
        !(peri2.water_pump_in == Some(true) && peri2.water_pump_out == Some(true)),
        "Both water_pump_in and water_pump_out cannot be true simultaneously"
    );
    assert_eq!(peri2.water_pump_in, Some(false));
    assert_eq!(peri2.water_pump_out, Some(false));

    // 3. SystemContext::apply_delta must never allow both pump_status flags to be true
    let mut ctx = SystemContext::default();
    ctx.apply_delta(&mut ContextDelta {
        peripherals: Some(PeripheralDelta {
            water_pump_in: Some(true),
            water_pump_out: Some(true),
            ..PeripheralDelta::default()
        }),
        ..ContextDelta::default()
    });
    assert!(
        !(ctx.peripherals.pump_status.water_pump_in && ctx.peripherals.pump_status.water_pump_out),
        "Invariant I2: ctx.peripherals.pump_status must never have water_pump_in and water_pump_out both true"
    );

    // 4. apply_decision with conflicting water_in_sec and water_out_sec must reject both
    let config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx4 = SystemContext::default();
    let conflict_control = ControlVector {
        water_in_sec: 10.0,
        water_out_sec: 10.0,
        ..ControlVector::default()
    };
    let decision = hydragrow_controller_core::core::adaptive::solver::SolveResult::Execute {
        control: conflict_control,
        target_ec: 1.5,
        target_ph: 6.0,
        pwm: 80,
    };
    let result4 = hydragrow_controller_core::core::fsm::phases::monitoring::apply_decision(
        decision,
        &mut ctx4,
        &config,
        &sensors,
        1000,
        1000,
        hydragrow_controller_core::core::fsm::tick_result::TickResult::default(),
        false,
    );
    assert!(
        !result4
            .events
            .iter()
            .any(|e| matches!(e, OrchestratorEvent::SetWaterPump { .. })),
        "Conflicting water decision must not emit any SetWaterPump event"
    );
    assert!(
        ctx4.water.is_idle(),
        "Water actor must remain idle after conflicting decision"
    );
}

// ---------------------------------------------------------------------------
// Scenario 9: Invariant I3 — Safety budget transaction & dispatch fault recovery
// ---------------------------------------------------------------------------

#[test]
fn dispatch_failure_does_not_commit_safety_budget_and_aborts_actor() {
    let config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = SystemContext {
        phase: SystemPhase::Monitoring,
        ..SystemContext::default()
    };

    let uptime_ms = 50_000u64;
    let uptime_sec = uptime_ms / 1000;

    // Decision requesting dosing of NutrientA
    let control = ControlVector {
        nutrient_a_ml: 10.0,
        ..ControlVector::default()
    };
    let decision = hydragrow_controller_core::core::adaptive::solver::SolveResult::Execute {
        control,
        target_ec: 1.5,
        target_ph: 6.0,
        pwm: 80,
    };

    let result = hydragrow_controller_core::core::fsm::phases::monitoring::apply_decision(
        decision,
        &mut ctx,
        &config,
        &sensors,
        uptime_ms,
        uptime_ms,
        hydragrow_controller_core::core::fsm::tick_result::TickResult::default(),
        false,
    );

    // 1. Before dispatch: safety transaction must be staged in result, NOT committed to ctx.safety
    assert!(
        result.safety_transaction.is_some(),
        "Result must contain staged SafetyTransaction"
    );
    let dose_before_dispatch = ctx.safety.get_hourly_dose("NutrientA", uptime_sec);
    assert_eq!(
        dose_before_dispatch, 0.0,
        "Safety budget must NOT be committed before dispatch succeeds"
    );

    // 2. Simulate hardware dispatch failure
    // Apply tick delta first as fsm_loop does
    let mut delta = result.delta;
    ctx.apply_delta(&mut delta);
    assert!(!ctx.dosing.is_idle(), "Actor was started for the cycle");

    // Dispatch failure recovery (as performed by apply_dispatch_fault in fsm_loop)
    let mut recovery_result = hydragrow_controller_core::core::fsm::TickResult::default();
    recovery_result.delta.phase = Some(SystemPhase::Fault(FaultCode::EcDosingFailed));
    orchestrator::fault_all_outputs_off(&mut recovery_result);
    ctx.apply_delta(&mut recovery_result.delta);

    // Assert actor aborted, shadow cleared, and safety budget NOT committed
    assert!(
        ctx.dosing.is_idle(),
        "Dosing actor must be Idle after dispatch fault recovery"
    );
    assert!(
        !ctx.peripherals.pump_status.pump_a,
        "Shadow pump_a must be false"
    );
    assert_eq!(
        ctx.phase,
        SystemPhase::Fault(FaultCode::EcDosingFailed),
        "FSM must be in Fault"
    );
    let dose_after_fail = ctx.safety.get_hourly_dose("NutrientA", uptime_sec);
    assert_eq!(
        dose_after_fail, 0.0,
        "Invariant I3: Safety budget must remain uncommitted when dispatch fails"
    );

    // 3. Verify companion case: successful dispatch commits the transaction
    let safety_tx = result.safety_transaction.unwrap();
    ctx.commit_safety_transaction(&safety_tx);
    let dose_after_success = ctx.safety.get_hourly_dose("NutrientA", uptime_sec);
    assert!(
        dose_after_success > 0.0,
        "Safety budget must be committed after commit_safety_transaction"
    );
}
