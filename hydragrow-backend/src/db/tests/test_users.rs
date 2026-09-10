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
        upsert_user(
            &pool,
            "firebase-uid-xyz",
            "charlie@example.com",
            None,
            &scopes,
        )
        .await
        .unwrap();
        let found = find_active_by_firebase_uid(&pool, "firebase-uid-xyz")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, "charlie@example.com");
    }

    #[sqlx::test]
    async fn provision_creates_user_with_read_scope(pool: sqlx::PgPool) {
        let user = provision_default_user(&pool, "self-reg-uid-1", "dana@example.com")
            .await
            .unwrap();
        assert_eq!(user.firebase_uid, "self-reg-uid-1");
        assert_eq!(user.email, "dana@example.com");
        assert!(user.is_active);
        assert_eq!(user.scopes, vec!["read:telemetry".to_string()]);
    }

    #[sqlx::test]
    async fn provision_does_not_wipe_admin_scopes(pool: sqlx::PgPool) {
        let admin_scopes = vec!["admin".to_string(), "write:config".to_string()];
        upsert_user(
            &pool,
            "self-reg-uid-2",
            "erin@example.com",
            Some("Erin"),
            &admin_scopes,
        )
        .await
        .unwrap();
        // Lần truy cập sau của user admin phải giữ nguyên scope đã được cấp.
        let user = provision_default_user(&pool, "self-reg-uid-2", "erin@example.com")
            .await
            .unwrap();
        assert_eq!(user.scopes, admin_scopes);
        assert_eq!(user.display_name.as_deref(), Some("Erin"));
    }

    #[sqlx::test]
    async fn list_users_and_update_user(pool: sqlx::PgPool) {
        let user = provision_default_user(&pool, "uid-list-test", "list@example.com")
            .await
            .unwrap();
        assert_eq!(user.role_or_viewer(), "viewer");

        let all = list_users(&pool).await.unwrap();
        assert!(all.iter().any(|u| u.id == user.id));

        let updated = update_user(
            &pool,
            user.id,
            Some("admin"),
            Some(&["*".to_string()]),
            Some(true),
        )
        .await
        .unwrap()
        .expect("User must be found");

        assert_eq!(updated.role.as_deref(), Some("admin"));
        assert_eq!(updated.scopes, vec!["*".to_string()]);

        let prefs = serde_json::json!({"weekly_report": true});
        let with_prefs = update_user_preferences(&pool, "uid-list-test", &prefs)
            .await
            .unwrap()
            .expect("User preferences must be updated");
        assert_eq!(with_prefs.preferences, Some(prefs));
    }
}
