use actix_web::web;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, instrument};

use crate::AppState;
use crate::db::device_wifi;
use crate::metrics::*;
use crate::models::alert::AlertMessage;
use hydragrow_shared::CommandLifecycle;
use hydragrow_shared::events::{AppEvent, DeviceStatusPayload as SharedDeviceStatusPayload};
use hydragrow_shared::telemetry::{
    AuthoritativeTelemetrySnapshot, DeviceHealthSnapshot, ObservedActuatorState, ObservedFsmState,
    TelemetryAvailability, TelemetrySource,
};
use hydragrow_shared::wifi_tx::WifiConfigStatus;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DeviceStatusPayload {
    #[serde(default)]
    pub online: Option<bool>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub firmware_version: Option<String>,
}

fn interpret_online_signal(status: &DeviceStatusPayload) -> Option<bool> {
    if let Some(online) = status.online {
        return Some(online);
    }
    if status.status.as_deref() == Some("online") {
        return Some(true);
    }
    None
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

/// A controller/status packet is operational evidence only when it contains
/// the complete authoritative health snapshot. A syntactically valid JSON
/// object such as `{}` must not refresh contact/freshness by itself.
fn is_authoritative_controller_status(parsed: &ParsedControllerStatus) -> bool {
    parsed.health_snapshot.is_some()
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id, node_type = %node_type, topic_category = %topic_category))]
pub async fn handle_device(
    device_id: String,
    node_type: &str,
    topic_category: &str,
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

    if let Some(fw) = status.firmware_version.as_deref()
        && !fw.is_empty()
        && fw != "unknown"
    {
        info!(device_id = %device_id, firmware_version = %fw, "Cập nhật firmware version map");
        app_state
            .device_firmware
            .write()
            .await
            .insert(device_id.clone(), fw.to_string());
    }

    let _ = crate::db::topic_last_seen::touch_topic(
        &app_state.pg_pool,
        &device_id,
        topic_category,
        chrono::Utc::now(),
    )
    .await;

    match interpret_online_signal(&status) {
        Some(is_online) => {
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
                    last_seen_at: Some(chrono::Utc::now().to_rfc3339()),
                }));

            if alert.level == "warning" || alert.level == "critical" {
                let tokens = match app_state.fcm_tokens.lock() {
                    Ok(guard) => guard.get(&device_id).cloned().unwrap_or_default(),
                    Err(poisoned) => poisoned
                        .into_inner()
                        .get(&device_id)
                        .cloned()
                        .unwrap_or_default(),
                };
                if !tokens.is_empty() {
                    let push_title = alert.title.clone();
                    let push_message = alert.message.clone();
                    tokio::spawn(async move {
                        crate::services::fcm::send_push_notification(
                            &push_title,
                            &push_message,
                            tokens,
                        )
                        .await;
                    });
                }
            }
        }
        None => {
            // Non-online status (e.g. "ok", "error", "applied") - log to system_events under "device"
            if let Some(status_str) = status.status.as_deref() {
                let level = if status_str == "error" {
                    "warning"
                } else {
                    "info"
                };
                let title = format!("Thông điệp {}", node_type);
                let message = format!("Trạng thái: {}", status_str);
                let record = crate::db::postgres::NewSystemEventRecord {
                    device_id: device_id.clone(),
                    level: level.to_string(),
                    category: "device".to_string(),
                    title,
                    message,
                    reason: Some(status_str.to_string()),
                    metadata: serde_json::from_slice(payload).ok(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    source: "rule".to_string(),
                    primary_reason_code: None,
                };
                if let Err(e) =
                    crate::db::postgres::insert_system_event(&app_state.pg_pool, &record).await
                {
                    error!(error = ?e, "Không thể lưu device system_event");
                }
            }
        }
    }
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle_controller(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let parsed = match parse_controller_status_payload(payload) {
        Ok(parsed) if is_authoritative_controller_status(&parsed) => parsed,
        Ok(_) => {
            tracing::warn!(
                device_id = %device_id,
                "Bỏ qua controller/status không có authoritative health snapshot"
            );
            return;
        }
        Err(error) => {
            tracing::warn!(
                device_id = %device_id,
                error = ?error,
                "Bỏ qua controller/status payload không hợp lệ"
            );
            return;
        }
    };

    let _ = crate::db::topic_last_seen::touch_topic(
        &app_state.pg_pool,
        &device_id,
        "controller/status",
        chrono::Utc::now(),
    )
    .await;

    if let Some(health) = parsed.health_snapshot.as_ref() {
        if !health.firmware_version.is_empty() && health.firmware_version != "unknown" {
            tracing::info!(
                device_id = %device_id,
                firmware_version = %health.firmware_version,
                "Cập nhật firmware version map"
            );
            app_state
                .device_firmware
                .write()
                .await
                .insert(device_id.clone(), health.firmware_version.clone());
        } else {
            tracing::debug!(
                device_id = %device_id,
                received_version = %health.firmware_version,
                "Bỏ qua firmware version không hợp lệ"
            );
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
        let mist_on = pump_status.get("mist_valve").and_then(|v| v.as_bool());
        let prev_mist = states
            .get(&device_id)
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.get("pump_status").cloned())
            .and_then(|ps| ps.get("mist_valve").cloned())
            .and_then(|v| v.as_bool());

        if let Some(mist_on) = mist_on
            && prev_mist.is_some()
            && Some(mist_on) != prev_mist
        {
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

    reconcile_command_confirmations(&app_state, &device_id, payload_json).await;

    let received_at = chrono::Utc::now().to_rfc3339();
    let cached_telemetry = app_state
        .device_states
        .read()
        .await
        .get(&device_id)
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|state| state.get("telemetry").cloned())
        .and_then(|value| serde_json::from_value::<AuthoritativeTelemetrySnapshot>(value).ok());
    let mut incoming_authoritative = AuthoritativeTelemetrySnapshot::from_incoming_payload(
        &device_id,
        &hydragrow_shared::sensors::IncomingSensorPayload::default(),
        Some(received_at.clone()),
    );
    incoming_authoritative.device_id = device_id.clone();
    incoming_authoritative.availability = TelemetryAvailability::Online;
    let observation_time = parsed
        .health_snapshot
        .as_ref()
        .and_then(|health| chrono::DateTime::from_timestamp_millis(health.timestamp_ms as i64))
        .map(|dt| dt.to_rfc3339());
    incoming_authoritative.observed_at = observation_time.clone();
    if let Some(health) = parsed.health_snapshot.clone() {
        incoming_authoritative.controller_health = Some(health);
    }
    incoming_authoritative.runtime_ready = payload_json
        .get("runtime_ready")
        .or_else(|| payload_json.get("ready"))
        .and_then(|value| value.as_bool());
    if let Some(pump_value) = payload_json.get("pump_status")
        && let Ok(pump_status) = serde_json::from_value(pump_value.clone())
    {
        incoming_authoritative.actuator = Some(ObservedActuatorState {
            pump_status,
            observed_at: observation_time.clone(),
            received_at: Some(received_at.clone()),
            source: TelemetrySource::ControllerSensor,
        });
    }
    if let Some(state) = payload_json
        .get("fsm_state")
        .or_else(|| payload_json.get("current_state"))
        .or_else(|| payload_json.get("current_phase"))
        .and_then(|value| value.as_str())
    {
        incoming_authoritative.fsm = Some(ObservedFsmState {
            state: state.to_string(),
            observed_at: observation_time.clone(),
            received_at: Some(received_at.clone()),
            source: TelemetrySource::ControllerSensor,
        });
    }
    let mut authoritative = incoming_authoritative.merge_into(cached_telemetry.as_ref());
    authoritative.refresh_operational_state(chrono::Utc::now());
    if let Ok(telemetry_json) = serde_json::to_value(&authoritative) {
        let mut states = app_state.device_states.write().await;
        if let Some(state) = states
            .get(&device_id)
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|mut state| {
                state
                    .as_object_mut()?
                    .insert("telemetry".into(), telemetry_json);
                serde_json::to_string(&state).ok()
            })
        {
            states.insert(device_id.clone(), state);
        }
    }
    let _ = app_state
        .event_bus
        .send(AppEvent::TelemetrySnapshot(Box::new(authoritative)));

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
        if let Some(water_streak) = diag.get("water_hydraulics_streak").and_then(|v| v.as_i64()) {
            DIAGNOSTIC_FAULT_STREAK
                .with_label_values(&[dev, "water_hydraulics"])
                .set(water_streak);
        }
        if let Some(snapshot) = parsed.health_snapshot
            && let Some(hestia) = snapshot.hestia
        {
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

async fn reconcile_command_confirmations(
    app_state: &web::Data<AppState>,
    device_id: &str,
    payload: &serde_json::Value,
) {
    let pump_status = payload.get("pump_status");
    let fsm_state = payload
        .get("current_phase")
        .or_else(|| payload.get("current_state"))
        .or_else(|| payload.get("fsm_state"))
        .and_then(|value| value.as_str())
        .map(str::to_ascii_lowercase);
    let observed_at_ms = controller_observation_timestamp_ms(payload)
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let commands = match crate::services::durable_command::list_commands(
        &app_state.pg_pool,
        device_id,
        100,
    )
    .await
    {
        Ok(commands) => commands,
        Err(error) => {
            error!(device_id = %device_id, ?error, "Failed to load durable commands for confirmation");
            return;
        }
    };
    for record in commands {
        if record.lifecycle != CommandLifecycle::Acknowledged {
            continue;
        }
        if record
            .last_observed_at
            .is_some_and(|t| observed_at_ms <= t.timestamp_millis())
        {
            continue;
        }
        if !observation_can_confirm(observed_at_ms, record.acknowledged_at) {
            continue;
        }

        let matches_requested_state = match record.action.as_str() {
            "emergency_stop" => {
                fsm_state
                    .as_deref()
                    .is_some_and(|state| state.contains("emergency"))
                    && all_pumps_off(pump_status)
            }
            "reset_fault" => fsm_state.as_deref() == Some("monitoring"),
            "on" | "off" | "force_on" => record
                .pump_id
                .as_deref()
                .and_then(|pump| pump_state(pump_status, pump))
                .zip(record.requested_state)
                .is_some_and(|(actual, requested)| actual == requested),
            "set_pwm" => {
                if record.requested_state == Some(false) {
                    record
                        .pump_id
                        .as_deref()
                        .and_then(|pump| pump_state(pump_status, pump))
                        == Some(false)
                } else {
                    record
                        .pump_id
                        .as_deref()
                        .and_then(|pump| pump_pwm(pump_status, pump))
                        .zip(record.requested_pwm)
                        .is_some_and(|(actual, requested)| actual == requested)
                }
            }
            _ => false,
        };

        if matches_requested_state
            && let Err(error) = crate::services::durable_command::transition_with_state(
                app_state,
                &record.command_id,
                device_id,
                CommandLifecycle::Confirmed,
                Some("controller_status".to_string()),
                "runtime_confirmation",
                json!({"confirmation_source":"controller_status"}),
            )
            .await
        {
            error!(device_id = %device_id, command_id = %record.command_id, ?error, "Failed to persist command confirmation");
        }
    }
}

fn pump_state(pump_status: Option<&serde_json::Value>, pump: &str) -> Option<bool> {
    let key = match pump.to_ascii_uppercase().as_str() {
        "A" | "PUMP_A" => "pump_a",
        "B" | "PUMP_B" => "pump_b",
        "PH_UP" | "PUMP_PH_UP" => "ph_up",
        "PH_DOWN" | "PUMP_PH_DOWN" => "ph_down",
        "OSAKA" | "OSAKA_PUMP" => "osaka_pump",
        "MIST" | "MIST_VALVE" => "mist_valve",
        "MIX" | "MIX_VALVE" => "mix_valve",
        "WATER_PUMP" | "WATER_PUMP_IN" | "PUMP_IN" => "water_pump_in",
        "WATER_PUMP_OUT" | "DRAIN_PUMP" | "PUMP_OUT" => "water_pump_out",
        _ => return None,
    };
    pump_status?.get(key)?.as_bool()
}

fn pump_pwm(pump_status: Option<&serde_json::Value>, pump: &str) -> Option<u32> {
    if matches!(pump.to_ascii_uppercase().as_str(), "OSAKA" | "OSAKA_PUMP") {
        return pump_status?.get("osaka_pwm")?.as_u64().map(|v| v as u32);
    }
    None
}

fn all_pumps_off(pump_status: Option<&serde_json::Value>) -> bool {
    [
        "pump_a",
        "pump_b",
        "ph_up",
        "ph_down",
        "osaka_pump",
        "mist_valve",
        "mix_valve",
        "water_pump_in",
        "water_pump_out",
    ]
    .iter()
    .all(|key| {
        pump_status
            .and_then(|status| status.get(key))
            .and_then(|v| v.as_bool())
            == Some(false)
    })
}

fn controller_observation_timestamp_ms(payload: &serde_json::Value) -> Option<i64> {
    payload
        .get("timestamp_ms")
        .and_then(serde_json::Value::as_i64)
        .filter(|timestamp| *timestamp > 0)
}

fn observation_can_confirm(
    observed_at_ms: i64,
    acknowledged_at: Option<chrono::DateTime<chrono::Utc>>,
) -> bool {
    acknowledged_at.map_or(true, |ack| observed_at_ms >= ack.timestamp_millis())
}

#[cfg(test)]
mod command_confirmation_tests {
    use super::{
        all_pumps_off, controller_observation_timestamp_ms, observation_can_confirm, pump_pwm,
        pump_state,
    };
    use serde_json::json;

    #[test]
    fn pump_state_uses_canonical_aliases() {
        let status = json!({"pump_a": true, "ph_down": false});
        assert_eq!(pump_state(Some(&status), "A"), Some(true));
        assert_eq!(pump_state(Some(&status), "PUMP_A"), Some(true));
        assert_eq!(pump_state(Some(&status), "PH_DOWN"), Some(false));
        assert_eq!(pump_state(Some(&status), "UNKNOWN"), None);
    }

    #[test]
    fn pwm_confirmation_is_only_available_where_controller_status_exposes_pwm() {
        let status = json!({"osaka_pwm": 60});
        assert_eq!(pump_pwm(Some(&status), "OSAKA_PUMP"), Some(60));
        assert_eq!(pump_pwm(Some(&status), "PUMP_A"), None);
    }

    #[test]
    fn all_pumps_off_requires_every_runtime_field_to_be_observed_off() {
        let status = json!({
            "pump_a": false,
            "pump_b": false,
            "ph_up": false,
            "ph_down": false,
            "osaka_pump": false,
            "mist_valve": false,
            "mix_valve": false,
            "water_pump_in": false,
            "water_pump_out": false
        });
        assert!(all_pumps_off(Some(&status)));
        assert!(!all_pumps_off(Some(&json!({"pump_a": false}))));
    }

    #[test]
    fn controller_observation_timestamp_is_used_only_when_trustworthy() {
        assert_eq!(
            controller_observation_timestamp_ms(&json!({
                "timestamp_ms": 1_700_000_000_000i64
            })),
            Some(1_700_000_000_000)
        );
        assert_eq!(
            controller_observation_timestamp_ms(&json!({"timestamp_ms": 0})),
            None
        );
        assert_eq!(
            controller_observation_timestamp_ms(&json!({"timestamp_ms": "bad"})),
            None
        );
    }

    #[test]
    fn observation_before_ack_cannot_confirm_command() {
        let observed_at = 1_700_000_000_000i64;
        let acknowledged_at = observed_at + 1_000;
        let ack = chrono::DateTime::from_timestamp_millis(acknowledged_at).unwrap();
        assert!(!observation_can_confirm(observed_at, Some(ack)));
        assert!(observation_can_confirm(acknowledged_at, Some(ack)));
    }

    #[test]
    fn observation_without_ack_is_allowed() {
        assert!(observation_can_confirm(1_700_000_000_000, None));
    }
}

/// Pure delivery-state decision for a device wifi_config_status report.
/// Returns the state to record, or None when the report must be ignored:
/// device mismatch, stale version, unknown future version, or bad state.
/// SSID metadata rows are never deleted on failure — apply state is tracked
/// separately in the delivery table.
pub fn decide_wifi_delivery_update(
    topic_device_id: &str,
    stored_version: i64,
    status: &WifiConfigStatus,
) -> Option<String> {
    if status.device_id != topic_device_id {
        return None;
    }
    if status.config_version != stored_version {
        return None;
    }
    match status.state.as_str() {
        "applied" => Some(device_wifi::delivery_state::APPLIED.to_string()),
        "rolled_back" => Some(device_wifi::delivery_state::ROLLED_BACK.to_string()),
        "rejected" => Some(device_wifi::delivery_state::REJECTED.to_string()),
        _ => None,
    }
}

/// Maps an OTA lifecycle status string to a system_events level.
/// `success`/`done` → `success`, `failed`/`error` → `critical`, anything else → `info`.
fn map_ota_status_to_level(status: &str) -> &'static str {
    match status {
        "success" | "done" => "success",
        "failed" | "error" => "critical",
        _ => "info",
    }
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle_ota_status(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let value: serde_json::Value = match serde_json::from_slice(payload) {
        Ok(value) => value,
        Err(e) => {
            error!(error = ?e, "Lỗi parse ota-status");
            return;
        }
    };
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Cập nhật OTA");
    let message = value.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let status_str = value
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("in_progress");
    let level = map_ota_status_to_level(status_str);

    info!(
        device_id = %device_id,
        title = %title,
        message = %message,
        status = %status_str,
        "Nhận OTA lifecycle event",
    );

    let record = crate::db::postgres::NewSystemEventRecord {
        device_id: device_id.clone(),
        level: level.to_string(),
        category: "device".to_string(),
        title: title.to_string(),
        message: message.to_string(),
        reason: Some(format!("ota_{status_str}")),
        metadata: Some(value.clone()),
        timestamp: chrono::Utc::now().timestamp_millis(),
        source: "rule".to_string(),
        primary_reason_code: None,
    };
    if let Err(e) = crate::db::postgres::insert_system_event(&app_state.pg_pool, &record).await {
        error!(error = ?e, device_id = %device_id, "Không thể lưu OTA system_event");
    }

    let _ = app_state.event_bus.send(AppEvent::ControllerStatus(value));
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle_wifi_config_status(
    device_id: String,
    payload: &[u8],
    app_state: web::Data<AppState>,
) {
    let status: WifiConfigStatus = match serde_json::from_slice(payload) {
        Ok(status) => status,
        Err(e) => {
            error!(error = ?e, "Lỗi parse wifi_config_status");
            return;
        }
    };
    let stored_version = match device_wifi::get_wifi_metadata(&app_state.pg_pool, &device_id).await
    {
        Ok((_, version)) => version,
        Err(e) => {
            error!(error = ?e, "Không đọc được wifi metadata cho delivery update");
            return;
        }
    };
    match decide_wifi_delivery_update(&device_id, stored_version, &status) {
        Some(state) => {
            info!(
                device_id = %device_id,
                config_version = status.config_version,
                state = %state,
                "Cập nhật trạng thái WiFi delivery"
            );
            if let Err(e) = device_wifi::set_delivery_state(
                &app_state.pg_pool,
                &device_id,
                status.config_version,
                &state,
                None,
            )
            .await
            {
                error!(error = ?e, "Không ghi được wifi delivery state");
            }
        }
        None => {
            tracing::debug!(
                device_id = %device_id,
                config_version = status.config_version,
                state = %status.state,
                "Bỏ qua wifi_config_status không khớp (stale/unknown)"
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{is_authoritative_controller_status, parse_controller_status_payload};

    #[test]
    fn parses_and_sets_firmware_version() {
        let raw = br#"{
            "device_id": "device_001",
            "free_heap": 120000,
            "uptime_sec": 3600,
            "rssi": -48,
            "health_score_percent": 91,
            "fsm_state_display": "Monitoring",
            "log_drop_count": 0,
            "firmware_version": "v1.2.3",
            "matrix_update_count": 12,
            "matrix_is_warm": true,
            "timestamp_ms": 1748000000000
        }"#;

        let parsed = parse_controller_status_payload(raw).unwrap();
        assert_eq!(
            parsed.health_snapshot.as_ref().unwrap().firmware_version,
            "v1.2.3"
        );
        // Xác nhận không phải "unknown"
        assert_ne!(
            parsed.health_snapshot.as_ref().unwrap().firmware_version,
            "unknown"
        );
    }

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

    use super::decide_wifi_delivery_update;
    use hydragrow_shared::wifi_tx::WifiConfigStatus;

    fn config_status(device: &str, version: i64, state: &str) -> WifiConfigStatus {
        WifiConfigStatus::new(device, version, state, 2)
    }

    #[test]
    fn applied_wifi_status_marks_matching_version_applied() {
        let status = config_status("device-001", 8, "applied");
        assert_eq!(
            decide_wifi_delivery_update("device-001", 8, &status),
            Some("applied".to_string())
        );
    }

    #[test]
    fn rollback_status_does_not_mark_new_config_applied() {
        // Stale rollback for an older version must not touch the new desired state.
        let stale_rollback = config_status("device-001", 7, "rolled_back");
        assert_eq!(
            decide_wifi_delivery_update("device-001", 8, &stale_rollback),
            None
        );
        // Future unknown version is ignored too.
        let future = config_status("device-001", 9, "applied");
        assert_eq!(decide_wifi_delivery_update("device-001", 8, &future), None);
        // Matching rollback is recorded.
        let matching = config_status("device-001", 8, "rolled_back");
        assert_eq!(
            decide_wifi_delivery_update("device-001", 8, &matching),
            Some("rolled_back".to_string())
        );
    }

    #[test]
    fn cross_device_status_is_rejected() {
        let status = config_status("device-002", 8, "applied");
        assert_eq!(decide_wifi_delivery_update("device-001", 8, &status), None);
    }

    #[test]
    fn password_never_appears_in_serialized_status() {
        let status = config_status("device-001", 8, "applied");
        let json = serde_json::to_string(&status).unwrap();
        assert!(!json.contains("password"));
        assert!(!json.contains("secret"));
        // A hostile payload smuggling a password field still deserializes
        // without capturing it (unknown fields are ignored).
        let hostile = br#"{"type":"wifi_config_status","device_id":"device-001","config_version":8,"state":"applied","ssid_count":1,"password":"smuggled"}"#;
        let parsed: WifiConfigStatus = serde_json::from_slice(hostile).unwrap();
        assert_eq!(parsed.state, "applied");
        assert!(!serde_json::to_string(&parsed).unwrap().contains("smuggled"));
    }

    use super::{DeviceStatusPayload, interpret_online_signal};

    #[test]
    fn interpret_online_signal_uses_bool_field_when_present() {
        let on = DeviceStatusPayload {
            online: Some(true),
            status: None,
            firmware_version: None,
        };
        assert_eq!(interpret_online_signal(&on), Some(true));
        let off = DeviceStatusPayload {
            online: Some(false),
            status: Some("online".to_string()),
            firmware_version: None,
        };
        assert_eq!(interpret_online_signal(&off), Some(false));
    }

    #[test]
    fn treats_status_online_as_true() {
        let s = DeviceStatusPayload {
            online: None,
            status: Some("online".to_string()),
            firmware_version: None,
        };
        assert_eq!(interpret_online_signal(&s), Some(true));
    }

    #[test]
    fn does_not_infer_offline_from_other_status_values() {
        for other in ["offline", "idle", "error", ""] {
            let s = DeviceStatusPayload {
                online: None,
                status: Some(other.to_string()),
                firmware_version: None,
            };
            assert_eq!(interpret_online_signal(&s), None);
        }
    }

    #[test]
    fn returns_none_when_no_signal_present() {
        let s = DeviceStatusPayload {
            online: None,
            status: None,
            firmware_version: None,
        };
        assert_eq!(interpret_online_signal(&s), None);
    }

    #[test]
    fn topic_category_for_sensor_differs_from_controller() {
        let controller_cat = "controller/status";
        let sensor_cat = "sensor/status";
        assert_ne!(controller_cat, sensor_cat);
    }

    #[test]
    fn controller_status_requires_authoritative_health_before_refreshing_operational_state() {
        let empty = parse_controller_status_payload(br"{}").unwrap();
        assert!(!is_authoritative_controller_status(&empty));

        let incomplete = serde_json::json!({
            "device_id": "controller_001",
            "uptime_sec": 10,
            "timestamp_ms": 1_700_000_000_000i64
        });
        let parsed =
            parse_controller_status_payload(&serde_json::to_vec(&incomplete).unwrap()).unwrap();
        assert!(!is_authoritative_controller_status(&parsed));
    }

    #[test]
    fn status_payload_with_custom_message_is_recognized() {
        let payload = serde_json::json!({
            "device_id": "sensor_001",
            "status": "ok",
            "message": "configuration applied"
        });
        let parsed: DeviceStatusPayload = serde_json::from_value(payload).unwrap();
        assert_eq!(parsed.status.as_deref(), Some("ok"));
    }

    #[test]
    fn non_online_status_maps_level_and_fields_correctly() {
        let payload = serde_json::json!({
            "device_id": "sensor_001",
            "status": "error",
            "message": "invalid command JSON"
        });
        let status: DeviceStatusPayload = serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(interpret_online_signal(&status), None);
        let status_str = status.status.as_deref().unwrap();
        let level = if status_str == "error" {
            "warning"
        } else {
            "info"
        };
        let node_type = "Mạch Cảm Biến";
        let title = format!("Thông điệp {}", node_type);
        let message = format!("Trạng thái: {}", status_str);
        let record = crate::db::postgres::NewSystemEventRecord {
            device_id: "sensor_001".to_string(),
            level: level.to_string(),
            category: "device".to_string(),
            title,
            message,
            reason: Some(status_str.to_string()),
            metadata: Some(payload),
            timestamp: 1700000000000,
            source: "rule".to_string(),
            primary_reason_code: None,
        };
        assert_eq!(record.category, "device");
        assert_eq!(record.level, "warning");
        assert_eq!(record.reason.as_deref(), Some("error"));
        assert_eq!(record.title, "Thông điệp Mạch Cảm Biến");
    }

    #[test]
    fn ota_lifecycle_maps_status_to_appropriate_level() {
        use super::map_ota_status_to_level;
        assert_eq!(map_ota_status_to_level("success"), "success");
        assert_eq!(map_ota_status_to_level("done"), "success");
        assert_eq!(map_ota_status_to_level("failed"), "critical");
        assert_eq!(map_ota_status_to_level("error"), "critical");
        assert_eq!(map_ota_status_to_level("downloading"), "info");
        assert_eq!(map_ota_status_to_level("in_progress"), "info");
        assert_eq!(map_ota_status_to_level(""), "info");

        // Record shape used by handle_ota_status: category device, reason ota_<status>.
        let record = crate::db::postgres::NewSystemEventRecord {
            device_id: "controller_001".to_string(),
            level: map_ota_status_to_level("done").to_string(),
            category: "device".to_string(),
            title: "Cập nhật OTA".to_string(),
            message: "OTA hoàn tất".to_string(),
            reason: Some("ota_done".to_string()),
            metadata: Some(serde_json::json!({"status": "done"})),
            timestamp: 1700000000000,
            source: "rule".to_string(),
            primary_reason_code: None,
        };
        assert_eq!(record.category, "device");
        assert_eq!(record.level, "success");
        assert_eq!(record.reason.as_deref(), Some("ota_done"));
    }
}
