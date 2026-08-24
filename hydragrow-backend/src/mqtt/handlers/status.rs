use actix_web::web;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, instrument, warn};

use crate::AppState;
use crate::metrics::*;
use crate::models::alert::AlertMessage;
use hydragrow_shared::events::{AppEvent, DeviceStatusPayload as SharedDeviceStatusPayload};
use hydragrow_shared::telemetry::DeviceHealthSnapshot;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DeviceStatusPayload {
    pub online: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedControllerStatus {
    pub raw_json: serde_json::Value,
    pub health_snapshot: Option<DeviceHealthSnapshot>,
}

pub fn parse_controller_status_payload(
    payload: &[u8],
) -> Result<ParsedControllerStatus, serde_json::Error> {
    let raw_json = serde_json::from_slice::<serde_json::Value>(payload)?;
    let health_snapshot = serde_json::from_value::<DeviceHealthSnapshot>(raw_json.clone()).ok();

    Ok(ParsedControllerStatus {
        raw_json,
        health_snapshot,
    })
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id, node_type = %node_type))]
pub async fn handle_device(
    device_id: String,
    node_type: &str,
    payload: &[u8],
    app_state: web::Data<AppState>,
) {
    let status: DeviceStatusPayload = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!(error = ?e, "Lỗi parse DeviceStatus");
            return;
        }
    };

    let is_online = status.online;

    info!(
        "Trạng thái: {}",
        if is_online { "ONLINE" } else { "OFFLINE (LWT)" }
    );

    let alert = AlertMessage {
        level: if is_online {
            "success".to_string()
        } else {
            "warning".to_string()
        },
        category: "system".to_string(),
        title: format!("Trạng thái {}", node_type),
        message: format!(
            "{} ({}) vừa {}",
            node_type,
            device_id,
            if is_online {
                "Trực tuyến"
            } else {
                "Mất kết nối"
            }
        ),
        device_id: device_id.clone(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reason: None,
        metadata: Some(json!({ "event_type": "device_status" })),
    };
    let _ = app_state
        .event_bus
        .send(AppEvent::SystemAlert(alert.clone()));

    let _ = app_state
        .event_bus
        .send(AppEvent::DeviceStatus(SharedDeviceStatusPayload {
            device_id: device_id.clone(),
            is_online,
        }));

    if alert.level == "warning" || alert.level == "critical" {
        let tokens = match app_state.fcm_tokens.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        if !tokens.is_empty() {
            let push_title = alert.title.clone();
            let push_message = alert.message.clone();
            tokio::spawn(async move {
                crate::services::fcm::send_push_notification(&push_title, &push_message, tokens)
                    .await;
            });
        }
    }
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle_controller(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    if let Ok(parsed) = parse_controller_status_payload(payload) {
        if let Some(health) = parsed.health_snapshot.as_ref() {
            if !health.firmware_version.is_empty() && health.firmware_version != "unknown" {
                app_state
                    .device_firmware
                    .write()
                    .await
                    .insert(device_id.clone(), health.firmware_version.clone());
            }
        }
        let payload_json = &parsed.raw_json;
        let dev = &device_id;
        let mut states = app_state.device_states.write().await;

        let mut merged = states
            .get(&device_id)
            .and_then(|existing_str| serde_json::from_str::<serde_json::Value>(existing_str).ok())
            .unwrap_or_else(|| json!({ "device_id": device_id.clone() }));

        if let (Some(merged_obj), Some(incoming_obj)) =
            (merged.as_object_mut(), payload_json.as_object())
        {
            for (key, value) in incoming_obj {
                merged_obj.insert(key.clone(), value.clone());
            }
            merged_obj.insert("device_id".to_string(), json!(device_id.clone()));
            merged_obj.insert(
                "controller_status_ts".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
        }

        // Phát hiện misting qua pump_status
        if let Some(pump_status) = payload_json.get("pump_status") {
            let mist_on = pump_status
                .get("mist_valve")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let prev_mist = states
                .get(&device_id)
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .and_then(|v| v.get("pump_status").cloned())
                .and_then(|ps| ps.get("mist_valve").cloned())
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if mist_on != prev_mist {
                let mist_alert = AlertMessage {
                    level: "FSM_UPDATE".to_string(),
                    category: "system".to_string(),
                    title: "FSM_SYNC".to_string(),
                    message: if mist_on {
                        "Misting".to_string()
                    } else {
                        "Monitoring".to_string()
                    },
                    device_id: device_id.clone(),
                    timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    reason: None,
                    metadata: None,
                };
                let _ = app_state.event_bus.send(AppEvent::SystemAlert(mist_alert));
            }
        }

        if let Ok(updated_str) = serde_json::to_string(&merged) {
            states.insert(device_id.clone(), updated_str);
        }

        drop(states);

        let _ = app_state
            .event_bus
            .send(AppEvent::ControllerStatus(payload_json.clone()));

        // 1. Cập nhật thông số phần cứng ESP32
        if let Some(heap) = payload_json.get("free_heap").and_then(|v| v.as_u64()) {
            CONTROLLER_FREE_HEAP_BYTES
                .with_label_values(&[dev])
                .set(heap as i64);
        }
        if let Some(rssi) = payload_json.get("rssi").and_then(|v| v.as_i64()) {
            CONTROLLER_WIFI_RSSI_DBM.with_label_values(&[dev]).set(rssi);
        }
        if let Some(uptime) = payload_json.get("uptime_sec").and_then(|v| v.as_u64()) {
            CONTROLLER_UPTIME_SECONDS
                .with_label_values(&[dev])
                .set(uptime as i64);
        }
        if let Some(drops) = payload_json.get("log_drop_count").and_then(|v| v.as_u64()) {
            CONTROLLER_LOG_DROPPED_TOTAL
                .with_label_values(&[dev])
                .set(drops as i64);
        }

        // 2. Cập nhật Budgets & Streaks từ FsmSnapshot nếu có
        if let Some(budgets) = payload_json.get("budgets") {
            if let Some(ec_ml) = budgets.get("ec_ml").and_then(|v| v.as_f64()) {
                SAFETY_HOURLY_DOSE_ML
                    .with_label_values(&[dev, "ec"])
                    .set(ec_ml);
            }
            if let Some(ph_ml) = budgets.get("ph_ml").and_then(|v| v.as_f64()) {
                SAFETY_HOURLY_DOSE_ML
                    .with_label_values(&[dev, "ph"])
                    .set(ph_ml);
            }
            if let Some(refills) = budgets.get("refill_count").and_then(|v| v.as_i64()) {
                SAFETY_HOURLY_WATER_CYCLES
                    .with_label_values(&[dev, "refill"])
                    .set(refills);
            }
            if let Some(drains) = budgets.get("drain_count").and_then(|v| v.as_i64()) {
                SAFETY_HOURLY_WATER_CYCLES
                    .with_label_values(&[dev, "drain"])
                    .set(drains);
            }
        }

        if let Some(diag) = payload_json.get("diagnostics") {
            if let Some(ec_streak) = diag.get("ec_pump_streak").and_then(|v| v.as_i64()) {
                DIAGNOSTIC_FAULT_STREAK
                    .with_label_values(&[dev, "ec_pump"])
                    .set(ec_streak);
            }
            if let Some(ph_streak) = diag.get("ph_pump_streak").and_then(|v| v.as_i64()) {
                DIAGNOSTIC_FAULT_STREAK
                    .with_label_values(&[dev, "ph_pump"])
                    .set(ph_streak);
            }
            if let Some(water_streak) = diag.get("water_hydraulics_streak").and_then(|v| v.as_i64())
            {
                DIAGNOSTIC_FAULT_STREAK
                    .with_label_values(&[dev, "water_hydraulics"])
                    .set(water_streak);
            }
            if let Some(snapshot) = parsed.health_snapshot {
                if let Some(hestia) = snapshot.hestia {
                    HESTIA_CONFIDENCE
                        .with_label_values(&[dev])
                        .set(hestia.confidence as f64);

                    HESTIA_AXIS_WEIGHT
                        .with_label_values(&[dev, "ec"])
                        .set(hestia.axes.ec.weight as f64);
                    HESTIA_AXIS_WEIGHT
                        .with_label_values(&[dev, "ph"])
                        .set(hestia.axes.ph.weight as f64);
                    HESTIA_AXIS_WEIGHT
                        .with_label_values(&[dev, "water_level"])
                        .set(hestia.axes.water_level.weight as f64);
                    HESTIA_AXIS_WEIGHT
                        .with_label_values(&[dev, "temp"])
                        .set(hestia.axes.temp.weight as f64);

                    HESTIA_AXIS_ACTION_FACTOR
                        .with_label_values(&[dev, "ec"])
                        .set(hestia.axes.ec.action_factor as f64);
                    HESTIA_AXIS_ACTION_FACTOR
                        .with_label_values(&[dev, "ph"])
                        .set(hestia.axes.ph.action_factor as f64);
                    HESTIA_AXIS_ACTION_FACTOR
                        .with_label_values(&[dev, "water_level"])
                        .set(hestia.axes.water_level.action_factor as f64);
                    HESTIA_AXIS_ACTION_FACTOR
                        .with_label_values(&[dev, "temp"])
                        .set(hestia.axes.temp.action_factor as f64);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_controller_status_payload;

    #[test]
    fn parses_new_device_health_snapshot_payload() {
        let raw = br#"{
            "device_id": "device_001",
            "free_heap": 120000,
            "uptime_sec": 3600,
            "rssi": -48,
            "health_score_percent": 91,
            "fsm_state_display": "Monitoring",
            "log_drop_count": 2,
            "kalman_confidence": {
                "nutrient_a": 0.9,
                "nutrient_b": 0.8,
                "ph_up": 0.7,
                "ph_down": 0.6,
                "water_in": 0.5,
                "water_out": 0.4,
                "osaka_mixing": 0.3,
                "misting": 0.2
            },
            "matrix_update_count": 12,
            "matrix_is_warm": true,
            "timestamp_ms": 1748000000000
        }"#;

        let parsed = parse_controller_status_payload(raw).unwrap();

        assert_eq!(
            parsed
                .health_snapshot
                .as_ref()
                .unwrap()
                .health_score_percent,
            91
        );
        assert_eq!(
            parsed.health_snapshot.as_ref().unwrap().fsm_state_display,
            "Monitoring"
        );
        assert_eq!(parsed.raw_json["matrix_update_count"], 12);
    }

    #[test]
    fn parses_legacy_controller_health_payload() {
        let raw = br#"{
            "free_heap": 64000,
            "uptime_sec": 20,
            "rssi": -62,
            "pump_status": {"pump_a": true}
        }"#;

        let parsed = parse_controller_status_payload(raw).unwrap();

        assert!(parsed.health_snapshot.is_none());
        assert_eq!(parsed.raw_json["pump_status"]["pump_a"], true);
    }
}
