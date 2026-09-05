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

#[test]
fn manual_mode_returns_to_monitoring_when_auto_reenabled() {
    let config = auto_config();
    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::ManualMode;

    let result = one_tick(&mut ctx, &config, &balanced_sensors(), 10_000);

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::Monitoring),
        "Khi control_mode là Auto và is_enabled là true, ManualMode phải chuyển về Monitoring"
    );
    assert_eq!(result.delta.phase_start_ms, Some(None));
    assert_eq!(result.delta.phase_finish_ms, Some(None));
    assert!(result.delta.reset_stabilizer);
}

#[test]
fn manual_mode_actuator_stop_is_complete() {
    let mut config = auto_config();
    config.control_mode = hydragrow_shared::ControlMode::Manual;

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::WaterRefilling;
    ctx.peripherals.pump_status.water_pump_in = true;
    ctx.peripherals.pump_status.osaka_pump = true;
    ctx.peripherals.pump_status.mist_valve = true;
    ctx.peripherals.pump_status.mix_valve = true;

    let result = one_tick(&mut ctx, &config, &balanced_sensors(), 10_000);

    assert_eq!(result.delta.phase, Some(SystemPhase::ManualMode));

    // Verify all actuator groups are commanded OFF in events
    let has_water_stop = result.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetWaterPump { direction } if *direction == WaterDirection::Stop
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
    let has_pump_a_off = result.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetDosingPump {
                pump: hydragrow_controller_core::core::fsm::events::DosingPumpTarget::NutrientA,
                on: false,
                ..
            }
        )
    });
    let has_ph_up_off = result.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetDosingPump {
                pump: hydragrow_controller_core::core::fsm::events::DosingPumpTarget::PhUp,
                on: false,
                ..
            }
        )
    });

    assert!(has_water_stop, "Must emit SetWaterPump Stop");
    assert!(has_mist_off, "Must emit SetMistValve false");
    assert!(has_mix_off, "Must emit SetMixValve false");
    assert!(has_osaka_off, "Must emit SetOsakaPump 0");
    assert!(has_pump_a_off, "Must emit SetDosingPump A false");
    assert!(has_ph_up_off, "Must emit SetDosingPump PhUp false");

    // Logical peripherals must also be set to false
    let peri = result
        .delta
        .peripherals
        .expect("Must have peripheral delta");
    assert_eq!(peri.water_pump_in, Some(false));
    assert_eq!(peri.mist_valve, Some(false));
    assert_eq!(peri.mix_valve, Some(false));
    assert_eq!(peri.osaka_pump, Some(false));
    assert_eq!(peri.osaka_pwm, Some(0));
    assert_eq!(peri.pump_a, Some(false));
    assert_eq!(peri.ph_up, Some(false));
}

#[test]
fn merge_tick_results_preserves_independent_peripheral_fields() {
    use hydragrow_controller_core::core::fsm::orchestrator::merge_tick_results;
    use hydragrow_controller_core::core::fsm::tick_result::{PeripheralDelta, TickResult};

    let mut base = TickResult::default();
    base.delta.peripherals = Some(PeripheralDelta {
        water_pump_in: Some(true),
        ..Default::default()
    });
    let mut addition = TickResult::default();
    addition.delta.peripherals = Some(PeripheralDelta {
        mist_valve: Some(true),
        ..Default::default()
    });
    merge_tick_results(&mut base, addition);
    let pd = base.delta.peripherals.expect("merged peripheral delta");
    assert_eq!(pd.water_pump_in, Some(true));
    assert_eq!(pd.mist_valve, Some(true));
}

#[test]
fn merge_peripheral_deltas_resolves_valve_conflict_by_dosing_ownership() {
    use hydragrow_controller_core::core::fsm::tick_result::PeripheralDelta;

    // Case 1: Dosing addition takes ownership and sets mist_valve = true over scheduled false
    let mut scheduled_base = PeripheralDelta {
        mist_valve: Some(false),
        misting_started_by_dosing: Some(false),
        ..Default::default()
    };
    let dosing_addition = PeripheralDelta {
        mist_valve: Some(true),
        misting_started_by_dosing: Some(true),
        ..Default::default()
    };
    scheduled_base.merge_from(dosing_addition);
    assert_eq!(scheduled_base.mist_valve, Some(true));
    assert_eq!(scheduled_base.misting_started_by_dosing, Some(true));

    // Case 2: Scheduled addition (non-dosing) cannot override existing dosing-owned mist_valve = true
    let mut dosing_base = PeripheralDelta {
        mist_valve: Some(true),
        misting_started_by_dosing: Some(true),
        ..Default::default()
    };
    let scheduled_addition = PeripheralDelta {
        mist_valve: Some(false),
        misting_started_by_dosing: Some(false),
        ..Default::default()
    };
    dosing_base.merge_from(scheduled_addition);
    assert_eq!(
        dosing_base.mist_valve,
        Some(true),
        "Scheduled cannot override dosing ownership"
    );
    assert_eq!(dosing_base.misting_started_by_dosing, Some(true));

    // Case 3: Same for mix valve
    let mut mix_base = PeripheralDelta {
        mix_valve: Some(true),
        mix_valve_started_by_dosing: Some(true),
        ..Default::default()
    };
    let scheduled_mix = PeripheralDelta {
        mix_valve: Some(false),
        mix_valve_started_by_dosing: Some(false),
        ..Default::default()
    };
    mix_base.merge_from(scheduled_mix);
    assert_eq!(
        mix_base.mix_valve,
        Some(true),
        "Scheduled cannot override dosing mix ownership"
    );
    assert_eq!(mix_base.mix_valve_started_by_dosing, Some(true));
}
