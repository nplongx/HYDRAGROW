#[cfg(test)]
mod tests {
    use crate::db::service_api_keys::*;

    #[sqlx::test]
    async fn service_api_key_roundtrip(pool: sqlx::PgPool) {
        let scopes = vec!["read:telemetry".to_string(), "write:config".to_string()];
        let (record, raw_key) = create_service_api_key(&pool, "test-service", &scopes)
            .await
            .unwrap();
        assert!(raw_key.starts_with("svc_"));
        assert_eq!(record.label, "test-service");
        assert_eq!(record.scopes, scopes);
        assert!(record.is_active);

        let found = find_active_by_key_hash(&pool, &sha256_hex(&raw_key)).await;
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, record.id);
        assert_eq!(found.scopes, scopes);
    }

    #[sqlx::test]
    async fn service_api_key_inactive_returns_none(pool: sqlx::PgPool) {
        let scopes = vec!["read:telemetry".to_string()];
        let (record, raw_key) = create_service_api_key(&pool, "to-deactivate", &scopes)
            .await
            .unwrap();
        deactivate_service_api_key(&pool, record.id).await.unwrap();

        let found = find_active_by_key_hash(&pool, &sha256_hex(&raw_key)).await;
        assert!(found.is_none());
    }

    #[sqlx::test]
    async fn service_api_key_unknown_hash_returns_none(pool: sqlx::PgPool) {
        let found = find_active_by_key_hash(&pool, "deadbeef").await;
        assert!(found.is_none());
    }
}
