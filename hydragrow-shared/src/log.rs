use serde::{Deserialize, Serialize};

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
