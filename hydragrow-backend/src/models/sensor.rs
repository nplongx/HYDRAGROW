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
    pub pump_a_pwm: Option<u32>,
    pub pump_b_pwm: Option<u32>,
    pub ph_up_pwm: Option<u32>,
    pub ph_down_pwm: Option<u32>,
    pub osaka_pwm: Option<u32>,
    pub dosing_pulse_active: Option<bool>,
    pub dosing_pulse_count: Option<u32>,
}

/// Cấu trúc Sensor đẩy vào InfluxDB và trả về Client
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SensorData {
    pub device_id: String,

    #[validate(range(min = 0.0, max = 20.0))]
    pub ec: f64,

    #[validate(range(min = 0.0, max = 14.0))]
    pub ph: f64,

    #[validate(range(min = -10.0, max = 100.0))]
    pub temp: f64,

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
    pub ec: f64,
    pub ph: f64,
    pub temp: f64,
    pub water_level: f64,
    pub pump_status: String,

    pub time: DateTime<FixedOffset>,
}

impl From<SensorDataRow> for SensorData {
    fn from(row: SensorDataRow) -> Self {
        let pump_status = serde_json::from_str(&row.pump_status).unwrap_or_default();
        Self {
            device_id: row.device_id,
            ec: row.ec,
            ph: row.ph,
            temp: row.temp,
            water_level: row.water_level,
            pump_status,

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

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PumpCommandReq {
    pub target: Option<String>,

    pub action: String,

    #[serde(default)]
    pub params: Option<PumpCommandParams>,

    #[serde(default, alias = "pump")]
    pub pump_id: Option<String>,
    #[serde(default)]
    pub duration_sec: Option<u64>,
    #[serde(default)]
    pub pwm: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PumpCommandParams {
    #[serde(default)]
    pub pump_id: Option<String>,
    #[serde(default)]
    pub duration_sec: Option<u64>,
    #[serde(default)]
    pub pwm: Option<u32>,
    #[serde(default)]
    pub state: Option<bool>,
}
