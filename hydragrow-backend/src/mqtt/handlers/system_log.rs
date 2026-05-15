use actix_web::web;
use hydragrow_shared::{LogLevel, SystemLogEvent, UnifiedSystemLog};
use tracing::{error, info, warn};

use crate::AppState;
use crate::db::postgres::{NewSystemEventRecord, insert_system_event};
use crate::models::alert::AlertMessage;

pub async fn handle(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    // 1. Deserialize trực tiếp từ JSON sang Struct
    let log_data: UnifiedSystemLog = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!("❌ [SYSTEM LOG] Lỗi Parse JSON từ {}: {:?}", device_id, e);
            return;
        }
    };

    let level_str = serde_json::to_value(&log_data.level)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    let category_str = serde_json::to_value(&log_data.category)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    // Dịch nghĩa event thành message chi tiết thay vì hardcode
    let message_str = match &log_data.event {
        SystemLogEvent::BasicSystemLog { message } => message.clone(),
        SystemLogEvent::SystemAlert(meta) => {
            format!("Nguồn: {} (Thử lại: {})", meta.source, meta.retry_count)
        }
        SystemLogEvent::CalibrationUpdate(meta) => {
            if let Some(reason) = &meta.skip_reason {
                format!("Bỏ qua cập nhật: {}", reason)
            } else {
                format!("Đã cập nhật hệ số: {}", meta.parameter)
            }
        }
        SystemLogEvent::WaterEvent(meta) => format!(
            "Mực nước: {:.1} -> {:.1}",
            meta.level_before, meta.level_after
        ),
        SystemLogEvent::DosingCycleComplete(_) => "Hoàn tất chu kỳ châm phân".to_string(),
        _ => log_data.title.clone(), // Fallback: Dùng luôn tiêu đề nếu không khớp loại nào
    };

    // 2. Chuyển đổi thành Record để lưu Database
    let db_record = NewSystemEventRecord {
        device_id: log_data.device_id.clone(),
        level: level_str.clone(),
        category: category_str.clone(),
        title: log_data.title.clone(),
        message: message_str.clone(),
        reason: None,
        metadata: Some(serde_json::to_value(&log_data.event).unwrap()),
        timestamp: log_data.timestamp_ms as i64,
    };

    // 3. Lưu vào Postgres
    if let Err(e) = insert_system_event(&app_state.pg_pool, &db_record).await {
        error!("❌ [SYSTEM LOG] Lỗi lưu Database: {:?}", e);
    }

    // 4. Quyết định xem có bắn Push Notification / Toast Alert lên App không
    let is_critical = log_data.level == LogLevel::Critical;
    let is_warning = log_data.level == LogLevel::Warning;
    let is_dosing_done = matches!(log_data.event, SystemLogEvent::DosingCycleComplete(_));

    if is_critical || is_warning || is_dosing_done {
        info!("🚨 Gửi Alert tới UI: [{}] {}", device_id, log_data.title);

        let alert = AlertMessage {
            level: level_str.clone(),
            category: category_str.clone(),
            title: log_data.title.clone(),
            message: message_str.clone(),
            device_id: log_data.device_id.clone(),
            timestamp: log_data.timestamp_ms,
            reason: None,
            metadata: Some(serde_json::to_value(&log_data.event).unwrap()),
        };

        let _ = app_state.alert_sender.send(alert.clone());

        // Push FCM cho lỗi nghiêm trọng
        if is_critical || is_warning {
            let tokens = app_state.fcm_tokens.lock().unwrap().clone();
            if !tokens.is_empty() {
                tokio::spawn(async move {
                    crate::services::fcm::send_push_notification(
                        &alert.title,
                        &alert.message,
                        tokens,
                    )
                    .await;
                });
            }
        }
    }
}
