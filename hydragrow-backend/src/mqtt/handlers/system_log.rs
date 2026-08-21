// src/mqtt/handlers/system_log.rs
use crate::AppState;
use crate::db::postgres::{NewSystemEventRecord, insert_system_event};
use crate::models::alert::AlertMessage;
use actix_web::web;
use hydragrow_shared::events::AppEvent;
use hydragrow_shared::log::{LogCategory, LogLevel, SystemLogEvent, UnifiedSystemLog};
use tracing::{error, info, warn};

pub async fn handle(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let log_data: UnifiedSystemLog = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            let raw_preview = std::str::from_utf8(payload)
                .map(|s| &s[..s.len().min(200)])
                .unwrap_or("<invalid utf8>");
            error!(
                target: "esp32_device",
                device_id = %device_id,
                "Lỗi parse UnifiedSystemLog: {:?}. Raw: {}", e, raw_preview
            );
            return;
        }
    };

    let level_str = log_data.level.as_str().to_string();
    let category_str = log_data.category.as_str().to_string();
    let message_str = match &log_data.event {
        SystemLogEvent::BasicSystemLog(meta) => {
            if let Some(reason) = &meta.skip_reason {
                warn!(
                    target: "esp32_device",
                    device_id = %log_data.device_id,
                    cycle_id = ?meta.cycle_id,
                    skip_reason = %reason,
                    "Firmware báo skip cycle: {}", reason
                );
            }
            meta.message.clone()
        }
        SystemLogEvent::SystemAlert(meta) => {
            format!("Nguồn: {} (Thử lại: {})", meta.source, meta.retry_count)
        }
        SystemLogEvent::CalibrationUpdate(meta) => {
            if let Some(reason) = &meta.skip_reason {
                format!("Bỏ qua cập nhật: {}", reason)
            } else {
                format!("Cập nhật: {}", meta.parameter)
            }
        }
        SystemLogEvent::WaterEvent(meta) => format!(
            "Mức nước: {:.1} -> {:.1}",
            meta.level_before, meta.level_after
        ),
        SystemLogEvent::RecipeApplied(meta) => match &meta.stage_name {
            Some(stage_name) => format!(
                "Áp dụng recipe '{}' - stage '{}' từ {}",
                meta.recipe_name, stage_name, meta.source
            ),
            None => format!("Áp dụng recipe '{}' từ {}", meta.recipe_name, meta.source),
        },
        SystemLogEvent::RecipeRejected(meta) => {
            warn!(
                target: "esp32_device",
                device_id = %log_data.device_id,
                recipe_id = %meta.recipe_id,
                cycle_id = ?meta.cycle_id,
                reason = %meta.reason,
                "Recipe bị từ chối: {}", meta.reason
            );
            match &meta.recipe_name {
                Some(recipe_name) => format!(
                    "Từ chối recipe '{}' từ {}: {}",
                    recipe_name, meta.source, meta.reason
                ),
                None => format!(
                    "Từ chối recipe {} từ {}: {}",
                    meta.recipe_id, meta.source, meta.reason
                ),
            }
        }
        SystemLogEvent::RecipeStageChanged(meta) => match &meta.from_stage_name {
            Some(from_stage_name) => format!(
                "Recipe '{}' chuyển stage '{}' -> '{}'",
                meta.recipe_name, from_stage_name, meta.to_stage_name
            ),
            None => format!(
                "Recipe '{}' bắt đầu stage '{}'",
                meta.recipe_name, meta.to_stage_name
            ),
        },
        SystemLogEvent::RecipeCompleted(meta) => match &meta.final_stage_name {
            Some(final_stage_name) => format!(
                "Recipe '{}' hoàn tất tại stage '{}'",
                meta.recipe_name, final_stage_name
            ),
            None => format!("Recipe '{}' hoàn tất", meta.recipe_name),
        },
    };

    // =========================================================================
    // 🟢 BẮN LOG CẤU TRÚC SANG LOKI (Qua Tracing Layer)
    // =========================================================================
    match log_data.level {
        LogLevel::Critical => {
            error!(
                target: "hydragrow_backend::esp32_device",
                device_id = %log_data.device_id,
                category = %category_str,
                title = %log_data.title,
                "[{}] {}", log_data.title, message_str
            );
        }
        LogLevel::Warning => {
            warn!(
                target: "hydragrow_backend::esp32_device",
                device_id = %log_data.device_id,
                category = %category_str,
                title = %log_data.title,
                "[{}] {}", log_data.title, message_str
            );
        }
        _ => {
            info!(
                target: "hydragrow_backend::esp32_device",
                device_id = %log_data.device_id,
                category = %category_str,
                title = %log_data.title,
                "[{}] {}", log_data.title, message_str
            );
        }
    }
    // 2. Lưu vào CSDL PostgreSQL
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

    if let Err(e) = insert_system_event(&app_state.pg_pool, &db_record).await {
        error!("Lưu Database thất bại: {:?}", e);
    }

    // 3. Đẩy thông báo thời gian thực qua WebSocket & FCM
    let is_critical = log_data.level == LogLevel::Critical;
    let is_warning = log_data.level == LogLevel::Warning;
    let is_success = log_data.level == LogLevel::Success;
    let category_is_priority = matches!(
        log_data.category,
        LogCategory::Dosing | LogCategory::Water | LogCategory::Alert
    );
    let recipe_event_needs_notification = matches!(
        &log_data.event,
        SystemLogEvent::RecipeApplied(_)
            | SystemLogEvent::RecipeRejected(_)
            | SystemLogEvent::RecipeStageChanged(_)
            | SystemLogEvent::RecipeCompleted(_)
    );

    if is_critical
        || is_warning
        || is_success
        || category_is_priority
        || recipe_event_needs_notification
    {
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
        let _ = app_state
            .event_bus
            .send(AppEvent::SystemAlert(alert.clone()));

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
