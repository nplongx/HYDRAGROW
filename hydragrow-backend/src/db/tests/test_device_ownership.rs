#[cfg(test)]
mod tests {
    use crate::db::device_ownership::*;
    use crate::db::users::upsert_user;

    async fn create_test_user(pool: &sqlx::PgPool, user_id_int: i64) -> i64 {
        let firebase_uid = format!("uid-{}", user_id_int);
        let email = format!("user{}@example.com", user_id_int);
        let user = upsert_user(pool, &firebase_uid, &email, None, &[])
            .await
            .unwrap();
        user.id
    }

    // ── existing tests (preserved) ────────────────────────────────────────────

    #[sqlx::test]
    async fn is_owner_of_all_returns_false_when_one_not_owned(pool: sqlx::PgPool) {
        let uid = create_test_user(&pool, 1).await;
        let _ = claim_device(&pool, uid, "dev-a", None).await.unwrap();
        let result = is_owner_of_all(&pool, uid, &["dev-a", "dev-b"])
            .await
            .unwrap();
        assert!(!result);
    }

    #[sqlx::test]
    async fn list_device_ids_for_user_returns_only_owned(pool: sqlx::PgPool) {
        let uid = create_test_user(&pool, 2).await;
        let _ = claim_device(&pool, uid, "dev-x", None).await.unwrap();
        let _ = claim_device(&pool, uid, "dev-y", None).await.unwrap();
        let ids = list_device_ids_for_user(&pool, uid).await.unwrap();
        assert!(ids.contains(&"dev-x".to_string()));
        assert!(ids.contains(&"dev-y".to_string()));
        assert_eq!(ids.len(), 2);
    }

    // ── new tests ─────────────────────────────────────────────────────────────

    #[sqlx::test]
    async fn claim_device_returns_mqtt_credentials_on_first_claim(pool: sqlx::PgPool) {
        let uid = create_test_user(&pool, 10).await;
        let (record, credentials) = claim_device(&pool, uid, "dev-new", Some("My Sensor"))
            .await
            .unwrap();
        assert_eq!(record.device_id, "dev-new");
        assert_eq!(record.label.as_deref(), Some("My Sensor"));
        let creds = credentials.expect("first claim must return MQTT credentials");
        assert!(creds.mqtt_username.contains("dev-new"));
        assert!(!creds.mqtt_password.is_empty());
    }

    #[sqlx::test]
    async fn claim_device_does_not_regenerate_credentials_on_reclaim(pool: sqlx::PgPool) {
        let uid = create_test_user(&pool, 11).await;
        let (_, first_creds) = claim_device(&pool, uid, "dev-reclaim", None).await.unwrap();
        let first_user = first_creds.unwrap().mqtt_username;

        let (_, second_creds) = claim_device(&pool, uid, "dev-reclaim", Some("Updated"))
            .await
            .unwrap();
        assert!(
            second_creds.is_none(),
            "second claim on same device must not regenerate credentials"
        );

        // Verify username still exists in DB
        let devices = list_devices_for_user(&pool, uid).await.unwrap();
        assert_eq!(devices.len(), 1);
        drop(first_user);
    }

    #[sqlx::test]
    async fn unclaim_device_removes_ownership(pool: sqlx::PgPool) {
        let uid = create_test_user(&pool, 20).await;
        let _ = claim_device(&pool, uid, "dev-z", None).await.unwrap();
        let rows = unclaim_device(&pool, uid, "dev-z").await.unwrap();
        assert_eq!(rows, 1);
        assert!(!is_owner(&pool, uid, "dev-z").await.unwrap());
    }

    #[sqlx::test]
    async fn unclaim_device_returns_zero_for_not_owned(pool: sqlx::PgPool) {
        let rows = unclaim_device(&pool, 99, "nonexistent").await.unwrap();
        assert_eq!(rows, 0);
    }

    #[sqlx::test]
    async fn is_owner_returns_true_for_claimed(pool: sqlx::PgPool) {
        let uid = create_test_user(&pool, 30).await;
        let _ = claim_device(&pool, uid, "dev-owned", None).await.unwrap();
        assert!(is_owner(&pool, uid, "dev-owned").await.unwrap());
    }

    #[sqlx::test]
    async fn is_owner_returns_false_for_different_user(pool: sqlx::PgPool) {
        let uid40 = create_test_user(&pool, 40).await;
        let uid41 = create_test_user(&pool, 41).await;
        let _ = claim_device(&pool, uid40, "dev-shared", None)
            .await
            .unwrap();
        assert!(!is_owner(&pool, uid41, "dev-shared").await.unwrap());
    }

    #[sqlx::test]
    async fn list_devices_for_user_returns_empty_for_new_user(pool: sqlx::PgPool) {
        let devices = list_devices_for_user(&pool, 999).await.unwrap();
        assert!(devices.is_empty());
    }
}
