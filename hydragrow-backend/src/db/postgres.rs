use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::JsonRawValue;
use sqlx::{Error, Executor, FromRow, PgPool, Row, Value};
use tracing::instrument;

use crate::models::alert::AlertMessage;
use crate::models::config::{DeviceConfig, SafetyConfig};
use crate::models::crop_season::{CreateCropSeasonRequest, CropSeason};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct BlockchainRecord {
    pub id: i32,
    pub device_id: String,
    pub season_id: Option<String>,
    pub action: String,
    pub tx_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DosingReportRecord {
    pub id: i32,
    pub device_id: String,
    pub season_id: Option<String>,
    pub pump_a_ml: f32,
    pub pump_b_ml: f32,
    pub ph_up_ml: f32,
    pub ph_down_ml: f32,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// System Events

/// Struct dùng để GHI vào DB (không có id – SERIAL tự sinh).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSystemEventRecord {
    pub device_id: String,
    pub level: String,    // "info", "success", "warning", "critical"
    pub category: String, // "system", "dosing", "water", "calibration", "alert", ...
    pub title: String,
    pub message: String,
    pub reason: Option<String>,
    pub metadata: Option<serde_json::Value>, // Sử dụng JsonValue của sqlx
    pub timestamp: i64,
}

/// Struct dùng để ĐỌC từ DB (id là i32 do SERIAL).
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct SystemEventRecord {
    pub id: i32,
    pub device_id: String,
    pub level: String,
    pub category: String,
    pub title: String,
    pub message: String,
    pub reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub timestamp: i64,
}

// Device Config

#[instrument(skip(pool))]
pub async fn get_device_config(pool: &PgPool, device_id: &str) -> Result<DeviceConfig> {
    let config = sqlx::query_as::<_, DeviceConfig>(
        r#"SELECT
            device_id, ec_target, ec_tolerance, ph_target, ph_tolerance,
            control_mode, is_enabled,
            delay_between_a_and_b_sec, last_updated
        FROM device_config WHERE device_id = $1"#,
    )
    .bind(device_id)
    .fetch_one(pool)
    .await
    .context(format!("Failed to fetch device_config for {}", device_id))?;

    Ok(config)
}

#[instrument(skip(executor, config))]
pub async fn upsert_device_config(
    executor: impl Executor<'_, Database = sqlx::Postgres>,
    config: &DeviceConfig,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO device_config (
            device_id, ec_target, ec_tolerance, ph_target, ph_tolerance,
            control_mode, is_enabled,
            delay_between_a_and_b_sec, last_updated
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT(device_id) DO UPDATE SET
            ec_target = EXCLUDED.ec_target,
            ec_tolerance = EXCLUDED.ec_tolerance,
            ph_target = EXCLUDED.ph_target,
            ph_tolerance = EXCLUDED.ph_tolerance,
            control_mode = EXCLUDED.control_mode,
            is_enabled = EXCLUDED.is_enabled,
            delay_between_a_and_b_sec = EXCLUDED.delay_between_a_and_b_sec,
            last_updated = EXCLUDED.last_updated
        "#,
    )
    .bind(&config.device_id)
    .bind(config.ec_target)
    .bind(config.ec_tolerance)
    .bind(config.ph_target)
    .bind(config.ph_tolerance)
    .bind(&config.control_mode)
    .bind(config.is_enabled)
    .bind(config.delay_between_a_and_b_sec)
    .bind(&config.last_updated)
    .execute(executor)
    .await
    .context("Failed to upsert device_config")?;

    Ok(())
}

// Safety Config

#[instrument(skip(pool))]
pub async fn get_safety_config(pool: &PgPool, device_id: &str) -> Result<SafetyConfig> {
    let config = sqlx::query_as::<_, SafetyConfig>(
        r#"
        SELECT
            device_id, max_ec_limit, min_ec_limit, min_ph_limit, max_ph_limit, max_ec_delta, max_ph_delta,
            max_dose_per_cycle, cooldown_sec, max_dose_per_hour, water_level_critical_min,
            max_refill_cycles_per_hour, max_drain_cycles_per_hour, max_refill_duration_sec,
            max_drain_duration_sec, min_temp_limit, max_temp_limit, emergency_shutdown,
            ec_ack_threshold, ph_ack_threshold, water_ack_threshold, last_updated
        FROM safety_config
        WHERE device_id = $1
        "#,
    )
    .bind(device_id)
    .fetch_one(pool)
    .await
    .context(format!("Failed to fetch safety_config for {}", device_id))?;

    Ok(config)
}

#[instrument(skip(executor, config))]
pub async fn upsert_safety_config(
    executor: impl Executor<'_, Database = sqlx::Postgres>,
    config: &SafetyConfig,
) -> Result<()> {
    sqlx::query(
        r#"
    INSERT INTO safety_config (
        device_id, max_ec_limit, min_ec_limit, min_ph_limit, max_ph_limit, max_ec_delta, max_ph_delta,
        max_dose_per_cycle, cooldown_sec, max_dose_per_hour, water_level_critical_min,
        max_refill_cycles_per_hour, max_drain_cycles_per_hour, max_refill_duration_sec,
        max_drain_duration_sec, min_temp_limit, max_temp_limit, emergency_shutdown,
        ec_ack_threshold, ph_ack_threshold, water_ack_threshold, last_updated
    )
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
    ON CONFLICT(device_id) DO UPDATE SET
        max_ec_limit = EXCLUDED.max_ec_limit,
        min_ec_limit = EXCLUDED.min_ec_limit,
        min_ph_limit = EXCLUDED.min_ph_limit,
        max_ph_limit = EXCLUDED.max_ph_limit,
        max_ec_delta = EXCLUDED.max_ec_delta,
        max_ph_delta = EXCLUDED.max_ph_delta,
        max_dose_per_cycle = EXCLUDED.max_dose_per_cycle,
        cooldown_sec = EXCLUDED.cooldown_sec,
        max_dose_per_hour = EXCLUDED.max_dose_per_hour,
        water_level_critical_min = EXCLUDED.water_level_critical_min,
        max_refill_cycles_per_hour = EXCLUDED.max_refill_cycles_per_hour,
        max_drain_cycles_per_hour = EXCLUDED.max_drain_cycles_per_hour,
        max_refill_duration_sec = EXCLUDED.max_refill_duration_sec,
        max_drain_duration_sec = EXCLUDED.max_drain_duration_sec,
        min_temp_limit = EXCLUDED.min_temp_limit,
        max_temp_limit = EXCLUDED.max_temp_limit,
        emergency_shutdown = EXCLUDED.emergency_shutdown,
        ec_ack_threshold = EXCLUDED.ec_ack_threshold,
        ph_ack_threshold = EXCLUDED.ph_ack_threshold,
        water_ack_threshold = EXCLUDED.water_ack_threshold,
        last_updated = EXCLUDED.last_updated
    "#,
    )
    .bind(&config.device_id)
    .bind(config.max_ec_limit)
    .bind(config.min_ec_limit)
    .bind(config.min_ph_limit)
    .bind(config.max_ph_limit)
    .bind(config.max_ec_delta)
    .bind(config.max_ph_delta)
    .bind(config.max_dose_per_cycle)
    .bind(config.cooldown_sec)
    .bind(config.max_dose_per_hour)
    .bind(config.water_level_critical_min)
    .bind(config.max_refill_cycles_per_hour)
    .bind(config.max_drain_cycles_per_hour)
    .bind(config.max_refill_duration_sec)
    .bind(config.max_drain_duration_sec)
    .bind(config.min_temp_limit)
    .bind(config.max_temp_limit)
    .bind(config.emergency_shutdown)
    .bind(config.ec_ack_threshold)
    .bind(config.ph_ack_threshold)
    .bind(config.water_ack_threshold)
    .bind(&config.last_updated)
    .execute(executor)
    .await?;

    Ok(())
}

// Blockchain History

pub async fn insert_blockchain_tx(
    pool: &PgPool,
    device_id: &str,
    season_id: Option<&str>, // 🟢 Sửa thành Option để hỗ trợ NULL
    action: &str,
    tx_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO blockchain_logs (device_id, season_id, action, tx_id)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(device_id)
    .bind(season_id) // 🟢 Bind giá trị an toàn
    .bind(action)
    .bind(tx_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_dosing_report(
    pool: &PgPool,
    device_id: &str,
    season_id: Option<&str>,
    pump_a_ml: f32,
    pump_b_ml: f32,
    ph_up_ml: f32,
    ph_down_ml: f32,
    payload: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO dosing_reports (
            device_id, season_id, pump_a_ml, pump_b_ml, ph_up_ml, ph_down_ml, payload
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(device_id)
    .bind(season_id)
    .bind(pump_a_ml)
    .bind(pump_b_ml)
    .bind(ph_up_ml)
    .bind(ph_down_ml)
    .bind(payload)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_device_dosing_reports(
    pool: &PgPool,
    device_id: &str,
    season_id: Option<String>,
) -> Result<Vec<DosingReportRecord>, sqlx::Error> {
    match season_id {
        Some(s_id) => {
            sqlx::query_as::<_, DosingReportRecord>(
                r#"SELECT * FROM dosing_reports WHERE device_id = $1 AND season_id = $2 ORDER BY created_at DESC"#,
            )
            .bind(device_id)
            .bind(s_id)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, DosingReportRecord>(
                r#"SELECT * FROM dosing_reports WHERE device_id = $1 ORDER BY created_at DESC LIMIT 100"#,
            )
            .bind(device_id)
            .fetch_all(pool)
            .await
        }
    }
}
pub async fn get_device_blockchain_history(
    pool: &PgPool,
    device_id: &str,
    season_id: Option<String>,
) -> Result<Vec<BlockchainRecord>, sqlx::Error> {
    match season_id {
        Some(s_id) => {
            sqlx::query_as::<_, BlockchainRecord>(
                r#"SELECT * FROM blockchain_logs WHERE device_id = $1 AND season_id = $2 ORDER BY created_at DESC"#
            )
            .bind(device_id)
            .bind(s_id)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, BlockchainRecord>(
                r#"SELECT * FROM blockchain_logs WHERE device_id = $1 ORDER BY created_at DESC LIMIT 100"#
            )
            .bind(device_id)
            .fetch_all(pool)
            .await
        }
    }
}

// Crop Season

pub async fn get_active_crop_season(
    pool: &PgPool,
    device_id: &str,
) -> Result<Option<CropSeason>, sqlx::Error> {
    let season = sqlx::query_as::<_, CropSeason>(
        "SELECT id, device_id, name, plant_type, start_time::text as start_time, end_time::text as end_time, status, description
         FROM crop_seasons WHERE device_id = $1 AND status = 'active' LIMIT 1",
    )
    .bind(device_id)
    .fetch_optional(pool)
    .await?;
    Ok(season)
}

pub async fn get_crop_seasons_history(
    pool: &PgPool,
    device_id: &str,
) -> Result<Vec<CropSeason>, sqlx::Error> {
    let seasons = sqlx::query_as::<_, CropSeason>(
        "SELECT id, device_id, name, plant_type, start_time::text as start_time, end_time::text as end_time, status, description
         FROM crop_seasons WHERE device_id = $1 ORDER BY start_time DESC",
    )
    .bind(device_id)
    .fetch_all(pool)
    .await?;
    Ok(seasons)
}

pub async fn create_crop_season(
    pool: &PgPool,
    device_id: &str,
    req: CreateCropSeasonRequest,
) -> Result<Option<CropSeason>, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();

    // 🟢 SỬA LỖI: Thêm trường description và $5 vào câu SQL
    sqlx::query(
        "INSERT INTO crop_seasons (id, device_id, name, plant_type, status, description) VALUES ($1, $2, $3, $4, 'active', $5)",
    )
    .bind(&id)
    .bind(device_id)
    .bind(&req.name)
    .bind(&req.plant_type)
    .bind(&req.description) // 🟢 BIND thêm description từ request
    .execute(pool)
    .await?;

    get_active_crop_season(pool, device_id).await
}

pub async fn end_active_crop_season(pool: &PgPool, device_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE crop_seasons SET status = 'completed', end_time = CURRENT_TIMESTAMP WHERE device_id = $1 AND status = 'active'",
    )
    .bind(device_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_active_crop_season(
    pool: &PgPool,
    device_id: &str,
    name: &str,
    plant_type: Option<&str>,
    description: Option<&str>,
) -> Result<CropSeason, sqlx::Error> {
    let active_season_id: Option<String> =
        sqlx::query("SELECT id FROM crop_seasons WHERE device_id = $1 AND status = 'active'")
            .bind(device_id)
            .fetch_optional(pool)
            .await?
            .map(|row| row.get("id"));

    match active_season_id {
        Some(id) => {
            let updated = sqlx::query(
                r#"
                UPDATE crop_seasons
                SET name = $1, plant_type = $2, description = $3
                WHERE id = $4
                RETURNING id, device_id, name, plant_type,
                          start_time::text as start_time,
                          end_time::text as end_time,
                          status, description
                "#,
            )
            .bind(name)
            .bind(plant_type)
            .bind(description)
            .bind(&id)
            .fetch_one(pool)
            .await?;

            Ok(CropSeason {
                id: updated.get("id"),
                device_id: updated.get("device_id"),
                name: updated.get("name"),
                plant_type: updated.get("plant_type"),
                start_time: updated.get("start_time"),
                end_time: updated.get("end_time"),
                status: updated.get("status"),
                description: updated.get("description"),
            })
        }
        None => Err(sqlx::Error::RowNotFound),
    }
}

// System Events

pub async fn insert_system_event(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    record: &NewSystemEventRecord,
) -> Result<(), sqlx::Error> {
    let query = r#"
        INSERT INTO system_events (
            device_id, level, category, title, message, reason, metadata, timestamp
        ) 
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    "#;

    sqlx::query(query)
        .bind(&record.device_id)
        .bind(&record.level)
        .bind(&record.category)
        .bind(&record.title)
        .bind(&record.message)
        .bind(&record.reason)
        .bind(&record.metadata)
        .bind(record.timestamp)
        .execute(executor)
        .await?;

    Ok(())
}

pub async fn get_system_events(
    pool: &PgPool,
    device_id: &str,
    categories: &[String],
    limit: i64,
) -> Result<Vec<SystemEventRecord>, sqlx::Error> {
    sqlx::query_as::<_, SystemEventRecord>(
        r#"
        SELECT id, device_id, level, category, title, message, reason, metadata, timestamp
        FROM system_events
        WHERE device_id = $1
          AND (cardinality($2::text[]) = 0 OR category = ANY($2::text[]))
        ORDER BY timestamp DESC
        LIMIT $3
        "#,
    )
    .bind(device_id)
    .bind(categories)
    .bind(limit)
    .fetch_all(pool)
    .await
}
/// Tìm toàn bộ log liên quan đến một quy trình (cycle_id) cụ thể
pub async fn get_events_by_cycle_id(
    pool: &sqlx::PgPool,
    device_id: &str,
    cycle_id: &str,
) -> Result<Vec<SystemEventRecord>, sqlx::Error> {
    let query = r#"
        SELECT * FROM system_events 
        WHERE device_id = $1 
          AND metadata ->> 'cycle_id' = $2
        ORDER BY timestamp ASC
    "#;

    let records = sqlx::query_as::<_, SystemEventRecord>(query)
        .bind(device_id)
        .bind(cycle_id)
        .fetch_all(pool)
        .await?;

    Ok(records)
}
