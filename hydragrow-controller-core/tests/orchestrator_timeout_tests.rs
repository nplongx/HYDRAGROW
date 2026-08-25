//! Tests cho sensor timeout và noise detection trong orchestrator

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
