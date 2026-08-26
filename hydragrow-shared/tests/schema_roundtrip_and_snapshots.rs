use hydragrow_shared::{
    MqttCommandOut, MqttCommandParams, PumpStatus, SensorData,
    log::{
        AlertMetadata, BasicSystemLogMetadata, CalibrationMetadata, LogCategory, LogLevel,
        RecipeAppliedMetadata, RecipeCompletedMetadata, RecipeRejectedMetadata,
        RecipeStageChangedMetadata, SystemLogEvent, UnifiedSystemLog, WaterMetadata,
    },
    recipe::{CropRecipe, CropStage, RecipeStageChangedEvent},
};

fn sample_crop_recipe() -> CropRecipe {
    CropRecipe {
        schema_version: 2,
        recipe_id: "lettuce-romaine-v1".into(),
        season_id: "season-2026-08".into(),
        device_id: "device-001".into(),
        revision: 7,
        start_time_sec: 1_777_000_000,
        current_stage_index: 1,
        stages: vec![
            CropStage {
                name: "seedling".into(),
                duration_sec: 604_800,
                ec_target: 0.8,
                ec_tolerance: 0.1,
                ph_target: 6.0,
                ph_tolerance: 0.2,
                nutrient_a_ratio: 1.0,
                nutrient_b_ratio: 1.0,
                water_level_target: 18.0,
                water_change_interval_days: None,
                water_change_drain_cm: None,
                auto_dilute_ec_trigger: None,
                misting_on_duration_ms: 10_000,
                misting_off_duration_ms: 180_000,
                max_dose_per_cycle_ml: None,
            },
            CropStage {
                name: "vegetative".into(),
                duration_sec: 1_209_600,
                ec_target: 1.4,
                ec_tolerance: 0.15,
                ph_target: 6.1,
                ph_tolerance: 0.2,
                nutrient_a_ratio: 1.0,
                nutrient_b_ratio: 1.0,
                water_level_target: 20.0,
                water_change_interval_days: None,
                water_change_drain_cm: None,
                auto_dilute_ec_trigger: None,
                misting_on_duration_ms: 10_000,
                misting_off_duration_ms: 180_000,
                max_dose_per_cycle_ml: None,
            },
        ],
    }
}

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
        SystemLogEvent::RecipeApplied(RecipeAppliedMetadata {
            recipe_id: "recipe-lettuce-a".into(),
            recipe_name: "Lettuce A".into(),
            source: "web_app".into(),
            stage_id: Some("veg".into()),
            stage_name: Some("Vegetative".into()),
            cycle_id: Some("recipe-cycle-001".into()),
        }),
        SystemLogEvent::RecipeRejected(RecipeRejectedMetadata {
            recipe_id: "recipe-lettuce-b".into(),
            recipe_name: Some("Lettuce B".into()),
            source: "firmware".into(),
            reason: "ec_target_out_of_range".into(),
            stage_id: Some("bloom".into()),
            stage_name: Some("Bloom".into()),
            cycle_id: Some("recipe-cycle-002".into()),
        }),
        SystemLogEvent::RecipeStageChanged(RecipeStageChangedMetadata {
            recipe_id: "recipe-lettuce-a".into(),
            recipe_name: "Lettuce A".into(),
            source: "scheduler".into(),
            from_stage_id: Some("seedling".into()),
            from_stage_name: Some("Seedling".into()),
            to_stage_id: "veg".into(),
            to_stage_name: "Vegetative".into(),
            cycle_id: Some("recipe-cycle-003".into()),
        }),
        SystemLogEvent::RecipeCompleted(RecipeCompletedMetadata {
            recipe_id: "recipe-lettuce-a".into(),
            recipe_name: "Lettuce A".into(),
            source: "scheduler".into(),
            final_stage_id: Some("harvest".into()),
            final_stage_name: Some("Harvest".into()),
            cycle_id: Some("recipe-cycle-004".into()),
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
                candidates: None,
            }),
            ts: Some(1_771_000_000),
            nonce: Some("nonce-test".into()),
            signature: Some("sig-test".into()),
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
                candidates: None,
            }),
            ts: Some(1_771_000_000),
            nonce: Some("nonce-test".into()),
            signature: Some("sig-test".into()),
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
                candidates: None,
            }),
            ts: Some(1_771_000_000),
            nonce: Some("nonce-test".into()),
            signature: Some("sig-test".into()),
        },
        MqttCommandOut {
            target: "controller".into(),
            action: "set_mode".into(),
            params: None,
            ts: Some(1_771_000_000),
            nonce: Some("nonce-test".into()),
            signature: Some("sig-test".into()),
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
    let unified = sample_system_log(SystemLogEvent::RecipeStageChanged(
        RecipeStageChangedMetadata {
            recipe_id: "recipe-lettuce-a".into(),
            recipe_name: "Lettuce A".into(),
            source: "scheduler".into(),
            from_stage_id: Some("seedling".into()),
            from_stage_name: Some("Seedling".into()),
            to_stage_id: "veg".into(),
            to_stage_name: "Vegetative".into(),
            cycle_id: Some("recipe-cycle-003".into()),
        },
    ));

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
            candidates: None,
        }),
        ts: Some(1_771_000_000),
        nonce: Some("nonce-golden".into()),
        signature: Some("sig-golden".into()),
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

#[test]
fn crop_recipe_snapshot_round_trip_preserves_required_fields() {
    let original = sample_crop_recipe();
    let json = serde_json::to_string(&original).expect("serialize crop recipe");
    let decoded: CropRecipe = serde_json::from_str(&json).expect("deserialize crop recipe");

    assert_eq!(decoded, original);
    assert_eq!(decoded.schema_version, 2);
    assert_eq!(decoded.recipe_id, "lettuce-romaine-v1");
    assert_eq!(decoded.season_id, "season-2026-08");
    assert_eq!(decoded.device_id, "device-001");
    assert_eq!(decoded.revision, 7);
    assert_eq!(decoded.start_time_sec, 1_777_000_000);
    assert_eq!(decoded.current_stage_index, 1);
    assert_eq!(decoded.stages.len(), 2);
}

#[test]
fn golden_crop_recipe_snapshot() {
    insta::assert_json_snapshot!("crop_recipe_golden", sample_crop_recipe());
}

#[test]
fn recipe_stage_changed_event_round_trip() {
    let recipe = sample_crop_recipe();
    let event = RecipeStageChangedEvent {
        schema_version: recipe.schema_version,
        recipe_id: recipe.recipe_id,
        season_id: recipe.season_id,
        device_id: recipe.device_id,
        revision: recipe.revision,
        start_time_sec: recipe.start_time_sec,
        previous_stage_index: Some(0),
        current_stage_index: recipe.current_stage_index,
        changed_at_sec: 1_777_604_800,
        stages: recipe.stages,
    };

    let json = serde_json::to_string(&event).expect("serialize recipe stage event");
    let decoded: RecipeStageChangedEvent =
        serde_json::from_str(&json).expect("deserialize recipe stage event");

    assert_eq!(decoded, event);
}
#[test]
fn device_health_snapshot_deserializes_without_firmware_version_field() {
    let old_json = r#"{
        "device_id":"dev-1", "free_heap":12345, "uptime_sec":100, "rssi":-60,
        "health_score_percent":90, "fsm_state_display":"Monitoring", "log_drop_count":0,
        "matrix_update_count":0, "matrix_is_warm":false, "timestamp_ms":1000
    }"#;
    let snapshot: hydragrow_shared::telemetry::health::DeviceHealthSnapshot =
        serde_json::from_str(old_json).expect("legacy health payload must deserialize");
    assert_eq!(snapshot.firmware_version, "unknown");
}
