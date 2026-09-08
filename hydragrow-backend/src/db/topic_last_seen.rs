// hydragrow-backend/src/db/topic_last_seen.rs
//! Per-topic liveness tracking (LWT-independent), updated by MQTT handlers.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct TopicLastSeen {
    pub device_id: String,
    pub topic_category: String,
    pub last_seen_at: DateTime<Utc>,
}

/// Best-effort upsert of a topic's last-seen timestamp.
pub async fn touch_topic(
    pool: &PgPool,
    device_id: &str,
    topic_category: &str,
    last_seen_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO device_topic_last_seen (device_id, topic_category, last_seen_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (device_id, topic_category)
        DO UPDATE SET last_seen_at = EXCLUDED.last_seen_at
        "#,
    )
    .bind(device_id)
    .bind(topic_category)
    .bind(last_seen_at)
    .execute(pool)
    .await?;
    Ok(())
}

/// All tracked topics for a single device.
pub async fn get_topics_for_device(
    pool: &PgPool,
    device_id: &str,
) -> Result<Vec<TopicLastSeen>, sqlx::Error> {
    sqlx::query_as::<_, TopicLastSeen>(
        r#"
        SELECT device_id, topic_category, last_seen_at
        FROM device_topic_last_seen
        WHERE device_id = $1
        "#,
    )
    .bind(device_id)
    .fetch_all(pool)
    .await
}

/// Every tracked (device, topic) row.
pub async fn get_all_topics(pool: &PgPool) -> Result<Vec<TopicLastSeen>, sqlx::Error> {
    sqlx::query_as::<_, TopicLastSeen>(
        r#"
        SELECT device_id, topic_category, last_seen_at
        FROM device_topic_last_seen
        "#,
    )
    .fetch_all(pool)
    .await
}
