use actix_web::web;
use serde_json::{Value, json};
use tracing::{error, info, instrument, warn};

use crate::AppState;
use hydragrow_shared::{
    events::{AppEvent, FsmTransitionPayload},
    fsm::{FsmStatePayload, SystemPhase},
};

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle_state(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    // Để debug log trực tiếp ra console (có thể comment lại cho đỡ rối)
    let raw_payload = std::str::from_utf8(payload).unwrap_or("Lỗi UTF-8");
    // info!("📥 [MQTT-FSM] Nhận: {}", raw_payload);

    // 1. Parse payload JSON từ Firmware
    let json = match serde_json::from_slice::<Value>(payload) {
        Ok(j) => j,
        Err(e) => {
            error!("❌ [MQTT-FSM] Cấu trúc JSON bị sai định dạng: {:?}", e);
            return;
        }
    };

    let parsed_fsm_state = serde_json::from_slice::<FsmStatePayload>(payload).ok();

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

            // Parse và validate interaction_matrix (giữ nguyên validation hiện tại)
            let interaction_matrix_json: Option<serde_json::Value> = match coeffs
                .get("interaction_matrix")
            {
                Some(raw_matrix) => match raw_matrix.as_array() {
                    Some(items) if items.len() == 6 => {
                        let all_valid = items
                            .iter()
                            .all(|item| item.as_f64().map(|f| f.is_finite()).unwrap_or(false));
                        if all_valid {
                            // Giữ nguyên dạng JSON array cho JSONB column
                            Some(serde_json::Value::Array(items.clone()))
                        } else {
                            warn!(
                                "⚠️ [MQTT-FSM] interaction_matrix của {} có phần tử không hợp lệ, bỏ qua",
                                device_id
                            );
                            None
                        }
                    }
                    Some(items) => {
                        warn!(
                            "⚠️ [MQTT-FSM] interaction_matrix của {} sai shape ({} phần tử, cần 6), bỏ qua",
                            device_id,
                            items.len()
                        );
                        None
                    }
                    None => {
                        warn!(
                            "⚠️ [MQTT-FSM] interaction_matrix của {} không phải mảng JSON, bỏ qua",
                            device_id
                        );
                        None
                    }
                },
                None => None,
            };

            let matrix_update_count = coeffs.get("matrix_update_count").and_then(|v| v.as_i64());
            let matrix_is_warm = coeffs.get("matrix_is_warm").and_then(|v| v.as_bool());

            let query = r#"
                UPDATE dosing_calibration
                SET
                    ec_gain_per_ml = COALESCE($1, ec_gain_per_ml),
                    ph_shift_up_per_ml = COALESCE($2, ph_shift_up_per_ml),
                    ph_shift_down_per_ml = COALESCE($3, ph_shift_down_per_ml),
                    ec_step_ratio = COALESCE($4, ec_step_ratio),
                    ph_step_ratio = COALESCE($5, ph_step_ratio),
                    interaction_matrix = COALESCE($6, interaction_matrix),
                    matrix_update_count = COALESCE($7, matrix_update_count),
                    matrix_is_warm = COALESCE($8, matrix_is_warm),
                    last_calibrated = NOW()
                WHERE device_id = $9
            "#;

            match sqlx::query(query)
                .bind(ec_gain)
                .bind(ph_up)
                .bind(ph_down)
                .bind(step_ec)
                .bind(step_ph)
                .bind(interaction_matrix_json)
                .bind(matrix_update_count)
                .bind(matrix_is_warm)
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
    let (state, pump_status, budgets) = if let Some(fsm_state) = parsed_fsm_state {
        let state =
            match serde_json::from_str::<SystemPhase>(&format!("{}", fsm_state.current_state)) {
                Ok(phase) => format!("{:?}", phase),
                Err(_) => fsm_state.current_state,
            };
        (
            state,
            serde_json::to_value(fsm_state.pump_status).unwrap_or_else(|_| json!({})),
            serde_json::to_value(fsm_state.budgets).unwrap_or_else(|_| json!({})),
        )
    } else {
        let state = match json.get("current_state").and_then(|s| s.as_str()) {
            Some(s) => s.to_string(),
            None => {
                error!(
                    "❌ [MQTT-FSM] JSON hợp lệ nhưng thiếu trường 'current_state': {}",
                    raw_payload
                );
                return;
            }
        };
        (
            state,
            json.get("pump_status")
                .cloned()
                .unwrap_or_else(|| json!({})),
            json.get("budgets").cloned().unwrap_or_else(|| json!({})),
        )
    };

    // 3. Gửi thông tin trạng thái qua kênh Health/LiveStatus cho WebSocket/UI
    let fsm_status_payload = json!({
        "type": "fsm_status",
        "device_id": device_id.clone(),
        "fsm_state": state.clone(),
        "budgets": budgets,
        "pump_status": pump_status,
    });

    let _ = app_state
        .event_bus
        .send(AppEvent::FsmTransition(FsmTransitionPayload {
            device_id: device_id.clone(),
            state: state.clone(),
            pump_status: serde_json::from_value(pump_status.clone()).ok(),
        }));

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

#[cfg(test)]
mod tests {
    #[test]
    fn interaction_matrix_serializes_to_jsonb_value() {
        let raw: Vec<f64> = vec![0.015, 0.015, 0.0, 0.0, 0.0, 0.02];
        let as_json: serde_json::Value = serde_json::to_value(&raw).unwrap();
        assert!(as_json.is_array());
        assert_eq!(as_json.as_array().unwrap().len(), 6);
    }

    #[test]
    fn invalid_matrix_with_nan_is_rejected() {
        let items = vec![
            serde_json::json!(0.015),
            serde_json::json!(f64::NAN), // invalid
        ];
        let has_invalid = items
            .iter()
            .any(|v| v.as_f64().map(|f| !f.is_finite()).unwrap_or(true));
        assert!(has_invalid);
    }
}
