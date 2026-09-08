#[cfg(test)]
mod tests {
    use crate::db::device_wifi::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn replace_and_read_wifi_metadata_roundtrip(pool: sqlx::PgPool) {
        let entries = vec![
            WifiSsidEntry {
                ssid: "Farm-A".into(),
                priority: 0,
            },
            WifiSsidEntry {
                ssid: "Farm-Backup".into(),
                priority: 1,
            },
        ];
        replace_wifi_metadata(&pool, "device-001", &entries, 7)
            .await
            .unwrap();

        let (stored, version) = get_wifi_metadata(&pool, "device-001").await.unwrap();
        assert_eq!(version, 7);
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].ssid, "Farm-A");

        // Metadata rows carry no password column at all.
        let row: serde_json::Value = sqlx::query_scalar(
            "SELECT to_jsonb(t) FROM device_wifi_config t WHERE device_id = 'device-001' LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(row.get("password").is_none());
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn metadata_replace_is_scoped_per_device(pool: sqlx::PgPool) {
        replace_wifi_metadata(
            &pool,
            "device-A",
            &[WifiSsidEntry {
                ssid: "Farm-A".into(),
                priority: 0,
            }],
            3,
        )
        .await
        .unwrap();
        replace_wifi_metadata(
            &pool,
            "device-B",
            &[WifiSsidEntry {
                ssid: "Other".into(),
                priority: 0,
            }],
            5,
        )
        .await
        .unwrap();

        let (a, va) = get_wifi_metadata(&pool, "device-A").await.unwrap();
        let (b, vb) = get_wifi_metadata(&pool, "device-B").await.unwrap();
        assert_eq!((a[0].ssid.as_str(), va), ("Farm-A", 3));
        assert_eq!((b[0].ssid.as_str(), vb), ("Other", 5));

        // Replacing A must not touch B.
        replace_wifi_metadata(&pool, "device-A", &[], 4)
            .await
            .unwrap();
        let (a2, va2) = get_wifi_metadata(&pool, "device-A").await.unwrap();
        assert!(a2.is_empty());
        assert_eq!(va2, 0);
        let (b2, _) = get_wifi_metadata(&pool, "device-B").await.unwrap();
        assert_eq!(b2.len(), 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn delivery_state_tracks_pending_then_applied(pool: sqlx::PgPool) {
        replace_wifi_metadata(
            &pool,
            "device-001",
            &[WifiSsidEntry {
                ssid: "Farm-A".into(),
                priority: 0,
            }],
            8,
        )
        .await
        .unwrap();
        set_delivery_state(&pool, "device-001", 8, delivery_state::PENDING, None)
            .await
            .unwrap();

        let view = get_wifi_config_view(&pool, "device-001").await.unwrap();
        assert_eq!(view.config_version, 8);
        assert_eq!(view.state, delivery_state::PENDING);
        assert_eq!(view.ssids.len(), 1);

        set_delivery_state(&pool, "device-001", 8, delivery_state::APPLIED, Some("ok"))
            .await
            .unwrap();
        let view = get_wifi_config_view(&pool, "device-001").await.unwrap();
        assert_eq!(view.state, delivery_state::APPLIED);

        // Stale delivery rows report pending until the matching version applies.
        set_delivery_state(&pool, "device-001", 7, delivery_state::APPLIED, None)
            .await
            .unwrap();
        let view = get_wifi_config_view(&pool, "device-001").await.unwrap();
        assert_eq!(view.state, delivery_state::PENDING);
    }

    #[test]
    fn metadata_from_provision_never_contains_passwords() {
        use hydragrow_shared::{WifiProvisionConfig, WifiProvisionEntry, WifiSecretAction};
        let config = WifiProvisionConfig {
            config_version: 8,
            entries: vec![WifiProvisionEntry {
                ssid: "Farm-A".into(),
                priority: 0,
                secret_action: WifiSecretAction::Set,
                password: Some("top-secret".into()),
            }],
        };
        let metadata = metadata_from_provision(&config);
        assert_eq!(metadata.len(), 1);
        let json = serde_json::to_string(&metadata).unwrap();
        assert!(!json.contains("top-secret"));
        assert!(!json.contains("password"));
    }
}
