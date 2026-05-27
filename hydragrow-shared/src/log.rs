use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    mpsc::Sender,
};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;

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

    fn from_field(value: &str) -> Option<Self> {
        match value.trim_matches('"').to_ascii_lowercase().as_str() {
            "info" => Some(Self::Info),
            "success" => Some(Self::Success),
            "warning" | "warn" => Some(Self::Warning),
            "critical" | "error" => Some(Self::Critical),
            _ => None,
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

    fn from_field(value: &str) -> Option<Self> {
        match value.trim_matches('"').to_ascii_lowercase().as_str() {
            "system" => Some(Self::System),
            "dosing" => Some(Self::Dosing),
            "water" => Some(Self::Water),
            "calibration" => Some(Self::Calibration),
            "sensor" => Some(Self::Sensor),
            "alert" => Some(Self::Alert),
            "user_action" => Some(Self::UserAction),
            _ => None,
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

pub struct SystemLogRecord<'a> {
    pub device_id: &'a str,
    pub level: LogLevel,
    pub category: LogCategory,
    pub title: &'a str,
    pub source: &'a str,
    pub message: &'a str,
    pub cycle_id: Option<&'a str>,
    pub timestamp_ms: u64,
}

pub struct SystemLogLayer {
    tx: Sender<String>,
    drop_count: &'static AtomicU32,
}

#[derive(Default)]
struct SystemLogVisitor {
    device_id: Option<String>,
    level: Option<String>,
    category: Option<String>,
    title: Option<String>,
    source: Option<String>,
    message: Option<String>,
    cycle_id: Option<String>,
    timestamp_ms: Option<u64>,
    payload_json: Option<String>,
}

impl SystemLogLayer {
    pub fn new(tx: Sender<String>, drop_count: &'static AtomicU32) -> Self {
        Self { tx, drop_count }
    }

    fn publish_json(&self, json: String) {
        if self.tx.send(json).is_err() {
            let previous = self.drop_count.fetch_add(1, Ordering::Relaxed);
            if previous % 10 == 0 {
                tracing::warn!(
                    target: "hydragrow.system_log.layer",
                    dropped_logs = previous + 1,
                    "MQTT system log channel is full or closed"
                );
            }
        }
    }

    fn publish_visitor(&self, visitor: SystemLogVisitor) {
        if let Some(payload_json) = visitor.payload_json {
            self.publish_json(payload_json);
            return;
        }

        let Some(device_id) = visitor.device_id else {
            tracing::debug!(target: "hydragrow.system_log.layer", "missing device_id field");
            return;
        };
        let Some(level) = visitor.level.and_then(|value| LogLevel::from_field(&value)) else {
            tracing::debug!(target: "hydragrow.system_log.layer", "missing or invalid log_level field");
            return;
        };
        let Some(category) = visitor
            .category
            .and_then(|value| LogCategory::from_field(&value))
        else {
            tracing::debug!(target: "hydragrow.system_log.layer", "missing or invalid category field");
            return;
        };
        let Some(title) = visitor.title else {
            tracing::debug!(target: "hydragrow.system_log.layer", "missing title field");
            return;
        };
        let Some(source) = visitor.source else {
            tracing::debug!(target: "hydragrow.system_log.layer", "missing source field");
            return;
        };
        let Some(message) = visitor.message else {
            tracing::debug!(target: "hydragrow.system_log.layer", "missing message field");
            return;
        };

        let event = SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
            cycle_id: visitor.cycle_id,
            source,
            message,
            skip_reason: None,
        });
        let timestamp_ms = visitor.timestamp_ms.unwrap_or(0);
        let log = build_unified_system_log(device_id, level, category, title, event, timestamp_ms);

        match serde_json::to_string(&log) {
            Ok(json) => self.publish_json(json),
            Err(error) => tracing::error!(
                target: "hydragrow.system_log.layer",
                ?error,
                "failed to serialize system log"
            ),
        }
    }
}

impl<S> Layer<S> for SystemLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if event.metadata().target() != "hydragrow.system_log" {
            return;
        }

        let mut visitor = SystemLogVisitor::default();
        event.record(&mut visitor);
        self.publish_visitor(visitor);
    }
}

impl Visit for SystemLogVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_field(field.name(), value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let mut rendered = format!("{value:?}");
        if rendered.len() >= 2 && rendered.starts_with('"') && rendered.ends_with('"') {
            rendered = rendered[1..rendered.len() - 1].to_string();
        }
        self.record_field(field.name(), rendered);
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if field.name() == "timestamp_ms" && value >= 0 {
            self.timestamp_ms = Some(value as u64);
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == "timestamp_ms" {
            self.timestamp_ms = Some(value);
        }
    }
}

impl SystemLogVisitor {
    fn record_field(&mut self, name: &str, value: String) {
        match name {
            "device_id" => self.device_id = Some(value),
            "log_level" | "level" => self.level = Some(value),
            "category" => self.category = Some(value),
            "title" => self.title = Some(value),
            "source" => self.source = Some(value),
            "message" => self.message = Some(value),
            "cycle_id" if !value.is_empty() => self.cycle_id = Some(value),
            "payload_json" => self.payload_json = Some(value),
            _ => {}
        }
    }
}

pub fn emit_basic_system_log(record: SystemLogRecord<'_>) {
    tracing::event!(
        target: "hydragrow.system_log",
        tracing::Level::INFO,
        device_id = record.device_id,
        log_level = record.level.as_str(),
        category = record.category.as_str(),
        title = record.title,
        source = record.source,
        message = record.message,
        cycle_id = record.cycle_id.unwrap_or(""),
        timestamp_ms = record.timestamp_ms,
        "system log"
    );
}

pub fn emit_system_log_event(
    device_id: &str,
    level: LogLevel,
    category: LogCategory,
    title: &str,
    event: SystemLogEvent,
    timestamp_ms: u64,
) {
    match event {
        SystemLogEvent::BasicSystemLog(metadata) => {
            emit_basic_system_log(SystemLogRecord {
                device_id,
                level,
                category,
                title,
                source: &metadata.source,
                message: &metadata.message,
                cycle_id: metadata.cycle_id.as_deref(),
                timestamp_ms,
            });
        }
        event => {
            let log = build_unified_system_log(
                device_id.to_string(),
                level,
                category,
                title.to_string(),
                event,
                timestamp_ms,
            );
            match serde_json::to_string(&log) {
                Ok(payload_json) => emit_system_log_json(&payload_json),
                Err(error) => tracing::error!(
                    target: "hydragrow.system_log.layer",
                    ?error,
                    "failed to serialize system log event before tracing"
                ),
            }
        }
    }
}

pub fn emit_system_log_json(payload_json: &str) {
    tracing::event!(
        target: "hydragrow.system_log",
        tracing::Level::INFO,
        payload_json,
        "system log payload"
    );
}

fn build_unified_system_log(
    device_id: String,
    level: LogLevel,
    category: LogCategory,
    title: String,
    event: SystemLogEvent,
    timestamp_ms: u64,
) -> UnifiedSystemLog {
    match level {
        LogLevel::Info => UnifiedSystemLog::info(device_id, category, title, event, timestamp_ms),
        LogLevel::Warning => {
            UnifiedSystemLog::warning(device_id, category, title, event, timestamp_ms)
        }
        LogLevel::Critical => {
            UnifiedSystemLog::critical(device_id, category, title, event, timestamp_ms)
        }
        LogLevel::Success => {
            UnifiedSystemLog::success(device_id, category, title, event, timestamp_ms)
        }
    }
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

#[cfg(test)]
mod system_log_layer_tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::mpsc;
    use tracing_subscriber::prelude::*;

    #[test]
    fn system_log_layer_publishes_basic_event_from_tracing_fields() {
        let (tx, rx) = mpsc::channel();
        let drop_count = Box::leak(Box::new(AtomicU32::new(0)));
        let subscriber = tracing_subscriber::registry().with(SystemLogLayer::new(tx, drop_count));

        tracing::subscriber::with_default(subscriber, || {
            emit_basic_system_log(SystemLogRecord {
                device_id: "device-1",
                level: LogLevel::Warning,
                category: LogCategory::UserAction,
                title: "Safety Timeout",
                source: "fsm_command",
                message: "Pump stopped",
                cycle_id: Some("cycle-7"),
                timestamp_ms: 1234,
            });
        });

        let payload = rx.recv().expect("system log payload");
        let decoded: UnifiedSystemLog = serde_json::from_str(&payload).expect("valid json");
        assert_eq!(decoded.device_id, "device-1");
        assert_eq!(decoded.level, LogLevel::Warning);
        assert_eq!(decoded.category, LogCategory::UserAction);
        assert_eq!(decoded.title, "Safety Timeout");
        assert_eq!(decoded.timestamp_ms, 1234);
        match decoded.event {
            SystemLogEvent::BasicSystemLog(metadata) => {
                assert_eq!(metadata.source, "fsm_command");
                assert_eq!(metadata.message, "Pump stopped");
                assert_eq!(metadata.cycle_id.as_deref(), Some("cycle-7"));
            }
            _ => panic!("expected basic system log"),
        }
    }

    #[test]
    fn system_log_layer_ignores_unrelated_targets() {
        let (tx, rx) = mpsc::channel();
        let drop_count = Box::leak(Box::new(AtomicU32::new(0)));
        let subscriber = tracing_subscriber::registry().with(SystemLogLayer::new(tx, drop_count));

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(
                target: "hydragrow.other",
                device_id = "device-1",
                log_level = "warning",
                category = "user_action",
                title = "Safety Timeout",
                source = "fsm_command",
                message = "Pump stopped",
                timestamp_ms = 1234_u64,
                "system log"
            );
        });

        assert!(rx.try_recv().is_err());
        assert_eq!(drop_count.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn system_log_layer_increments_drop_count_when_channel_is_closed() {
        let (tx, rx) = mpsc::channel::<String>();
        drop(rx);
        let drop_count = Box::leak(Box::new(AtomicU32::new(0)));
        let subscriber = tracing_subscriber::registry().with(SystemLogLayer::new(tx, drop_count));

        tracing::subscriber::with_default(subscriber, || {
            emit_basic_system_log(SystemLogRecord {
                device_id: "device-1",
                level: LogLevel::Info,
                category: LogCategory::System,
                title: "Dropped",
                source: "test",
                message: "closed",
                cycle_id: None,
                timestamp_ms: 1,
            });
        });

        assert_eq!(drop_count.load(Ordering::Relaxed), 1);
    }
}
