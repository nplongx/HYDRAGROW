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
        // Canonical P1.9 wire name is `err_ec`; `err_tds` is input-only legacy.
        "err_ec": data.err_tds,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::sensor_manager::SensorData;

    #[test]
    fn sensor_payload_uses_canonical_wire_names() {
        let data = SensorData {
            tds: 1.7,
            ph: 6.2,
            temperature: 24.5,
            water_level: 20.0,
            ph_voltage_mv: 2530.0,
            err_temperature: false,
            err_water_level: true,
            err_ph: false,
            err_tds: true,
            ..Default::default()
        };
        let value = build_sensor_payload("sensor-1", &data, "2026-09-18T00:00:00Z", -55, 1234);
        assert_eq!(value["device_id"], "sensor-1");
        assert_eq!(value["ec"], 1.7);
        assert_eq!(value["ph"], 6.2);
        assert_eq!(value["temp"], 24.5);
        assert_eq!(value["water_level"], 20.0);
        assert_eq!(value["time"], "2026-09-18T00:00:00Z");
        assert_eq!(value["rssi"], -55);
        assert_eq!(value["free_heap"], 1234);
        assert_eq!(value["err_ec"], true);
        assert!(value.get("err_tds").is_none());
    }

    #[test]
    fn mqtt_topics_remain_canonical() {
        let topics = crate::mqtt::manager::MqttTopics::new("sensor-1");
        assert_eq!(topics.sensor, "AGITECH/sensor-1/sensors");
        assert_eq!(topics.status, "AGITECH/sensor-1/sensor/status");
        assert_eq!(topics.command, "AGITECH/sensor-1/command");
        assert_eq!(topics.config, "AGITECH/sensor-1/sensors/config");
    }
}
