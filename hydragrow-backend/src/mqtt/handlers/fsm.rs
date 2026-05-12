use actix_web::web;
use serde_json::json;
use tracing::{error, info, instrument};

use crate::AppState;

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle_state(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    // Để debug log trực tiếp ra console
    let raw_payload = std::str::from_utf8(payload).unwrap_or("Lỗi UTF-8");
    info!("📥 [MQTT-FSM] Cập nhật trạng thái (Live): {}", raw_payload);

    // 1. Parse payload JSON từ Firmware
    let json = match serde_json::from_slice::<serde_json::Value>(payload) {
        Ok(j) => j,
        Err(e) => {
            error!("❌ [MQTT-FSM] Cấu trúc JSON bị sai định dạng: {:?}", e);
            return;
        }
    };

    if json.get("event").is_some() && json.get("title").is_some() {
        crate::mqtt::handlers::system_log::handle(device_id, payload, app_state).await;
        return;
    }

    // 2. Trích xuất trường current_state
    let state = match json.get("current_state").and_then(|s| s.as_str()) {
        Some(s) => s.to_string(),
        None => {
            error!("❌ [MQTT-FSM] JSON hợp lệ nhưng thiếu trường 'current_state'!");
            return;
        }
    };

    // 3. Gửi thông tin trạng thái qua kênh Health/LiveStatus
    // Phục vụ cho giao diện Frontend (WebSocket/SSE) cập nhật theo thời gian thực.
    // LƯU Ý: Không sinh Alert Toast và không ghi Database ở đây để tránh spam!
    let fsm_status_payload = json!({
        "_msg_type": "fsm_status",
        "device_id": device_id.clone(),
        "fsm_state": state.clone(),
        "budgets": json.get("budgets").unwrap_or(&json!({})),
        "pump_status": json.get("pump_status").unwrap_or(&json!({})),
    });

    let _ = app_state.health_sender.send(fsm_status_payload);

    // 4. Cập nhật fsm_state vào bộ nhớ đệm (Cache tĩnh) của AppState
    // Phục vụ cho các API REST (Ví dụ: Frontend gọi GET /api/devices/:id/status để lấy dữ liệu lần đầu)
    let mut states = app_state.device_states.write().await;

    if let Some(existing_str) = states.get(&device_id) {
        // Nếu thiết bị đã có trong cache, tiến hành ghi đè/chèn thêm trường "fsm_state"
        if let Ok(mut cached_json) = serde_json::from_str::<serde_json::Value>(existing_str) {
            if let Some(obj) = cached_json.as_object_mut() {
                obj.insert("fsm_state".to_string(), json!(state.clone()));

                if let Ok(new_str) = serde_json::to_string(&obj) {
                    states.insert(device_id.clone(), new_str);
                }
            }
        }
    } else {
        // Nếu thiết bị chưa từng có dữ liệu trong cache, tạo mới
        states.insert(device_id.clone(), json!({"fsm_state": state}).to_string());
    }
}
