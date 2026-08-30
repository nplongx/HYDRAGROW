#[cfg(test)]
mod tests {
    use crate::db::postgres::*;
    use crate::models::config::DeviceConfig;
    use crate::models::crop_season::CreateCropSeasonRequest;

    // ── device_config ─────────────────────────────────────────────────────────

    #[sqlx::test]
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

    #[sqlx::test]
    async fn get_device_config_returns_error_for_unknown(pool: sqlx::PgPool) {
        let result = get_device_config(&pool, "does-not-exist").await;
        assert!(result.is_err());
    }

    // ── crop_seasons ──────────────────────────────────────────────────────────

    #[sqlx::test]
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

    #[sqlx::test]
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
        };
        insert_system_event(&pool, &event).await.unwrap();

        let events = get_system_events(&pool, "test-dev-evt", &[], 10, None, None, None)
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].category, "dosing");
    }

    // ── dosing_reports ────────────────────────────────────────────────────────

    #[sqlx::test]
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

    #[sqlx::test]
    async fn fetch_dosing_calibration_returns_none_when_missing(pool: sqlx::PgPool) {
        let result = fetch_dosing_calibration(&pool, "no-such-device")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test]
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
