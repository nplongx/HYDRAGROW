use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorData {
    pub device_id: String,
    pub ec: f64,
    pub ph: f64,
    pub temp: f64,
    pub water_level: f64,
    #[serde(default)]
    pub pump_status: PumpStatus,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertMessage {
    pub level: String,
    pub title: String,
    pub message: String,
    pub device_id: String,
    pub timestamp: u64,
    pub reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
