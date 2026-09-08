//! SSID-only WiFi metadata and delivery tracking.
//!
//! SECURITY INVARIANT: no password ever crosses this module's boundary.
//! Passwords live transiently in the HTTPS request body, are relayed to the
//! device over MQTT, and persist durably only in the target ESP32's NVS.
//! This module persists SSIDs, priorities, versions, and apply-state only.

use chrono::{DateTime, Utc};
use hydragrow_shared::WifiProvisionConfig;
use serde::Serialize;
use sqlx::{FromRow, PgPool};

use super::{DbError, DbResult};

/// A single SSID metadata row. Deliberately has NO password field —
/// adding one here must fail review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, FromRow)]
pub struct WifiSsidEntry {
    pub ssid: String,
    pub priority: i16,
}

/// Full database row for stored WiFi metadata.
#[derive(Debug, Clone, PartialEq, Serialize, FromRow)]
pub struct DeviceWifiConfigRow {
    pub device_id: String,
    pub ssid: String,
    pub priority: i16,
    pub config_version: i64,
    pub updated_at: DateTime<Utc>,
}

/// Desired-state view returned by GET /wifi. No secrets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WifiConfigView {
    pub device_id: String,
    pub ssids: Vec<WifiSsidEntry>,
    pub config_version: i64,
    pub state: String,
}

/// Delivery states for the latest desired config of a device.
pub mod delivery_state {
    pub const PENDING: &str = "pending";
    pub const APPLIED: &str = "applied";
    pub const ROLLED_BACK: &str = "rolled_back";
    pub const REJECTED: &str = "rejected";
    pub const UNKNOWN: &str = "unknown";
}

/// Strip a provision config down to SSID metadata for persistence.
/// Passwords are dropped here and never reach the DB layer.
pub fn metadata_from_provision(config: &WifiProvisionConfig) -> Vec<WifiSsidEntry> {
    let mut entries: Vec<WifiSsidEntry> = config
        .entries
        .iter()
        .map(|entry| WifiSsidEntry {
            ssid: entry.ssid.trim().to_string(),
            priority: entry.priority as i16,
        })
        .collect();

    entries.sort_by_key(|entry| entry.priority);
    entries
}

/// Replace the desired SSID metadata for a device in one transaction.
///
/// Bumps `config_version` monotonically for the device.
pub async fn replace_device_wifi_config(
    pool: &PgPool,
    device_id: &str,
    entries: &[WifiSsidEntry],
) -> DbResult<Vec<DeviceWifiConfigRow>> {
    let mut tx = pool.begin().await.map_err(DbError::PostgresError)?;

    let next_version: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(config_version), 0) + 1
         FROM device_wifi_config
         WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(DbError::PostgresError)?;

    sqlx::query("DELETE FROM device_wifi_config WHERE device_id = $1")
        .bind(device_id)
        .execute(&mut *tx)
        .await
        .map_err(DbError::PostgresError)?;

    let mut rows = Vec::with_capacity(entries.len());

    for entry in entries {
        let row = sqlx::query_as::<_, DeviceWifiConfigRow>(
            r#"
            INSERT INTO device_wifi_config
                (device_id, ssid, priority, config_version)
            VALUES ($1, $2, $3, $4)
            RETURNING device_id, ssid, priority, config_version, updated_at
            "#,
        )
        .bind(device_id)
        .bind(&entry.ssid)
        .bind(entry.priority)
        .bind(next_version)
        .fetch_one(&mut *tx)
        .await
        .map_err(DbError::PostgresError)?;

        rows.push(row);
    }

    tx.commit().await.map_err(DbError::PostgresError)?;
    Ok(rows)
}

/// Compatibility wrapper when caller already owns the config version.
///
/// Prefer `replace_device_wifi_config` for normal writes.
pub async fn replace_wifi_metadata(
    pool: &PgPool,
    device_id: &str,
    entries: &[WifiSsidEntry],
    version: i64,
) -> DbResult<()> {
    let mut tx = pool.begin().await.map_err(DbError::PostgresError)?;

    sqlx::query("DELETE FROM device_wifi_config WHERE device_id = $1")
        .bind(device_id)
        .execute(&mut *tx)
        .await
        .map_err(DbError::PostgresError)?;

    for entry in entries {
        sqlx::query(
            "INSERT INTO device_wifi_config
                (device_id, ssid, priority, config_version)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(device_id)
        .bind(&entry.ssid)
        .bind(entry.priority)
        .bind(version)
        .execute(&mut *tx)
        .await
        .map_err(DbError::PostgresError)?;
    }

    tx.commit().await.map_err(DbError::PostgresError)?;
    Ok(())
}

/// Load stored SSID metadata plus the latest stored version (0 when none).
pub async fn get_wifi_metadata(
    pool: &PgPool,
    device_id: &str,
) -> DbResult<(Vec<WifiSsidEntry>, i64)> {
    let rows: Vec<WifiSsidRow> = sqlx::query_as(
        "SELECT ssid, priority, config_version
         FROM device_wifi_config
         WHERE device_id = $1
         ORDER BY priority ASC",
    )
    .bind(device_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::PostgresError)?;

    let version = rows
        .iter()
        .map(|row| row.config_version)
        .max()
        .unwrap_or(0);

    let entries = rows
        .into_iter()
        .map(|row| WifiSsidEntry {
            ssid: row.ssid,
            priority: row.priority,
        })
        .collect();

    Ok((entries, version))
}

#[derive(FromRow)]
struct WifiSsidRow {
    ssid: String,
    priority: i16,
    config_version: i64,
}

/// Returns the backend's last-known WiFi SSID list for a device.
pub async fn get_device_wifi_config(
    pool: &PgPool,
    device_id: &str,
) -> DbResult<Vec<DeviceWifiConfigRow>> {
    sqlx::query_as::<_, DeviceWifiConfigRow>(
        r#"
        SELECT device_id, ssid, priority, config_version, updated_at
        FROM device_wifi_config
        WHERE device_id = $1
        ORDER BY priority
        "#,
    )
    .bind(device_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::PostgresError)
}

/// Record desired delivery state for a device config version.
pub async fn set_delivery_state(
    pool: &PgPool,
    device_id: &str,
    version: i64,
    state: &str,
    last_result: Option<&str>,
) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO device_wifi_delivery
            (device_id, config_version, desired_state, last_result, updated_at, applied_at)
         VALUES
            ($1, $2, $3, $4, CURRENT_TIMESTAMP,
             CASE WHEN $3 = 'applied' THEN CURRENT_TIMESTAMP ELSE NULL END)
         ON CONFLICT (device_id) DO UPDATE SET
            config_version = EXCLUDED.config_version,
            desired_state = EXCLUDED.desired_state,
            last_result = EXCLUDED.last_result,
            updated_at = CURRENT_TIMESTAMP,
            applied_at = CASE
                WHEN EXCLUDED.desired_state = 'applied'
                THEN CURRENT_TIMESTAMP
                ELSE device_wifi_delivery.applied_at
            END",
    )
    .bind(device_id)
    .bind(version)
    .bind(state)
    .bind(last_result)
    .execute(pool)
    .await
    .map_err(DbError::PostgresError)?;
    Ok(())
}

/// Load tracked delivery row, if any.
pub async fn get_delivery(
    pool: &PgPool,
    device_id: &str,
) -> DbResult<Option<(i64, String, Option<String>)>> {
    let row: Option<DeliveryRow> = sqlx::query_as(
        "SELECT config_version, desired_state, last_result
         FROM device_wifi_delivery
         WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::PostgresError)?;

    Ok(row.map(|row| {
        (
            row.config_version,
            row.desired_state,
            row.last_result,
        )
    }))
}

#[derive(FromRow)]
struct DeliveryRow {
    config_version: i64,
    desired_state: String,
    last_result: Option<String>,
}

/// Full desired-state view for GET /wifi: metadata + delivery state.
pub async fn get_wifi_config_view(
    pool: &PgPool,
    device_id: &str,
) -> DbResult<WifiConfigView> {
    let (ssids, version) = get_wifi_metadata(pool, device_id).await?;

    let state = match get_delivery(pool, device_id).await? {
        Some((delivered_version, state, _)) if delivered_version == version => state,
        Some(_) => delivery_state::PENDING.to_string(),
        None if version == 0 => delivery_state::UNKNOWN.to_string(),
        None => delivery_state::PENDING.to_string(),
    };

    Ok(WifiConfigView {
        device_id: device_id.to_string(),
        ssids,
        config_version: version,
        state,
    })
}