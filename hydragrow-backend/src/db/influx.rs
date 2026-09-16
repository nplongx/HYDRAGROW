use anyhow::{Context, Result};
use futures_util::stream;
use influxdb2::Client;
use influxdb2::models::DataPoint;
use tracing::{info, instrument};

use crate::models::sensor::{SensorData, SensorDataRow};

const MAX_QUERY_RANGE_SECS: i64 = 30 * 24 * 60 * 60;

pub(crate) fn validate_device_id(device_id: &str) -> Result<()> {
    if device_id.is_empty()
        || device_id.len() > 128
        || !device_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(anyhow::anyhow!("Invalid device identifier"));
    }
    Ok(())
}

pub(crate) fn quote_flux_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

pub(crate) fn validate_relative_range(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 16 {
        return Err(anyhow::anyhow!("Invalid query range"));
    }
    let split_at = value
        .bytes()
        .position(|b| !b.is_ascii_digit())
        .ok_or_else(|| anyhow::anyhow!("Invalid query range"))?;
    let (number, unit) = value.split_at(split_at);
    let amount: i64 = number
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid query range"))?;
    if amount <= 0 {
        return Err(anyhow::anyhow!("Invalid query range"));
    }
    let multiplier = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        "w" => 7 * 24 * 60 * 60,
        _ => return Err(anyhow::anyhow!("Unsupported query range unit")),
    };
    if amount > MAX_QUERY_RANGE_SECS / multiplier {
        return Err(anyhow::anyhow!("Query range exceeds maximum"));
    }
    Ok(())
}

pub(crate) fn validate_absolute_range(start: &str, end: Option<&str>) -> Result<()> {
    let start_at = start
        .parse::<chrono::DateTime<chrono::FixedOffset>>()
        .map_err(|_| anyhow::anyhow!("Invalid start time"))?
        .with_timezone(&chrono::Utc);
    let end_at = end
        .map(|value| {
            value
                .parse::<chrono::DateTime<chrono::FixedOffset>>()
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|_| anyhow::anyhow!("Invalid end time"))
        })
        .transpose()?;
    let effective_end = end_at.unwrap_or_else(chrono::Utc::now);
    if effective_end < start_at
        || effective_end.signed_duration_since(start_at).num_seconds() > MAX_QUERY_RANGE_SECS
    {
        return Err(anyhow::anyhow!("Query range is invalid or exceeds maximum"));
    }
    Ok(())
}

pub(crate) fn validate_resolution(value: &str) -> Result<()> {
    match value {
        "1m" | "5m" | "15m" | "30m" | "1h" | "6h" | "12h" | "1d" => Ok(()),
        _ => Err(anyhow::anyhow!("Unsupported resolution")),
    }
}

#[instrument(skip(client, data))]
pub async fn write_sensor_data(client: &Client, bucket: &str, data: &SensorData) -> Result<()> {
    validate_device_id(&data.device_id)?;
    let mut point_builder = DataPoint::builder("sensor_data")
        .tag("device_id", &data.device_id)
        .field("ec", data.ec as f64)
        .field("ph", data.ph as f64)
        .field("temp", data.temp as f64)
        .field("water_level", data.water_level as f64);

    if let Some(ph_voltage_mv) = data.ph_voltage_mv {
        point_builder = point_builder.field("ph_voltage_mv", ph_voltage_mv);
    }

    let point = point_builder
        .build()
        .context("Failed to build InfluxDB DataPoint")?;

    client
        .write(bucket, stream::iter(vec![point]))
        .await
        .context("Failed to write to InfluxDB")?;

    Ok(())
}

#[instrument(skip(client))]
pub async fn get_latest_sensor_data(
    client: &Client,
    bucket: &str,
    device_id: &str,
) -> Result<SensorData> {
    validate_device_id(device_id)?;
    let flux_query = format!(
        r#"
        from(bucket: "{}")
        |> range(start: -1h)
        |> filter(fn: (r) => r["_measurement"] == "sensor_data")
        |> filter(fn: (r) => r.device_id == "{}")
        |> sort(columns: ["_time"], desc: true)
        |> limit(n: 1)
        "#,
        quote_flux_string(bucket),
        quote_flux_string(device_id)
    );

    let query_obj = influxdb2::models::Query::new(flux_query);
    let tables = client
        .query::<SensorDataRow>(Some(query_obj))
        .await
        .context("Flux query failed")?;

    if let Some(table) = tables.first() {
        info!("Lasted sensor: {:?}", table);
        return Ok(table.to_owned().into());
    }

    Err(anyhow::anyhow!(
        "No sensor data found for device: {}",
        device_id
    ))
}

fn stat_to_flux_fn(stat: &str) -> Result<&'static str> {
    match stat {
        "mean" => Ok("mean()"),
        "min" => Ok("min()"),
        "max" => Ok("max()"),
        other => Err(anyhow::anyhow!("Unsupported stat: {}", other)),
    }
}

#[instrument(skip(client))]
pub async fn query_range_stat(
    client: &Client,
    bucket: &str,
    device_id: &str,
    field: &str,
    stat: &str,
    range_sec: i64,
) -> Result<f64> {
    validate_device_id(device_id)?;
    if !matches!(field, "ec" | "ph" | "temp" | "water_level") {
        return Err(anyhow::anyhow!("Unsupported sensor field"));
    }
    if !(1..=MAX_QUERY_RANGE_SECS).contains(&range_sec) {
        return Err(anyhow::anyhow!("Query range exceeds maximum"));
    }
    let stat_fn = stat_to_flux_fn(stat)?;

    let flux_query = format!(
        r#"
        from(bucket: "{}")
        |> range(start: -{}s)
        |> filter(fn: (r) => r["_measurement"] == "sensor_data")
        |> filter(fn: (r) => r["_field"] == "{}")
        |> filter(fn: (r) => r.device_id == "{}")
        |> {}
        |> keep(columns: ["_value"])
        "#,
        quote_flux_string(bucket),
        range_sec,
        field,
        quote_flux_string(device_id),
        stat_fn
    );

    let query_obj = influxdb2::models::Query::new(flux_query);
    // Since influxdb2 client doesn't easily expose raw float values, we parse it manually
    // from a custom struct
    #[derive(serde::Deserialize, Debug, Default, influxdb2::FromDataPoint)]
    struct StatRow {
        _value: f64,
    }

    let tables = client
        .query::<StatRow>(Some(query_obj))
        .await
        .context("Flux query failed")?;

    if let Some(table) = tables.first() {
        if !table._value.is_finite() {
            return Err(anyhow::anyhow!("Range stat returned non-finite value"));
        }
        return Ok(table._value);
    }

    Err(anyhow::anyhow!(
        "No range stat data found for device: {} field: {}",
        device_id,
        field
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_to_flux_fn_accepts_known_stats() {
        assert_eq!(stat_to_flux_fn("mean").unwrap(), "mean()");
        assert_eq!(stat_to_flux_fn("min").unwrap(), "min()");
        assert_eq!(stat_to_flux_fn("max").unwrap(), "max()");
    }

    #[test]
    fn stat_to_flux_fn_rejects_unknown_stat() {
        assert!(stat_to_flux_fn("median").is_err());
    }

    #[test]
    fn device_id_allowlist_rejects_injection() {
        assert!(validate_device_id("device-001").is_ok());
        assert!(validate_device_id("device\" or r.device_id != \"other").is_err());
        assert!(validate_device_id("device\nother").is_err());
    }

    #[test]
    fn flux_string_escaping_preserves_quotes_and_backslashes() {
        assert_eq!(quote_flux_string("device\\\"x"), "device\\\\\\\"x");
    }

    #[test]
    fn relative_range_is_bounded() {
        assert!(validate_relative_range("24h").is_ok());
        assert!(validate_relative_range("30d").is_ok());
        assert!(validate_relative_range("31d").is_err());
        assert!(validate_relative_range("1mo").is_err());
    }

    #[test]
    fn absolute_range_is_bounded_and_ordered() {
        assert!(
            validate_absolute_range("2026-09-01T00:00:00Z", Some("2026-09-02T00:00:00Z")).is_ok()
        );
        assert!(
            validate_absolute_range("2026-08-01T00:00:00Z", Some("2026-09-02T00:00:00Z")).is_err()
        );
        assert!(
            validate_absolute_range("2026-09-02T00:00:00Z", Some("2026-09-01T00:00:00Z")).is_err()
        );
    }

    #[test]
    fn resolution_is_allowlisted() {
        assert!(validate_resolution("5m").is_ok());
        assert!(validate_resolution("30m").is_ok());
        assert!(validate_resolution("1h").is_ok());
        assert!(validate_resolution("5m) |> drop() //").is_err());
    }
}
