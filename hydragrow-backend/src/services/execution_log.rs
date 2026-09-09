use sqlx::PgPool;
use uuid::Uuid;

pub async fn log_success(pool: &PgPool, script_id: Uuid, device_id: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO flow_execution_log (script_id, device_id, status) VALUES ($1, $2, 'success')",
    )
    .bind(script_id)
    .bind(device_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn log_error(
    pool: &PgPool,
    script_id: Uuid,
    device_id: &str,
    message: &str,
) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO flow_execution_log (script_id, device_id, status, error_message) VALUES ($1, $2, 'error', $3)")
        .bind(script_id)
        .bind(device_id)
        .bind(message)
        .execute(pool)
        .await?;
    Ok(())
}

/// % thành công trong 200 lần thực thi gần nhất của `device_id`. `None` nếu
/// chưa có lần thực thi nào (tránh chia 0, và tránh hiển thị 100% giả).
pub async fn success_rate_percent(pool: &PgPool, device_id: &str) -> anyhow::Result<Option<f64>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT status FROM flow_execution_log WHERE device_id = $1 ORDER BY created_at DESC LIMIT 200",
    )
    .bind(device_id)
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        return Ok(None);
    }
    let success_count = rows.iter().filter(|(s,)| s == "success").count();
    Ok(Some((success_count as f64 / rows.len() as f64) * 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn success_rate_percent_is_none_with_no_history(pool: sqlx::PgPool) {
        assert_eq!(
            success_rate_percent(&pool, "dev-empty").await.unwrap(),
            None
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn success_rate_percent_computes_the_real_ratio(pool: sqlx::PgPool) {
        let script_id = Uuid::new_v4();
        crate::db::postgres::upsert_device_config(
            &pool,
            &crate::models::config::DeviceConfig {
                device_id: "dev-rate".to_string(),
                ec_target: 1.8,
                ec_tolerance: 0.2,
                ph_target: 6.0,
                ph_tolerance: 0.3,
                control_mode: "auto".to_string(),
                is_enabled: true,
                delay_between_a_and_b_sec: 5,
                last_updated: chrono::Utc::now(),
            },
        )
        .await
        .unwrap();
        log_success(&pool, script_id, "dev-rate").await.unwrap();
        log_success(&pool, script_id, "dev-rate").await.unwrap();
        log_success(&pool, script_id, "dev-rate").await.unwrap();
        log_error(&pool, script_id, "dev-rate", "reconcile failed")
            .await
            .unwrap();
        let rate = success_rate_percent(&pool, "dev-rate")
            .await
            .unwrap()
            .unwrap();
        assert!((rate - 75.0).abs() < 0.01, "expected 75%, got {rate}");
    }
}
