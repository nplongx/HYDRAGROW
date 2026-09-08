#[cfg(test)]
mod tests {
    use crate::db::postgres::*;
    use crate::models::config::DeviceConfig;
    use crate::models::crop_season::CreateCropSeasonRequest;

    // ── device_config ─────────────────────────────────────────────────────────

    #[sqlx::test(migrations = "./migrations")]
    async fn upsert_and_get_device_config_roundtrip(pool: sqlx::PgPool) {
        let cfg = DeviceConfig {
            device_id: "test-dev-001".to_string(),
            ec_target: 1.8,
            ec_tolerance: 0.2,
            ph_target: 6.0,
            ph_tolerance: 0.3,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 5,
            last_updated: chrono::Utc::now(),
        };
        upsert_device_config(&pool, &cfg).await.unwrap();
        let fetched = get_device_config(&pool, "test-dev-001").await.unwrap();
        assert!((fetched.ec_target - 1.8).abs() < f32::EPSILON);
        assert_eq!(fetched.control_mode, "auto");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn get_device_config_returns_error_for_unknown(pool: sqlx::PgPool) {
        let result = get_device_config(&pool, "does-not-exist").await;
        assert!(result.is_err());
    }

    // ── crop_seasons ──────────────────────────────────────────────────────────

    #[sqlx::test(migrations = "./migrations")]
    async fn get_device_dosing_reports_in_range_filters_by_created_at(pool: sqlx::PgPool) {
        insert_dosing_report(
            &pool,
            "dev-range-1",
            None,
            1.0,
            1.0,
            0.0,
            0.0,
            &serde_json::json!({"cycle_id": "c1"}),
        )
        .await
        .unwrap();

        let start = chrono::Utc::now() - chrono::Duration::hours(1);
        let end = chrono::Utc::now() + chrono::Duration::hours(1);
        let in_range = get_device_dosing_reports_in_range(&pool, "dev-range-1", start, end)
            .await
            .unwrap();
        assert_eq!(in_range.len(), 1);

        let outside_start = chrono::Utc::now() + chrono::Duration::hours(2);
        let outside_end = chrono::Utc::now() + chrono::Duration::hours(3);
        let out_of_range =
            get_device_dosing_reports_in_range(&pool, "dev-range-1", outside_start, outside_end)
                .await
                .unwrap();
        assert_eq!(out_of_range.len(), 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn create_and_get_crop_season(pool: sqlx::PgPool) {
        // device_config is a FK prerequisite
        let cfg = DeviceConfig {
            device_id: "test-dev-002".to_string(),
            ec_target: 1.4,
            ec_tolerance: 0.1,
            ph_target: 6.0,
            ph_tolerance: 0.2,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 5,
            last_updated: chrono::Utc::now(),
        };
        upsert_device_config(&pool, &cfg).await.unwrap();

        let req = CreateCropSeasonRequest {
            name: "Lettuce Q3".to_string(),
            plant_type: Some("lettuce".to_string()),
            description: None,
        };
        let season = create_crop_season(&pool, "test-dev-002", req)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(season.device_id, "test-dev-002");
        assert_eq!(season.name, "Lettuce Q3");
    }

    // ── system_events ─────────────────────────────────────────────────────────

    #[sqlx::test(migrations = "./migrations")]
    async fn insert_and_query_system_event(pool: sqlx::PgPool) {
        let cfg = DeviceConfig {
            device_id: "test-dev-evt".to_string(),
            ec_target: 1.4,
            ec_tolerance: 0.1,
            ph_target: 6.0,
            ph_tolerance: 0.2,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 5,
            last_updated: chrono::Utc::now(),
        };
        upsert_device_config(&pool, &cfg).await.unwrap();

        let event = NewSystemEventRecord {
            device_id: "test-dev-evt".to_string(),
            level: "info".to_string(),
            category: "dosing".to_string(),
            title: "Dose OK".to_string(),
            message: "EC reached target".to_string(),
            reason: None,
            metadata: Some(serde_json::json!({ "recipe_id": "r1" })),
            timestamp: chrono::Utc::now().timestamp_millis(),
            source: "rule".to_string(),
            primary_reason_code: None,
        };
        insert_system_event(&pool, &event).await.unwrap();

        let events = get_system_events(&pool, "test-dev-evt", &[], 10, None, None, None)
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, "dosing");
        assert_eq!(events[0].source, "rule");
        assert!(events[0].primary_reason_code.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn insert_ai_supervisor_event_with_reason_code(pool: sqlx::PgPool) {
        let cfg = DeviceConfig {
            device_id: "test-dev-ai".to_string(),
            ec_target: 1.4,
            ec_tolerance: 0.1,
            ph_target: 6.0,
            ph_tolerance: 0.2,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 5,
            last_updated: chrono::Utc::now(),
        };
        upsert_device_config(&pool, &cfg).await.unwrap();

        let event = NewSystemEventRecord {
            device_id: "test-dev-ai".to_string(),
            level: "warning".to_string(),
            category: "alert".to_string(),
            title: "Suspected leak".to_string(),
            message: "Water level dropping fast".to_string(),
            reason: None,
            metadata: Some(serde_json::json!({ "reason_codes": ["leak_suspected"], "confidence": 0.9 })),
            timestamp: chrono::Utc::now().timestamp_millis(),
            source: "ai_supervisor".to_string(),
            primary_reason_code: Some("leak_suspected".to_string()),
        };
        insert_system_event(&pool, &event).await.unwrap();

        let events = get_system_events(&pool, "test-dev-ai", &[], 10, None, None, None)
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].source, "ai_supervisor");
        assert_eq!(events[0].primary_reason_code.as_deref(), Some("leak_suspected"));
    }

    // ── find_recent_alert (supervisor dedup) ──────────────────────────────────

    async fn dedup_cfg(pool: &sqlx::PgPool, device_id: &str) {
        let cfg = DeviceConfig {
            device_id: device_id.to_string(),
            ec_target: 1.4,
            ec_tolerance: 0.1,
            ph_target: 6.0,
            ph_tolerance: 0.2,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 5,
            last_updated: chrono::Utc::now(),
        };
        upsert_device_config(pool, &cfg).await.unwrap();
    }

    fn dedup_event(device_id: &str, ts: i64, source: &str, code: &str) -> NewSystemEventRecord {
        NewSystemEventRecord {
            device_id: device_id.to_string(),
            level: "warning".to_string(),
            category: "alert".to_string(),
            title: "t".to_string(),
            message: "m".to_string(),
            reason: None,
            metadata: None,
            timestamp: ts,
            source: source.to_string(),
            primary_reason_code: Some(code.to_string()),
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn find_recent_alert_returns_true_within_cooldown(pool: sqlx::PgPool) {
        dedup_cfg(&pool, "test-dev-dedup").await;
        let now = chrono::Utc::now().timestamp_millis();
        insert_system_event(&pool, &dedup_event("test-dev-dedup", now, "ai_supervisor", "leak_suspected"))
            .await
            .unwrap();
        let found =
            find_recent_alert(&pool, "test-dev-dedup", "ai_supervisor", "leak_suspected", 20)
                .await
                .unwrap();
        assert!(found);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn find_recent_alert_ignores_different_source(pool: sqlx::PgPool) {
        dedup_cfg(&pool, "test-dev-dedup").await;
        let now = chrono::Utc::now().timestamp_millis();
        insert_system_event(&pool, &dedup_event("test-dev-dedup", now, "watchdog", "topic_stale_controller_status"))
            .await
            .unwrap();
        let found = find_recent_alert(
            &pool,
            "test-dev-dedup",
            "ai_supervisor",
            "topic_stale_controller_status",
            20,
        )
        .await
        .unwrap();
        assert!(!found);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn find_recent_alert_returns_false_outside_cooldown(pool: sqlx::PgPool) {
        dedup_cfg(&pool, "test-dev-dedup").await;
        let old = chrono::Utc::now().timestamp_millis() - 30 * 60 * 1000;
        insert_system_event(&pool, &dedup_event("test-dev-dedup", old, "ai_supervisor", "leak_suspected"))
            .await
            .unwrap();
        let found =
            find_recent_alert(&pool, "test-dev-dedup", "ai_supervisor", "leak_suspected", 20)
                .await
                .unwrap();
        assert!(!found);
    }

    // ── dosing_reports ────────────────────────────────────────────────────────

    #[sqlx::test(migrations = "./migrations")]
    async fn insert_dosing_report_persists(pool: sqlx::PgPool) {
        let cfg = DeviceConfig {
            device_id: "test-dev-dose".to_string(),
            ec_target: 1.4,
            ec_tolerance: 0.1,
            ph_target: 6.0,
            ph_tolerance: 0.2,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 5,
            last_updated: chrono::Utc::now(),
        };
        upsert_device_config(&pool, &cfg).await.unwrap();

        insert_dosing_report(
            &pool,
            "test-dev-dose",
            None,
            1.5, // pump_a_ml
            1.5, // pump_b_ml
            0.2, // ph_up_ml
            0.0, // ph_down_ml
            &serde_json::json!({ "status": "ok" }),
        )
        .await
        .unwrap();

        let reports = get_device_dosing_reports(&pool, "test-dev-dose", None)
            .await
            .unwrap();
        assert_eq!(reports.len(), 1);
        assert!((reports[0].pump_a_ml - 1.5).abs() < f32::EPSILON);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn insert_and_fetch_dosing_action_log(pool: sqlx::PgPool) {
        let cfg = DeviceConfig {
            device_id: "dev-log-1".to_string(),
            ec_target: 1.4,
            ec_tolerance: 0.1,
            ph_target: 6.0,
            ph_tolerance: 0.2,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 5,
            last_updated: chrono::Utc::now(),
        };
        upsert_device_config(&pool, &cfg).await.unwrap();

        insert_dosing_action(&pool, "dev-log-1", "PH_DOWN", 3.0)
            .await
            .unwrap();
        insert_dosing_action(&pool, "dev-log-1", "PH_DOWN", 2.0)
            .await
            .unwrap();

        let history = get_dosing_history_last_hour(&pool, "dev-log-1")
            .await
            .unwrap();
        assert_eq!(history.len(), 2);
        let total: f32 = history.iter().map(|(_, ml)| ml).sum();
        assert!((total - 5.0).abs() < 1e-4);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn get_last_dose_at_returns_none_when_no_history(pool: sqlx::PgPool) {
        let cfg = DeviceConfig {
            device_id: "dev-log-2".to_string(),
            ec_target: 1.4,
            ec_tolerance: 0.1,
            ph_target: 6.0,
            ph_tolerance: 0.2,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 5,
            last_updated: chrono::Utc::now(),
        };
        upsert_device_config(&pool, &cfg).await.unwrap();
        let result = get_last_dose_at(&pool, "dev-log-2").await.unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn fetch_dosing_calibration_returns_none_when_missing(pool: sqlx::PgPool) {
        let result = fetch_dosing_calibration(&pool, "no-such-device")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn fetch_dosing_calibration_returns_row_when_present(pool: sqlx::PgPool) {
        let cfg = DeviceConfig {
            device_id: "dev-cal-1".to_string(),
            ec_target: 1.8,
            ec_tolerance: 0.2,
            ph_target: 6.0,
            ph_tolerance: 0.3,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 5,
            last_updated: chrono::Utc::now(),
        };
        upsert_device_config(&pool, &cfg).await.unwrap(); // FK prerequisite

        sqlx::query(
            r#"
            INSERT INTO dosing_calibration (
                device_id, ec_gain_per_ml, ph_shift_up_per_ml, ph_shift_down_per_ml,
                active_mixing_sec, sensor_stabilize_sec, ec_step_ratio, ph_step_ratio,
                pump_a_capacity_ml_per_sec, pump_b_capacity_ml_per_sec,
                pump_ph_up_capacity_ml_per_sec, pump_ph_down_capacity_ml_per_sec,
                soft_start_duration, last_calibrated,
                scheduled_mixing_interval_sec, scheduled_mixing_duration_sec,
                dosing_pwm_percent, osaka_mixing_pwm_percent, osaka_misting_pwm_percent,
                dosing_min_pwm_percent, pump_a_min_pwm_percent, pump_b_min_pwm_percent,
                pump_ph_up_min_pwm_percent, pump_ph_down_min_pwm_percent, dosing_pulse_on_ms,
                dosing_pulse_off_ms, dosing_min_dose_ml, dosing_max_pulse_count_per_cycle
            ) VALUES (
                'dev-cal-1', 0.01, 0.01, 0.01,
                300, 60, 1.0, 1.0,
                1.2, 1.2,
                0.8, 0.8,
                3000, NOW(),
                3600, 300,
                50, 60, 100,
                10, 10, 10,
                10, 10, 200,
                200, 0.1, 20
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let fetched = fetch_dosing_calibration(&pool, "dev-cal-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.device_id, "dev-cal-1");
        assert!((fetched.pump_ph_down_capacity_ml_per_sec - 0.8).abs() < f32::EPSILON);
    }
}
