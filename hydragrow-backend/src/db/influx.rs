use anyhow::{Context, Result};
use futures_util::stream;
use influxdb2::Client;
use influxdb2::models::DataPoint;
use tracing::{info, instrument};

use crate::models::sensor::{SensorData, SensorDataRow};

#[instrument(skip(client, data))]
pub async fn write_sensor_data(client: &Client, bucket: &str, data: &SensorData) -> Result<()> {
    let pump_status_json = serde_json::to_string(&data.pump_status)
        .context("Failed to serialize pump_status to JSON")?;

    let mut point_builder = DataPoint::builder("sensor_data")
        .tag("device_id", &data.device_id)
        .field("ec_value", data.ec_value as f64)
        .field("ph_value", data.ph_value as f64)
        .field("temp_value", data.temp_value as f64)
        .field("water_level", data.water_level as f64)
        .field("pump_status", pump_status_json);

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
    // 🟢 ĐÃ THÊM HÀM PIVOT BẮT BUỘC CỦA INFLUXDB2
    let flux_query = format!(
        r#"
        from(bucket: "{}")
        |> range(start: -24h)
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
