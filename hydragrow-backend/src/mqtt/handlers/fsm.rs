use actix_web::web;
use serde_json::json;
use tracing::{error, info, instrument, warn};

use crate::AppState;

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle_state(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    // Để debug log trực tiếp ra console (có thể comment lại cho đỡ rối)
    let raw_payload = std::str::from_utf8(payload).unwrap_or("Lỗi UTF-8");
    // info!("📥 [MQTT-FSM] Nhận: {}", raw_payload);

    // 1. Parse payload JSON từ Firmware
    let json = match serde_json::from_slice::<serde_json::Value>(payload) {
        Ok(j) => j,
        Err(e) => {
            error!("❌ [MQTT-FSM] Cấu trúc JSON bị sai định dạng: {:?}", e);
            return;
        }
    };

    // -----------------------------------------------------------------------
    // THÊM MỚI: Bắt payload yêu cầu cập nhật hệ số Calibration (EMA) vào DB
    // -----------------------------------------------------------------------
    if json.get("type").and_then(|t| t.as_str()) == Some("runtime_calibration_update") {
        info!(
            "🛠️ [MQTT-FSM] Nhận yêu cầu cập nhật Calibration (EMA) từ {}",
            device_id
        );

        if let Some(coeffs) = json.get("runtime_coefficients") {
            // Lấy các hệ số mới
            let ec_gain = coeffs.get("ec_gain_per_ml").and_then(|v| v.as_f64());
            let ph_up = coeffs.get("ph_shift_up_per_ml").and_then(|v| v.as_f64());
            let ph_down = coeffs.get("ph_shift_down_per_ml").and_then(|v| v.as_f64());
            let step_ec = coeffs.get("step_ratio_ec").and_then(|v| v.as_f64());
            let step_ph = coeffs.get("step_ratio_ph").and_then(|v| v.as_f64());

            // Câu lệnh SQL linh hoạt, chỉ update những biến có giá trị (không null)
            let query = r#"
                UPDATE dosing_calibration 
                SET 
                    ec_gain_per_ml = COALESCE($1, ec_gain_per_ml),
                    ph_shift_up_per_ml = COALESCE($2, ph_shift_up_per_ml),
                    ph_shift_down_per_ml = COALESCE($3, ph_shift_down_per_ml),
                    ec_step_ratio = COALESCE($4, ec_step_ratio),
                    ph_step_ratio = COALESCE($5, ph_step_ratio),
                    updated_at = NOW()
                WHERE device_id = $6
            "#;

            match sqlx::query(query)
                .bind(ec_gain)
                .bind(ph_up)
                .bind(ph_down)
                .bind(step_ec)
                .bind(step_ph)
                .bind(&device_id)
                .execute(&app_state.pg_pool)
                .await
            {
                Ok(res) => {
                    if res.rows_affected() > 0 {
                        info!(
                            "✅ [DB] Đã cập nhật thành công hệ số Calibration mới cho {}",
                            device_id
                        );
                    } else {
                        warn!(
                            "⚠️ [DB] Không tìm thấy cấu hình Calibration của {}",
                            device_id
                        );
                    }
                }
                Err(e) => error!("❌ [DB] Lỗi UPDATE dosing_calibration: {:?}", e),
            }
        }

        // Return luôn vì đây là bản tin cập nhật DB, không phải cập nhật trạng thái hoạt động (current_state)
        return;
    }

    // 2. Trích xuất trường current_state cho các bản tin FSM bình thường
    let state = match json.get("current_state").and_then(|s| s.as_str()) {
        Some(s) => s.to_string(),
        None => {
            // Firmware gửi thứ gì đó không phải state FSM và cũng không phải Calibration Update
            error!(
                "❌ [MQTT-FSM] JSON hợp lệ nhưng thiếu trường 'current_state': {}",
                raw_payload
            );
            return;
        }
    };

    // 3. Gửi thông tin trạng thái qua kênh Health/LiveStatus cho WebSocket/UI
    let fsm_status_payload = json!({
        "_msg_type": "fsm_status",
        "device_id": device_id.clone(),
        "fsm_state": state.clone(),
        "budgets": json.get("budgets").unwrap_or(&json!({})),
        "pump_status": json.get("pump_status").unwrap_or(&json!({})),
    });

    let _ = app_state.health_sender.send(fsm_status_payload);

    // 4. Cập nhật fsm_state vào bộ nhớ đệm (Cache tĩnh) của AppState
    let mut states = app_state.device_states.write().await;
    if let Some(existing_str) = states.get(&device_id) {
        if let Ok(mut cached_json) = serde_json::from_str::<serde_json::Value>(existing_str) {
            if let Some(obj) = cached_json.as_object_mut() {
                obj.insert("fsm_state".to_string(), json!(state.clone()));
                if let Ok(new_str) = serde_json::to_string(&obj) {
                    states.insert(device_id.clone(), new_str);
                }
            }
        }
    } else {
        states.insert(device_id.clone(), json!({"fsm_state": state}).to_string());
    }
}

