#[cfg(test)]
mod tests {
    use crate::db::device_wifi::*;

    #[sqlx::test]
    async fn replace_device_wifi_config_inserts_entries_with_version_one(pool: sqlx::PgPool) {
        let entries = vec![
            WifiSsidEntry {
                ssid: "HomeNet".into(),
                priority: 0,
            },
            WifiSsidEntry {
                ssid: "Backup4G".into(),
                priority: 1,
            },
        ];
        let rows = replace_device_wifi_config(&pool, "esp-001", &entries)
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.config_version == 1));
    }

    #[sqlx::test]
    async fn replace_device_wifi_config_bumps_version_on_second_call(pool: sqlx::PgPool) {
        let first = vec![WifiSsidEntry {
            ssid: "HomeNet".into(),
            priority: 0,
        }];
        replace_device_wifi_config(&pool, "esp-002", &first)
            .await
            .unwrap();

        let second = vec![WifiSsidEntry {
            ssid: "NewNet".into(),
            priority: 0,
        }];
        let rows = replace_device_wifi_config(&pool, "esp-002", &second)
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].ssid, "NewNet");
        assert_eq!(rows[0].config_version, 2);
    }

    #[sqlx::test]
    async fn replace_device_wifi_config_does_not_affect_other_devices(pool: sqlx::PgPool) {
        replace_device_wifi_config(
            &pool,
            "esp-a",
            &[WifiSsidEntry {
                ssid: "A".into(),
                priority: 0,
            }],
        )
        .await
        .unwrap();
        replace_device_wifi_config(
            &pool,
            "esp-b",
            &[WifiSsidEntry {
                ssid: "B".into(),
                priority: 0,
            }],
        )
        .await
        .unwrap();

        let a = get_device_wifi_config(&pool, "esp-a").await.unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].ssid, "A");
    }

    #[sqlx::test]
    async fn get_device_wifi_config_returns_empty_for_unknown_device(pool: sqlx::PgPool) {
        let rows = get_device_wifi_config(&pool, "never-seen").await.unwrap();
        assert!(rows.is_empty());
    }

    #[sqlx::test]
    async fn get_device_wifi_config_orders_by_priority(pool: sqlx::PgPool) {
        let entries = vec![
            WifiSsidEntry {
                ssid: "Second".into(),
                priority: 1,
            },
            WifiSsidEntry {
                ssid: "First".into(),
                priority: 0,
            },
        ];
        replace_device_wifi_config(&pool, "esp-order", &entries)
            .await
            .unwrap();
        let rows = get_device_wifi_config(&pool, "esp-order").await.unwrap();
        assert_eq!(rows[0].ssid, "First");
        assert_eq!(rows[1].ssid, "Second");
    }
}
