use actix_web::web;
use hydragrow_shared::{LogLevel, SystemLogEvent, UnifiedSystemLog};
use tracing::{error, info, warn};

use crate::AppState;
use crate::db::postgres::{NewSystemEventRecord, insert_system_event};
use crate::models::alert::AlertMessage;

pub async fn handle(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    // 1. Deserialize trực tiếp từ JSON sang Struct (Nhanh và không bao giờ sai lệch)
    let log_data: UnifiedSystemLog = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!("❌ [SYSTEM LOG] Lỗi Parse JSON từ {}: {:?}", device_id, e);
            // Có thể log thêm payload gốc ra để debug nếu cần
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

    // 2. Chuyển đổi thành Record để lưu Database
    // Vì log_data.event là Enum, serde_json::to_value sẽ tự động map ra JSONB tuyệt đẹp
    let db_record = NewSystemEventRecord {
        device_id: log_data.device_id.clone(),
        level: level_str,
        category: category_str,
        title: log_data.title.clone(),
        message: "Log từ thiết bị".to_string(), // Tùy bạn custom (hoặc thêm field msg vào struct)
        reason: None, // Nếu bạn muốn bóc tách skip_reason ra cột riêng thì lấy ở đây
        metadata: Some(serde_json::to_value(&log_data.event).unwrap()),
        timestamp: log_data.timestamp_ms as i64,
    };

    // 3. Lưu vào Postgres
    if let Err(e) = insert_system_event(&app_state.pg_pool, &db_record).await {
        error!("❌ [SYSTEM LOG] Lỗi lưu Database: {:?}", e);
    }

    // 4. Quyết định xem có bắn Push Notification / Toast Alert lên App không
    // Chỉ bắn Alert với các sự kiện Quan trọng (Critical, Warning) hoặc DosingCycleComplete
    let is_critical = log_data.level == LogLevel::Critical;
    let is_warning = log_data.level == LogLevel::Warning;

    // Bạn có thể check cụ thể loại event:
    let is_dosing_done = matches!(log_data.event, SystemLogEvent::DosingCycleComplete(_));

    if is_critical || is_warning || is_dosing_done {
        info!("🚨 Gửi Alert tới UI: [{}] {}", device_id, log_data.title);

        let alert = AlertMessage {
            level: format!("{:?}", log_data.level).to_lowercase(),
            category: format!("{:?}", log_data.category).to_lowercase(),
            title: log_data.title.clone(),
            message: "Xem chi tiết trong lịch sử".to_string(),
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
                        &alert.message, // Truyền msg cụ thể nếu muốn
                        tokens,
                    )
                    .await;
                });
            }
        }
    }
}
