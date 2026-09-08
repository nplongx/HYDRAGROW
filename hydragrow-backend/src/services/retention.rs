//! Retention service: xóa system_events cũ hơn 90 ngày, chạy mỗi 24 giờ.
//!
//! Xóa theo batch (thay vì 1 câu DELETE không giới hạn) để tránh giữ khóa/WAL
//! tăng đột biến một lần mỗi ngày khi bảng system_events đã lớn.

use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

const RETENTION_DAYS: i64 = 90;
const RUN_INTERVAL_HOURS: u64 = 24;
const DELETE_BATCH_SIZE: i64 = 5000;
const BATCH_DELAY: Duration = Duration::from_millis(50);

/// Spawn tokio task chạy vô hạn, mỗi 24h xóa log cũ hơn 90 ngày.
///
/// Call một lần trong `main()` sau khi pool sẵn sàng.
pub fn spawn(pool: PgPool) {
    tokio::spawn(async move {
        // Delay khởi động 60s để tránh chạy ngay khi server vừa boot
        tokio::time::sleep(Duration::from_secs(60)).await;

        loop {
            run_once(&pool).await;
            tokio::time::sleep(Duration::from_secs(RUN_INTERVAL_HOURS * 3600)).await;
        }
    });
}

fn compute_cutoff_ms() -> i64 {
    let now = chrono::Utc::now();
    (now - chrono::Duration::days(RETENTION_DAYS)).timestamp_millis()
}

/// Xóa tối đa `batch_size` dòng có `timestamp < cutoff_ms`, trả về số dòng đã xóa.
async fn delete_one_batch(
    pool: &PgPool,
    cutoff_ms: i64,
    batch_size: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        WITH batch AS (
            SELECT id FROM system_events
            WHERE timestamp < $1
            ORDER BY id
            LIMIT $2
        )
        DELETE FROM system_events
        WHERE id IN (SELECT id FROM batch)
        "#,
    )
    .bind(cutoff_ms)
    .bind(batch_size)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Lặp `delete_one_batch` cho tới khi không còn dòng nào cũ hơn cutoff, ngủ
/// `delay_between_batches` giữa 2 batch để tránh giữ khóa liên tục. Nếu một
/// batch lỗi, dừng lại — lần chạy 24h kế tiếp sẽ tiếp tục dọn phần còn lại.
async fn delete_expired_in_batches(
    pool: &PgPool,
    cutoff_ms: i64,
    batch_size: i64,
    delay_between_batches: Duration,
) -> u64 {
    let mut total_deleted: u64 = 0;

    loop {
        let deleted = match delete_one_batch(pool, cutoff_ms, batch_size).await {
            Ok(deleted) => deleted,
            Err(e) => {
                error!("❌ [Retention] Lỗi xóa system_events cũ: {:?}", e);
                break;
            }
        };

        total_deleted += deleted;

        if (deleted as i64) < batch_size {
            break;
        }

        tokio::time::sleep(delay_between_batches).await;
    }

    total_deleted
}

async fn run_once(pool: &PgPool) {
    let cutoff_ms = compute_cutoff_ms();
    let total_deleted =
        delete_expired_in_batches(pool, cutoff_ms, DELETE_BATCH_SIZE, BATCH_DELAY).await;

    if total_deleted > 0 {
        info!(
            "🗑️ [Retention] Đã xóa {} system_events cũ hơn {} ngày.",
            total_deleted, RETENTION_DAYS
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::postgres::{NewSystemEventRecord, insert_system_event};

    #[test]
    fn cutoff_ms_is_90_days_ago() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let cutoff_ms = compute_cutoff_ms();
        let diff_days = (now_ms - cutoff_ms) / (1000 * 60 * 60 * 24);
        assert!(
            (89..=91).contains(&diff_days),
            "cutoff phải khoảng 90 ngày trước, thực tế: {} ngày",
            diff_days
        );
    }

    fn old_event(days_ago: i64, tag: &str) -> NewSystemEventRecord {
        let ts = (chrono::Utc::now() - chrono::Duration::days(days_ago)).timestamp_millis();
        NewSystemEventRecord {
            device_id: "dev-retention-test".to_string(),
            level: "info".to_string(),
            category: "system".to_string(),
            title: format!("test-{tag}"),
            message: "retention test event".to_string(),
            reason: None,
            metadata: None,
            timestamp: ts,
            source: "rule".to_string(),
            primary_reason_code: None,
        }
    }

    #[sqlx::test]
    async fn delete_one_batch_deletes_at_most_batch_size_rows(pool: sqlx::PgPool) {
        for i in 0..5 {
            insert_system_event(&pool, &old_event(100, &format!("old-{i}")))
                .await
                .unwrap();
        }

        let cutoff_ms = compute_cutoff_ms();
        let deleted = delete_one_batch(&pool, cutoff_ms, 2).await.unwrap();

        assert_eq!(deleted, 2);

        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM system_events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, 3);
    }

    #[sqlx::test]
    async fn delete_expired_in_batches_deletes_all_matching_rows_across_multiple_batches(
        pool: sqlx::PgPool,
    ) {
        for i in 0..5 {
            insert_system_event(&pool, &old_event(100, &format!("old-{i}")))
                .await
                .unwrap();
        }
        for i in 0..2 {
            insert_system_event(&pool, &old_event(1, &format!("recent-{i}")))
                .await
                .unwrap();
        }

        let cutoff_ms = compute_cutoff_ms();
        let total_deleted =
            delete_expired_in_batches(&pool, cutoff_ms, 2, Duration::from_millis(0)).await;

        assert_eq!(total_deleted, 5);

        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM system_events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, 2);
    }
}
