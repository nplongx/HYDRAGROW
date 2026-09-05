//! Tests for in-flight job semantics and safety aborts (Task 14)

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors};

use hydragrow_controller_core::core::actors::dosing_actor::{
    DosingActor, DosingEvent, DosingSubState,
};
use hydragrow_controller_core::core::adaptive::matrix::ControlVector;
use hydragrow_controller_core::core::fsm::recipe_manager::tick_recipe_engine;
use hydragrow_controller_core::core::fsm::{OrchestratorEvent, SystemContext, orchestrator};
use hydragrow_shared::fsm::{FaultCode, SystemPhase};
use hydragrow_shared::recipe::{CropRecipe, CropStage};
use hydragrow_shared::{ControlMode, ControllerConfig};

fn make_test_recipe(id: &str, rev: u64, start_time_sec: u64) -> CropRecipe {
    CropRecipe {
        schema_version: 1,
        recipe_id: id.to_string(),
        season_id: "season_1".to_string(),
        device_id: "test_dev".to_string(),
        revision: rev,
        start_time_sec,
        current_stage_index: 0,
        stages: vec![
            CropStage {
                name: "Stage 1".to_string(),
                duration_sec: 3600,
                ec_target: 1.8,
                ec_tolerance: 0.05,
                ph_target: 6.0,
                ph_tolerance: 0.1,
                nutrient_a_ratio: 1.0,
                nutrient_b_ratio: 1.0,
                water_level_target: 20.0,
                water_change_interval_days: None,
                water_change_drain_cm: None,
                auto_dilute_ec_trigger: None,
                max_dose_per_cycle_ml: Some(10.0),
                misting_on_duration_ms: 5000,
                misting_off_duration_ms: 60000,
            },
            CropStage {
                name: "Stage 2".to_string(),
                duration_sec: 3600,
                ec_target: 2.2,
                ec_tolerance: 0.05,
                ph_target: 6.2,
                ph_tolerance: 0.1,
                nutrient_a_ratio: 1.0,
                nutrient_b_ratio: 1.0,
                water_level_target: 20.0,
                water_change_interval_days: None,
                water_change_drain_cm: None,
                auto_dilute_ec_trigger: None,
                max_dose_per_cycle_ml: Some(10.0),
                misting_on_duration_ms: 5000,
                misting_off_duration_ms: 60000,
            },
        ],
    }
}

#[test]
fn inflight_dosing_cycle_preserves_initial_config_snapshot_while_active() {
    let mut actor = DosingActor::new();
    let mut config_a = auto_config();
    config_a.delay_between_a_and_b_sec = 10;
    config_a.dosing_pulse_on_ms = 50;
    config_a.dosing_pulse_off_ms = 50;
    config_a.soft_start_duration = 0;
    config_a.pump_a_capacity_ml_per_sec = 1.0;
    config_a.pump_b_capacity_ml_per_sec = 1.0;

    let sensors = balanced_sensors();
    let control = ControlVector {
        nutrient_a_ml: 0.1,
        nutrient_b_ml: 0.1,
        ..ControlVector::default()
    };

    // Start cycle under Config A
    let _ = actor.start_matrix_cycle(1000, &control, 1.8, 6.0, 80, &config_a, &sensors);

    // Config is changed to Config B mid-cycle (user changes delay to 2 sec and pump_b capacity to 0.5)
    let mut config_b = config_a.clone();
    config_b.delay_between_a_and_b_sec = 2;
    config_b.pump_b_capacity_ml_per_sec = 0.5;

    // Tick pump A to completion using config_b
    let mut current_ms = 1000u64;
    let mut transitioned_to_wait = false;
    for _ in 0..20 {
        current_ms += 100;
        let (ev, _) = actor.tick(current_ms, &config_b);
        if matches!(ev, DosingEvent::PhaseTransition) {
            transitioned_to_wait = true;
            break;
        }
    }

    assert!(
        transitioned_to_wait,
        "Pump A must complete and transition to WaitingAtoB"
    );

    // Must preserve Config A's 10-second delay (finish_ms = current_ms + 10_000, not + 2_000)
    // and Config A's pump capacity on_ms (125ms, not Config B's 250ms)
    match &actor.sub_state {
        DosingSubState::WaitingAtoB { finish_ms, b_job } => {
            assert_eq!(
                *finish_ms,
                current_ms + 10_000,
                "In-flight inter-pump delay must use snapshotted Config A delay (10s), not Config B (2s)"
            );
            assert_eq!(
                b_job.on_ms, 125,
                "Pump B job must use snapshotted Config A capacity on_ms (125ms), not Config B (250ms)"
            );
        }
        other => panic!("Expected WaitingAtoB, got {:?}", other),
    }
}

#[test]
fn safety_critical_emergency_shutdown_aborts_active_actors_and_emits_off() {
    let mut config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = SystemContext {
        phase: SystemPhase::MimoDosing,
        ..Default::default()
    };

    // Set active dosing state
    let control = ControlVector {
        nutrient_a_ml: 5.0,
        ..ControlVector::default()
    };
    let _ = ctx
        .dosing
        .start_matrix_cycle(1000, &control, 1.8, 6.0, 80, &config, &sensors);
    ctx.peripherals.pump_status.pump_a = true;

    assert!(!ctx.dosing.is_idle());

    // Emergency shutdown occurs
    config.emergency_shutdown = true;

    let result = orchestrator::tick(2000, 2000, &config, &sensors, 2000, &mut ctx);
    ctx.apply_delta(&mut { result.delta });

    assert_eq!(ctx.phase, SystemPhase::Fault(FaultCode::EmergencyStop));
    assert!(
        ctx.dosing.is_idle(),
        "Emergency stop must reset dosing actor to idle"
    );
    assert!(
        !ctx.peripherals.pump_status.pump_a,
        "Peripheral pump A must be marked off"
    );

    // All off events must be emitted
    assert!(
        result
            .events
            .iter()
            .any(|e| matches!(e, OrchestratorEvent::SetDosingPump { on: false, .. })),
        "Must emit dosing pump OFF events"
    );
}

#[test]
fn safety_critical_manual_mode_aborts_active_actors_and_emits_off() {
    let mut config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = SystemContext {
        phase: SystemPhase::MimoDosing,
        ..Default::default()
    };

    let control = ControlVector {
        nutrient_a_ml: 5.0,
        ..ControlVector::default()
    };
    let _ = ctx
        .dosing
        .start_matrix_cycle(1000, &control, 1.8, 6.0, 80, &config, &sensors);
    ctx.peripherals.pump_status.pump_a = true;

    // Switch to manual mode
    config.control_mode = ControlMode::Manual;

    let result = orchestrator::tick(2000, 2000, &config, &sensors, 2000, &mut ctx);
    ctx.apply_delta(&mut { result.delta });

    assert_eq!(ctx.phase, SystemPhase::ManualMode);
    assert!(
        ctx.dosing.is_idle(),
        "Manual mode must reset dosing actor to idle"
    );
    assert!(!ctx.peripherals.pump_status.pump_a);
    assert!(
        result
            .events
            .iter()
            .any(|e| matches!(e, OrchestratorEvent::SetDosingPump { on: false, .. })),
        "Must emit dosing pump OFF events on switch to manual"
    );
}

#[test]
fn recipe_id_or_revision_change_resets_completion_and_stage_cursor() {
    let mut ctx = SystemContext {
        recipe_completed: true,
        current_stage_index: None,
        last_recipe_check_sec: 1_700_000_000,
        ..Default::default()
    };

    // Old recipe was completed at 1_700_000_000 + 7200
    let mut config = ControllerConfig::default();
    let recipe1 = make_test_recipe("tomato_recipe", 1, 1_700_000_000);
    config.active_recipe = Some(recipe1);

    // New recipe revision (rev 2) arrives with new start time
    let recipe2 = make_test_recipe("tomato_recipe", 2, 1_700_010_000);
    config.active_recipe = Some(recipe2);

    // Tick at 1_700_010_000
    let result = tick_recipe_engine(&mut config, &ctx, 1_700_010_000);
    ctx.apply_delta(&mut { result.delta });

    assert!(
        !ctx.recipe_completed,
        "New recipe revision must reset recipe_completed"
    );
    assert_eq!(
        ctx.current_stage_index,
        Some(0),
        "New recipe revision must start at stage 0"
    );
    assert_eq!(config.ec_target, 1.8, "Stage 0 overrides must be applied");
}
