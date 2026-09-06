//! E2E: Giả lập một chu kỳ dosing hoàn chỉnh từ Monitoring → MimoDosing → Cooldown → Monitoring

use hydragrow_controller_core::core::fsm::{
    context::SystemContext, events::OrchestratorEvent, orchestrator,
};
use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::{ControlMode, ControllerConfig, SensorData};

/// Config tối ưu cho E2E test: cooldown ngắn, timeout ngắn
fn e2e_config() -> ControllerConfig {
    ControllerConfig {
        control_mode: ControlMode::Auto,
        is_enabled: true,
        ec_target: 1.5,
        ec_tolerance: 0.05,
        ph_target: 6.0,
        ph_tolerance: 0.1,
        enable_ec_sensor: true,
        enable_ph_sensor: true,
        enable_water_level_sensor: false,
        enable_temp_sensor: false,
        cooldown_sec: 2,        // Ngắn để test nhanh
        soft_start_duration: 0, // Bỏ soft start
        max_dose_per_hour: 50.0,
        max_dose_per_cycle: 10.0,
        max_refill_cycles_per_hour: 4,
        max_drain_cycles_per_hour: 4,
        pump_a_capacity_ml_per_sec: 2.0,
        pump_b_capacity_ml_per_sec: 2.0,
        pump_ph_up_capacity_ml_per_sec: 1.0,
        pump_ph_down_capacity_ml_per_sec: 1.0,
        dosing_pwm_percent: 80,
        dosing_min_pwm_percent: 30,
        dosing_pulse_on_ms: 50,
        dosing_pulse_off_ms: 50,
        dosing_min_dose_ml: 0.1,
        dosing_max_pulse_count_per_cycle: 100,
        ec_step_ratio: 1.0,
        ph_step_ratio: 1.0,
        best_ec_ratio: 1.0,
        best_ph_ratio: 1.0,
        ec_gain_per_ml: 0.3,
        ph_shift_up_per_ml: 0.15,
        ph_shift_down_per_ml: 0.15,
        adaptive_mixing_sec: 5,
        adaptive_stabilize_sec: 5,
        effective_ec_tolerance: 0.05,
        effective_ph_tolerance: 0.1,
        active_mixing_sec: 5,
        sensor_stabilize_sec: 5,
        max_ec_delta: 1.0,
        max_ph_delta: 1.0,
        max_ec_limit: 3.0,
        min_ec_limit: 0.1,
        min_ph_limit: 4.5,
        max_ph_limit: 8.0,
        min_temp_limit: 15.0,
        max_temp_limit: 35.0,
        water_level_min: 15.0,
        water_level_target: 20.0,
        water_level_max: 24.0,
        water_level_tolerance: 1.0,
        water_level_critical_min: 10.0,
        max_refill_duration_sec: 60,
        max_drain_duration_sec: 60,
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
        auto_refill_enabled: false,
        auto_drain_overflow: false,
        auto_dilute_enabled: false,
        dilute_drain_amount_cm: 0.0,
        scheduled_water_change_enabled: false,
        water_change_cron: String::new(),
        scheduled_drain_amount_cm: 0.0,
        water_change_interval_days: None,
        emergency_shutdown: false,
        nutrient_a_ratio: 1.0,
        nutrient_b_ratio: 1.0,
        device_id: "e2e_test".to_string(),
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

/// E2E Test: Monitoring + low EC → MimoDosing → (wait timeout) → Cooldown → Monitoring
#[test]
fn e2e_full_dosing_cycle_monitoring_to_cooldown_to_monitoring() {
    let config = e2e_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    // Sensor: EC thấp rõ ràng để trigger dosing
    let low_ec_sensor = SensorData {
        device_id: "e2e_test".to_string(),
        ec: 0.8, // << target 1.5
        ph: 6.0,
        temp: 25.0,
        water_level: 20.0,
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
        ec_received_ms: None,
        ph_received_ms: None,
        temp_received_ms: None,
        water_received_ms: None,
    };

    // === Phase 1: Monitoring → MimoDosing ===
    let mut uptime_ms = 10_000u64;
    let _events1 = tick_apply(&mut ctx, &config, &low_ec_sensor, uptime_ms);
    assert_eq!(
        ctx.phase,
        SystemPhase::MimoDosing,
        "EC = 0.8 << target 1.5 phải trigger MimoDosing"
    );
    // Tick trong MimoDosing để DosingActor hoàn thành SoftStarting và bật bơm
    let mut has_dosing = false;
    for _step in 1..=5 {
        uptime_ms += 100;
        let events = tick_apply(&mut ctx, &config, &low_ec_sensor, uptime_ms);
        if events
            .iter()
            .any(|e| matches!(e, OrchestratorEvent::SetDosingPump { .. }))
        {
            has_dosing = true;
            break;
        }
    }
    assert!(has_dosing, "MimoDosing phải emit SetDosingPump events");
    eprintln!("✅ Phase 1: Monitoring → MimoDosing OK");

    // === Phase 2: Tick trong MimoDosing cho đến Cooldown ===
    // Force timeout bằng cách đặt phase_finish_ms ở quá khứ
    let phase_start = uptime_ms;
    ctx.phase_start_ms = Some(phase_start);
    ctx.phase_finish_ms = Some(phase_start + 100); // Timeout sau 100ms
    uptime_ms = phase_start + 10_100; // >> finish_ms + 5000ms buffer
    let _events = tick_apply(&mut ctx, &config, &low_ec_sensor, uptime_ms);
    assert_eq!(
        ctx.phase,
        SystemPhase::Cooldown,
        "Sau MimoDosing timeout phải vào Cooldown"
    );
    eprintln!("✅ Phase 2: MimoDosing → Cooldown OK");

    // === Phase 3: Cooldown timeout → Monitoring ===
    let cooldown_start = uptime_ms;
    ctx.phase_finish_ms = Some(cooldown_start + 2_000); // Cooldown 2s
    uptime_ms = cooldown_start + 3_000; // > finish
    let balanced_sensor = SensorData {
        ec: 1.5, // Balanced
        ph: 6.0,
        ..low_ec_sensor.clone()
    };
    let _events = tick_apply(&mut ctx, &config, &balanced_sensor, uptime_ms);
    assert_eq!(
        ctx.phase,
        SystemPhase::Monitoring,
        "Sau Cooldown timeout phải về Monitoring"
    );
    eprintln!("✅ Phase 3: Cooldown → Monitoring OK");
}

/// E2E Test: Verify events emitted khi bắt đầu dosing
#[test]
fn e2e_dosing_start_emits_correct_hardware_events() {
    let config = e2e_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    let low_ec = SensorData {
        device_id: "e2e_test".to_string(),
        ec: 0.5, // Rất thấp → cần dose nhiều
        ph: 6.0,
        temp: 25.0,
        water_level: 20.0,
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
        ec_received_ms: None,
        ph_received_ms: None,
        temp_received_ms: None,
        water_received_ms: None,
    };

    let mut all_events = Vec::new();
    let mut uptime_ms = 10_000;
    for _ in 0..5 {
        let events = tick_apply(&mut ctx, &config, &low_ec, uptime_ms);
        all_events.extend(events);
        uptime_ms += 100;
    }

    // Verify events
    let has_dosing_pump = all_events
        .iter()
        .any(|e| matches!(e, OrchestratorEvent::SetDosingPump { on: true, .. }));
    let has_system_log = all_events
        .iter()
        .any(|e| matches!(e, OrchestratorEvent::PublishSystemLog { .. }));

    assert!(has_dosing_pump, "Dosing phải emit SetDosingPump on=true");
    assert!(has_system_log, "Dosing phải emit PublishSystemLog");
    eprintln!("✅ Events verified: dosing pump + system log emitted");
}

/// E2E Test: pH thấp → trigger pH Up dosing
#[test]
fn e2e_low_ph_triggers_ph_up_dosing() {
    let config = e2e_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    let low_ph = SensorData {
        device_id: "e2e_test".to_string(),
        ec: 1.5, // EC balanced
        ph: 5.0, // pH thấp << target 6.0
        temp: 25.0,
        water_level: 20.0,
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
        ec_received_ms: None,
        ph_received_ms: None,
        temp_received_ms: None,
        water_received_ms: None,
    };

    let _events = tick_apply(&mut ctx, &config, &low_ph, 10_000);

    assert_eq!(
        ctx.phase,
        SystemPhase::MimoDosing,
        "pH thấp phải trigger MimoDosing cho pH Up"
    );
    eprintln!("✅ pH Up dosing triggered");
}
