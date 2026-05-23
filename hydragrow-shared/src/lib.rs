use serde::{Deserialize, Serialize};

pub mod events;
pub mod fsm;
pub mod helper;
pub mod topics;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pump_a_pwm: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pump_b_pwm: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ph_up_pwm: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ph_down_pwm: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub osaka_pwm: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dosing_pulse_active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dosing_pulse_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorData {
    pub device_id: String,
    pub ec: f32,
    pub ph: f32,
    pub temp: f32,
    pub water_level: f32,
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
    pub category: String,
    pub title: String,
    pub message: String,
    pub device_id: String,
    pub timestamp: u64,
    pub reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttCommandOut {
    pub target: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<MqttCommandParams>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttCommandIn {
    pub action: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub params: Option<MqttCommandInParams>,
    #[serde(default)]
    pub pump: Option<String>,
    #[serde(default)]
    pub duration_sec: Option<u64>,
    #[serde(default)]
    pub pwm: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttCommandInParams {
    #[serde(default)]
    pub pump_id: Option<String>,
    #[serde(default)]
    pub duration_sec: Option<u64>,
    #[serde(default)]
    pub pwm: Option<u32>,
    #[serde(default)]
    pub state: Option<bool>,
    #[serde(default)]
    pub ota_url: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MqttCommandParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pump_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_sec: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pwm: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ota_url: Option<String>,
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
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PhaseData {
    pub ec: f32,
    pub ph: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub water_level: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DoseData {
    pub pump_a_ml: f32,
    pub pump_b_ml: f32,
    pub ph_up_ml: f32,
    pub ph_down_ml: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DosingReportPayload {
    pub cycle_id: String,
    pub trigger: String,
    pub pre: PhaseData,
    pub dose: DoseData,
    pub post_mixing: PhaseData,
    pub post_stable: PhaseData,
    pub delta_ec: f32,
    pub delta_ph: f32,
    pub target_ec: f32,
    pub target_ph: f32,
    pub error_ec: f32,
    pub error_ph: f32,
    pub duration_ms: u64,
    pub ema_ec_gain_used: f32,
    pub ema_ph_shift_used: f32,

    // 👇 BỔ SUNG: Dữ liệu Step Ratio từ thuật toán Auto-Tune
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_ratio_ec: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_ratio_ph: Option<f32>,

    // Giữ lại trường này nếu bạn cần tính toán cửa sổ ổn định ở hàm dynamic_learning
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stabilized_window_sec: Option<u32>,
}

/// Cấp độ nghiêm trọng của Log
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Critical,
}

/// Danh mục hệ thống để Frontend dễ dàng filter
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LogCategory {
    System,      // Khởi động, kết nối mạng, chuyển đổi FSM
    Dosing,      // Châm phân, chỉnh pH
    Water,       // Cấp/xả nước
    Calibration, // Hiệu chuẩn cảm biến, cập nhật EMA, Auto-tune
    Sensor,      // Tín hiệu cảm biến (nhiễu, lỗi)
    Alert,       // Cảnh báo an toàn (rate limit, emergency)
    UserAction,  // Can thiệp thủ công từ App/Web
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

impl LogCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dosing => "dosing",
            Self::Water => "water",
            Self::Calibration => "calibration",
            Self::Sensor => "sensor",
            Self::Alert => "alert",
            Self::UserAction => "user_action",
        }
    }
}

/// Metadata cho các sự kiện Cấp/Xả nước
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaterMetadata {
    pub source: String,
    pub trigger: String, // VD: "auto_refill", "scheduled_change", "dilute"
    pub level_before: f32,
    pub level_after: f32,
    pub target_level: f32,
    pub duration_sec: u64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u32>,
}

/// Metadata cho các cảnh báo An toàn / Lỗi hệ thống
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertMetadata {
    pub alert_type: String, // VD: "rate_limit", "hardware_fault", "sensor_noise"
    pub source: String,     // VD: "ec_dosing", "drain_pump", "sensor_ec"
    pub retry_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_value: Option<f32>, // Dùng nếu vượt ngưỡng (VD: max_hourly_ml)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_before: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_after: Option<f32>,
}

/// Metadata cho các sự kiện AI Learning & Calibration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationMetadata {
    pub source: String,
    pub parameter: String, // VD: "ec_gain_per_ml", "ec_step_ratio", "skipped"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_value: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>, // VD: "noise", "short_mixing"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicSystemLogMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycle_id: Option<String>,
    pub source: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

// Đối với Dosing, bạn đã có sẵn `DosingReportPayload` ở Backend,
// ta có thể tái sử dụng nó hoặc bọc lại bằng một struct thống nhất.

/// Giao thức chung giao tiếp giữa Firmware và Backend cho System Log
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")] // Tự động chèn field "event_type": "WaterEvent" vào JSON
pub enum SystemLogEvent {
    WaterEvent(WaterMetadata),

    SystemAlert(AlertMetadata),

    CalibrationUpdate(CalibrationMetadata),

    /// Dành cho các log text cơ bản không cần metadata phức tạp
    BasicSystemLog(BasicSystemLogMetadata),
}

/// Struct cuối cùng gói toàn bộ thông tin để lưu vào DB / bắn Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSystemLog {
    pub device_id: String,
    pub level: LogLevel,
    pub category: LogCategory,
    pub title: String,
    pub event: SystemLogEvent, // Chứa cả event_type và metadata chi tiết
    pub timestamp_ms: u64,
}

impl UnifiedSystemLog {
    pub fn mqtt_topic_suffix() -> &'static str {
        "system_log"
    }

    pub fn info(
        device_id: impl Into<String>,
        category: LogCategory,
        title: impl Into<String>,
        event: SystemLogEvent,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            device_id: device_id.into(),
            level: LogLevel::Info,
            category,
            title: title.into(),
            event,
            timestamp_ms,
        }
    }

    pub fn warning(
        device_id: impl Into<String>,
        category: LogCategory,
        title: impl Into<String>,
        event: SystemLogEvent,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            device_id: device_id.into(),
            level: LogLevel::Warning,
            category,
            title: title.into(),
            event,
            timestamp_ms,
        }
    }

    pub fn critical(
        device_id: impl Into<String>,
        category: LogCategory,
        title: impl Into<String>,
        event: SystemLogEvent,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            device_id: device_id.into(),
            level: LogLevel::Critical,
            category,
            title: title.into(),
            event,
            timestamp_ms,
        }
    }

    pub fn success(
        device_id: impl Into<String>,
        category: LogCategory,
        title: impl Into<String>,
        event: SystemLogEvent,
        timestamp_ms: u64,
    ) -> Self {
        Self {
            device_id: device_id.into(),
            level: LogLevel::Success,
            category,
            title: title.into(),
            event,
            timestamp_ms,
        }
    }

    pub fn build_basic_log_json(
        device_id: impl Into<String>,
        level: LogLevel,
        category: LogCategory,
        title: impl Into<String>,
        message: impl Into<String>,
        cycle_id: Option<&str>,
        source: impl Into<String>,
    ) -> String {
        let event = SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
            source: source.into(),
            message: message.into(),
            skip_reason: None,
            cycle_id: cycle_id.map(|s| s.to_string()),
        });

        // Dùng current timestamp nếu không truyền vào.
        // Trên embedded (no_std-lite) không có SystemTime, nên timestamp = 0 là chấp nhận được;
        // backend sẽ ghi đè bằng NOW() khi insert DB.
        // Firmware nên truyền now_ms vào — xem overload bên dưới.
        let log = match level {
            LogLevel::Info => Self::info(device_id, category, title, event, 0),
            LogLevel::Warning => Self::warning(device_id, category, title, event, 0),
            LogLevel::Critical => Self::critical(device_id, category, title, event, 0),
            LogLevel::Success => Self::success(device_id, category, title, event, 0),
        };

        serde_json::to_string(&log).unwrap_or_else(|_| "{}".to_string())
    }

    /// Overload với timestamp_ms — dùng trong firmware có đồng hồ.
    pub fn build_basic_log_json_with_ts(
        device_id: impl Into<String>,
        level: LogLevel,
        category: LogCategory,
        title: impl Into<String>,
        message: impl Into<String>,
        cycle_id: Option<&str>,
        source: impl Into<String>,
        timestamp_ms: u64,
    ) -> String {
        let event = SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
            source: source.into(),
            message: message.into(),
            skip_reason: None,
            cycle_id: cycle_id.map(|s| s.to_string()),
        });

        let log = match level {
            LogLevel::Info => Self::info(device_id, category, title, event, timestamp_ms),
            LogLevel::Warning => Self::warning(device_id, category, title, event, timestamp_ms),
            LogLevel::Critical => Self::critical(device_id, category, title, event, timestamp_ms),
            LogLevel::Success => Self::success(device_id, category, title, event, timestamp_ms),
        };

        serde_json::to_string(&log).unwrap_or_else(|_| "{}".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerHealthPayload {
    pub free_heap: u32,
    pub uptime_sec: u64,
    pub rssi: i8,
    pub pump_status: PumpStatus,
}

impl ControllerConfig {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.ec_target <= 0.0 || self.ec_target > 10.0 {
            errors.push("ec_target phải trong khoảng (0, 10]".into());
        }
        if self.ec_tolerance < 0.0 || self.ec_tolerance >= self.ec_target {
            errors.push("ec_tolerance phải >= 0 và < ec_target".into());
        }
        if self.ph_target < 0.0 || self.ph_target > 14.0 {
            errors.push("ph_target phải trong khoảng [0, 14]".into());
        }
        if self.dosing_pwm_percent < 1 || self.dosing_pwm_percent > 100 {
            errors.push("dosing_pwm_percent phải trong [1, 100]".into());
        }
        if self.dosing_min_pwm_percent > self.dosing_pwm_percent {
            errors.push("dosing_min_pwm_percent không được vượt dosing_pwm_percent".into());
        }
        if self.pump_a_capacity_ml_per_sec <= 0.0 {
            errors.push("pump_a_capacity_ml_per_sec phải > 0".into());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl DosingReportPayload {
    pub fn to_metadata_json(&self) -> serde_json::Value {
        let mut v = serde_json::to_value(self).unwrap_or_default();
        if let Some(obj) = v.as_object_mut() {
            obj.insert(
                "event_type".into(),
                serde_json::Value::String("dosing_cycle".into()),
            );
        }
        v
    }
}
