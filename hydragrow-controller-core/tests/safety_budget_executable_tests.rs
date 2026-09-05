//! Regression tests for transactional safety budget charging and executable dosing planning

#![allow(clippy::field_reassign_with_default)]

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors};

use hydragrow_controller_core::core::actors::dosing_actor::{
    DosingActor, DosingEvent, DosingPlanRejection, DosingPlanResult,
};
use hydragrow_controller_core::core::adaptive::matrix::ControlVector;
use hydragrow_controller_core::core::fsm::{context::SystemContext, orchestrator};
use hydragrow_controller_core::utils::DosePumpKind;
use hydragrow_shared::fsm::SystemPhase;

#[test]
fn flow_a_unavailable_does_not_commit_a_budget() {
    let mut config = auto_config();
    // Disable pump A flow capacity
    config.pump_a_capacity_ml_per_sec = 0.0;

    let mut sensors = balanced_sensors();
    sensors.ec = 0.8; // Triggers need for nutrients

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    let now_ms = 100_000;
    let uptime_ms = 100_000;

    // Check baseline safety dose
    let before_dose = ctx.safety.get_hourly_dose("NutrientA", uptime_ms / 1000);
    assert_eq!(before_dose, 0.0);

    let _result = orchestrator::tick(now_ms, uptime_ms, &config, &sensors, now_ms, &mut ctx);

    // If flow A was unavailable, NutrientA dose must NOT have been committed to safety budget
    let after_dose = ctx.safety.get_hourly_dose("NutrientA", uptime_ms / 1000);
    assert_eq!(
        after_dose, 0.0,
        "NutrientA budget must NOT be committed when flow A is unavailable"
    );
}

#[test]
fn a_zero_b_positive_job_survives_and_is_executed() {
    let mut actor = DosingActor::new();
    let config = auto_config();
    let sensors = balanced_sensors();

    let mut control = ControlVector::default();
    control.nutrient_a_ml = 0.0;
    control.nutrient_b_ml = 2.5;

    let plan_result = actor.start_matrix_cycle(1000, &control, 1.5, 6.0, 80, &config, &sensors);

    match plan_result {
        DosingPlanResult::Prepared(jobs) => {
            assert_eq!(
                jobs.len(),
                1,
                "Should prepare exactly one job for Nutrient B"
            );
            assert_eq!(jobs[0].pump, DosePumpKind::PumpB);
            assert_eq!(jobs[0].target_ml, 2.5);
        }
        other => panic!("Expected DosingPlanResult::Prepared, got {:?}", other),
    }

    assert!(
        !actor.is_idle(),
        "Actor must not be idle when B > 0 even if A == 0"
    );

    // Tick past soft-start to verify it moves into PumpingB
    let (event, _) = actor.tick(1000 + config.soft_start_duration as u64 + 10, &config);
    assert_eq!(event, DosingEvent::SoftStartDone);
}

#[test]
fn conflicting_ph_up_and_down_is_rejected_without_budget_charge() {
    let mut actor = DosingActor::new();
    let config = auto_config();
    let sensors = balanced_sensors();

    let mut control = ControlVector::default();
    control.ph_up_ml = 1.0;
    control.ph_down_ml = 1.0;

    let plan_result = actor.start_matrix_cycle(1000, &control, 1.5, 6.0, 80, &config, &sensors);

    assert_eq!(
        plan_result,
        DosingPlanResult::Rejected(DosingPlanRejection::ConflictingPh),
        "Conflicting pH Up and pH Down in the same cycle must be explicitly rejected"
    );
    assert!(actor.is_idle(), "Actor must remain idle on rejected plan");
}

#[test]
fn committed_hourly_dose_equals_sum_of_prepared_executable_jobs() {
    let mut actor = DosingActor::new();
    let config = auto_config();
    let sensors = balanced_sensors();

    let mut control = ControlVector::default();
    control.nutrient_a_ml = 1.5;
    control.nutrient_b_ml = 1.5;
    control.ph_down_ml = 0.5;

    let plan_result = actor.start_matrix_cycle(1000, &control, 1.5, 6.0, 80, &config, &sensors);

    let jobs = match plan_result {
        DosingPlanResult::Prepared(jobs) => jobs,
        other => panic!("Expected Prepared jobs, got {:?}", other),
    };

    let total_planned_ml: f32 = jobs.iter().map(|j| j.target_ml).sum();
    assert_eq!(total_planned_ml, 1.5 + 1.5 + 0.5);
    assert_eq!(jobs.len(), 3);
}
