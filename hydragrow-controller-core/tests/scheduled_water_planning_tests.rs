//! Tests for scheduled water planning and safety guardrails (Task 13)

use hydragrow_controller_core::WaterDirection;
use hydragrow_controller_core::core::actors::water_actor::WaterSubState;
use hydragrow_controller_core::core::fsm::{
    context::SystemContext, events::OrchestratorEvent, orchestrator,
};
use hydragrow_controller_core::core::optimizer::{
    DEFAULT_WATER_RATE_CM_PER_SEC, plan_water_operation,
};
use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::{ControlMode, ControllerConfig, SensorData};

fn base_config() -> ControllerConfig {
    ControllerConfig {
        control_mode: ControlMode::Auto,
        is_enabled: true,
        enable_water_level_sensor: true,
        scheduled_water_change_enabled: true,
        water_change_cron: String::new(),
        water_change_interval_days: Some(7),
        scheduled_drain_amount_cm: 2.0,
        max_drain_duration_sec: 60,
        water_level_min: 15.0,
        water_level_target: 20.0,
        water_level_max: 24.0,
        water_level_tolerance: 1.0,
        water_level_critical_min: 8.0,
        ..ControllerConfig::default()
    }
}

mod helpers;
use helpers::fixtures::balanced_sensors;

fn make_sensors(water_level: f32) -> SensorData {
    SensorData {
        water_level,
        ..balanced_sensors()
    }
}

#[test]
fn scheduled_drain_amount_drives_planning_duration_not_max_duration() {
    let mut config = base_config();
    config.scheduled_drain_amount_cm = 2.0;
    config.max_drain_duration_sec = 60; // Max duration is 60s, but 2.0cm @ 0.1cm/s should take 20s

    let mut ctx = SystemContext {
        phase: SystemPhase::Monitoring,
        last_water_change_sec: 1_700_000_000,
        ..Default::default()
    };

    let sensors = make_sensors(20.0);
    // Interval due: last_water_change_sec = 1_700_000_000 -> next due = 1_700_000_000 + 7*86400
    let due_sec = 1_700_000_000 + 7 * 86400;
    let due_ms = due_sec * 1000;

    let result = orchestrator::tick(due_ms, due_ms, &config, &sensors, due_ms, &mut ctx);

    assert!(
        result.events.iter().any(|e| matches!(
            e,
            OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Out
            }
        )),
        "Must emit Out event on scheduled water change"
    );

    // Draining job in ctx must reflect the requested amount and target
    match &ctx.water.sub_state {
        WaterSubState::Draining { job } => {
            // Target level should be 20.0 - 2.0 = 18.0, NOT 20.0
            assert_eq!(
                job.target_level, 18.0,
                "Draining target must be current_level - scheduled_drain_amount_cm"
            );
            assert_eq!(job.trigger, "scheduled_water_change");
        }
        other => panic!("Expected WaterSubState::Draining, got {:?}", other),
    }
}

#[test]
fn scheduled_water_change_violating_critical_min_is_rejected() {
    let mut config = base_config();
    config.water_level_critical_min = 8.0;
    config.scheduled_drain_amount_cm = 2.0;

    let mut ctx = SystemContext {
        phase: SystemPhase::Monitoring,
        last_water_change_sec: 1_700_000_000,
        ..Default::default()
    };

    // Water level is already at critical minimum!
    let sensors = make_sensors(7.5);
    let due_sec = 1_700_000_000 + 7 * 86400;
    let due_ms = due_sec * 1000;

    let result = orchestrator::tick(due_ms, due_ms, &config, &sensors, due_ms, &mut ctx);

    // Must NOT start draining when water level is at/below critical minimum
    assert!(
        !result.events.iter().any(|e| matches!(
            e,
            OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Out
            }
        )),
        "Must NOT emit Out event when water level <= critical_min"
    );
    assert!(
        !matches!(ctx.water.sub_state, WaterSubState::Draining { .. }),
        "Water actor must not enter Draining state"
    );
}

#[test]
fn common_water_planning_clamps_to_critical_min_and_max_duration() {
    let mut config = base_config();
    config.water_level_critical_min = 8.0;
    config.max_drain_duration_sec = 60;

    // Current 10.0, requested drain 5.0 -> safe max drain is 10.0 - 8.0 = 2.0
    let plan = plan_water_operation(
        WaterDirection::Out,
        5.0,
        10.0,
        Some(DEFAULT_WATER_RATE_CM_PER_SEC),
        &config,
    )
    .expect("Plan should succeed with clamped amount");

    assert_eq!(plan.target_level, 8.0);
    assert_eq!(plan.amount_cm, 2.0);
    assert_eq!(plan.duration_sec, 20.0); // 2.0 / 0.1 = 20.0
}
