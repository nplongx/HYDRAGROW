use actix_web::web;
use serde_json::json;
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

    let mut sensor_data = SensorData {
        device_id: device_id.clone(),
        temp: incoming.temp,
        tds: incoming.tds,
        ph: incoming.ph,
        water_level: incoming.water_level,
        pump_status: incoming.pump_status,
        time,
        controller_received_ms: incoming.controller_received_ms,
        rssi: incoming.rssi,
        free_heap: incoming.free_heap,
        uptime: incoming.uptime,
        err_water: incoming.err_water,
        err_temp: incoming.err_temp,
        err_ph: incoming.err_ph,
        err_tds: incoming.err_tds,
        is_continuous: incoming.is_continuous,
        ph_voltage_mv: incoming.ph_voltage_mv,
    };

    debug!(
        "Nhận dữ liệu cảm biến: ph={:.2}, tds={:.2}",
        sensor_data.ph, sensor_data.tds
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

    let cached_state = {
        let states = app_state.device_states.read().await;
        states
            .get(&device_id)
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
    };
    if let Some(cached_pump_status) = cached_state
        .as_ref()
        .and_then(|cached| cached.get("pump_status"))
        .and_then(|value| serde_json::from_value::<PumpStatus>(value.clone()).ok())
    {
        sensor_data.pump_status = cached_pump_status;
    }
    let merged_state = merge_sensor_state_cache(cached_state, &sensor_data);
    if let Ok(json_str) = serde_json::to_string(&merged_state) {
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

fn merge_sensor_state_cache(
    existing: Option<serde_json::Value>,
    sensor_data: &SensorData,
) -> serde_json::Value {
    let mut merged = existing.unwrap_or_else(|| json!({ "device_id": sensor_data.device_id }));
    let sensor_json = serde_json::to_value(sensor_data).unwrap_or_else(|_| json!({}));

    if let (Some(merged_obj), Some(sensor_obj)) = (merged.as_object_mut(), sensor_json.as_object())
    {
        for (key, value) in sensor_obj {
            if key == "pump_status" && merged_obj.contains_key("pump_status") {
                continue;
            }
            merged_obj.insert(key.clone(), value.clone());
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::merge_sensor_state_cache;
    use crate::models::sensor::{PumpStatus, SensorData};
    use serde_json::json;

    fn sensor_data() -> SensorData {
        SensorData {
            device_id: "device_001".to_string(),
            tds: 1.2,
            ph: 6.1,
            temp: 25.0,
            water_level: 80.0,
            pump_status: PumpStatus::default(),
            time: "2026-05-28T00:00:00Z".to_string(),
            controller_received_ms: None,
            rssi: None,
            free_heap: None,
            uptime: None,
            err_water: None,
            err_temp: None,
            err_ph: None,
            err_tds: None,
            is_continuous: None,
            ph_voltage_mv: Some(2450.0),
        }
    }

    #[test]
    fn sensor_update_preserves_fsm_pump_status_in_device_cache() {
        let existing = json!({
            "device_id": "device_001",
            "fsm_state": "Monitoring",
            "budgets": { "ec_ml": 2.0, "ph_ml": 1.0 },
            "pump_status": { "pump_a": true, "pump_b": false }
        });

        let merged = merge_sensor_state_cache(Some(existing), &sensor_data());

        assert_eq!(merged["pump_status"]["pump_a"], true);
        assert_eq!(merged["fsm_state"], "Monitoring");
        assert_eq!(merged["budgets"]["ph_ml"], 1.0);
        assert_eq!(merged["ph_voltage_mv"], 2450.0);
    }
}
