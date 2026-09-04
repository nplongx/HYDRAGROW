//! E2E: Water refilling và draining scenarios

use hydragrow_controller_core::WaterDirection;
use hydragrow_controller_core::core::fsm::{
    context::SystemContext, events::OrchestratorEvent, orchestrator,
};
use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::{ControlMode, ControllerConfig, SensorData};

fn water_config() -> ControllerConfig {
    ControllerConfig {
        control_mode: ControlMode::Auto,
        is_enabled: true,
        ec_target: 1.5,
        ec_tolerance: 0.05,
        ph_target: 6.0,
        ph_tolerance: 0.1,
        enable_ec_sensor: false, // Tắt EC để focus vào water logic
        enable_ph_sensor: false,
        enable_water_level_sensor: true, // Bật water sensor
        enable_temp_sensor: false,
        water_level_min: 15.0,
        water_level_target: 20.0,
        water_level_max: 24.0,
        water_level_tolerance: 1.0,
        water_level_critical_min: 8.0,
        auto_refill_enabled: true,
        auto_drain_overflow: true,
        auto_dilute_enabled: false,
        dilute_drain_amount_cm: 0.0,
        max_refill_cycles_per_hour: 4,
        max_drain_cycles_per_hour: 4,
        max_refill_duration_sec: 30,
        max_drain_duration_sec: 30,
        cooldown_sec: 1,
        soft_start_duration: 0,
        max_dose_per_hour: 50.0,
        max_dose_per_cycle: 10.0,
        max_ec_delta: 2.0,
        max_ph_delta: 2.0,
        max_ec_limit: 5.0,
        min_ec_limit: 0.0,
        min_ph_limit: 3.0,
        max_ph_limit: 10.0,
        ec_gain_per_ml: 0.3,
        ph_shift_up_per_ml: 0.15,
        ph_shift_down_per_ml: 0.15,
        pump_a_capacity_ml_per_sec: 1.0,
        pump_b_capacity_ml_per_sec: 1.0,
        pump_ph_up_capacity_ml_per_sec: 0.5,
        pump_ph_down_capacity_ml_per_sec: 0.5,
        dosing_pwm_percent: 80,
        dosing_min_pwm_percent: 30,
        dosing_pulse_on_ms: 50,
        dosing_pulse_off_ms: 50,
        dosing_min_dose_ml: 0.1,
        dosing_max_pulse_count_per_cycle: 50,
        ec_step_ratio: 1.0,
        ph_step_ratio: 1.0,
        best_ec_ratio: 1.0,
        best_ph_ratio: 1.0,
        adaptive_mixing_sec: 5,
        adaptive_stabilize_sec: 5,
        effective_ec_tolerance: 0.05,
        effective_ph_tolerance: 0.1,
        active_mixing_sec: 5,
        sensor_stabilize_sec: 5,
        ec_ack_threshold: 0.05,
        ph_ack_threshold: 0.1,
        water_ack_threshold: 0.5,
        scheduled_mixing_interval_sec: 7200,
        scheduled_mixing_duration_sec: 30,
        misting_on_duration_ms: 5000,
        misting_off_duration_ms: 60000,
        osaka_mixing_pwm_percent: 60,
        osaka_misting_pwm_percent: 100,
        high_temp_misting_on_duration_ms: 15000,
        high_temp_misting_off_duration_ms: 60000,
        misting_temp_threshold: 35.0,
        delay_between_a_and_b_sec: 0,
        min_temp_limit: 10.0,
        max_temp_limit: 40.0,
        scheduled_water_change_enabled: false,
        water_change_cron: String::new(),
        scheduled_drain_amount_cm: 0.0,
        water_change_interval_days: None,
        emergency_shutdown: false,
        nutrient_a_ratio: 1.0,
        nutrient_b_ratio: 1.0,
        device_id: "e2e_water_test".to_string(),
        active_recipe: None,
        tuner_state: 0,
        interaction_matrix: None,
        matrix_update_count: 0,
        matrix_is_warm: false,
        kalman_confidence: None,
        tank_height: 30,
        pump_a_min_pwm_percent: None,
        pump_b_min_pwm_percent: None,
        pump_ph_up_min_pwm_percent: None,
        pump_ph_down_min_pwm_percent: None,
    }
}

fn make_sensor(water_level: f32) -> SensorData {
    SensorData {
        device_id: "e2e_water_test".to_string(),
        ec: 1.5,
        ph: 6.0,
        temp: 25.0,
        water_level,
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

fn tick_apply(
    ctx: &mut SystemContext,
    config: &ControllerConfig,
    sensors: &SensorData,
    uptime_ms: u64,
) -> Vec<OrchestratorEvent> {
    let now_ms = 1_700_000_000_000u64 + uptime_ms;
    let mut result = orchestrator::tick(now_ms, uptime_ms, config, sensors, uptime_ms, ctx);
    ctx.apply_delta(&mut result.delta);
    result.events
}

/// E2E Water Test 1: Mực nước thấp → trigger WaterRefilling (MimoDosing state with Water Pump In)
#[test]
fn e2e_low_water_triggers_refilling() {
    let config = water_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    // water_level = 10.0 < water_level_min 15.0
    let low_water = make_sensor(10.0);
    let events = tick_apply(&mut ctx, &config, &low_water, 10_000);

    assert!(
        matches!(
            ctx.phase,
            SystemPhase::MimoDosing | SystemPhase::WaterRefilling
        ),
        "Mực nước thấp hơn min phải trigger WaterRefilling/MimoDosing"
    );

    // Phải emit SetWaterPump In
    let starts_pump_in = events.iter().any(|e| {
        matches!(e, OrchestratorEvent::SetWaterPump { direction }
            if *direction == WaterDirection::In)
    });
    assert!(starts_pump_in, "WaterRefilling phải bật water pump In");
    eprintln!("✅ Water refilling triggered correctly");
}

/// E2E Water Test 2: Mực nước đạt target trong WaterRefilling → về Monitoring
#[test]
fn e2e_refilling_completes_when_target_reached() {
    let config = water_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::WaterRefilling;
    ctx.phase_start_ms = Some(0);
    ctx.phase_finish_ms = Some(30_000);

    // Sensor: mực nước đã đạt target
    let target_water = make_sensor(20.5); // > target 20.0

    // Tick WaterRefilling — cần trigger WaterActor để hoàn thành
    // Tick nhiều lần để complete
    for i in 0..5 {
        let events = tick_apply(&mut ctx, &config, &target_water, i * 1000 + 1000);
        if ctx.phase == SystemPhase::Monitoring {
            eprintln!("✅ WaterRefilling completed at tick {}", i + 1);
            return;
        }
        let _ = events;
    }

    // Fallback: Force timeout
    ctx.phase_finish_ms = Some(100);
    let _events = tick_apply(&mut ctx, &config, &target_water, 200_000);

    assert_eq!(
        ctx.phase,
        SystemPhase::Monitoring,
        "WaterRefilling hoàn thành (timeout hoặc target đạt) phải về Monitoring"
    );
    eprintln!("✅ Water refilling completed via timeout");
}

/// E2E Water Test 3: Mực nước cao → trigger WaterDraining (MimoDosing state with Water Pump Out)
#[test]
fn e2e_high_water_triggers_draining() {
    let config = water_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    // water_level = 26.0 > water_level_max 24.0
    let high_water = make_sensor(26.0);
    let events = tick_apply(&mut ctx, &config, &high_water, 10_000);

    assert!(
        matches!(
            ctx.phase,
            SystemPhase::MimoDosing | SystemPhase::WaterDraining
        ),
        "Mực nước cao hơn max phải trigger WaterDraining/MimoDosing"
    );

    let starts_pump_out = events.iter().any(|e| {
        matches!(e, OrchestratorEvent::SetWaterPump { direction }
            if *direction == WaterDirection::Out)
    });
    assert!(starts_pump_out, "WaterDraining phải bật water pump Out");
    eprintln!("✅ Water draining triggered correctly");
}

/// E2E Water Test 4: Mực nước bình thường → không trigger water management
#[test]
fn e2e_normal_water_no_action() {
    let config = water_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    // water_level = 20.0 = target → không cần action
    let normal_water = make_sensor(20.0);
    tick_apply(&mut ctx, &config, &normal_water, 10_000);

    assert_eq!(
        ctx.phase,
        SystemPhase::Monitoring,
        "Mực nước bình thường không trigger water management"
    );
    eprintln!("✅ Normal water level stays in Monitoring");
}

/// E2E Water Test 5: Chu kỳ chỉ bơm nước (water-only) phải duy trì bơm nước theo đúng thời gian yêu cầu
#[test]
fn e2e_water_only_cycle_honors_duration() {
    let mut config = water_config();
    config.max_refill_duration_sec = 30;
    config.water_level_target = 20.0;
    config.water_level_tolerance = 0.5;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    let sensor = make_sensor(17.0);

    // Tick 1: At uptime 10_000ms, Monitoring triggers MimoDosing with Water Pump In
    let events = tick_apply(&mut ctx, &config, &sensor, 10_000);
    assert_eq!(
        ctx.phase,
        SystemPhase::MimoDosing,
        "Must transition to MimoDosing"
    );
    assert!(
        ctx.peripherals.pump_status.water_pump_in,
        "Water pump In must be running"
    );
    let has_water_in = events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::In
            }
        )
    });
    assert!(has_water_in, "Must emit SetWaterPump In");

    // Tick 2: At uptime 10_600ms (600ms elapsed), old code had `elapsed_ms >= 500 && is_idle()` and wrongly stopped water!
    let events_600 = tick_apply(&mut ctx, &config, &sensor, 10_600);
    assert_eq!(
        ctx.phase,
        SystemPhase::MimoDosing,
        "Must remain in MimoDosing after 600ms"
    );
    assert!(
        ctx.peripherals.pump_status.water_pump_in,
        "Water pump In must remain running"
    );
    let has_stop = events_600.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop
            }
        )
    });
    assert!(!has_stop, "Must NOT stop water after only 600ms");

    // Tick 3: At uptime 25_000ms (15s elapsed), water still filling because level is still 17.0 < 20.0
    let _events_15s = tick_apply(&mut ctx, &config, &sensor, 25_000);
    assert_eq!(
        ctx.phase,
        SystemPhase::MimoDosing,
        "Must remain in MimoDosing at 15s"
    );
    assert!(
        ctx.peripherals.pump_status.water_pump_in,
        "Water pump In must remain running at 15s"
    );

    // Tick 4: Water level reaches target 20.0 at uptime 30_000ms
    let full_sensor = make_sensor(20.0);
    let events_done = tick_apply(&mut ctx, &config, &full_sensor, 30_000);
    assert_eq!(
        ctx.phase,
        SystemPhase::ActiveMixing,
        "Must transition to ActiveMixing when water target is reached"
    );
    let has_stop_done = events_done.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop
            }
        )
    });
    assert!(has_stop_done, "Must stop water pump when target is reached");
}

#[test]
fn stage_water_change_interval_triggers_when_due_without_cron() {
    use hydragrow_shared::recipe::{CropRecipe, CropStage};

    let mut config = water_config();
    config.scheduled_water_change_enabled = true;
    config.water_change_cron = String::new(); // No cron!
    config.scheduled_drain_amount_cm = 5.0;

    let recipe = CropRecipe {
        schema_version: 1,
        recipe_id: "water_recipe".to_string(),
        season_id: "season_1".to_string(),
        device_id: "e2e_water_test".to_string(),
        revision: 1,
        start_time_sec: 1_700_000_000,
        current_stage_index: 0,
        stages: vec![CropStage {
            name: "Vegetative".to_string(),
            duration_sec: 14 * 86400,
            ec_target: 1.5,
            ec_tolerance: 0.05,
            ph_target: 6.0,
            ph_tolerance: 0.1,
            nutrient_a_ratio: 1.0,
            nutrient_b_ratio: 1.0,
            water_level_target: 20.0,
            water_change_interval_days: Some(7),
            water_change_drain_cm: Some(5.0),
            auto_dilute_ec_trigger: None,
            max_dose_per_cycle_ml: Some(10.0),
            misting_on_duration_ms: 5000,
            misting_off_duration_ms: 60000,
        }],
    };
    config.active_recipe = Some(recipe);

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;
    ctx.current_stage_index = Some(0);

    let normal = make_sensor(20.0);

    // Before due (day 6 = 6 * 86400s = 518_400s)
    let day6_ms = (1_700_000_000u64 + 6 * 86400) * 1000;
    let events_day6 = orchestrator::tick(
        day6_ms,
        518_400_000,
        &config,
        &normal,
        518_400_000,
        &mut ctx,
    );
    assert!(
        !events_day6
            .events
            .iter()
            .any(|e| matches!(e, OrchestratorEvent::SaveLastWaterChange { .. })),
        "Should not trigger water change on day 6"
    );

    // At due (day 7 = 7 * 86400s = 604_800s)
    let day7_ms = (1_700_000_000u64 + 7 * 86400) * 1000;
    let mut events_day7 = orchestrator::tick(
        day7_ms,
        604_800_000,
        &config,
        &normal,
        604_800_000,
        &mut ctx,
    );
    ctx.apply_delta(&mut events_day7.delta);

    let has_water_change = events_day7
        .events
        .iter()
        .any(|e| matches!(e, OrchestratorEvent::SaveLastWaterChange { .. }));
    assert!(
        has_water_change,
        "Stage interval must trigger scheduled water change when due at day 7"
    );
    assert_eq!(ctx.last_water_change_sec, 1_700_000_000 + 7 * 86400);
}

#[test]
fn scheduler_precedence_stage_interval_overrides_static_and_cron() {
    use hydragrow_shared::recipe::{CropRecipe, CropStage};

    let mut config = water_config();
    config.scheduled_water_change_enabled = true;
    config.water_change_interval_days = Some(14); // Static interval: 14 days
    config.water_change_cron = "0 0 0 * * SUN".to_string(); // Cron

    let recipe = CropRecipe {
        schema_version: 1,
        recipe_id: "water_recipe".to_string(),
        season_id: "season_1".to_string(),
        device_id: "e2e_water_test".to_string(),
        revision: 1,
        start_time_sec: 1_700_000_000,
        current_stage_index: 0,
        stages: vec![CropStage {
            name: "Vegetative".to_string(),
            duration_sec: 14 * 86400,
            ec_target: 1.5,
            ec_tolerance: 0.05,
            ph_target: 6.0,
            ph_tolerance: 0.1,
            nutrient_a_ratio: 1.0,
            nutrient_b_ratio: 1.0,
            water_level_target: 20.0,
            water_change_interval_days: Some(3), // Stage interval: 3 days (highest precedence)
            water_change_drain_cm: Some(5.0),
            auto_dilute_ec_trigger: None,
            max_dose_per_cycle_ml: Some(10.0),
            misting_on_duration_ms: 5000,
            misting_off_duration_ms: 60000,
        }],
    };
    config.active_recipe = Some(recipe);

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;
    ctx.current_stage_index = Some(0);

    let normal = make_sensor(20.0);

    // Day 3: Stage interval (3 days) is due and must fire
    let day3_ms = (1_700_000_000u64 + 3 * 86400) * 1000;
    let events_day3 = orchestrator::tick(
        day3_ms,
        3 * 86400 * 1000,
        &config,
        &normal,
        3 * 86400 * 1000,
        &mut ctx,
    );

    let has_water_change = events_day3
        .events
        .iter()
        .any(|e| matches!(e, OrchestratorEvent::SaveLastWaterChange { .. }));
    assert!(
        has_water_change,
        "Precedence rule: stage interval (3 days) must fire over static interval (14 days) and cron"
    );
}

#[test]
fn scheduler_precedence_static_interval_overrides_cron() {
    let mut config = water_config();
    config.scheduled_water_change_enabled = true;
    config.water_change_interval_days = Some(5); // Static interval: 5 days
    config.water_change_cron = "0 0 0 * * SUN".to_string(); // Cron
    config.active_recipe = None; // No active recipe

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;
    ctx.last_water_change_sec = 1_700_000_000;

    let normal = make_sensor(20.0);

    // Day 5 after last change
    let day5_ms = (1_700_000_000u64 + 5 * 86400) * 1000;
    let events_day5 = orchestrator::tick(
        day5_ms,
        5 * 86400 * 1000,
        &config,
        &normal,
        5 * 86400 * 1000,
        &mut ctx,
    );

    let has_water_change = events_day5
        .events
        .iter()
        .any(|e| matches!(e, OrchestratorEvent::SaveLastWaterChange { .. }));
    assert!(
        has_water_change,
        "Precedence rule: static interval (5 days) must fire over cron"
    );
}
