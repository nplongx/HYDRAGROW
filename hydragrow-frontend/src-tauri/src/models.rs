// src-tauri/src/models.rs
use serde::{Deserialize, Serialize};

// ── Settings ─────────────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub api_key: String,
    pub backend_url: String,
    pub device_id: String,
}

// ── Sensor & Relay Data ──────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    pub ph_voltage_mv: Option<f64>,
    pub is_continuous: Option<bool>,
    pub err_water: Option<bool>,
    pub err_temp: Option<bool>,
    pub err_ph: Option<bool>,
    pub err_ec: Option<bool>,
    pub rssi: Option<i32>,
    pub free_heap: Option<u32>,
    pub uptime: Option<u32>,
    pub pump_status: PumpStatus,
    pub time: String,
}

// ── Configurations (DB Entities) ─────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceConfig {
    pub device_id: String,
    pub ec_target: f64,
    pub ec_tolerance: f64,
    pub ph_target: f64,
    pub ph_tolerance: f64,
    pub temp_target: f64,
    pub temp_tolerance: f64,
    pub control_mode: String,
    pub is_enabled: i64, // SQLite không có boolean, dùng INTEGER (0, 1)
    pub last_updated: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PumpCalibration {
    pub id: String,
    pub device_id: String,
    pub pump_type: String,
    pub flow_rate_ml_per_sec: f64,
    pub min_activation_sec: f64,
    pub max_activation_sec: f64,
    pub last_calibrated: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DosingCalibration {
    pub device_id: String,
    pub tank_volume_l: f64,
    pub ec_gain_per_ml: f64,
    pub ph_shift_up_per_ml: f64,
    pub ph_shift_down_per_ml: f64,
    pub mixing_delay_sec: i64,
    pub ec_step_ratio: f64,
    pub ph_step_ratio: f64,
    pub pump_capacity_ml_per_sec: f64,
    // Đã thêm 2 trường từ state.rs: active_mixing_sec và sensor_stabilize_sec
    pub active_mixing_sec: i64,
    pub sensor_stabilize_sec: i64,
    pub last_calibrated: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SafetyConfig {
    pub device_id: String,
    pub max_ec_limit: f64,
    pub min_ec_limit: f64,
    pub min_ph_limit: f64,
    pub max_ph_limit: f64,
    pub max_ec_delta: f64,
    pub max_ph_delta: f64,
    pub max_dose_per_cycle: f64,
    pub cooldown_sec: i64,
    pub max_dose_per_hour: f64,

    pub water_level_critical_min: f64, // Mức nước nguy hiểm
    pub max_refill_cycles_per_hour: i64,
    pub max_drain_cycles_per_hour: i64,
    pub max_refill_duration_sec: i64,
    pub max_drain_duration_sec: i64,

    pub min_temp_limit: f64,
    pub max_temp_limit: f64,

    pub emergency_shutdown: i64,
    pub last_updated: String,
}

// Struct phụ trợ cho quy trình water_sequence (Cấp/xả nước tự động)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WaterConfig {
    pub device_id: String,
    pub water_level_min: f64,
    pub water_level_target: f64,
    pub water_level_max: f64,
    pub water_level_drain: f64,
    pub circulation_mode: String,
    pub circulation_on_sec: i64,
    pub circulation_off_sec: i64,

    pub water_level_tolerance: f64,
    pub auto_refill_enabled: i64,
    pub auto_drain_overflow: i64,
    pub auto_dilute_enabled: i64,
    pub dilute_drain_amount_cm: f64,
    pub scheduled_water_change_enabled: i64,
    pub water_change_interval_sec: i64,
    pub scheduled_drain_amount_cm: f64,

    pub last_updated: String,
}

// ── Aggregated Data (Đẩy xuống ESP32) ────────────────────────────────
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Esp32AggregatedConfig {
    pub device_id: String,
    pub control_mode: String,
    pub is_enabled: bool,

    // --- 1. DEVICE CONFIG ---
    pub ec_target: f64,
    pub ec_tolerance: f64,
    pub ph_target: f64,
    pub ph_tolerance: f64,

    // --- 2. WATER CONFIG ---
    pub water_level_min: f64,
    pub water_level_target: f64,
    pub water_level_max: f64,
    pub water_level_tolerance: f64,
    pub auto_refill_enabled: bool,
    pub auto_drain_overflow: bool,

    pub auto_dilute_enabled: bool,
    pub dilute_drain_amount_cm: f64,

    pub scheduled_water_change_enabled: bool,
    pub water_change_interval_sec: i64,
    pub scheduled_drain_amount_cm: f64,

    // --- 3. SAFETY CONFIG ---
    pub emergency_shutdown: bool,
    pub max_ec_limit: f64,
    pub min_ph_limit: f64,
    pub max_ph_limit: f64,
    pub max_ec_delta: f64,
    pub max_ph_delta: f64,
    pub max_dose_per_cycle: f64,
    pub water_level_critical_min: f64,
    pub max_refill_duration_sec: i64,
    pub max_drain_duration_sec: i64,

    // --- 4. DOSING & PUMP ---
    pub ec_gain_per_ml: f64,
    pub ph_shift_up_per_ml: f64,
    pub ph_shift_down_per_ml: f64,
    pub mixing_delay_sec: i64,
    pub ec_step_ratio: f64,
    pub ph_step_ratio: f64,
    pub pump_capacity_ml_per_sec: f64,
    // Đã thêm 2 trường từ state.rs
    pub active_mixing_sec: i64,
    pub sensor_stabilize_sec: i64,
}

// ── WebSocket Payloads (Gửi sang React UI) ───────────────────────────
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlertPayload {
    pub device_id: String,
    pub level: String, // "info" | "warning" | "critical" | "success"
    pub title: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusPayload {
    pub is_online: bool,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorCalibration {
    pub device_id: String,
    pub ph_v7: f32,
    pub ph_v4: f32,
    pub ec_factor: f32,
    pub ec_offset: f32,
    pub temp_offset: f32,
    pub temp_compensation_beta: f32,
    pub sampling_interval: i64,
    pub publish_interval: i64,
    pub moving_average_window: i64,
    pub enable_ph_sensor: bool,
    pub enable_ec_sensor: bool,
    pub enable_temp_sensor: bool,
    pub enable_water_level_sensor: bool,
    pub last_calibrated: String,
}
