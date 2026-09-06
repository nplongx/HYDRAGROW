//! Tests for per-channel sensor freshness and disabled sensor policy (Invariant I6)

mod helpers;
use helpers::fixtures::{auto_config, balanced_sensors};

use hydragrow_controller_core::core::fsm::{context::SystemContext, orchestrator};
use hydragrow_shared::fsm::{FaultCode, SystemPhase};

#[test]
fn per_channel_freshness_stale_ph_triggers_fault_while_ec_fresh() {
    let mut config = auto_config();
    config.enable_ec_sensor = true;
    config.enable_ph_sensor = true;

    let mut sensors = balanced_sensors();
    let uptime_ms = 100_000u64;

    // EC is fresh (received 1s ago), but pH is stale (received 95s ago > 90s threshold)
    sensors.ec_received_ms = Some(99_000);
    sensors.ph_received_ms = Some(5_000);

    let mut ctx = SystemContext {
        phase: SystemPhase::Monitoring,
        ..SystemContext::default()
    };

    let result = orchestrator::tick(uptime_ms, uptime_ms, &config, &sensors, uptime_ms, &mut ctx);

    assert_eq!(
        result.delta.phase,
        Some(SystemPhase::Fault(FaultCode::SensorTimeout)),
        "Stale pH channel must trigger SensorTimeout even if EC channel is fresh"
    );
}

#[test]
fn disabled_channel_with_nan_does_not_fault() {
    let mut config = auto_config();
    config.enable_ec_sensor = true;
    config.enable_ph_sensor = false; // pH disabled

    let mut sensors = balanced_sensors();
    sensors.ph = f32::NAN; // NaN on disabled sensor
    sensors.ec = 1.8;

    let uptime_ms = 10_000u64;
    let mut ctx = SystemContext {
        phase: SystemPhase::Monitoring,
        ..SystemContext::default()
    };

    let result = orchestrator::tick(uptime_ms, uptime_ms, &config, &sensors, uptime_ms, &mut ctx);

    assert_ne!(
        result.delta.phase,
        Some(SystemPhase::Fault(FaultCode::SensorTimeout)),
        "Disabled sensor channel with NaN must not trigger SensorTimeout fault"
    );
}

#[test]
fn disabled_channel_with_stale_timestamp_does_not_fault() {
    let mut config = auto_config();
    config.enable_ec_sensor = true;
    config.enable_ph_sensor = false; // pH disabled

    let mut sensors = balanced_sensors();
    let uptime_ms = 100_000u64;

    // EC is fresh, pH has very old timestamp but is disabled
    sensors.ec_received_ms = Some(99_000);
    sensors.ph_received_ms = Some(1_000);

    let mut ctx = SystemContext {
        phase: SystemPhase::Monitoring,
        ..SystemContext::default()
    };

    let result = orchestrator::tick(uptime_ms, uptime_ms, &config, &sensors, uptime_ms, &mut ctx);

    assert_ne!(
        result.delta.phase,
        Some(SystemPhase::Fault(FaultCode::SensorTimeout)),
        "Disabled sensor channel with stale timestamp must not trigger SensorTimeout fault"
    );
}
