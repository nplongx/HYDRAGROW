use hydragrow_shared::{
    AlertMetadata, CalibrationMetadata, DosingReportPayload, LogCategory, LogLevel, MqttCommandParams,
    MqttCommandPayload, PumpStatus, SensorData, SystemLogEvent, UnifiedSystemLog, WaterMetadata,
};

fn sample_system_log(event: SystemLogEvent) -> UnifiedSystemLog {
    UnifiedSystemLog {
        device_id: "device-001".into(),
        level: LogLevel::Info,
        category: LogCategory::System,
        title: "schema-test".into(),
        event,
        timestamp_ms: 1_717_171_717_000,
    }
}

#[test]
fn unified_system_log_round_trip_for_all_event_variants() {
    let variants = vec![
        SystemLogEvent::WaterEvent(WaterMetadata {
            trigger: "auto_refill".into(),
            level_before: 11.5,
            level_after: 19.9,
            target_level: 20.0,
            duration_sec: 24,
            success: true,
            cycle_id: Some("cycle-water-001".into()),
        }),
        SystemLogEvent::DosingCycleComplete(DosingReportPayload {
            cycle_id: "dose-001".into(),
            result: "ok".into(),
            target_ec: Some(2.1),
            current_ec: Some(1.9),
            target_ph: Some(6.0),
            current_ph: Some(6.4),
            dose_a_ml: 1.2,
            dose_b_ml: 0.8,
            dose_ph_up_ml: 0.0,
            dose_ph_down_ml: 0.4,
            notes: Some("stable".into()),
            timestamp_ms: 1_717_171_717_001,
        }),
        SystemLogEvent::SystemAlert(AlertMetadata {
            alert_type: "rate_limit".into(),
            source: "ec_dosing".into(),
            retry_count: 2,
            limit_value: Some(200.0),
        }),
        SystemLogEvent::CalibrationUpdate(CalibrationMetadata {
            parameter: "ec_step_ratio".into(),
            old_value: Some(0.3),
            new_value: Some(0.4),
            skip_reason: None,
            cycle_id: Some("cal-001".into()),
        }),
        SystemLogEvent::BasicSystemLog {
            message: "node boot".into(),
        },
    ];

    for event in variants {
        let original = sample_system_log(event);
        let json = serde_json::to_string(&original).expect("serialize unified log");
        let decoded: UnifiedSystemLog = serde_json::from_str(&json).expect("deserialize unified log");

        assert_eq!(decoded.device_id, original.device_id);
        assert_eq!(decoded.level, original.level);
        assert_eq!(decoded.category, original.category);
        assert_eq!(decoded.title, original.title);
        assert_eq!(decoded.timestamp_ms, original.timestamp_ms);
        assert_eq!(serde_json::to_value(&decoded.event).unwrap(), serde_json::to_value(&original.event).unwrap());
    }
}

#[test]
fn sensor_data_round_trip_with_and_without_optional_fields() {
    let base = SensorData {
        device_id: "sensor-001".into(),
        ec: 1.7,
        ph: 6.2,
        temp: 28.3,
        water_level: 18.0,
        pump_status: PumpStatus::default(),
        time: "2026-05-19T00:00:00Z".into(),
        rssi: None,
        free_heap: None,
        uptime: None,
        err_water: None,
        err_temp: None,
        err_ph: None,
        err_ec: None,
        is_continuous: None,
        ph_voltage_mv: None,
    };

    let with_optionals = SensorData {
        rssi: Some(-64),
        free_heap: Some(180_000),
        uptime: Some(3_600),
        err_water: Some(false),
        err_temp: Some(false),
        err_ph: Some(true),
        err_ec: Some(false),
        is_continuous: Some(true),
        ph_voltage_mv: Some(2123.5),
        ..base.clone()
    };

    for original in [base, with_optionals] {
        let json = serde_json::to_string(&original).expect("serialize sensor data");
        let decoded: SensorData = serde_json::from_str(&json).expect("deserialize sensor data");
        assert_eq!(serde_json::to_value(&decoded).unwrap(), serde_json::to_value(&original).unwrap());
    }
}

#[test]
fn mqtt_command_payload_round_trip_for_common_actions() {
    let actions = vec![
        MqttCommandPayload {
            target: "pump_a".into(),
            action: "start".into(),
            params: Some(MqttCommandParams {
                pump_id: "pump_a".into(),
                duration_sec: Some(8),
                pwm: Some(70),
                state: Some(true),
            }),
        },
        MqttCommandPayload {
            target: "pump_a".into(),
            action: "stop".into(),
            params: Some(MqttCommandParams {
                pump_id: "pump_a".into(),
                duration_sec: None,
                pwm: None,
                state: Some(false),
            }),
        },
        MqttCommandPayload {
            target: "controller".into(),
            action: "set_pwm".into(),
            params: Some(MqttCommandParams {
                pump_id: "osaka_pump".into(),
                duration_sec: None,
                pwm: Some(55),
                state: None,
            }),
        },
        MqttCommandPayload {
            target: "controller".into(),
            action: "set_mode".into(),
            params: None,
        },
    ];

    for original in actions {
        let json = serde_json::to_string(&original).expect("serialize mqtt payload");
        let decoded: MqttCommandPayload = serde_json::from_str(&json).expect("deserialize mqtt payload");
        assert_eq!(serde_json::to_value(&decoded).unwrap(), serde_json::to_value(&original).unwrap());
    }
}

#[test]
fn golden_payload_snapshots() {
    let unified = sample_system_log(SystemLogEvent::CalibrationUpdate(CalibrationMetadata {
        parameter: "ec_gain_per_ml".into(),
        old_value: Some(0.012),
        new_value: Some(0.015),
        skip_reason: None,
        cycle_id: Some("c-42".into()),
    }));

    let sensor = SensorData {
        device_id: "sensor-001".into(),
        ec: 1.55,
        ph: 5.98,
        temp: 27.1,
        water_level: 20.0,
        pump_status: PumpStatus::default(),
        time: "2026-05-19T12:00:00Z".into(),
        rssi: Some(-59),
        free_heap: Some(176_320),
        uptime: Some(42_000),
        err_water: Some(false),
        err_temp: Some(false),
        err_ph: Some(false),
        err_ec: Some(false),
        is_continuous: Some(true),
        ph_voltage_mv: Some(2105.2),
    };

    let command = MqttCommandPayload {
        target: "pump_b".into(),
        action: "start".into(),
        params: Some(MqttCommandParams {
            pump_id: "pump_b".into(),
            duration_sec: Some(12),
            pwm: Some(65),
            state: Some(true),
        }),
    };

    insta::assert_json_snapshot!("unified_system_log_golden", unified);
    insta::assert_json_snapshot!("sensor_data_golden", sensor);
    insta::assert_json_snapshot!("mqtt_command_payload_golden", command);
}
