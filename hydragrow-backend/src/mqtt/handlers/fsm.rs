use actix_web::web;
use serde_json::json;
use tracing::{error, info, warn};

use crate::AppState;
use crate::db::postgres::{NewSystemEventRecord, insert_system_event};
use crate::metrics::*;
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

#[derive(Debug, Default)]
struct RuntimeCalibrationUpdate {
    ec_gain: Option<f64>,
    ph_up: Option<f64>,
    ph_down: Option<f64>,
    step_ec: Option<f64>,
    step_ph: Option<f64>,

    step_ec_a: Option<f64>,
    step_ec_b: Option<f64>,
    step_ph_up: Option<f64>,
    step_ph_down: Option<f64>,

    interaction_matrix_json: Option<serde_json::Value>,
    matrix_update_count: Option<i64>,
    matrix_is_warm: Option<bool>,

    best_ec_ratio: Option<f64>,
    best_ph_ratio: Option<f64>,

    best_ec_a_ratio: Option<f64>,
    best_ec_b_ratio: Option<f64>,

    best_ph_up_ratio: Option<f64>,
    best_ph_down_ratio: Option<f64>,

    tuner_state: Option<i64>,
    kalman_confidence: Option<serde_json::Value>,
    adaptive_mixing_sec: Option<i64>,
    adaptive_stabilize_sec: Option<i64>,
    effective_ec_tolerance: Option<f64>,
    effective_ph_tolerance: Option<f64>,
}

fn runtime_calibration_update_from_coeffs(
    device_id: &str,
    coeffs: &serde_json::Value,
) -> RuntimeCalibrationUpdate {
    let interaction_matrix_json: Option<serde_json::Value> = match coeffs.get("interaction_matrix")
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

    RuntimeCalibrationUpdate {
        ec_gain: coeffs.get("ec_gain_per_ml").and_then(|v| v.as_f64()),
        ph_up: coeffs.get("ph_shift_up_per_ml").and_then(|v| v.as_f64()),
        ph_down: coeffs.get("ph_shift_down_per_ml").and_then(|v| v.as_f64()),
        step_ec: coeffs.get("step_ratio_ec").and_then(|v| v.as_f64()),
        step_ph: coeffs.get("step_ratio_ph").and_then(|v| v.as_f64()),
        step_ec_a: coeffs.get("step_ratio_ec_a").and_then(|v| v.as_f64()),
        step_ec_b: coeffs.get("step_ratio_ec_b").and_then(|v| v.as_f64()),
        step_ph_up: coeffs.get("step_ratio_ph_up").and_then(|v| v.as_f64()),
        step_ph_down: coeffs.get("step_ratio_ph_down").and_then(|v| v.as_f64()),
        interaction_matrix_json,
        matrix_update_count: coeffs.get("matrix_update_count").and_then(|v| v.as_i64()),
        matrix_is_warm: coeffs.get("matrix_is_warm").and_then(|v| v.as_bool()),
        best_ec_ratio: coeffs.get("best_ec_ratio").and_then(|v| v.as_f64()),
        best_ph_ratio: coeffs.get("best_ph_ratio").and_then(|v| v.as_f64()),
        best_ec_a_ratio: coeffs.get("best_ec_a_ratio").and_then(|v| v.as_f64()),
        best_ec_b_ratio: coeffs.get("best_ec_b_ratio").and_then(|v| v.as_f64()),
        best_ph_up_ratio: coeffs.get("best_ph_up_ratio").and_then(|v| v.as_f64()),
        best_ph_down_ratio: coeffs.get("best_ph_down_ratio").and_then(|v| v.as_f64()),
        tuner_state: coeffs.get("state").and_then(|v| v.as_i64()),
        kalman_confidence: coeffs
            .get("kalman_confidence")
            .and_then(validated_kalman_confidence),
        adaptive_mixing_sec: coeffs.get("adaptive_mixing_sec").and_then(|v| v.as_i64()),
        adaptive_stabilize_sec: coeffs
            .get("adaptive_stabilize_sec")
            .and_then(|v| v.as_i64()),
        effective_ec_tolerance: coeffs
            .get("effective_ec_tolerance")
            .and_then(|v| v.as_f64()),
        effective_ph_tolerance: coeffs
            .get("effective_ph_tolerance")
            .and_then(|v| v.as_f64()),
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
        let update = runtime_calibration_update_from_coeffs(device_id, coeffs);

        // =====================================================================
        // Prometheus Metrics Recording
        // =====================================================================
        if let Some(v) = update.ec_gain {
            ADAPTIVE_GAIN_PER_ML
                .with_label_values(&[device_id, "ec"])
                .set(v);
        }
        if let Some(v) = update.ph_up {
            ADAPTIVE_GAIN_PER_ML
                .with_label_values(&[device_id, "ph_up"])
                .set(v);
        }
        if let Some(v) = update.ph_down {
            ADAPTIVE_GAIN_PER_ML
                .with_label_values(&[device_id, "ph_down"])
                .set(v);
        }
        if let Some(v) = update.step_ec {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "ec"])
                .set(v);
        }
        if let Some(v) = update.step_ph {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "ph"])
                .set(v);
        }

        if let Some(v) = update.step_ec_a {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "ec_a"])
                .set(v);
        }
        if let Some(v) = update.step_ec_b {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "ec_b"])
                .set(v);
        }
        if let Some(v) = update.step_ph_up {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "ph_up"])
                .set(v);
        }
        if let Some(v) = update.step_ph_down {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "ph_down"])
                .set(v);
        }

        if let Some(v) = update.best_ec_ratio {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "best_ec"])
                .set(v);
        }
        if let Some(v) = update.best_ph_ratio {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "best_ph"])
                .set(v);
        }

        if let Some(v) = update.best_ec_a_ratio {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "best_ec_a"])
                .set(v);
        }
        if let Some(v) = update.best_ec_b_ratio {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "best_ph_b"])
                .set(v);
        }
        if let Some(v) = update.best_ph_up_ratio {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "best_ph_up"])
                .set(v);
        }
        if let Some(v) = update.best_ph_down_ratio {
            ADAPTIVE_STEP_RATIO
                .with_label_values(&[device_id, "best_ph_down"])
                .set(v);
        }

        if let Some(v) = update.tuner_state {
            ADAPTIVE_TUNER_STATE.with_label_values(&[device_id]).set(v);
        }
        if let Some(v) = update.matrix_is_warm {
            ADAPTIVE_MATRIX_IS_WARM
                .with_label_values(&[device_id])
                .set(if v { 1 } else { 0 });
        }
        if let Some(v) = update.matrix_update_count {
            ADAPTIVE_MATRIX_UPDATE_COUNT
                .with_label_values(&[device_id])
                .set(v);
        }
        if let Some(v) = update.effective_ec_tolerance {
            ADAPTIVE_EFFECTIVE_TOLERANCE
                .with_label_values(&[device_id, "ec"])
                .set(v);
        }
        if let Some(v) = update.effective_ph_tolerance {
            ADAPTIVE_EFFECTIVE_TOLERANCE
                .with_label_values(&[device_id, "ph"])
                .set(v);
        }
        if let Some(v) = update.adaptive_mixing_sec {
            ADAPTIVE_FLUID_TIME_SECONDS
                .with_label_values(&[device_id, "mixing"])
                .set(v);
        }
        if let Some(v) = update.adaptive_stabilize_sec {
            ADAPTIVE_FLUID_TIME_SECONDS
                .with_label_values(&[device_id, "stabilizing"])
                .set(v);
        }

        // Cập nhật Kalman Confidences 8 kênh nếu có
        if let Some(arr) = update.kalman_confidence.as_ref().and_then(|k| k.as_array()) {
            let labels = [
                "nutrient_a",
                "nutrient_b",
                "ph_up",
                "ph_down",
                "water_in",
                "water_out",
                "osaka_mixing",
                "misting",
            ];
            for (idx, label) in labels.iter().enumerate() {
                if let Some(val) = arr.get(idx).and_then(|v| v.as_f64()) {
                    KALMAN_ACTUATOR_CONFIDENCE
                        .with_label_values(&[device_id, label])
                        .set(val);
                }
            }
        }

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
                    adaptive_mixing_sec = COALESCE($13, adaptive_mixing_sec),
                    adaptive_stabilize_sec = COALESCE($14, adaptive_stabilize_sec),
                    effective_ec_tolerance = COALESCE($15, effective_ec_tolerance),
                    effective_ph_tolerance = COALESCE($16, effective_ph_tolerance),
                    last_calibrated = NOW()
                WHERE device_id = $17
            "#;

        match sqlx::query(query)
            .bind(update.ec_gain)
            .bind(update.ph_up)
            .bind(update.ph_down)
            .bind(update.step_ec)
            .bind(update.step_ph)
            .bind(update.interaction_matrix_json)
            .bind(update.matrix_update_count)
            .bind(update.matrix_is_warm)
            .bind(update.best_ec_ratio)
            .bind(update.best_ph_ratio)
            .bind(update.tuner_state)
            .bind(update.kalman_confidence)
            .bind(update.adaptive_mixing_sec)
            .bind(update.adaptive_stabilize_sec)
            .bind(update.effective_ec_tolerance)
            .bind(update.effective_ph_tolerance)
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

    if let Some(record) = transition_system_event_record(&event) {
        if let Err(err) = insert_system_event(&app_state.pg_pool, &record).await {
            tracing::error!(error = ?err, "Không thể lưu FSM fault transition vào system_events");
        }
    }

    // Fan-out
    let _ = app_state.event_bus.send(AppEvent::FsmTransition(event));
}

fn transition_system_event_record(event: &FsmTransitionEvent) -> Option<NewSystemEventRecord> {
    match &event.to_phase {
        hydragrow_shared::fsm::SystemPhase::Fault(code) => {
            let fault_code = code.as_str();
            Some(NewSystemEventRecord {
                device_id: event.device_id.clone(),
                level: "critical".to_string(),
                category: "alert".to_string(),
                title: format!("Lỗi hệ thống: {}", fault_code),
                message: format!("FSM chuyển vào Fault do {}.", fault_code),
                reason: Some(fault_code.to_string()),
                metadata: Some(serde_json::json!({
                    "event_type": "fsm_fault_transition",
                    "from_phase": event.from_phase.as_ref().map(ToString::to_string),
                    "to_phase": event.to_phase.to_string(),
                    "reason": event.reason,
                    "phase_duration_ms": event.phase_duration_ms,
                })),
                timestamp: event.timestamp_ms as i64,
            })
        }
        hydragrow_shared::fsm::SystemPhase::EmergencyStop(reason) => Some(NewSystemEventRecord {
            device_id: event.device_id.clone(),
            level: "critical".to_string(),
            category: "alert".to_string(),
            title: "Dừng khẩn cấp hệ thống".to_string(),
            message: format!("FSM chuyển vào EmergencyStop: {}.", reason),
            reason: Some(reason.clone()),
            metadata: Some(serde_json::json!({
                "event_type": "fsm_emergency_transition",
                "from_phase": event.from_phase.as_ref().map(ToString::to_string),
                "to_phase": event.to_phase.to_string(),
                "reason": event.reason,
                "phase_duration_ms": event.phase_duration_ms,
            })),
            timestamp: event.timestamp_ms as i64,
        }),
        _ => None,
    }
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
    use super::{
        runtime_calibration_update_from_coeffs, transition_system_event_record,
        validated_runtime_interaction_matrix,
    };
    use hydragrow_shared::fsm::{FaultCode, SystemPhase};
    use hydragrow_shared::telemetry::transition::{FsmTransitionEvent, TransitionReason};

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

    #[test]
    fn runtime_calibration_update_extracts_adaptive_runtime_fields() {
        let coeffs = serde_json::json!({
            "adaptive_mixing_sec": 42,
            "adaptive_stabilize_sec": 24,
            "effective_ec_tolerance": 0.08,
            "effective_ph_tolerance": 0.16
        });

        let update = runtime_calibration_update_from_coeffs("device-1", &coeffs);

        assert_eq!(update.adaptive_mixing_sec, Some(42));
        assert_eq!(update.adaptive_stabilize_sec, Some(24));
        assert_eq!(update.effective_ec_tolerance, Some(0.08));
        assert_eq!(update.effective_ph_tolerance, Some(0.16));
    }

    #[test]
    fn fault_transition_becomes_critical_system_event_record() {
        let event = FsmTransitionEvent {
            device_id: "controller-1".to_string(),
            from_phase: Some(SystemPhase::Monitoring),
            to_phase: SystemPhase::Fault(FaultCode::EcDosingFailed),
            reason: TransitionReason::FaultDetected {
                fault_code: FaultCode::EcDosingFailed,
                consecutive_failures: 3,
            },
            timestamp_ms: 123_456,
            phase_duration_ms: Some(12_000),
        };

        let record = transition_system_event_record(&event).expect("fault should be logged");

        assert_eq!(record.device_id, "controller-1");
        assert_eq!(record.level, "critical");
        assert_eq!(record.category, "alert");
        assert_eq!(record.title, "Lỗi hệ thống: EC_DOSING_FAILED");
        assert_eq!(record.reason.as_deref(), Some("EC_DOSING_FAILED"));
        assert_eq!(record.timestamp, 123_456);
    }

    #[test]
    fn normal_transition_does_not_create_system_event_record() {
        let event = FsmTransitionEvent {
            device_id: "controller-1".to_string(),
            from_phase: Some(SystemPhase::Booting),
            to_phase: SystemPhase::Monitoring,
            reason: TransitionReason::BootComplete,
            timestamp_ms: 123_456,
            phase_duration_ms: Some(3_000),
        };

        assert!(transition_system_event_record(&event).is_none());
    }
}
