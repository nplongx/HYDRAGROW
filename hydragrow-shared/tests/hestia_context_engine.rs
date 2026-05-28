use hydragrow_shared::hestia::{
    HestiaAction, HestiaContext, HestiaEngine, HestiaState, HestiaTrendDirection,
};
use hydragrow_shared::{ControllerConfig, PumpStatus, SensorData};

fn sensor(ec: f32, ph: f32, water_level: f32, temp: f32) -> SensorData {
    SensorData {
        device_id: "device_001".to_string(),
        ec,
        ph,
        water_level,
        temp,
        pump_status: PumpStatus::default(),
        time: "2026-05-28T00:00:00Z".to_string(),
        rssi: None,
        free_heap: None,
        uptime: None,
        err_water: None,
        err_temp: None,
        err_ph: None,
        err_ec: None,
        is_continuous: None,
        ph_voltage_mv: None,
    }
}

#[test]
fn hestia_marks_recent_intervention_as_recovery_and_brakes_related_axis() {
    let config = ControllerConfig {
        enable_ec_sensor: true,
        enable_ph_sensor: true,
        enable_water_level_sensor: true,
        enable_temp_sensor: true,
        ..ControllerConfig::default()
    };
    let current = sensor(1.2, 6.0, 20.0, 25.0);
    let previous = sensor(1.2, 6.0, 17.0, 25.0);
    let context = HestiaContext {
        previous: Some(previous),
        minutes_since_previous: Some(30.0),
        minutes_since_last_intervention: Some(12.0),
        last_action: HestiaAction::WaterRefill,
        matrix_is_warm: true,
        mean_kalman_confidence: Some(0.8),
        phase: Some("Cooldown".to_string()),
    };

    let assessment = HestiaEngine::evaluate(&current, &config, &context);

    assert_eq!(assessment.state, HestiaState::Recovery);
    assert_eq!(
        assessment.axes.water_level.trend,
        HestiaTrendDirection::Improving
    );
    assert!(assessment.axes.water_level.action_factor < 1.0);
    assert!(assessment.score >= 80.0);
    assert!(
        assessment
            .reasons
            .iter()
            .any(|r| r == "recent_water_refill")
    );
}

#[test]
fn hestia_raises_critical_when_ph_and_water_are_degrading_fast() {
    let config = ControllerConfig {
        enable_ec_sensor: true,
        enable_ph_sensor: true,
        enable_water_level_sensor: true,
        enable_temp_sensor: true,
        ..ControllerConfig::default()
    };
    let current = sensor(1.2, 8.2, 4.0, 34.0);
    let previous = sensor(1.2, 6.2, 16.0, 27.0);
    let context = HestiaContext {
        previous: Some(previous),
        minutes_since_previous: Some(30.0),
        minutes_since_last_intervention: Some(90.0),
        last_action: HestiaAction::None,
        matrix_is_warm: false,
        mean_kalman_confidence: None,
        phase: Some("Monitoring".to_string()),
    };

    let assessment = HestiaEngine::evaluate(&current, &config, &context);

    assert_eq!(assessment.state, HestiaState::Critical);
    assert!(assessment.score < 60.0);
    assert!(assessment.axes.ph.trend_factor > 1.0);
    assert!(assessment.axes.water_level.trend_factor > 1.0);
    assert!(assessment.reasons.iter().any(|r| r == "ph_out_of_range"));
    assert!(
        assessment
            .reasons
            .iter()
            .any(|r| r == "water_level_critical")
    );
}
