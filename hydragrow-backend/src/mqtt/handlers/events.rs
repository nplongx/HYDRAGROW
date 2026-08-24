// src/mqtt/handlers/events.rs
use crate::AppState;
use crate::db::postgres::{NewSystemEventRecord, insert_system_event};
use crate::models::alert::AlertMessage;
use actix_web::web;
use hydragrow_shared::events::AppEvent;
use serde_json::json;
use tracing::{error, info};

pub async fn handle(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let payload_json: serde_json::Value = match serde_json::from_slice(payload) {
        Ok(v) => v,
        Err(e) => {
            error!("  [MQTT-EVENTS] Lỗi parse JSON từ {}: {:?}", device_id, e);
            return;
        }
    };

    let event_type = payload_json
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("");
    match event_type {
        "system_alert" => handle_system_alert(device_id, payload_json, app_state).await,
        _ => {
            tracing::debug!("  [MQTT-EVENTS] Chưa xử lý event type: {}", event_type);
        }
    }
}

async fn handle_system_alert(
    device_id: String,
    json: serde_json::Value,
    app_state: web::Data<AppState>,
) {
    let level = json
        .get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("warning")
        .to_lowercase();
    let category = json
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("dosing")
        .to_string();
    let title = json
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Cảnh báo mức dung dịch")
        .to_string();
    let timestamp_ms = json
        .get("timestamp_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() as u64);
    let details = json.get("details").cloned();

    // 1. Tạo chuỗi thông báo thân thiện từ trạng thái các bình
    let mut empty_tanks = Vec::new();
    if let Some(ref d) = details {
        if d.get("tank_a_low")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            empty_tanks.push("Dinh dưỡng A");
        }
        if d.get("tank_b_low")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            empty_tanks.push("Dinh dưỡng B");
        }
        if d.get("tank_ph_down_low")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            empty_tanks.push("pH Down");
        }
        if d.get("tank_ph_up_low")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            empty_tanks.push("pH Up");
        }
    }

    let message = if !empty_tanks.is_empty() {
        format!(
            "Cạn dung dịch tại các bình: {}. Vui lòng bổ sung!",
            empty_tanks.join(", ")
        )
    } else {
        "Mức dung dịch tại các bình chứa đã ổn định trở lại.".to_string()
    };

    // 2. Lưu vào CSDL PostgreSQL (system_events)
    let record = NewSystemEventRecord {
        device_id: device_id.clone(),
        level: level.clone(),
        category: category.clone(),
        title: title.clone(),
        message: message.clone(),
        reason: Some("tank_level_alert".to_string()),
        metadata: details.clone(),
        timestamp: timestamp_ms as i64,
    };

    if let Err(e) = insert_system_event(&app_state.pg_pool, &record).await {
        error!("  [MQTT-EVENTS] Lỗi lưu PostgreSQL system_events: {:?}", e);
    }

    // 3. Cập nhật trạng thái vào in-memory cache device_states
    if let Some(ref d) = details {
        let mut states = app_state.device_states.write().await;
        let mut cached = states
            .get(&device_id)
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .unwrap_or_else(|| json!({ "device_id": device_id }));

        if let Some(obj) = cached.as_object_mut() {
            obj.insert("tank_alert".into(), d.clone());
        }
        if let Ok(s) = serde_json::to_string(&cached) {
            states.insert(device_id.clone(), s);
        }
    }

    // 4. Bắn sự kiện ra EventBus cho WebSocket kết nối tới App/Frontend
    let alert = AlertMessage {
        level: level.clone(),
        category,
        title: title.clone(),
        message: message.clone(),
        device_id: device_id.clone(),
        timestamp: timestamp_ms,
        reason: Some("tank_level_alert".to_string()),
        metadata: details,
    };
    let _ = app_state.event_bus.send(AppEvent::SystemAlert(alert));

    // 5. Gửi thông báo đẩy Firebase Cloud Messaging (FCM)
    if level == "warning" || level == "critical" {
        let tokens = match app_state.fcm_tokens.lock() {
            Ok(guard) => guard.get(&device_id).cloned().unwrap_or_default(),
            Err(poisoned) => poisoned.into_inner().get(&device_id).cloned().unwrap_or_default(),
        };
        if !tokens.is_empty() {
            let notification_message = message.clone();
            tokio::spawn(async move {
                crate::services::fcm::send_push_notification(&title, &notification_message, tokens)
                    .await;
            });
        }
    }

    info!(
        "  [MQTT-EVENTS] Đã xử lý cảnh báo bình dung dịch cho {}: {}",
        device_id, message
    );
}
