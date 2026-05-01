use serde::{Deserialize, Serialize};

pub mod helper;

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

#[derive(Debug, Serialize)]
pub struct MqttCommandPayload {
    pub target: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<MqttCommandParams>,
}

#[derive(Debug, Serialize)]
pub struct MqttCommandParams {
    pub pump_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_sec: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pwm: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ControlMode {
    Auto,
    Manual,
}

impl ControlMode {
    pub fn from_string(str: &str) -> Self {
        match str {
            "auto" => Self::Auto,
            _ => Self::Manual,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ControllerConfig {
    pub device_id: String,
    pub control_mode: ControlMode,
    pub is_enabled: bool,

    // 1. NGƯỠNG MỤC TIÊU
    pub ec_target: f32,
    pub ec_tolerance: f32,
    pub ph_target: f32,
    pub ph_tolerance: f32,

    // 2. QUẢN LÝ NƯỚC
    pub water_level_min: f32,
    pub water_level_target: f32,
    pub water_level_max: f32,
    pub water_level_tolerance: f32,

    // 🟢 THÊM: Sục khí / Tuần hoàn
    // pub circulation_mode: String,
    // pub circulation_on_sec: u64,
    // pub circulation_off_sec: u64,
    pub auto_refill_enabled: bool,
    pub auto_drain_overflow: bool,
    pub auto_dilute_enabled: bool,
    pub dilute_drain_amount_cm: f32,
    pub scheduled_water_change_enabled: bool,
    pub water_change_cron: String,
    pub scheduled_drain_amount_cm: f32,
    pub misting_on_duration_ms: i32,
    pub misting_off_duration_ms: i32,

    // 3. AN TOÀN
    pub emergency_shutdown: bool,
    pub max_ec_limit: f32,
    pub min_ec_limit: f32,
    pub min_ph_limit: f32,
    pub max_ph_limit: f32,
    pub max_ec_delta: f32,
    pub max_ph_delta: f32,
    pub max_dose_per_cycle: f32,
    pub min_temp_limit: f32,
    pub max_temp_limit: f32,

    // 🟢 THÊM: Giới hạn an toàn bơm
    pub max_dose_per_hour: f32,
    pub cooldown_sec: i32,
    pub max_refill_cycles_per_hour: i32,
    pub max_drain_cycles_per_hour: i32,

    pub water_level_critical_min: f32,
    pub max_refill_duration_sec: i32,
    pub max_drain_duration_sec: i32,
    pub ec_ack_threshold: f32,
    pub ph_ack_threshold: f32,
    pub water_ack_threshold: f32,

    // 4. CHÂM PHÂN
    pub ec_gain_per_ml: f32,
    pub ph_shift_up_per_ml: f32,
    pub ph_shift_down_per_ml: f32,
    pub active_mixing_sec: i32,
    pub sensor_stabilize_sec: i32,
    pub ec_step_ratio: f32,
    pub ph_step_ratio: f32,

    pub pump_a_capacity_ml_per_sec: f32,
    pub pump_b_capacity_ml_per_sec: f32,
    pub delay_between_a_and_b_sec: i32,
    pub pump_ph_up_capacity_ml_per_sec: f32,
    pub pump_ph_down_capacity_ml_per_sec: f32,

    pub soft_start_duration: i32,
    pub scheduled_mixing_interval_sec: i32,
    pub scheduled_mixing_duration_sec: i32,

    // 5. CẢM BIẾN
    // pub ph_v7: f32,
    // pub ph_v4: f32,
    // pub ec_factor: f32,
    // pub ec_offset: f32,
    // pub temp_offset: f32,
    // pub temp_compensation_beta: f32,
    // pub tank_height: f32,

    // 🔴 BỎ: Các trường dư thừa
    // pub sampling_interval: u64,
    // pub publish_interval: u64,
    // pub moving_average_window: u32,
    pub enable_ec_sensor: bool,
    pub enable_ph_sensor: bool,
    pub enable_water_level_sensor: bool,
    pub enable_temp_sensor: bool,

    pub tank_height: i32,

    // 🔴 BỎ: Chuyển sang check logic (temp_compensation_beta > 0)
    // pub enable_ec_tc: bool,
    // pub enable_ph_tc: bool,

    // 6. THÔNG SỐ LOCAL CỦA ESP32 (Backend không có cũng không sao)
    pub dosing_pwm_percent: i32,
    pub dosing_min_pwm_percent: i32,
    pub pump_a_min_pwm_percent: Option<i32>,
    pub pump_b_min_pwm_percent: Option<i32>,
    pub pump_ph_up_min_pwm_percent: Option<i32>,
    pub pump_ph_down_min_pwm_percent: Option<i32>,
    pub dosing_pulse_on_ms: i32,
    pub dosing_pulse_off_ms: i32,
    pub dosing_min_dose_ml: f32,
    pub dosing_max_pulse_count_per_cycle: i32,
    pub osaka_mixing_pwm_percent: i32,
    pub osaka_misting_pwm_percent: i32,
    pub misting_temp_threshold: f32,
    pub high_temp_misting_on_duration_ms: i64,
    pub high_temp_misting_off_duration_ms: i64,

    pub scheduled_dosing_enabled: bool,
    pub scheduled_dosing_cron: String, // Sử dụng Cron (VD: "0 0 8 * * *")
    pub scheduled_dose_a_ml: f32,
    pub scheduled_dose_b_ml: f32,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            device_id: "device_001".to_string(),
            control_mode: ControlMode::Manual,
            is_enabled: true,

            ec_target: 1.2,
            ec_tolerance: 0.05,
            ph_target: 6.0,
            ph_tolerance: 0.1,

            water_level_min: 15.0,
            water_level_target: 20.0,
            water_level_max: 24.0,
            water_level_tolerance: 1.0,

            // 🟢 THÊM mặc định
            // circulation_mode: "always_on".to_string(),
            // circulation_on_sec: 1800,
            // circulation_off_sec: 900,
            auto_refill_enabled: true,
            auto_drain_overflow: true,
            auto_dilute_enabled: true,
            dilute_drain_amount_cm: 2.0,
            scheduled_water_change_enabled: false,
            water_change_cron: "0 0 7 * * SUN".to_string(),
            scheduled_drain_amount_cm: 5.0,
            misting_on_duration_ms: 10000,
            misting_off_duration_ms: 180000,

            emergency_shutdown: false,
            max_ec_limit: 3.5,
            min_ec_limit: 1.0,
            min_ph_limit: 4.0,
            max_ph_limit: 8.5,
            max_ec_delta: 1.0,
            max_ph_delta: 1.5,
            max_dose_per_cycle: 2.0,
            min_temp_limit: 15.0,
            max_temp_limit: 35.0,

            // 🟢 THÊM mặc định
            max_dose_per_hour: 200.0,
            cooldown_sec: 60,
            max_refill_cycles_per_hour: 3,
            max_drain_cycles_per_hour: 3,

            water_level_critical_min: 5.0,
            max_refill_duration_sec: 120,
            max_drain_duration_sec: 120,
            ec_ack_threshold: 0.05,
            ph_ack_threshold: 0.1,
            water_ack_threshold: 0.5,

            ec_gain_per_ml: 0.015,
            ph_shift_up_per_ml: 0.02,
            ph_shift_down_per_ml: 0.025,
            active_mixing_sec: 5,
            sensor_stabilize_sec: 5,
            ec_step_ratio: 0.4,
            ph_step_ratio: 0.2,

            pump_a_capacity_ml_per_sec: 1.2,
            pump_b_capacity_ml_per_sec: 1.15,
            delay_between_a_and_b_sec: 10, // Độ trễ (Mix) giữa A và B
            pump_ph_up_capacity_ml_per_sec: 1.2,
            pump_ph_down_capacity_ml_per_sec: 1.2,

            soft_start_duration: 3000,
            scheduled_mixing_interval_sec: 3600,
            scheduled_mixing_duration_sec: 300,

            // ph_v7: 1650.0,
            // ph_v4: 1846.4,
            // ec_factor: 880.0,
            // ec_offset: 0.0,
            // temp_offset: 0.0,
            // temp_compensation_beta: 0.02,
            // tank_height: 100.0,

            // 🔴 BỎ
            // sampling_interval: 1000,
            // publish_interval: 5000,
            // moving_average_window: 10,
            enable_ec_sensor: false,
            enable_ph_sensor: false,
            enable_water_level_sensor: false,
            enable_temp_sensor: false,

            // để tạm thuận tiện để lấy thông số cho sensor node
            tank_height: 50,

            // 🔴 BỎ
            // enable_ec_tc: true,
            // enable_ph_tc: true,
            dosing_pwm_percent: 50,
            dosing_min_pwm_percent: 35,
            pump_a_min_pwm_percent: None,
            pump_b_min_pwm_percent: None,
            pump_ph_up_min_pwm_percent: None,
            pump_ph_down_min_pwm_percent: None,
            dosing_pulse_on_ms: 250,
            dosing_pulse_off_ms: 300,
            dosing_min_dose_ml: 0.4,
            dosing_max_pulse_count_per_cycle: 40,
            osaka_mixing_pwm_percent: 60,
            osaka_misting_pwm_percent: 100,
            misting_temp_threshold: 30.0,
            high_temp_misting_on_duration_ms: 15000,
            high_temp_misting_off_duration_ms: 60000,

            scheduled_dosing_enabled: false,
            scheduled_dosing_cron: "0 0 8 * * *".to_string(),
            scheduled_dose_a_ml: 10.0,
            scheduled_dose_b_ml: 10.0,
        }
    }
}
