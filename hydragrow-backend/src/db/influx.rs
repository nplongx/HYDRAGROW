use anyhow::{Context, Result};
use futures_util::stream;
use influxdb2::Client;
use influxdb2::models::DataPoint;
use tracing::{info, instrument};

use crate::models::sensor::{SensorData, SensorDataRow};

#[instrument(skip(client, data))]
pub async fn write_sensor_data(client: &Client, bucket: &str, data: &SensorData) -> Result<()> {
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
    let flux_query = format!(
        r#"
        from(bucket: "{}")
        |> range(start: -1h)
        |> filter(fn: (r) => r["_measurement"] == "sensor_data")
        |> filter(fn: (r) => r.device_id == "{}")
        |> sort(columns: ["_time"], desc: true)
        |> limit(n: 1)
        "#,
        bucket, device_id
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

#[instrument(skip(client))]
pub async fn query_range_stat(
    client: &Client,
    bucket: &str,
    device_id: &str,
    field: &str,
    stat: &str,
    range_h: i64,
) -> Result<f64> {
    let stat_fn = match stat {
        "mean" => "mean()",
        "min" => "min()",
        "max" => "max()",
        _ => return Err(anyhow::anyhow!("Unsupported stat: {}", stat)),
    };

    let flux_query = format!(
        r#"
        from(bucket: "{}")
        |> range(start: -{}h)
        |> filter(fn: (r) => r["_measurement"] == "sensor_data")
        |> filter(fn: (r) => r["_field"] == "{}")
        |> filter(fn: (r) => r.device_id == "{}")
        |> {}
        |> keep(columns: ["_value"])
        "#,
        bucket, range_h, field, device_id, stat_fn
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
        return Ok(table._value);
    }

    Ok(0.0)
}
