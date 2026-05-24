use actix_web::web;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, instrument, warn};

use crate::AppState;
use crate::models::alert::AlertMessage;
use hydragrow_shared::events::{AppEvent, DeviceStatusPayload as SharedDeviceStatusPayload};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DeviceStatusPayload {
    pub online: bool,
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
    let _ = app_state.event_bus.send(AppEvent::SystemAlert(alert));

    let _ = app_state
        .event_bus
        .send(AppEvent::DeviceStatus(SharedDeviceStatusPayload {
            device_id: device_id.clone(),
            is_online,
        }));
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle_controller(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    if let Ok(payload_json) = serde_json::from_slice::<serde_json::Value>(payload) {
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
    } else {
        warn!("Lỗi parse JSON Health Data từ {}", device_id);
    }
}
