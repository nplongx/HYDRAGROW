use crate::sensors::sensor_manager::SensorData;
use serde_json::{json, Value};

/// Serialize SensorData thành JSON payload theo định dạng AGITECH backend.
pub fn build_sensor_payload(
    device_id: &str,
    data: &SensorData,
    timestamp_iso: &str,
    rssi: i32,
    free_heap: u32,
) -> Value {
    json!({
        "device_id": device_id,
        "ec": data.tds,
        "ph": data.ph,
        "temp": data.temperature,
        "water_level": data.water_level,
        "ph_voltage_mv": data.ph_voltage_mv,
        "time": timestamp_iso,
        "rssi": rssi,
        "free_heap": free_heap,
        "err_temp": data.err_temperature,
        "err_water": data.err_water_level,
        "err_ph": data.err_ph,
        "err_tds": data.err_tds,
    })
}

/// JSON status message.
pub fn build_status_payload(device_id: &str, status: &str, message: &str) -> Value {
    json!({
        "device_id": device_id,
        "status": status,
        "message": message,
    })
}
