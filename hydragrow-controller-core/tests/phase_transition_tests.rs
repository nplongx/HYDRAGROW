//! Tests cho FSM phase transitions: Monitoring → MimoDosing → Cooldown → Monitoring

#![allow(clippy::field_reassign_with_default)]

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors, low_ec_sensors};

use hydragrow_controller_core::WaterDirection;
use hydragrow_controller_core::core::fsm::{
    context::SystemContext, events::OrchestratorEvent, orchestrator,
};
use hydragrow_shared::fsm::SystemPhase;

fn one_tick(
    ctx: &mut SystemContext,
    config: &hydragrow_shared::ControllerConfig,
    sensors: &hydragrow_shared::SensorData,
    uptime_ms: u64,
) -> hydragrow_controller_core::core::fsm::tick_result::TickResult {
    let now_ms = 1_700_000_000_000u64 + uptime_ms;
    orchestrator::tick(now_ms, uptime_ms, config, sensors, uptime_ms, ctx)
}

// Test 1: Monitoring + EC thấp → trigger MimoDosing
#[test]
fn monitoring_triggers_dosing_when_ec_low() {
    let config = auto_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    let sensors = low_ec_sensors(); // ec = 1.0, target = 1.5
    let result = one_tick(&mut ctx, &config, &sensors, 10_000);

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::MimoDosing),
        "EC thấp trong Auto mode phải trigger MimoDosing"
    );
}

// Test 2: Monitoring + sensors balanced → không trigger (Idle)
#[test]
fn monitoring_idle_when_sensors_balanced() {
    let config = auto_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;

    let sensors = balanced_sensors();
    let result = one_tick(&mut ctx, &config, &sensors, 10_000);

    // Phase không thay đổi hoặc không có delta phase
    let changed_to_dosing = result.delta.phase == Some(SystemPhase::MimoDosing);
    assert!(
        !changed_to_dosing,
        "Sensors balanced phải Idle, không dosing"
    );
}

// Test 3: MimoDosing hard timeout → chuyển sang Cooldown
#[test]
fn mimo_dosing_hard_timeout_goes_to_cooldown() {
    let config = auto_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::MimoDosing;

    // Đặt phase_finish_ms ở quá khứ (timeout đã xảy ra)
    ctx.phase_start_ms = Some(1000);
    ctx.phase_finish_ms = Some(5000); // finish ở uptime 5000ms

    let sensors = balanced_sensors();
    let uptime_ms = 15_000u64; // uptime 15s >> phase_finish 5s (+ 5000ms buffer)

    let result = one_tick(&mut ctx, &config, &sensors, uptime_ms);

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::Cooldown),
        "MimoDosing timeout phải chuyển sang Cooldown"
    );
}

// Test 4: Cooldown timeout → quay về Monitoring
#[test]
fn cooldown_expires_returns_to_monitoring() {
    let config = auto_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Cooldown;

    // Cooldown đã hết hạn
    ctx.phase_finish_ms = Some(5000);
    let uptime_ms = 6_000u64; // > phase_finish

    let sensors = balanced_sensors();
    let result = one_tick(&mut ctx, &config, &sensors, uptime_ms);

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::Monitoring),
        "Cooldown hết hạn phải về Monitoring"
    );
}

// Test 5: Cooldown chưa hết hạn → giữ nguyên
#[test]
fn cooldown_keeps_phase_while_not_expired() {
    let config = auto_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Cooldown;

    ctx.phase_finish_ms = Some(30_000);
    let uptime_ms = 10_000u64; // < phase_finish

    let result = one_tick(&mut ctx, &config, &balanced_sensors(), uptime_ms);

    assert!(
        result.delta.phase.is_none() || result.delta.phase == Some(SystemPhase::Cooldown),
        "Cooldown chưa hết không được chuyển phase"
    );
}

// Test 6: ActiveMixing stable → chuyển sang Stabilizing
#[test]
fn active_mixing_stable_transitions_to_stabilizing() {
    let mut config = auto_config();
    config.enable_ec_sensor = true;
    config.enable_ph_sensor = true;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::ActiveMixing;
    ctx.phase_start_ms = Some(0);
    ctx.phase_finish_ms = Some(60_000);

    // Push 5 mẫu ổn định (range < 0.05)
    for _ in 0..5 {
        ctx.stabilizer_tracker.push(1.5, 6.0);
    }

    let uptime_ms = 20_000u64; // > 15_000ms min
    let sensors = balanced_sensors();

    let result = one_tick(&mut ctx, &config, &sensors, uptime_ms);

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::Stabilizing),
        "ActiveMixing với tracker ổn định phải chuyển sang Stabilizing"
    );
}

// Test 7: Stop automation khi đang dosing và chuyển Manual
#[test]
fn automation_stop_emits_hardware_off_events() {
    let mut config = auto_config();
    config.control_mode = hydragrow_shared::ControlMode::Manual;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::MimoDosing; // đang dosing
    ctx.phase_start_ms = Some(0);

    let result = one_tick(&mut ctx, &config, &balanced_sensors(), 10_000);

    // Phải emit SetWaterPump Stop
    let stops_water = result.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetWaterPump { direction }
                if *direction == WaterDirection::Stop
        )
    });
    assert!(
        stops_water,
        "Chuyển Manual khi đang dosing phải dừng water pump"
    );
    assert_eq!(result.delta.phase, Some(SystemPhase::ManualMode));
}
