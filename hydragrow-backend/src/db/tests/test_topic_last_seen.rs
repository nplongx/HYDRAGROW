#[cfg(test)]
mod tests {
    use crate::db::topic_last_seen::*;

    #[sqlx::test]
    async fn upsert_and_fetch_single_device(pool: sqlx::PgPool) {
        let now = chrono::Utc::now();
        touch_topic(&pool, "device-001", "controller/status", now)
            .await
            .unwrap();

        let topics = get_topics_for_device(&pool, "device-001").await.unwrap();
        assert_eq!(topics.len(), 1);
        assert_eq!(topics[0].device_id, "device-001");
        assert_eq!(topics[0].topic_category, "controller/status");
    }

    #[sqlx::test]
    async fn touch_updates_rather_than_duplicating(pool: sqlx::PgPool) {
        let first = chrono::Utc::now();
        touch_topic(&pool, "device-001", "controller/status", first)
            .await
            .unwrap();
        let second = first + chrono::Duration::seconds(60);
        touch_topic(&pool, "device-001", "controller/status", second)
            .await
            .unwrap();

        let topics = get_topics_for_device(&pool, "device-001").await.unwrap();
        assert_eq!(topics.len(), 1);
        // TIMESTAMPTZ stores microsecond precision; allow sub-millisecond truncation.
        assert!(
            (topics[0].last_seen_at - second)
                .num_microseconds()
                .unwrap()
                .abs()
                < 1000
        );
    }

    #[sqlx::test]
    async fn get_all_devices_returns_every_device(pool: sqlx::PgPool) {
        let now = chrono::Utc::now();
        touch_topic(&pool, "device-001", "controller/status", now)
            .await
            .unwrap();
        touch_topic(&pool, "device-002", "controller/status", now)
            .await
            .unwrap();

        let all = get_all_topics(&pool).await.unwrap();
        let ids: Vec<&str> = all.iter().map(|t| t.device_id.as_str()).collect();
        assert!(ids.contains(&"device-001"));
        assert!(ids.contains(&"device-002"));
    }
}
