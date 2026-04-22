use chrono::{DateTime, FixedOffset};
use influxdb2::FromDataPoint;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeviceState {
    On,
    #[default]
    Off,
}

/// Trạng thái hoạt động của các máy bơm (Pump)
/// ĐÃ SỬA: Chuyển sang dùng bool và tên biến chuẩn khớp 100% với ESP32
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PumpStatus {
    pub pump_a: bool,
    pub pump_b: bool,
    pub ph_up: bool,
    pub ph_down: bool,
    pub osaka_pump: bool,
    pub mist_valve: bool,
    pub water_pump_in: bool,
    pub water_pump_out: bool,
}

/// Cấu trúc Sensor đẩy vào InfluxDB và trả về Client
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SensorData {
    pub device_id: String,

    #[validate(range(min = 0.0, max = 20.0))]
    pub ec_value: f64,

    #[validate(range(min = 0.0, max = 14.0))]
    pub ph_value: f64,

    #[validate(range(min = -10.0, max = 100.0))]
    pub temp_value: f64,

    #[validate(range(min = 0.0))]
    pub water_level: f64,

    #[serde(default)]
    pub pump_status: PumpStatus,
    // #[serde(default)]
    // pub timestamp: String,
    pub time: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssi: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_heap: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err_water: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err_temp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err_ph: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err_ec: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_continuous: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ph_voltage_mv: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromDataPoint, Default)]
pub struct SensorDataRow {
    pub device_id: String,
    pub ec_value: f64,
    pub ph_value: f64,
    pub temp_value: f64,
    pub water_level: f64,
    pub pump_status: String,

    // ĐÃ SỬA: Bỏ serde rename, đổi String thành DateTime<FixedOffset>
    pub time: DateTime<FixedOffset>,
}

impl From<SensorDataRow> for SensorData {
    fn from(row: SensorDataRow) -> Self {
        let pump_status = serde_json::from_str(&row.pump_status).unwrap_or_default();
        Self {
            device_id: row.device_id,
            ec_value: row.ec_value,
            ph_value: row.ph_value,
            temp_value: row.temp_value,
            water_level: row.water_level,
            pump_status,

            // ĐÃ SỬA: Format thời gian về lại String chuẩn RFC3339 cho Client
            time: row.time.to_rfc3339(),
            rssi: None,
            free_heap: None,
            is_continuous: None,
            uptime: None,
            err_water: None,
            err_temp: None,
            err_ph: None,
            err_ec: None,
            ph_voltage_mv: None,
        }
    }
}

// Trong src/models/sensor.rs (Backend)
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PumpCommandReq {
    #[validate(length(min = 1))]
    #[serde(rename = "pump")]
    // SỬA Ở ĐÂY: Giao tiếp với FE/MQTT bằng key "pump" thay vì "pump_id"
    pub pump_id: String,

    pub action: String,

    // NÊN BỔ SUNG: Khớp với MqttCommandPayload của ESP32 để hỗ trợ Set PWM và Auto Timeout
    pub duration_sec: Option<u64>,
    pub pwm: Option<u32>,
}
