use hydragrow_shared::{
    MqttCommandOut, MqttCommandParams, PumpStatus, SensorData,
    log::{
        AlertMetadata, BasicSystemLogMetadata, CalibrationMetadata, LogCategory, LogLevel,
        SystemLogEvent, UnifiedSystemLog, WaterMetadata,
    },
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
            source: "scheduler".into(),
            trigger: "auto_refill".into(),
            level_before: 11.5,
            level_after: 19.9,
            target_level: 20.0,
            duration_sec: 24,
            success: true,
            cycle_id: Some("cycle-water-001".into()),
            retry_count: Some(0),
        }),
        SystemLogEvent::SystemAlert(AlertMetadata {
            alert_type: "rate_limit".into(),
            source: "ec_dosing".into(),
            retry_count: 2,
            limit_value: Some(200.0),
            threshold_before: None,
            threshold_after: None,
        }),
        SystemLogEvent::CalibrationUpdate(CalibrationMetadata {
            source: "auto_tuner".into(),
            parameter: "ec_step_ratio".into(),
            old_value: Some(0.3),
            new_value: Some(0.4),
            skip_reason: None,
            cycle_id: Some("cal-001".into()),
        }),
        SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
            cycle_id: None,
            source: "system".into(),
            message: "node boot".into(),
            skip_reason: None,
        }),
    ];

    for event in variants {
        let original = sample_system_log(event);
        let json = serde_json::to_string(&original).expect("serialize unified log");
        let decoded: UnifiedSystemLog =
            serde_json::from_str(&json).expect("deserialize unified log");
        assert_eq!(decoded.device_id, original.device_id);
        assert_eq!(decoded.level, original.level);
        assert_eq!(decoded.category, original.category);
        assert_eq!(decoded.title, original.title);
        assert_eq!(decoded.timestamp_ms, original.timestamp_ms);
        assert_eq!(
            serde_json::to_value(&decoded.event).unwrap(),
            serde_json::to_value(&original.event).unwrap()
        );
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
        controller_received_ms: None,
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
        controller_received_ms: Some(123_456),
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
        assert_eq!(
            serde_json::to_value(&decoded).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
    }
}

#[test]
fn mqtt_command_payload_round_trip_for_common_actions() {
    let actions = vec![
        MqttCommandOut {
            target: "pump_a".into(),
            action: "start".into(),
            params: Some(MqttCommandParams {
                pump_id: Some("pump_a".into()), // 👈 Chuyển thành Some()
                duration_sec: Some(8),
                pwm: Some(70),
                state: Some(true),
                ota_url: None,
            }),
        },
        MqttCommandOut {
            target: "pump_a".into(),
            action: "stop".into(),
            params: Some(MqttCommandParams {
                pump_id: Some("pump_a".into()), // 👈 Chuyển thành Some()
                duration_sec: None,
                pwm: None,
                state: Some(false),
                ota_url: None,
            }),
        },
        MqttCommandOut {
            target: "controller".into(),
            action: "set_pwm".into(),
            params: Some(MqttCommandParams {
                pump_id: Some("osaka_pump".into()), // 👈 Chuyển thành Some()
                duration_sec: None,
                pwm: Some(55),
                state: None,
                ota_url: None,
            }),
        },
        MqttCommandOut {
            target: "controller".into(),
            action: "set_mode".into(),
            params: None,
        },
    ];

    for original in actions {
        let json = serde_json::to_string(&original).expect("serialize mqtt payload");
        let decoded: MqttCommandOut =
            serde_json::from_str(&json).expect("deserialize mqtt payload");
        assert_eq!(
            serde_json::to_value(&decoded).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
    }
}

#[test]
fn golden_payload_snapshots() {
    let unified = sample_system_log(SystemLogEvent::CalibrationUpdate(CalibrationMetadata {
        source: "auto_tuner".into(),
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
        controller_received_ms: None,
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

    let command = MqttCommandOut {
        target: "pump_b".into(),
        action: "start".into(),
        params: Some(MqttCommandParams {
            pump_id: Some("pump_b".into()),
            duration_sec: Some(12),
            pwm: Some(65),
            state: Some(true),
            ota_url: None,
        }),
    };

    insta::assert_json_snapshot!("unified_system_log_golden", unified);
    insta::assert_json_snapshot!("sensor_data_golden", sensor);
    insta::assert_json_snapshot!("mqtt_command_payload_golden", command);
}

#[test]
fn basic_system_log_metadata_cycle_id_round_trips() {
    let with_cycle = SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
        source: "orchestrator".into(),
        message: "Bắt đầu châm EC".into(),
        skip_reason: None,
        cycle_id: Some("ec-1748000000000".into()),
    });

    let json = serde_json::to_string(&with_cycle).expect("serialize");
    assert!(
        json.contains("ec-1748000000000"),
        "cycle_id phải xuất hiện trong JSON: {}",
        json
    );

    let decoded: SystemLogEvent = serde_json::from_str(&json).expect("deserialize");
    match decoded {
        SystemLogEvent::BasicSystemLog(meta) => {
            assert_eq!(meta.cycle_id.as_deref(), Some("ec-1748000000000"));
        }
        _ => panic!("Sai variant"),
    }
}

#[test]
fn basic_system_log_metadata_without_cycle_id_omits_field() {
    let without_cycle = SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
        source: "system".into(),
        message: "Khởi động".into(),
        skip_reason: None,
        cycle_id: None,
    });

    let json = serde_json::to_string(&without_cycle).expect("serialize");
    // Với skip_serializing_if = "Option::is_none", field không được xuất hiện
    assert!(
        !json.contains("cycle_id"),
        "cycle_id None phải bị omit: {}",
        json
    );
}

#[test]
fn golden_basic_system_log_with_cycle_id() {
    let log = UnifiedSystemLog {
        device_id: "device-001".into(),
        level: LogLevel::Info,
        category: LogCategory::Dosing,
        title: "Bắt đầu châm EC".into(),
        event: SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
            source: "orchestrator".into(),
            message: "Bơm A+B: 5.00ml | EC hiện tại: 1.20 | Mục tiêu: 2.00 | PWM: 50%".into(),
            skip_reason: None,
            cycle_id: Some("ec-1748000000000".into()),
        }),
        timestamp_ms: 1_748_000_000_000,
    };

    insta::assert_json_snapshot!("basic_system_log_with_cycle_id_golden", log);
}

#[test]
fn golden_build_basic_log_json_with_ts_snapshot() {
    let json_str = UnifiedSystemLog::build_basic_log_json_with_ts(
        "device-001",
        LogLevel::Info,
        LogCategory::Dosing,
        "Bắt đầu chu kỳ MIMO",
        "A/B: 5.0ml | pH_Up/Down: 1.0/0.0ml | Water_In: 0.0s",
        Some("mimo-1748000000000"),
        "monitoring_phase",
        1_748_000_000_000,
    );

    // Parse lại để verify struct
    let decoded: UnifiedSystemLog = serde_json::from_str(&json_str)
        .expect("build_basic_log_json_with_ts phải tạo JSON parse được bởi backend");

    insta::assert_json_snapshot!("build_basic_log_json_with_ts_golden", decoded);
}
