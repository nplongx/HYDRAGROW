// use hydragrow_shared::DosingReportPayload;
// use serde::{Deserialize, Serialize};
//
// /// Cấp độ nghiêm trọng của Log
// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// #[serde(rename_all = "snake_case")]
// pub enum LogLevel {
//     Info,
//     Success,
//     Warning,
//     Critical,
// }
//
// /// Danh mục hệ thống để Frontend dễ dàng filter
// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// #[serde(rename_all = "snake_case")]
// pub enum LogCategory {
//     System,      // Khởi động, kết nối mạng, chuyển đổi FSM
//     Dosing,      // Châm phân, chỉnh pH
//     Water,       // Cấp/xả nước
//     Calibration, // Hiệu chuẩn cảm biến, cập nhật EMA, Auto-tune
//     Sensor,      // Tín hiệu cảm biến (nhiễu, lỗi)
//     Alert,       // Cảnh báo an toàn (rate limit, emergency)
//     UserAction,  // Can thiệp thủ công từ App/Web
// }
//
// /// Metadata cho các sự kiện Cấp/Xả nước
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct WaterMetadata {
//     pub trigger: String, // VD: "auto_refill", "scheduled_change", "dilute"
//     pub level_before: f32,
//     pub level_after: f32,
//     pub target_level: f32,
//     pub duration_sec: u64,
//     pub success: bool,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub cycle_id: Option<String>,
// }
//
// /// Metadata cho các cảnh báo An toàn / Lỗi hệ thống
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct AlertMetadata {
//     pub alert_type: String, // VD: "rate_limit", "hardware_fault", "sensor_noise"
//     pub source: String,     // VD: "ec_dosing", "drain_pump", "sensor_ec"
//     pub retry_count: u32,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub limit_value: Option<f32>, // Dùng nếu vượt ngưỡng (VD: max_hourly_ml)
// }
//
// /// Metadata cho các sự kiện AI Learning & Calibration
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct CalibrationMetadata {
//     pub parameter: String, // VD: "ec_gain_per_ml", "ec_step_ratio", "skipped"
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub old_value: Option<f32>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub new_value: Option<f32>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub skip_reason: Option<String>, // VD: "noise", "short_mixing"
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub cycle_id: Option<String>,
// }
//
// // Đối với Dosing, bạn đã có sẵn `DosingReportPayload` ở Backend,
// // ta có thể tái sử dụng nó hoặc bọc lại bằng một struct thống nhất.
//
// /// Giao thức chung giao tiếp giữa Firmware và Backend cho System Log
// #[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(tag = "event_type")] // Tự động chèn field "event_type": "WaterEvent" vào JSON
// pub enum SystemLogEvent {
//     WaterEvent(WaterMetadata),
//
//     // Sử dụng lại struct DosingReportPayload bạn đã định nghĩa
//     DosingCycleComplete(DosingReportPayload),
//
//     SystemAlert(AlertMetadata),
//
//     CalibrationUpdate(CalibrationMetadata),
//
//     /// Dành cho các log text cơ bản không cần metadata phức tạp
//     BasicSystemLog {
//         message: String,
//     },
// }
//
// /// Struct cuối cùng gói toàn bộ thông tin để lưu vào DB / bắn Alert
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct UnifiedSystemLog {
//     pub device_id: String,
//     pub level: LogLevel,
//     pub category: LogCategory,
//     pub title: String,
//     pub event: SystemLogEvent, // Chứa cả event_type và metadata chi tiết
//     pub timestamp_ms: u64,
// }

