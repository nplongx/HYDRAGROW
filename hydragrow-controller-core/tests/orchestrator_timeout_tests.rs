//! Tests cho sensor timeout và noise detection trong orchestrator

#![allow(clippy::field_reassign_with_default)]

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors, noisy_ec_sensors};

use hydragrow_controller_core::WaterDirection;
use hydragrow_controller_core::core::fsm::{
    context::SystemContext, events::OrchestratorEvent, orchestrator,
};
use hydragrow_shared::fsm::{FaultCode, SystemPhase};

fn make_ctx_monitoring() -> SystemContext {
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;
    ctx
}

// Test 1: Timeout sau 90s → chuyển vào Fault(SensorTimeout)
#[test]
fn sensor_timeout_triggers_fault_after_90s() {
    let config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = make_ctx_monitoring();

    let now_ms = 200_000_000u64;
    let uptime_ms = 200_000u64;
    // sensor_last_update_ms được set ở uptime 0 (100s trước uptime hiện tại)
    let sensor_last_update_ms = 0u64;

    let result = orchestrator::tick(
        now_ms,
        uptime_ms,
        &config,
        &sensors,
        sensor_last_update_ms,
        &mut ctx,
    );

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::Fault(FaultCode::SensorTimeout)),
        "Sau 90s không có sensor data, phải vào Fault(SensorTimeout)"
    );

    // Phải dừng water pump
    let has_stop_water = result.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetWaterPump { direction }
                if *direction == WaterDirection::Stop
        )
    });
    assert!(has_stop_water, "Khi timeout phải dừng water pump");
}

// Test 2: Timeout 89s → vẫn bình thường (chưa đến ngưỡng)
#[test]
fn no_fault_at_89s_sensor_gap() {
    let config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = make_ctx_monitoring();

    let uptime_ms = 89_000u64; // 89s elapsed
    let sensor_last_update_ms = 0u64;

    let result = orchestrator::tick(
        1_700_000_000_000,
        uptime_ms,
        &config,
        &sensors,
        sensor_last_update_ms,
        &mut ctx,
    );

    assert_ne!(
        result.delta.phase,
        Some(SystemPhase::Fault(FaultCode::SensorTimeout)),
        "89s chưa đủ 90s threshold, không được fault"
    );
}

// Test 3: Đang ở Fault(SensorTimeout), nhận sensor mới → tự động recover về Monitoring
#[test]
fn sensor_recovery_exits_fault_to_monitoring() {
    let config = auto_config();
    let sensors = balanced_sensors();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Fault(FaultCode::SensorTimeout);

    let uptime_ms = 200_000u64;
    // sensor vừa được update (1s trước = 199_000ms)
    let sensor_last_update_ms = 199_000u64;

    let result = orchestrator::tick(
        1_700_000_000_000,
        uptime_ms,
        &config,
        &sensors,
        sensor_last_update_ms,
        &mut ctx,
    );

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::Monitoring),
        "Nhận sensor mới sau SensorTimeout phải tự recover về Monitoring"
    );
}

// Test 4: Noise detection — EC spike lớn → bỏ qua tick, không dosing
#[test]
fn noise_spike_aborts_dosing_decision() {
    let mut config = auto_config();
    config.enable_ec_sensor = true;
    config.max_ec_delta = 0.3;

    let mut ctx = make_ctx_monitoring();
    // Set previous EC
    ctx.peripherals.previous_ec = Some(1.5);

    // EC spike: 1.5 → 2.5 (delta = 1.0 > max_ec_delta 0.3)
    let noisy = noisy_ec_sensors(1.5);

    let uptime_ms = 10_000u64;
    let result = orchestrator::tick(
        1_700_000_000_000,
        uptime_ms,
        &config,
        &noisy,
        uptime_ms,
        &mut ctx,
    );

    // Phải không có phase transition sang MimoDosing
    assert_ne!(
        result.delta.phase,
        Some(SystemPhase::MimoDosing),
        "Noise spike không được trigger dosing"
    );
}

// Test 5: Manual mode → không dosing, chuyển về ManualMode
#[test]
fn manual_mode_stops_automation() {
    let mut config = auto_config();
    config.control_mode = hydragrow_shared::ControlMode::Manual;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    let sensors = balanced_sensors();
    let uptime_ms = 10_000u64;

    let result = orchestrator::tick(
        1_700_000_000_000,
        uptime_ms,
        &config,
        &sensors,
        uptime_ms,
        &mut ctx,
    );

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::ManualMode),
        "Manual mode phải chuyển sang ManualMode"
    );
}

// Test 6: is_enabled = false → dừng automation
#[test]
fn disabled_controller_stops_automation() {
    let mut config = auto_config();
    config.control_mode = hydragrow_shared::ControlMode::Auto;
    config.is_enabled = false;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    let result = orchestrator::tick(
        1_700_000_000_000,
        10_000,
        &config,
        &balanced_sensors(),
        10_000,
        &mut ctx,
    );

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::ManualMode),
        "is_enabled = false phải dừng automation"
    );
}

// Test 7: FORCE ON override remains active with divergent clocks (uptime vs Unix wall-clock)
#[test]
fn force_on_override_stays_active_with_divergent_clocks() {
    let config = auto_config();
    let mut sensors = balanced_sensors();
    sensors.ec = 0.5; // low EC triggers dosing
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;
    // Exhaust hourly budget so without override it would fault with MaxHourlyDoseEc
    ctx.safety
        .commit_hourly_dose("NutrientA", 0, config.max_dose_per_hour * 2.0);

    // Override active for 10s at uptime 10_000ms -> expires at uptime 20_000ms
    ctx.safety.safety_override_until = 20_000;

    // Tick at uptime 19_999ms, but wall-clock time is Unix timestamp
    let result = orchestrator::tick(
        1_700_000_000_000,
        19_999,
        &config,
        &sensors,
        19_999,
        &mut ctx,
    );

    assert_ne!(
        result.delta.phase,
        Some(SystemPhase::Fault(FaultCode::MaxHourlyDoseEc)),
        "Override must be active based on uptime_ms, bypassing budget even when Unix wall-clock is much larger"
    );
}

// Test 8: Water pump timeout is based on water_pump_started_uptime_ms, not phase_start_ms
#[test]
fn water_timeout_uses_pump_started_uptime_not_phase_start() {
    use hydragrow_controller_core::core::actors::dosing_actor::DosingSubState;
    use hydragrow_controller_core::core::fsm::phase_tick::PhaseTick;
    use hydragrow_controller_core::core::fsm::phases::mimo_dosing::MimoDosingPhase;

    let mut config = auto_config();
    config.max_refill_duration_sec = 5; // 5s timeout

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::MimoDosing;
    ctx.phase_start_ms = Some(1_000);
    ctx.phase_finish_ms = Some(30_000); // not timed out
    ctx.dosing.sub_state = DosingSubState::SoftStarting {
        finish_ms: 30_000,
        next_state: Box::new(DosingSubState::Idle),
    };
    ctx.peripherals.pump_status.water_pump_in = true;
    ctx.peripherals.water_pump_started_uptime_ms = Some(10_000);

    let sensors = balanced_sensors();
    let phase = MimoDosingPhase;

    // Tick at uptime 10_500ms (elapsed for pump is 500ms, but phase elapsed is 9_500ms)
    let result = phase.tick(1_700_000_000_000, 10_500, &config, &sensors, &mut ctx);

    // Should NOT stop water pump after only 500ms
    let has_water_stop = result.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetWaterPump { direction }
                if *direction == WaterDirection::Stop
        )
    });
    assert!(
        !has_water_stop,
        "Water pump should not time out after only 500ms of pump running"
    );

    // Tick at uptime 15_000ms (elapsed for pump is 5000ms >= max_refill_duration_sec)
    let result_timeout = phase.tick(1_700_000_000_000, 15_000, &config, &sensors, &mut ctx);
    let has_water_stop_timeout = result_timeout.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetWaterPump { direction }
                if *direction == WaterDirection::Stop
        )
    });
    assert!(
        has_water_stop_timeout,
        "Water pump must time out when pump running duration reaches max_refill_duration_sec"
    );
    assert_eq!(
        result_timeout
            .delta
            .peripherals
            .as_ref()
            .and_then(|p| p.water_pump_in),
        Some(false)
    );
    assert_eq!(
        result_timeout
            .delta
            .peripherals
            .as_ref()
            .and_then(|p| p.water_pump_started_uptime_ms),
        Some(None)
    );
}

// Test 9: Noise detection invalidates pending calibration sample and AutoTuner refuses to learn
#[test]
fn noise_invalidates_pending_calibration_sample_and_tuner_refuses_learning() {
    use hydragrow_controller_core::core::fsm::tick_result::CalibrationDelta;
    use hydragrow_controller_core::core::fsm::types::PendingCalibrationSample;

    let mut config = auto_config();
    config.enable_ec_sensor = true;
    config.max_ec_delta = 0.3;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;
    ctx.peripherals.previous_ec = Some(1.5);

    let sample = PendingCalibrationSample {
        cycle_id: "test-cycle".to_string(),
        trigger: "mimo_test".to_string(),
        start_ec: 1.5,
        start_ph: 6.0,
        start_water_level: 20.0,
        start_temp: 25.0,
        target_ec: 2.0,
        target_ph: 6.0,
        dose_a_ml: 5.0,
        dose_b_ml: 5.0,
        dose_ph_up_ml: 0.0,
        dose_ph_down_ml: 0.0,
        water_in_sec: 0.0,
        water_out_sec: 0.0,
        post_mixing_ec: 0.0,
        post_mixing_ph: 0.0,
        start_ms: 1000,
        active_mixing_finish_ms: 0,
        stabilizing_start_ms: None,
        stabilizing_finish_ms: None,
        invalid_by_noise: false,
        invalid_by_water_change: false,
    };
    ctx.apply_delta(&mut hydragrow_controller_core::core::fsm::ContextDelta {
        calibration: Some(CalibrationDelta::Start(sample)),
        ..Default::default()
    });

    assert!(ctx.calibration.pending_sample.is_some());
    assert!(
        !ctx.calibration
            .pending_sample
            .as_ref()
            .unwrap()
            .invalid_by_noise
    );

    // EC spike: 1.5 -> 2.5 (> max_ec_delta 0.3)
    let noisy = noisy_ec_sensors(1.5);
    let mut result =
        orchestrator::tick(1_700_000_000_000, 10_000, &config, &noisy, 10_000, &mut ctx);

    // ContextDelta must contain CalibrationDelta::Invalidate
    assert_eq!(
        result.delta.calibration,
        Some(CalibrationDelta::Invalidate),
        "Orchestrator must emit CalibrationDelta::Invalidate when sensor noise is detected"
    );

    // Apply delta to context
    ctx.apply_delta(&mut result.delta);
    let pending = ctx.calibration.pending_sample.as_ref().unwrap();
    assert!(
        pending.invalid_by_noise,
        "Pending sample must be marked invalid_by_noise"
    );

    // AutoTuner refuses to learn
    let learned = ctx
        .tuner
        .learn_from_cycle(pending, 2.0, 6.0, 20.0, 25.0, &config, 10);
    assert!(
        !learned,
        "AutoTuner must refuse to learn from sample marked invalid_by_noise"
    );
}
