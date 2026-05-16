use actix_web::web;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, instrument};

use crate::AppState;
use hydragrow_shared::events::AppEvent;
use crate::db::influx::write_sensor_data;
use crate::models::sensor::{PumpStatus, SensorData};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IncomingSensorPayload {
    pub temp: Option<f64>,
    pub ec: Option<f64>,
    pub ph: Option<f64>,
    pub water_level: Option<f64>,
    #[serde(rename = "last_update_ms", alias = "timestamp_ms")]
    pub timestamp_ms: Option<u64>,
    pub time: Option<String>,
    pub pump_status: Option<PumpStatus>,

    pub rssi: Option<i32>,
    pub free_heap: Option<u32>,
    pub uptime: Option<u32>,

    pub err_water: Option<bool>,
    pub err_temp: Option<bool>,
    pub err_ph: Option<bool>,
    pub err_ec: Option<bool>,

    pub is_continuous: Option<bool>,
    pub ph_voltage_mv: Option<f64>,
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let incoming: IncomingSensorPayload = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!(error = ?e, "Lỗi parse JSON SensorData");
            return;
        }
    };

    let time = incoming
        .time
        .clone()
        .or_else(|| {
            incoming
                .timestamp_ms
                .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64))
                .map(|dt| dt.to_rfc3339())
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    let sensor_data = SensorData {
        device_id: device_id.clone(),
        temp: incoming.temp.unwrap_or(0.0),
        ec: incoming.ec.unwrap_or(0.0),
        ph: incoming.ph.unwrap_or(0.0),
        water_level: incoming.water_level.unwrap_or(0.0),
        pump_status: incoming.pump_status.unwrap_or_default(),
        time,
        rssi: incoming.rssi,
        free_heap: incoming.free_heap,
        uptime: incoming.uptime,
        err_water: incoming.err_water,
        err_temp: incoming.err_temp,
        err_ph: incoming.err_ph,
        err_ec: incoming.err_ec,
        is_continuous: incoming.is_continuous,
        ph_voltage_mv: incoming.ph_voltage_mv,
    };

    debug!(
        "Nhận dữ liệu cảm biến: ph={:.2}, ec={:.2}",
        sensor_data.ph, sensor_data.ec
    );

    if let Some(ph_voltage_mv) = incoming.ph_voltage_mv {
        let observed_at = chrono::DateTime::parse_from_rfc3339(&sensor_data.time)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let mut sample_map = app_state.ph_voltage_samples.write().await;
        let samples = sample_map.entry(device_id.clone()).or_default();
        samples.push_back(crate::PhVoltageSample {
            voltage_mv: ph_voltage_mv,
            observed_at,
            received_at: std::time::Instant::now(),
        });

        while samples
            .front()
            .is_some_and(|sample| sample.received_at.elapsed().as_secs() > 120)
        {
            samples.pop_front();
        }
    }

    if let Ok(json_str) = serde_json::to_string(&sensor_data) {
        let mut states = app_state.device_states.write().await;
        states.insert(device_id.clone(), json_str);
    }

    if let Err(e) = write_sensor_data(
        &app_state.influx_client,
        &app_state.influx_bucket,
        &sensor_data,
    )
    .await
    {
        error!(error = ?e, "Lỗi lưu SensorData vào InfluxDB");
    }

    let _ = app_state.event_bus.send(AppEvent::SensorUpdate(sensor_data));
}

