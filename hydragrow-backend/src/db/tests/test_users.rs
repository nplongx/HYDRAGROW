#[cfg(test)]
mod tests {
    use crate::db::users::*;

    #[sqlx::test]
    async fn upsert_user_creates_record(pool: sqlx::PgPool) {
        let scopes = vec!["device:read".to_string(), "device:write".to_string()];
        let user = upsert_user(
            &pool,
            "firebase-uid-abc",
            "alice@example.com",
            Some("Alice"),
            &scopes,
        )
        .await
        .unwrap();
        assert_eq!(user.firebase_uid, "firebase-uid-abc");
        assert_eq!(user.email, "alice@example.com");
        assert!(user.is_active);
        assert_eq!(user.scopes.len(), 2);
    }

    #[sqlx::test]
    async fn upsert_user_updates_email_on_conflict(pool: sqlx::PgPool) {
        let scopes = vec!["device:read".to_string()];
        upsert_user(&pool, "firebase-uid-dup", "old@example.com", None, &scopes)
            .await
            .unwrap();
        let updated = upsert_user(
            &pool,
            "firebase-uid-dup",
            "new@example.com",
            Some("Bob"),
            &scopes,
        )
        .await
        .unwrap();
        assert_eq!(updated.email, "new@example.com");
        assert_eq!(updated.display_name.as_deref(), Some("Bob"));
    }

    #[sqlx::test]
    async fn find_active_by_firebase_uid_returns_none_for_unknown(pool: sqlx::PgPool) {
        let result = find_active_by_firebase_uid(&pool, "no-such-uid")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test]
    async fn find_active_by_firebase_uid_returns_user(pool: sqlx::PgPool) {
        let scopes = vec!["admin".to_string()];
        upsert_user(&pool, "firebase-uid-xyz", "charlie@example.com", None, &scopes)
            .await
            .unwrap();
        let found = find_active_by_firebase_uid(&pool, "firebase-uid-xyz")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, "charlie@example.com");
    }
}
