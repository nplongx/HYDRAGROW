use actix_web::web;
use tracing::{debug, error, instrument};

use crate::AppState;
use crate::db::influx::write_sensor_data;
use crate::models::sensor::{PumpStatus, SensorData};
use hydragrow_shared::events::AppEvent;

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let incoming: SensorData = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!(error = ?e, "Lỗi parse JSON SensorData");
            return;
        }
    };

    let time = incoming.time.clone();

    // if !validate_payload_schema("sensor_update", &device_id, incoming.schema_version) {
    //     return;
    // }

    let sensor_data = SensorData {
        device_id: device_id.clone(),
        temp: incoming.temp,
        ec: incoming.ec,
        ph: incoming.ph,
        water_level: incoming.water_level,
        pump_status: incoming.pump_status,
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

    let _ = app_state
        .event_bus
        .send(AppEvent::SensorUpdate(sensor_data));
}
