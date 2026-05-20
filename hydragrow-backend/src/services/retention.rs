//! Retention service: xóa system_events cũ hơn 90 ngày, chạy mỗi 24 giờ.

use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

const RETENTION_DAYS: i64 = 90;
const RUN_INTERVAL_HOURS: u64 = 24;

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

async fn run_once(pool: &PgPool) {
    let cutoff_ms = {
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::days(RETENTION_DAYS);
        cutoff.timestamp_millis()
    };

    match sqlx::query(
        r#"
        DELETE FROM system_events
        WHERE timestamp < $1
        "#,
    )
    .bind(cutoff_ms)
    .execute(pool)
    .await
    {
        Ok(result) => {
            let deleted = result.rows_affected();
            if deleted > 0 {
                info!(
                    "🗑️ [Retention] Đã xóa {} system_events cũ hơn {} ngày.",
                    deleted, RETENTION_DAYS
                );
            }
        }
        Err(e) => {
            error!("❌ [Retention] Lỗi xóa system_events cũ: {:?}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutoff_ms_is_90_days_ago() {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let cutoff_ms = {
            let now = chrono::Utc::now();
            let cutoff = now - chrono::Duration::days(RETENTION_DAYS);
            cutoff.timestamp_millis()
        };
        let diff_days = (now_ms - cutoff_ms) / (1000 * 60 * 60 * 24);
        assert!(
            diff_days >= 89 && diff_days <= 91,
            "cutoff phải khoảng 90 ngày trước, thực tế: {} ngày",
            diff_days
        );
    }
}
