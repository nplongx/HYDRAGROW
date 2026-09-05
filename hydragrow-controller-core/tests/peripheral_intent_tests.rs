//! Regression tests for same-tick peripheral intent and deterministic ownership resolution

#![allow(clippy::field_reassign_with_default)]

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors};

use hydragrow_controller_core::core::fsm::events::OrchestratorEvent;
use hydragrow_controller_core::core::fsm::{context::SystemContext, orchestrator};
use hydragrow_shared::fsm::SystemPhase;

#[test]
fn phase_turning_misting_on_makes_osaka_use_misting_pwm_in_same_tick() {
    let mut config = auto_config();
    config.osaka_misting_pwm_percent = 90;
    config.osaka_mixing_pwm_percent = 40;
    config.high_temp_misting_on_duration_ms = 5000;
    config.misting_on_duration_ms = 5000;
    config.misting_off_duration_ms = 0; // immediate trigger

    let sensors = balanced_sensors();

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;
    ctx.peripherals.is_misting_active = false;
    ctx.peripherals.pump_status.osaka_pump = false;
    ctx.peripherals.last_mist_toggle_time = 0;

    let now_ms = 10_000;
    let uptime_ms = 10_000;

    let result = orchestrator::tick(now_ms, uptime_ms, &config, &sensors, now_ms, &mut ctx);

    // Misting should be triggered in this tick
    let peri_delta = result.delta.peripherals.expect("Must produce peripheral delta");
    assert_eq!(
        peri_delta.is_misting_active,
        Some(true),
        "Misting must be activated"
    );

    // Osaka decision in the SAME tick MUST use misting PWM (90%), not mixing PWM (40%)
    assert_eq!(
        peri_delta.osaka_pwm,
        Some(90),
        "Same-tick Osaka decision must use the resolved misting intent (90%), not stale state (40%)"
    );

    let has_soft_start_90 = result.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::StartOsakaSoft {
                target_pwm_percent: 90
            }
        )
    });
    assert!(
        has_soft_start_90,
        "Must emit StartOsakaSoft with 90% for misting"
    );
}

#[test]
fn mix_valve_conflict_dosing_owner_wins_and_only_one_command_emitted() {
    let mut config = auto_config();
    config.scheduled_mixing_interval_sec = 60;
    config.scheduled_mixing_duration_sec = 10;

    let sensors = balanced_sensors();

    let mut ctx = SystemContext::default();
    // Phase is ActiveMixing: dosing owns the mix valve
    ctx.phase = SystemPhase::ActiveMixing;
    ctx.phase_start_ms = Some(1000);
    ctx.phase_finish_ms = Some(100_000);
    ctx.peripherals.mix_valve_started_by_dosing = true;
    ctx.peripherals.pump_status.mix_valve = true;

    // Set scheduled mixing state so scheduler wants to end mixing (now >= last_start + duration)
    ctx.peripherals.is_scheduled_mixing_active = true;
    ctx.peripherals.last_mixing_start_sec = 0; // 0 + 10 = 10s -> at 20s, scheduler wants OFF!

    let now_ms = 20_000;
    let uptime_ms = 20_000;

    let result = orchestrator::tick(now_ms, uptime_ms, &config, &sensors, now_ms, &mut ctx);

    // Filter all SetMixValve events
    let mix_events: Vec<_> = result
        .events
        .iter()
        .filter(|e| matches!(e, OrchestratorEvent::SetMixValve { .. }))
        .collect();

    // Must NOT emit conflicting events (both ON and OFF) or emit OFF when dosing owns it
    assert!(
        mix_events.iter().all(|e| matches!(e, OrchestratorEvent::SetMixValve { on: true })),
        "Dosing owner must keep mix valve ON, and scheduler must not emit SetMixValve OFF"
    );
    assert!(
        mix_events.len() <= 1,
        "At most one SetMixValve command should be emitted per tick, got: {:?}",
        mix_events
    );
}

#[test]
fn same_tick_phase_mix_valve_ownership_suppresses_scheduled_mixing_off() {
    let mut config = auto_config();
    config.scheduled_mixing_interval_sec = 60;
    config.scheduled_mixing_duration_sec = 10;

    let sensors = balanced_sensors();

    let mut ctx = SystemContext::default();
    ctx.phase = SystemPhase::Monitoring;
    // Prior state in ctx has NO dosing ownership
    ctx.peripherals.mix_valve_started_by_dosing = false;

    // Scheduled mixing is active and due to finish (scheduler wants OFF)
    ctx.peripherals.is_scheduled_mixing_active = true;
    ctx.peripherals.last_mixing_start_sec = 0;

    let now_ms = 20_000;
    let uptime_ms = 20_000;

    // Simulate phase producing a mix valve request with dosing ownership in THIS tick
    let mut phase_result = hydragrow_controller_core::core::fsm::tick_result::TickResult::default();
    let mut phase_peri = hydragrow_controller_core::core::fsm::PeripheralDelta::default();
    phase_peri.mix_valve = Some(true);
    phase_peri.mix_valve_started_by_dosing = Some(true);
    phase_result.delta.peripherals = Some(phase_peri);
    phase_result.events.push(OrchestratorEvent::SetMixValve { on: true });

    // Run orchestrator tick merging this phase result
    let mut result = hydragrow_controller_core::core::fsm::tick_result::TickResult::default();
    orchestrator::merge_tick_results(&mut result, phase_result);

    // Now call tick_peripheral_systems
    // We observe whether SetMixValve { on: false } gets appended
    let tick_result = orchestrator::tick_peripheral_systems(
        result,
        &ctx,
        &sensors,
        now_ms,
        uptime_ms,
        &config,
        true, // is_dosing_active
    );

    let mix_events: Vec<_> = tick_result
        .events
        .iter()
        .filter(|e| matches!(e, OrchestratorEvent::SetMixValve { .. }))
        .collect();

    // Must NOT have emitted SetMixValve { on: false }
    assert!(
        mix_events.iter().all(|e| matches!(e, OrchestratorEvent::SetMixValve { on: true })),
        "Dosing ownership declared in current tick must suppress scheduled mixing OFF event"
    );
    assert_eq!(mix_events.len(), 1, "Must emit exactly one SetMixValve event");
}
