#[cfg(test)]
mod tests {
    use crate::db::postgres::{delete_fcm_token, get_fcm_tokens_for_device, upsert_fcm_token};

    #[sqlx::test(migrations = "./migrations")]
    async fn upsert_new_token_stores_it(pool: sqlx::PgPool) {
        upsert_fcm_token(&pool, "device_001", "token_abc")
            .await
            .unwrap();
        let tokens = get_fcm_tokens_for_device(&pool, "device_001")
            .await
            .unwrap();
        assert_eq!(tokens, vec!["token_abc".to_string()]);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn upsert_duplicate_token_is_idempotent(pool: sqlx::PgPool) {
        upsert_fcm_token(&pool, "device_001", "token_abc")
            .await
            .unwrap();
        upsert_fcm_token(&pool, "device_001", "token_abc")
            .await
            .unwrap(); // duplicate
        let tokens = get_fcm_tokens_for_device(&pool, "device_001")
            .await
            .unwrap();
        assert_eq!(tokens.len(), 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn get_tokens_returns_empty_for_unknown_device(pool: sqlx::PgPool) {
        let tokens = get_fcm_tokens_for_device(&pool, "unknown_device")
            .await
            .unwrap();
        assert!(tokens.is_empty());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn delete_token_removes_it(pool: sqlx::PgPool) {
        upsert_fcm_token(&pool, "device_001", "token_abc")
            .await
            .unwrap();
        delete_fcm_token(&pool, "device_001", "token_abc")
            .await
            .unwrap();
        let tokens = get_fcm_tokens_for_device(&pool, "device_001")
            .await
            .unwrap();
        assert!(tokens.is_empty());
    }
}
