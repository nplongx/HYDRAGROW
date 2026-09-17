use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Executor, FromRow, Postgres};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConfigurationSync {
    pub device_id: String,
    pub config_version: i64,
    pub desired_controller_config: Value,
    pub desired_sensor_config: Value,
    pub controller_state: String,
    pub sensor_state: String,
    pub controller_attempts: i32,
    pub sensor_attempts: i32,
    pub controller_last_attempt_at: Option<DateTime<Utc>>,
    pub sensor_last_attempt_at: Option<DateTime<Utc>>,
    pub controller_applied_at: Option<DateTime<Utc>>,
    pub sensor_applied_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

pub const PENDING: &str = "pending";
pub const PUBLISHED: &str = "published";
pub const APPLIED: &str = "applied";
pub const FAILED: &str = "failed";

pub async fn next_version(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    device_id: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, ()>("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(device_id)
        .fetch_one(&mut **tx)
        .await?;

    sqlx::query_scalar(
        "SELECT COALESCE(MAX(config_version), 0) + 1 FROM configuration_sync WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_one(&mut **tx)
    .await
}

pub async fn upsert_desired<'e, E>(
    executor: E,
    device_id: &str,
    config_version: i64,
    controller: &Value,
    sensor: &Value,
) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        r#"
        INSERT INTO configuration_sync
            (device_id, config_version, desired_controller_config, desired_sensor_config,
             controller_state, sensor_state, controller_attempts, sensor_attempts,
             controller_last_attempt_at, sensor_last_attempt_at,
             controller_applied_at, sensor_applied_at, last_error, updated_at)
        VALUES ($1, $2, $3, $4, 'pending', 'pending', 0, 0, NULL, NULL, NULL, NULL, NULL, CURRENT_TIMESTAMP)
        ON CONFLICT (device_id) DO UPDATE SET
            config_version = EXCLUDED.config_version,
            desired_controller_config = EXCLUDED.desired_controller_config,
            desired_sensor_config = EXCLUDED.desired_sensor_config,
            controller_state = 'pending',
            sensor_state = 'pending',
            controller_attempts = 0,
            sensor_attempts = 0,
            controller_last_attempt_at = NULL,
            sensor_last_attempt_at = NULL,
            controller_applied_at = NULL,
            sensor_applied_at = NULL,
            last_error = NULL,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(device_id)
    .bind(config_version)
    .bind(controller)
    .bind(sensor)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn get(pool: &sqlx::PgPool, device_id: &str) -> Result<Option<ConfigurationSync>> {
    sqlx::query_as::<_, ConfigurationSync>(
        r#"
        SELECT device_id, config_version, desired_controller_config, desired_sensor_config,
               controller_state, sensor_state, controller_attempts, sensor_attempts,
               controller_last_attempt_at, sensor_last_attempt_at,
               controller_applied_at, sensor_applied_at, last_error, updated_at
        FROM configuration_sync
        WHERE device_id = $1
        "#,
    )
    .bind(device_id)
    .fetch_optional(pool)
    .await
    .context("failed to read configuration synchronization state")
}

pub async fn list_pending(pool: &sqlx::PgPool, limit: i64) -> Result<Vec<ConfigurationSync>> {
    sqlx::query_as::<_, ConfigurationSync>(
        r#"
        SELECT device_id, config_version, desired_controller_config, desired_sensor_config,
               controller_state, sensor_state, controller_attempts, sensor_attempts,
               controller_last_attempt_at, sensor_last_attempt_at,
               controller_applied_at, sensor_applied_at, last_error, updated_at
        FROM configuration_sync
        WHERE controller_state IN ('pending', 'published') OR sensor_state IN ('pending', 'published')
        ORDER BY updated_at ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("failed to list pending configuration synchronization")
}

pub async fn mark_published(
    pool: &sqlx::PgPool,
    device_id: &str,
    version: i64,
    target: &str,
) -> Result<()> {
    let column = match target {
        "controller" => "controller_state",
        "sensor" => "sensor_state",
        _ => return Err(anyhow::anyhow!("invalid configuration sync target")),
    };
    let attempts_column = match target {
        "controller" => "controller_attempts",
        _ => "sensor_attempts",
    };
    let attempt_column = match target {
        "controller" => "controller_last_attempt_at",
        _ => "sensor_last_attempt_at",
    };
    let query = format!(
        "UPDATE configuration_sync SET {column} = 'published', {attempts_column} = {attempts_column} + 1, {attempt_column} = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP, last_error = NULL WHERE device_id = $1 AND config_version = $2 AND {column} = 'pending'"
    );
    sqlx::query(&query)
        .bind(device_id)
        .bind(version)
        .execute(pool)
        .await
        .context("failed to mark configuration sync published")?;
    Ok(())
}

pub async fn mark_publish_failed(
    pool: &sqlx::PgPool,
    device_id: &str,
    version: i64,
    target: &str,
    error: &str,
) -> Result<()> {
    let (state, attempts) = match target {
        "controller" => ("controller_state", "controller_attempts"),
        "sensor" => ("sensor_state", "sensor_attempts"),
        _ => return Err(anyhow::anyhow!("invalid configuration sync target")),
    };
    let attempt_column = match target {
        "controller" => "controller_last_attempt_at",
        _ => "sensor_last_attempt_at",
    };
    let query = format!(
        "UPDATE configuration_sync SET {state} = CASE WHEN {attempts} + 1 >= 5 THEN 'failed' ELSE 'pending' END, {attempts} = {attempts} + 1, {attempt_column} = CURRENT_TIMESTAMP, last_error = $3, updated_at = CURRENT_TIMESTAMP WHERE device_id = $1 AND config_version = $2 AND {state} = 'pending'"
    );
    sqlx::query(&query)
        .bind(device_id)
        .bind(version)
        .bind(error)
        .execute(pool)
        .await
        .context("failed to record configuration sync publish failure")?;
    Ok(())
}

pub async fn mark_applied(
    pool: &sqlx::PgPool,
    device_id: &str,
    version: i64,
    target: &str,
) -> Result<bool> {
    let column = match target {
        "controller" => "controller_state",
        "sensor" => "sensor_state",
        _ => return Err(anyhow::anyhow!("invalid configuration sync target")),
    };
    let applied_column = match target {
        "controller" => "controller_applied_at",
        _ => "sensor_applied_at",
    };
    let query = format!(
        "UPDATE configuration_sync SET {column} = 'applied', {applied_column} = COALESCE({applied_column}, CURRENT_TIMESTAMP), updated_at = CURRENT_TIMESTAMP WHERE device_id = $1 AND config_version = $2 AND {column} IN ('pending', 'published')"
    );
    let result = sqlx::query(&query)
        .bind(device_id)
        .bind(version)
        .execute(pool)
        .await
        .context("failed to mark configuration sync applied")?;
    Ok(result.rows_affected() == 1)
}
