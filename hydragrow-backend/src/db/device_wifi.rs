//! Persists which WiFi networks a device has been told to use — SSID,
//! priority, and a monotonically increasing config_version. The password is
//! never written here; it only ever lives in the ESP32's NVS (see
//! ESP32-C3-CONTROLLER-NODE/src/hw/wifi_store.rs).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, PartialEq)]
pub struct WifiSsidEntry {
    pub ssid: String,
    pub priority: i16,
}

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct DeviceWifiConfigRow {
    pub device_id: String,
    pub ssid: String,
    pub priority: i16,
    pub config_version: i64,
    pub updated_at: DateTime<Utc>,
}

/// Replaces the full WiFi SSID list the backend has on file for a device.
/// Deletes prior rows and reinserts the new set inside one transaction,
/// bumping `config_version` so the UI can tell when it last changed.
/// `entries` has no field to put a password in — that's intentional.
pub async fn replace_device_wifi_config(
    pool: &PgPool,
    device_id: &str,
    entries: &[WifiSsidEntry],
) -> Result<Vec<DeviceWifiConfigRow>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let next_version: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(config_version), 0) + 1 FROM device_wifi_config WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM device_wifi_config WHERE device_id = $1")
        .bind(device_id)
        .execute(&mut *tx)
        .await?;

    let mut rows = Vec::with_capacity(entries.len());
    for entry in entries {
        let row = sqlx::query_as::<_, DeviceWifiConfigRow>(
            r#"
            INSERT INTO device_wifi_config (device_id, ssid, priority, config_version)
            VALUES ($1, $2, $3, $4)
            RETURNING device_id, ssid, priority, config_version, updated_at
            "#,
        )
        .bind(device_id)
        .bind(&entry.ssid)
        .bind(entry.priority)
        .bind(next_version)
        .fetch_one(&mut *tx)
        .await?;
        rows.push(row);
    }

    tx.commit().await?;
    Ok(rows)
}

/// Returns the backend's last-known WiFi SSID list for a device, ordered by
/// priority. Empty if the device has never had a list pushed to it.
pub async fn get_device_wifi_config(
    pool: &PgPool,
    device_id: &str,
) -> Result<Vec<DeviceWifiConfigRow>, sqlx::Error> {
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
}
