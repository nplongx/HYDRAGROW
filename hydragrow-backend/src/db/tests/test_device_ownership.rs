#[cfg(test)]
mod tests {
    use crate::db::device_ownership::*;

    #[sqlx::test]
    async fn is_owner_of_all_returns_false_when_one_not_owned(pool: sqlx::PgPool) {
        let _ = claim_device(&pool, 1, "dev-a", None).await.unwrap();
        // dev-b không thuộc user 1
        let result = is_owner_of_all(&pool, 1, &["dev-a", "dev-b"])
            .await
            .unwrap();
        assert!(!result);
    }

    #[sqlx::test]
    async fn list_device_ids_for_user_returns_only_owned(pool: sqlx::PgPool) {
        let _ = claim_device(&pool, 2, "dev-x", None).await.unwrap();
        let _ = claim_device(&pool, 2, "dev-y", None).await.unwrap();
        let ids = list_device_ids_for_user(&pool, 2).await.unwrap();
        assert!(ids.contains(&"dev-x".to_string()));
        assert!(ids.contains(&"dev-y".to_string()));
        assert_eq!(ids.len(), 2);
    }
}
