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
}
