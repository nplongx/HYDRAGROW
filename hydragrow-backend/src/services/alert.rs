// use crate::AppState;
// use crate::models::{config::SafetyConfig, sensor::SensorData};
// use anyhow::Result;
// use chrono::Utc;
// use std::sync::Arc;
// use tracing::{error, instrument, warn};
//
// #[instrument(skip(app_state, sensor_data))]
// pub async fn check_and_trigger_alerts(
//     app_state: &AppState,
//     sensor_data: &SensorData,
// ) -> Result<()> {
//     let device_id = &sensor_data.device_id;
//
//     // 1. Lấy Safety Config từ SQLite
//     // Lưu ý: Trong thực tế, bạn có thể cache cái này trong bộ nhớ (DashMap) để giảm tải DB
//     // Nhưng với SQLite (file local) và hệ thống single-user, query trực tiếp vẫn cực kỳ nhanh.
//     let safety_config = sqlx::query_as!(
//         SafetyConfig,
//         r#"
//         SELECT
//             device_id, max_ec_limit, min_ec_limit, min_ph_limit, max_ph_limit, max_ec_delta, max_ph_delta,
//             max_dose_per_cycle, cooldown_sec, max_dose_per_hour, water_level_critical_min,
//             max_refill_cycles_per_hour, max_drain_cycles_per_hour, max_refill_duration_sec,
//             max_drain_duration_sec, min_temp_limit, max_temp_limit, emergency_shutdown,
//             ec_ack_threshold, ph_ack_threshold, water_ack_threshold, -- 🟢 Thêm 3 trường ACK để khớp với struct
//             last_updated
//         FROM safety_config
//         WHERE device_id = ?
//         "#,
//         device_id
//     )
//     .fetch_optional(&app_state.sqlite_pool)
//     .await?;
//
//     let config = match safety_config {
//         Some(c) => c,
//         None => return Ok(()), // Chưa có cấu hình thì bỏ qua kiểm tra
//     };
//
//     // 2. Kiểm tra các điều kiện an toàn & Gửi cảnh báo
//     let mut alerts = Vec::new();
//
//     // -- Kiểm tra EC --
//     if sensor_data.ec_value > config.max_ec_limit {
//         alerts.push(AlertMessage {
//             alert_type: "safety_breach".to_string(),
//             device_id: device_id.clone(),
//             metric: "ec".to_string(),
//             value: sensor_data.ec_value,
//             severity: "critical".to_string(),
//             timestamp: Utc::now().to_string(),
//         });
//     }
//
//     // -- Kiểm tra pH --
//     if sensor_data.ph_value < config.min_ph_limit {
//         alerts.push(AlertMessage {
//             alert_type: "safety_breach".to_string(),
//             device_id: device_id.clone(),
//             metric: "ph".to_string(),
//             value: sensor_data.ph_value,
//             severity: "critical".to_string(),
//             timestamp: Utc::now().to_string(),
//         });
//     } else if sensor_data.ph_value > config.max_ph_limit {
//         alerts.push(AlertMessage {
//             alert_type: "safety_breach".to_string(),
//             device_id: device_id.clone(),
//             metric: "ph".to_string(),
//             value: sensor_data.ph_value,
//             severity: "critical".to_string(),
//             timestamp: Utc::now().to_string(),
//         });
//     }
//
//     // 3. Broadcast các cảnh báo qua WebSocket
//     for alert in alerts {
//         warn!(
//             device_id = %alert.device_id,
//             metric = %alert.metric,
//             value = %alert.value,
//             "Safety limit breached! Triggering alert."
//         );
//
//         // Gửi qua channel. Bỏ qua lỗi nếu không có client nào đang kết nối (WS trống)
//         if let Ok(json_str) = serde_json::to_string(&alert) {
//             let _ = app_state.alert_sender.send(json_str);
//         }
//
//         // NẾU vi phạm quá nghiêm trọng (hoặc emergency_shutdown == 1), ra lệnh ngắt bơm
//         if config.emergency_shutdown == 1 {
//             if let Err(e) =
//                 crate::services::command::trigger_emergency_stop(app_state, device_id).await
//             {
//                 error!("Failed to trigger emergency stop: {}", e);
//             }
//         }
//     }
//
//     Ok(())
// }
//
// pub fn check_alerts(sensor: &SensorData, safety: &SafetyConfig) -> Vec<AlertMessage> {
//     let mut alerts = Vec::new();
//
//     if sensor.ec_value > safety.max_ec_limit {
//         alerts.push(AlertMessage {
//             alert_type: "EC_HIGH".into(),
//             device_id: sensor.device_id.clone(),
//             metric: "ec".into(),
//             value: sensor.ec_value,
//             severity: "critical".into(),
//             timestamp: chrono::Utc::now().to_rfc3339(),
//         });
//     }
//
//     if sensor.ph_value < safety.min_ph_limit {
//         alerts.push(AlertMessage {
//             alert_type: "PH_LOW".into(),
//             device_id: sensor.device_id.clone(),
//             metric: "ph".into(),
//             value: sensor.ph_value,
//             severity: "warning".into(),
//             timestamp: chrono::Utc::now().to_rfc3339(),
//         });
//     }
//
//     if sensor.ph_value > safety.max_ph_limit {
//         alerts.push(AlertMessage {
//             alert_type: "PH_HIGH".into(),
//             device_id: sensor.device_id.clone(),
//             metric: "ph".into(),
//             value: sensor.ph_value,
//             severity: "warning".into(),
//             timestamp: chrono::Utc::now().to_rfc3339(),
//         });
//     }
//
//     alerts
// }
