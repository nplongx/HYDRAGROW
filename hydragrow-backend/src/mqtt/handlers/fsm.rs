use actix_web::web;
use serde_json::json;
use tracing::{error, info, warn};

use crate::AppState;
use hydragrow_shared::{
    PumpStatus, events::AppEvent, fsm::FsmSnapshot, telemetry::FsmTransitionEvent,
};

pub async fn handle_state(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let snapshot: FsmSnapshot = match serde_json::from_slice(payload) {
        Ok(j) => j,
        Err(e) => {
            error!("❌ [MQTT-FSM] JSON parse error: {:?}", e);
            return;
        }
    };

    // 1. Cập nhật cache (Redis hoặc In-memory) để API GET /state lấy được ngay
    update_device_state_cache(&device_id, &snapshot, &app_state).await;

    let _ = app_state.event_bus.send(AppEvent::FsmStateUpdate(snapshot));
}

pub fn validated_runtime_interaction_matrix(
    raw_matrix: &serde_json::Value,
) -> Option<serde_json::Value> {
    let items = raw_matrix.as_array()?;
    if items.len() != 32 {
        return None;
    }
    let all_valid = items
        .iter()
        .all(|item| item.as_f64().map(|f| f.is_finite()).unwrap_or(false));
    if all_valid {
        Some(serde_json::Value::Array(items.clone()))
    } else {
        None
    }
}

fn validated_kalman_confidence(raw: &serde_json::Value) -> Option<serde_json::Value> {
    match raw {
        serde_json::Value::Array(items) if items.len() == 8 => {
            if items
                .iter()
                .all(|item| item.as_f64().map(|f| f.is_finite()).unwrap_or(false))
            {
                Some(raw.clone())
            } else {
                None
            }
        }
        serde_json::Value::Object(obj) => {
            if obj
                .values()
                .all(|item| item.as_f64().map(|f| f.is_finite()).unwrap_or(false))
            {
                Some(raw.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

pub async fn handle_calibration_update(
    device_id: &str,
    json: &serde_json::Value,
    app_state: web::Data<AppState>,
) {
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

        let interaction_matrix_json: Option<serde_json::Value> = match coeffs
            .get("interaction_matrix")
        {
            Some(raw_matrix) => {
                let validated = validated_runtime_interaction_matrix(raw_matrix);
                if validated.is_none() {
                    warn!(
                        "⚠️ [MQTT-FSM] interaction_matrix của {} không đúng shape 4x8 phẳng hoặc có phần tử không hợp lệ, bỏ qua",
                        device_id
                    );
                }
                validated
            }
            None => None,
        };

        let matrix_update_count = coeffs.get("matrix_update_count").and_then(|v| v.as_i64());
        let matrix_is_warm = coeffs.get("matrix_is_warm").and_then(|v| v.as_bool());
        let best_ec_ratio = coeffs.get("best_ec_ratio").and_then(|v| v.as_f64());
        let best_ph_ratio = coeffs.get("best_ph_ratio").and_then(|v| v.as_f64());
        let tuner_state = coeffs.get("state").and_then(|v| v.as_i64());
        let kalman_confidence = coeffs
            .get("kalman_confidence")
            .and_then(validated_kalman_confidence);

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
                    best_ec_ratio = COALESCE($9, best_ec_ratio),
                    best_ph_ratio = COALESCE($10, best_ph_ratio),
                    tuner_state = COALESCE($11, tuner_state),
                    kalman_confidence = COALESCE($12, kalman_confidence),
                    last_calibrated = NOW()
                WHERE device_id = $13
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
            .bind(best_ec_ratio)
            .bind(best_ph_ratio)
            .bind(tuner_state)
            .bind(kalman_confidence)
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

pub async fn handle_fsm_transition(
    device_id: String,
    payload: &[u8],
    app_state: web::Data<AppState>,
) {
    let event: FsmTransitionEvent = match serde_json::from_slice(payload) {
        Ok(e) => e,
        Err(err) => {
            tracing::error!(error = ?err, "Lỗi parse FsmTransitionEvent");
            return;
        }
    };

    // Cập nhật fsm_state trong device_states cache (dùng Display string để backward compat)
    let state_str = event.to_phase.to_string();
    let mut states = app_state.device_states.write().await;
    if let Some(existing_str) = states.get(&device_id) {
        if let Ok(mut cached) = serde_json::from_str::<serde_json::Value>(existing_str) {
            if let Some(obj) = cached.as_object_mut() {
                obj.insert("fsm_state".into(), serde_json::json!(state_str));
                if let Ok(s) = serde_json::to_string(&obj) {
                    states.insert(device_id.clone(), s);
                }
            }
        }
    } else {
        states.insert(
            device_id.clone(),
            serde_json::json!({"fsm_state": state_str}).to_string(),
        );
    }
    drop(states);

    // Fan-out
    let _ = app_state.event_bus.send(AppEvent::FsmTransition(event));
}

async fn update_device_state_cache(
    device_id: &str,
    snapshot: &FsmSnapshot,
    app_state: &web::Data<AppState>,
) {
    let mut states = app_state.device_states.write().await;
    let mut cached = states
        .get(device_id)
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .unwrap_or_else(|| serde_json::json!({ "device_id": device_id }));
    if let Some(obj) = cached.as_object_mut() {
        let state = snapshot.current_phase.to_string();
        obj.insert(
            "fsm_phase".into(),
            serde_json::json!(snapshot.current_phase.as_str()),
        );
        obj.insert("fsm_state".into(), serde_json::json!(state));
        obj.insert("pump_status".into(), json!(snapshot.pump_status.clone()));
        obj.insert("budgets".into(), json!(snapshot.budgets.clone()));
    }
    if let Ok(s) = serde_json::to_string(&cached) {
        states.insert(device_id.to_string(), s);
    }
}

#[cfg(test)]
mod tests {
    use super::validated_runtime_interaction_matrix;

    #[test]
    fn runtime_interaction_matrix_accepts_controller_4x8_flat_shape() {
        let raw: Vec<f64> = (0..32).map(|i| i as f64 * 0.01).collect();
        let as_json: serde_json::Value = serde_json::to_value(&raw).unwrap();
        let validated = validated_runtime_interaction_matrix(&as_json).unwrap();

        assert!(validated.is_array());
        assert_eq!(validated.as_array().unwrap().len(), 32);
    }

    #[test]
    fn runtime_interaction_matrix_rejects_legacy_6_value_shape() {
        let raw: Vec<f64> = vec![0.015, 0.015, 0.0, 0.0, 0.0, 0.02];
        let as_json: serde_json::Value = serde_json::to_value(&raw).unwrap();

        assert!(validated_runtime_interaction_matrix(&as_json).is_none());
    }
}
