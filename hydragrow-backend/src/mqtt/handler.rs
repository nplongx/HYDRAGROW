use actix_web::web;
use rumqttc::Publish;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info, instrument, warn};

use crate::AppState;
use crate::db::influx::write_sensor_data;
use crate::db::postgres::{NewSystemEventRecord, insert_system_event};
use crate::models::alert::AlertMessage;
use crate::models::config::DosingCalibration;
use crate::models::sensor::{PumpStatus, SensorData};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DosingReportPayload {
    pub start_ec: f32,
    pub start_ph: f32,
    pub pump_a_ml: f32,
    pub pump_b_ml: f32,
    pub ph_up_ml: f32,
    pub ph_down_ml: f32,
    pub target_ec: f32,
    pub target_ph: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_ec: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_ec: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stabilized_ec: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_ph: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_ph: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stabilized_ph: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stabilized_window_sec: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DeviceStatusPayload {
    pub online: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IncomingSensorPayload {
    pub temp: Option<f64>,
    pub ec: Option<f64>,
    pub ph: Option<f64>,
    pub water_level: Option<f64>,
    #[serde(rename = "last_update_ms", alias = "timestamp_ms")]
    pub timestamp_ms: Option<u64>,
    pub time: Option<String>,
    pub pump_status: Option<PumpStatus>,

    pub rssi: Option<i32>,
    pub free_heap: Option<u32>,
    pub uptime: Option<u32>,

    pub err_water: Option<bool>,
    pub err_temp: Option<bool>,
    pub err_ph: Option<bool>,
    pub err_ec: Option<bool>,

    pub is_continuous: Option<bool>,
    pub ph_voltage_mv: Option<f64>,
}

#[inline]
fn parse_agitech_topic(topic: &str) -> Option<(String, String)> {
    let prefix = "AGITECH/";
    if !topic.starts_with(prefix) {
        return None;
    }
    let rest = &topic[prefix.len()..];
    let slash = rest.find('/')?;
    let device_id = rest[..slash].to_string();
    let suffix = rest[slash..].to_string();
    Some((device_id, suffix))
}

#[instrument(skip(app_state, publish), fields(topic = %publish.topic))]
pub async fn process_message(publish: Publish, app_state: web::Data<AppState>) {
    let topic = publish.topic.clone();
    let payload_bytes = publish.payload;

    let (device_id, suffix) = match parse_agitech_topic(&topic) {
        Some(v) => v,
        None => {
            warn!("Bỏ qua topic không đúng chuẩn hệ thống: {}", topic);
            return;
        }
    };

    match suffix.as_str() {
        "/sensors" => {
            handle_sensor_data(device_id, &payload_bytes, app_state).await;
        }
        "/status" => {
            handle_device_status(device_id, "Trạm Điều Khiển", &payload_bytes, app_state).await;
        }
        "/sensor/status" => {
            handle_device_status(device_id, "Mạch Cảm Biến", &payload_bytes, app_state).await;
        }
        "/fsm" => {
            handle_fsm_state(device_id, &payload_bytes, app_state).await;
        }
        "/dosing_report" => {
            handle_dosing_report(device_id, &payload_bytes, app_state).await;
        }
        "/controller/status" => {
            if let Ok(payload_json) = serde_json::from_slice::<serde_json::Value>(&payload_bytes) {
                let mut states = app_state.device_states.write().await;

                let mut merged = states
                    .get(&device_id)
                    .and_then(|existing_str| {
                        serde_json::from_str::<serde_json::Value>(existing_str).ok()
                    })
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
                        let mist_alert = if mist_on {
                            AlertMessage {
                                level: "FSM_UPDATE".to_string(),
                                title: "FSM_SYNC".to_string(),
                                message: "Misting".to_string(),
                                device_id: device_id.clone(),
                                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                                reason: None,
                                metadata: None,
                            }
                        } else {
                            AlertMessage {
                                level: "FSM_UPDATE".to_string(),
                                title: "FSM_SYNC".to_string(),
                                message: "Monitoring".to_string(),
                                device_id: device_id.clone(),
                                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                                reason: None,
                                metadata: None,
                            }
                        };
                        let _ = app_state.alert_sender.send(mist_alert);
                    }
                }

                if let Ok(updated_str) = serde_json::to_string(&merged) {
                    states.insert(device_id.clone(), updated_str);
                }

                let _ = app_state.health_sender.send(payload_json);
            } else {
                warn!("Lỗi parse JSON Health Data từ {}", device_id);
            }
        }
        _ => {
            debug!("Nhận được topic không quản lý: {}", topic);
        }
    }
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
async fn handle_sensor_data(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let incoming: IncomingSensorPayload = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!(error = ?e, "Lỗi parse JSON SensorData");
            return;
        }
    };

    let time = incoming
        .time
        .clone()
        .or_else(|| {
            incoming
                .timestamp_ms
                .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64))
                .map(|dt| dt.to_rfc3339())
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    let sensor_data = SensorData {
        device_id: device_id.clone(),
        temp: incoming.temp.unwrap_or(0.0),
        ec: incoming.ec.unwrap_or(0.0),
        ph: incoming.ph.unwrap_or(0.0),
        water_level: incoming.water_level.unwrap_or(0.0),
        pump_status: incoming.pump_status.unwrap_or_default(),
        time,
        rssi: incoming.rssi,
        free_heap: incoming.free_heap,
        uptime: incoming.uptime,
        err_water: incoming.err_water,
        err_temp: incoming.err_temp,
        err_ph: incoming.err_ph,
        err_ec: incoming.err_ec,
        is_continuous: incoming.is_continuous,
        ph_voltage_mv: incoming.ph_voltage_mv,
    };

    debug!(
        "Nhận dữ liệu cảm biến: ph={:.2}, ec={:.2}",
        sensor_data.ph, sensor_data.ec
    );

    if let Some(ph_voltage_mv) = incoming.ph_voltage_mv {
        let observed_at = chrono::DateTime::parse_from_rfc3339(&sensor_data.time)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let mut sample_map = app_state.ph_voltage_samples.write().await;
        let samples = sample_map.entry(device_id.clone()).or_default();
        samples.push_back(crate::PhVoltageSample {
            voltage_mv: ph_voltage_mv,
            observed_at,
            received_at: std::time::Instant::now(),
        });

        while samples
            .front()
            .is_some_and(|sample| sample.received_at.elapsed().as_secs() > 120)
        {
            samples.pop_front();
        }
    }

    if let Ok(json_str) = serde_json::to_string(&sensor_data) {
        let mut states = app_state.device_states.write().await;
        states.insert(device_id.clone(), json_str);
    }

    if let Err(e) = write_sensor_data(
        &app_state.influx_client,
        &app_state.influx_bucket,
        &sensor_data,
    )
    .await
    {
        error!(error = ?e, "Lỗi lưu SensorData vào InfluxDB");
    }

    let _ = app_state.sensor_sender.send(sensor_data);
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id, node_type = %node_type))]
async fn handle_device_status(
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
    let now_iso = chrono::Utc::now().to_rfc3339();

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
        metadata: None,
    };
    let _ = app_state.alert_sender.send(alert);

    let status_payload = serde_json::json!({
        "_msg_type": "device_status",
        "is_online": is_online,
        "last_seen": now_iso
    });
    let _ = app_state.health_sender.send(status_payload);
}

/// Xây dựng metadata kết hợp dữ liệu tĩnh (từ cache cảm biến) và dữ liệu động (từ payload FSM)
#[inline]
fn build_relevant_metadata(
    state: &str,
    cache: Option<&serde_json::Value>,
    fsm_payload: &serde_json::Value,
) -> Option<serde_json::Value> {
    let mut m = serde_json::Map::new();

    // 1. Lấy trạng thái cảm biến hiện tại từ cache
    if let Some(c) = cache {
        let keys = match state {
            s if s.starts_with("EmergencyStop") || s.starts_with("SystemFault") => {
                vec![
                    "ec",
                    "ph",
                    "temp",
                    "water_level",
                    "err_ec",
                    "err_ph",
                    "err_temp",
                    "err_water",
                    "time",
                ]
            }
            "DosingPumpA" | "DosingPumpB" | "DosingPH" | "StartingOsakaPump" => {
                vec!["ec", "ph", "time"]
            }
            "DosingCycleComplete" | "Stabilizing" => vec!["ec", "ph", "temp", "time"],
            "WaterRefilling" | "WaterDraining" => vec!["water_level", "ec", "time"],
            s if s.starts_with("SensorCalibration") => vec!["ph", "ph_voltage_mv", "time"],
            _ => vec![],
        };
        for k in keys {
            if let Some(v) = c.get(k) {
                m.insert(k.to_string(), v.clone());
            }
        }
    }

    // 2. Lấy thêm thông tin hành động bơm thực tế từ MQTT FSM payload
    // Map sao cho Frontend nhận diện đúng: pump_a_ml, pump_b_ml, ph_up_ml, ph_down_ml
    let dose_ml = fsm_payload
        .get("dose_target_ml")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let is_up = fsm_payload
        .get("is_up")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    match state {
        "DosingPumpA" => {
            m.insert("pump_a_ml".to_string(), json!(dose_ml));
        }
        "DosingPumpB" => {
            m.insert("pump_b_ml".to_string(), json!(dose_ml));
        }
        "DosingPH" => {
            if is_up {
                m.insert("ph_up_ml".to_string(), json!(dose_ml));
            } else {
                m.insert("ph_down_ml".to_string(), json!(dose_ml));
            }
        }
        "WaterRefilling" | "WaterDraining" => {
            // Lấy lượng nước mục tiêu nếu FSM có gửi
            if let Some(target) = fsm_payload.get("target_level") {
                m.insert("target_level".to_string(), target.clone());
            }
        }
        _ => {}
    }

    if m.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(m))
    }
}

/// Map trạng thái FSM sang AlertMessage (sử dụng FSM payload để lấy log chính xác)
fn fsm_state_to_alert(
    state: &str,
    device_id: &str,
    alert_metadata: Option<serde_json::Value>,
    fsm_payload: &serde_json::Value,
) -> Option<AlertMessage> {
    let ts = chrono::Utc::now().timestamp_millis() as u64;

    let make = |level: &str, title: &str, message: &str| -> Option<AlertMessage> {
        Some(AlertMessage {
            level: level.to_string(),
            title: title.to_string(),
            message: message.to_string(),
            device_id: device_id.to_string(),
            timestamp: ts,
            reason: None,
            metadata: alert_metadata.clone(),
        })
    };

    if let Some(reason) = state.strip_prefix("EmergencyStop:") {
        return Some(AlertMessage {
            level: "critical".to_string(),
            title: "Dừng Khẩn Cấp!".to_string(),
            message: format!("Hệ thống bị ngắt khẩn cấp. Lý do: {}", reason),
            device_id: device_id.to_string(),
            timestamp: ts,
            reason: Some(reason.to_string()),
            metadata: alert_metadata.clone(),
        });
    }

    if let Some(reason) = state.strip_prefix("SystemFault:") {
        return Some(AlertMessage {
            level: "critical".to_string(),
            title: "Lỗi Hệ Thống!".to_string(),
            message: format!("Phát hiện lỗi phần cứng: {}. Vui lòng kiểm tra!", reason),
            device_id: device_id.to_string(),
            timestamp: ts,
            reason: Some(reason.to_string()),
            metadata: alert_metadata.clone(),
        });
    }

    if let Some(reason) = state.strip_prefix("Warning:") {
        return Some(AlertMessage {
            level: "warning".to_string(),
            title: "Cảnh Báo Hệ Thống".to_string(),
            message: format!("Phát hiện cảnh báo: {}", reason),
            device_id: device_id.to_string(),
            timestamp: ts,
            reason: Some(reason.to_string()),
            metadata: alert_metadata.clone(),
        });
    }

    if let Some(msg) = state.strip_prefix("LogInfo:") {
        return make("info", "Nhật Ký (Log)", msg);
    }

    if state.starts_with("SensorCalibration:") {
        let step = state.replace("SensorCalibration:", "");
        return make(
            "info",
            "Hiệu Chuẩn Cảm Biến",
            &format!("Đang hiệu chuẩn tại bước: {}.", step),
        );
    }

    if state.starts_with("Cooldown:") {
        return make(
            "info",
            "Hạ Nhiệt Bơm (Cooldown)",
            "Hệ thống đang chờ nguội trước khi tiếp tục châm phân.",
        );
    }

    match state {
        "SystemBooting" => make(
            "success",
            "Khởi Động Hệ Thống",
            "Trạm điều khiển vừa được cấp nguồn và đang hoạt động.",
        ),
        "ManualMode" => make(
            "info",
            "Điều Khiển Thủ Công",
            "Đang ở chế độ Manual. Hệ thống tắt tự động hóa.",
        ),
        "DosingCycleComplete" => make(
            "success",
            "Hoàn Tất Chu Trình",
            "Chu trình châm phân và điều chỉnh pH đã hoàn thành.",
        ),
        "EmergencyStop" => Some(AlertMessage {
            level: "critical".to_string(),
            title: "Dừng Khẩn Cấp!".to_string(),
            message: "Hệ thống đã bị ngắt khẩn cấp do vi phạm ngưỡng an toàn.".to_string(),
            device_id: device_id.to_string(),
            timestamp: ts,
            reason: None,
            metadata: alert_metadata.clone(),
        }),
        "WaterRefilling" => make("info", "Cấp Nước", "Hệ thống đang bơm cấp nước vào bồn."),
        "WaterDraining" => make("info", "Xả Nước", "Hệ thống đang xả bớt nước trong bồn."),
        "DosingPumpA" => {
            let dose_ml = fsm_payload
                .get("dose_target_ml")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            make(
                "info",
                "Châm Phân A",
                &format!(
                    "Đang tiến hành châm {:.1}ml phân bón Dinh Dưỡng A.",
                    dose_ml
                ),
            )
        }
        "DosingPumpB" => {
            let dose_ml = fsm_payload
                .get("dose_target_ml")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            make(
                "info",
                "Châm Phân B",
                &format!(
                    "Đang tiến hành châm {:.1}ml phân bón Dinh Dưỡng B.",
                    dose_ml
                ),
            )
        }
        "DosingPH" => {
            let is_up = fsm_payload
                .get("is_up")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let dose_ml = fsm_payload
                .get("dose_target_ml")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let direction = if is_up { "Tăng (Up)" } else { "Giảm (Down)" };
            make(
                "info",
                "Điều Chỉnh pH",
                &format!("Đang bơm {:.1}ml dung dịch pH {}.", dose_ml, direction),
            )
        }
        "ActiveMixing" => make(
            "info",
            "Sục Trộn Dinh Dưỡng",
            "Đang trộn đều dung dịch trong bồn (Jet Mixing).",
        ),
        "StartingOsakaPump" | "WaitingBetweenDose" | "Stabilizing" | "Monitoring" => {
            debug!("[FSM] Trạng thái nội bộ: {}", state);
            None
        }
        _ => {
            debug!("[FSM] Trạng thái không xác định: {}", state);
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum PrefixedFsmEventType {
    WaterEvent,
    SystemAlert,
    DosingCycle,
    EmaUpdate,
    AutoTune,
    SensorNoise,
}

#[inline]
fn parse_prefixed_fsm_message(
    raw_payload: &str,
) -> Option<(PrefixedFsmEventType, serde_json::Value)> {
    const PREFIXES: [(&str, PrefixedFsmEventType); 6] = [
        ("[WATER EVENT]", PrefixedFsmEventType::WaterEvent),
        ("[SYSTEM ALERT]", PrefixedFsmEventType::SystemAlert),
        ("[DOSING CYCLE]", PrefixedFsmEventType::DosingCycle),
        ("[EMA UPDATE]", PrefixedFsmEventType::EmaUpdate),
        ("[AUTO TUNE]", PrefixedFsmEventType::AutoTune),
        ("[SENSOR NOISE]", PrefixedFsmEventType::SensorNoise),
    ];

    for (prefix, event_type) in PREFIXES {
        if let Some(rest) = raw_payload.strip_prefix(prefix) {
            let parsed = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
            return Some((event_type, parsed));
        }
    }

    None
}

fn prefixed_event_to_system_record(
    device_id: String,
    event_type: PrefixedFsmEventType,
    payload: serde_json::Value,
) -> NewSystemEventRecord {
    let now = chrono::Utc::now().timestamp_millis();
    match event_type {
        PrefixedFsmEventType::WaterEvent => {
            let trigger = payload
                .get("trigger")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let success = payload
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let is_draining = trigger.contains("drain")
                || payload
                    .get("action")
                    .and_then(|v| v.as_str())
                    .is_some_and(|a| a.contains("drain"));
            NewSystemEventRecord {
                device_id,
                level: if success { "info" } else { "warning" }.to_string(),
                category: "water".to_string(),
                title: if is_draining {
                    "Xả nước".to_string()
                } else {
                    "Cấp nước".to_string()
                },
                message: format!("Sự kiện nước: trigger={} | success={}", trigger, success),
                reason: None,
                metadata: Some(payload),
                timestamp: now,
            }
        }
        PrefixedFsmEventType::SystemAlert => {
            let alert_type = payload
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let source = payload
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let message = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Không có mô tả");
            let (level, title) = match alert_type {
                "rate_limit" => ("warning", "Vượt giới hạn an toàn"),
                "fault" => ("critical", "Lỗi thiết bị"),
                _ => ("warning", "Cảnh báo hệ thống"),
            };
            NewSystemEventRecord {
                device_id,
                level: level.to_string(),
                category: "alert".to_string(),
                title: title.to_string(),
                message: format!("[{}] {}", source, message),
                reason: Some(alert_type.to_string()),
                metadata: Some(payload),
                timestamp: now,
            }
        }
        PrefixedFsmEventType::DosingCycle => {
            let cycle_id = payload
                .get("cycle_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let trigger = payload
                .get("trigger")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            NewSystemEventRecord {
                device_id,
                level: "success".to_string(),
                category: "dosing".to_string(),
                title: "Chu trình châm phân".to_string(),
                message: format!("Hoàn tất chu trình {} (trigger={})", cycle_id, trigger),
                reason: None,
                metadata: Some(payload),
                timestamp: now,
            }
        }
        PrefixedFsmEventType::EmaUpdate => NewSystemEventRecord {
            device_id,
            level: "info".to_string(),
            category: "calibration".to_string(),
            title: "Cập nhật hệ số EMA".to_string(),
            message: "Hệ số EMA runtime đã được cập nhật".to_string(),
            reason: None,
            metadata: Some(payload),
            timestamp: now,
        },
        PrefixedFsmEventType::AutoTune => NewSystemEventRecord {
            device_id,
            level: "info".to_string(),
            category: "calibration".to_string(),
            title: "Tự điều chỉnh bước châm".to_string(),
            message: "Auto-tune đã điều chỉnh thông số dosing".to_string(),
            reason: None,
            metadata: Some(payload),
            timestamp: now,
        },
        PrefixedFsmEventType::SensorNoise => {
            let sensor = payload
                .get("sensor")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            NewSystemEventRecord {
                device_id,
                level: "warning".to_string(),
                category: "sensor".to_string(),
                title: "Nhiễu cảm biến".to_string(),
                message: format!("Phát hiện mẫu nhiễu từ cảm biến {}", sensor),
                reason: None,
                metadata: Some(payload),
                timestamp: now,
            }
        }
    }
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
async fn handle_fsm_state(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let raw_payload = std::str::from_utf8(payload).unwrap_or("Lỗi UTF-8");
    info!("📥 [MQTT-FSM] nhận gói tin: {}", raw_payload);

    if let Some((event_type, parsed_payload)) = parse_prefixed_fsm_message(raw_payload) {
        let event = prefixed_event_to_system_record(device_id, event_type, parsed_payload);

        if let Err(e) = insert_system_event(&app_state.pg_pool, &event).await {
            error!(error = ?e, "❌ [MQTT-FSM] Lỗi lưu prefixed event vào DB");
        }
        return;
    }

    let json = match serde_json::from_slice::<serde_json::Value>(payload) {
        Ok(j) => j,
        Err(e) => {
            error!("❌ [MQTT-FSM] Cấu trúc JSON bị sai định dạng: {:?}", e);
            return;
        }
    };

    if let Some(msg_type) = json.get("type").and_then(|t| t.as_str()) {
        if msg_type == "runtime_calibration_update" {
            handle_runtime_calibration_update(device_id, &json, app_state.clone()).await;
            return;
        }
    }

    let state = match json.get("current_state").and_then(|s| s.as_str()) {
        Some(s) => s.to_string(),
        None => {
            error!("❌ [MQTT-FSM] JSON hợp lệ nhưng thiếu trường 'current_state'!");
            return;
        }
    };

    // Luôn gửi FSM_UPDATE để badge & badge realtime trên UI cập nhật ngay
    let fsm_sync_msg = AlertMessage {
        level: "FSM_UPDATE".to_string(),
        title: "FSM_SYNC".to_string(),
        message: state.clone(),
        device_id: device_id.clone(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reason: None,
        metadata: None,
    };
    let _ = app_state.alert_sender.send(fsm_sync_msg);

    // MỚI: Build metadata kết hợp (Sensors cache + FSM json payload)
    let alert_metadata = {
        let states = app_state.device_states.read().await;
        let cache = states
            .get(&device_id)
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
        build_relevant_metadata(&state, cache.as_ref(), &json)
    };

    // Truyền FSM payload vào để bóc tách text động
    if let Some(alert_msg) = fsm_state_to_alert(&state, &device_id, alert_metadata, &json) {
        if alert_msg.level == "critical" || alert_msg.level == "warning" {
            info!("🚨 KÍCH HOẠT BÁO ĐỘNG: {}", alert_msg.title);
        } else {
            info!("ℹ️ THAY ĐỔI TRẠNG THÁI: {}", alert_msg.title);
        }

        let _ = app_state.alert_sender.send(alert_msg.clone());

        // Push notification với trạng thái nghiêm trọng
        if alert_msg.level == "critical" || alert_msg.level == "warning" {
            let tokens = app_state.fcm_tokens.lock().unwrap().clone();
            if !tokens.is_empty() {
                tokio::spawn(async move {
                    crate::services::fcm::send_push_notification(
                        &alert_msg.title,
                        &alert_msg.message,
                        tokens,
                    )
                    .await;
                });
            }
        }
    }
}

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
async fn handle_dosing_report(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let report: DosingReportPayload = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!(error = ?e, "Lỗi parse DosingReport");
            return;
        }
    };

    info!(
        "🌿 Báo cáo châm phân: A: {:.2}ml, B: {:.2}ml. Đang lưu vào Database...",
        report.pump_a_ml, report.pump_b_ml
    );

    update_dosing_dynamic_learning(&device_id, &report, &app_state).await;

    let season_id_opt =
        match crate::db::postgres::get_active_crop_season(&app_state.pg_pool, &device_id).await {
            Ok(Some(season)) => Some(season.id.to_string()),
            _ => None,
        };

    let report_payload = json!({
        "device_id": device_id,
        "season_id": season_id_opt,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "dosing_data": report
    });

    if let Err(db_err) = crate::db::postgres::insert_dosing_report(
        &app_state.pg_pool,
        &device_id,
        season_id_opt.as_deref(),
        report.pump_a_ml,
        report.pump_b_ml,
        report.ph_up_ml,
        report.ph_down_ml,
        &report_payload,
    )
    .await
    {
        error!("❌ Lỗi lưu báo cáo châm phân vào Database: {:?}", db_err);
        return;
    }

    let alert_msg_text = format!(
        "Đã lưu báo cáo châm phân: A: {:.1}ml | B: {:.1}ml | pH Up: {:.1}ml | pH Down: {:.1}ml",
        report.pump_a_ml, report.pump_b_ml, report.ph_up_ml, report.ph_down_ml
    );

    let _ = crate::db::postgres::insert_system_event(
        &app_state.pg_pool,
        &crate::db::postgres::NewSystemEventRecord {
            device_id: device_id.clone(),
            level: "success".to_string(),
            category: "dosing".to_string(),
            title: "Lưu Báo Cáo Châm Phân Thành Công".to_string(),
            message: alert_msg_text.clone(),
            reason: None,
            metadata: Some(json!({"dosing_report": report})),
            timestamp: chrono::Utc::now().timestamp_millis(),
        },
    )
    .await;

    let alert = AlertMessage {
        level: "success".to_string(),
        title: "Lưu Báo Cáo Châm Phân Thành Công".to_string(),
        message: alert_msg_text,
        device_id: device_id.clone(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reason: None,
        metadata: None,
    };
    let _ = app_state.alert_sender.send(alert);
}

// Giữ nguyên các hàm `update_dosing_dynamic_learning` và `handle_runtime_calibration_update` như cũ
async fn update_dosing_dynamic_learning(
    device_id: &str,
    report: &DosingReportPayload,
    app_state: &web::Data<AppState>,
) {
    const MAX_SAMPLES: usize = 50;
    const SIGNIFICANT_COEF_DELTA_RATIO: f32 = 0.1;

    let dosing_cfg_res = sqlx::query_as::<_, DosingCalibration>(
        "SELECT * FROM dosing_calibration WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_optional(&app_state.pg_pool)
    .await;

    let dosing_cfg = match dosing_cfg_res {
        Ok(Some(cfg)) => cfg,
        Ok(None) => return,
        Err(e) => {
            warn!(
                "Không thể đọc dosing_calibration để học hệ số động {}: {:?}",
                device_id, e
            );
            return;
        }
    };

    let total_dosed_ml = report.pump_a_ml + report.pump_b_ml;
    if total_dosed_ml <= 0.0 || dosing_cfg.ec_gain_per_ml <= 0.0 {
        return;
    }

    let before_ec = report.before_ec.unwrap_or(report.start_ec);
    let after_ec = report.after_ec;
    let stabilized_ec = report.stabilized_ec.or(report.after_ec);

    let before_ph = report.before_ph.unwrap_or(report.start_ph);
    let after_ph = report.after_ph;
    let stabilized_ph = report.stabilized_ph.or(report.after_ph);

    let Some(stabilized_ec_value) = stabilized_ec else {
        return;
    };

    let observed_gain = (stabilized_ec_value - before_ec) / total_dosed_ml;
    if !observed_gain.is_finite() || observed_gain <= 0.0 {
        return;
    }

    let target_gain = (report.target_ec - before_ec) / total_dosed_ml;
    let quality = if target_gain.is_finite() && target_gain.abs() > f32::EPSILON {
        (1.0 - ((observed_gain - target_gain).abs() / target_gain.abs())).clamp(0.0, 1.0)
    } else {
        0.5
    };

    let sample = crate::DosingLearningSample {
        before_ec: Some(before_ec),
        after_ec,
        stabilized_ec: Some(stabilized_ec_value),
        before_ph: Some(before_ph),
        after_ph,
        stabilized_ph,
        stabilized_window_sec: report.stabilized_window_sec,
        reported_at: chrono::Utc::now(),
    };

    let mut states = app_state.dosing_dynamic_states.write().await;
    let state = states
        .entry(device_id.to_string())
        .or_insert_with(|| crate::DosingDynamicState {
            base_ec_gain_per_ml: dosing_cfg.ec_gain_per_ml,
            dynamic_ec_gain_per_ml: dosing_cfg.ec_gain_per_ml,
            confidence: 0.0,
            sample_count: 0,
            last_updated: chrono::Utc::now(),
            samples: std::collections::VecDeque::new(),
        });

    state.base_ec_gain_per_ml = dosing_cfg.ec_gain_per_ml;
    state.samples.push_back(sample);
    while state.samples.len() > MAX_SAMPLES {
        state.samples.pop_front();
    }

    let previous_dynamic = state.dynamic_ec_gain_per_ml;
    let observed_dynamic = observed_gain.clamp(
        dosing_cfg.ec_gain_per_ml * 0.5,
        dosing_cfg.ec_gain_per_ml * 1.5,
    );
    let alpha = 0.18;
    state.dynamic_ec_gain_per_ml =
        ((1.0 - alpha) * state.dynamic_ec_gain_per_ml + alpha * observed_dynamic).max(0.0001);
    state.sample_count = state.samples.len() as u32;
    let sample_confidence = (state.sample_count as f32 / 20.0).clamp(0.0, 1.0);
    state.confidence = ((state.confidence * 0.8) + (quality * 0.2)).max(sample_confidence * 0.6);
    state.last_updated = chrono::Utc::now();

    let delta_ratio = if previous_dynamic.abs() > f32::EPSILON {
        ((state.dynamic_ec_gain_per_ml - previous_dynamic).abs() / previous_dynamic.abs()).abs()
    } else {
        0.0
    };

    if delta_ratio >= SIGNIFICANT_COEF_DELTA_RATIO {
        let _ = insert_system_event(
            &app_state.pg_pool,
            &NewSystemEventRecord {
                device_id: device_id.to_string(),
                level: "info".to_string(),
                category: "calibration".to_string(),
                title: "Cập nhật hệ số châm phân động".to_string(),
                message: format!(
                    "Hệ số EC động thay đổi từ {:.5} lên {:.5} (Δ {:.1}%)",
                    previous_dynamic,
                    state.dynamic_ec_gain_per_ml,
                    delta_ratio * 100.0
                ),
                reason: None,
                metadata: Some(json!({
                    "base_ec_gain_per_ml": state.base_ec_gain_per_ml,
                    "dynamic_ec_gain_per_ml": state.dynamic_ec_gain_per_ml,
                    "confidence": state.confidence,
                    "sample_count": state.sample_count,
                    "latest_sample": state.samples.back(),
                    "stabilized_window_sec": report.stabilized_window_sec
                })),
                timestamp: chrono::Utc::now().timestamp_millis(),
            },
        )
        .await;
    }
}

async fn handle_runtime_calibration_update(
    device_id: String,
    json: &serde_json::Value,
    app_state: web::Data<AppState>,
) {
    info!(
        "🛠️ [EMA CALIBRATION] {} gửi yêu cầu cập nhật hệ số runtime...",
        device_id
    );

    // Kiểm tra xem controller có thực sự yêu cầu persist không
    let persist = json
        .get("persist")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !persist {
        debug!("ℹ️ [EMA CALIBRATION] Bỏ qua lưu DB vì persist = false");
        return;
    }

    let coeffs = match json.get("runtime_coefficients") {
        Some(c) => c,
        None => {
            warn!("⚠️ [EMA CALIBRATION] Thiếu 'runtime_coefficients' trong payload");
            return;
        }
    };

    // Lấy các hệ số. Nếu giá trị là null, as_f64() sẽ tự động trả về None
    let ec_gain = coeffs
        .get("ec_gain_per_ml")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32);
    let ph_up = coeffs
        .get("ph_shift_up_per_ml")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32);
    let ph_down = coeffs
        .get("ph_shift_down_per_ml")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32);

    if ec_gain.is_none() && ph_up.is_none() && ph_down.is_none() {
        debug!("ℹ️ [EMA CALIBRATION] Không có hệ số nào mới để cập nhật.");
        return;
    }

    // Câu lệnh SQL linh hoạt: Chỉ update các cột có giá trị (khác NULL).
    // Nếu controller gửi lên NULL (vì chưa tính được) -> Giữ nguyên (COALESCE).
    let query = r#"
        UPDATE dosing_calibration
        SET
            ec_gain_per_ml = COALESCE($1::real, ec_gain_per_ml),
            ph_shift_up_per_ml = COALESCE($2::real, ph_shift_up_per_ml),
            ph_shift_down_per_ml = COALESCE($3::real, ph_shift_down_per_ml),
            last_calibrated = NOW()
        WHERE device_id = $4
    "#;

    match sqlx::query(query)
        .bind(ec_gain)
        .bind(ph_up)
        .bind(ph_down)
        .bind(&device_id)
        .execute(&app_state.pg_pool)
        .await
    {
        Ok(res) => {
            if res.rows_affected() > 0 {
                info!(
                    "✅ [EMA CALIBRATION] Cập nhật thành công DB cho {}",
                    device_id
                );

                // Tạo log SystemEvent để lưu lại lịch sử
                let msg = format!(
                    "Controller gửi hệ số mới (EMA). Cập nhật DB: EC Gain: {:?}, pH Up: {:?}, pH Down: {:?}",
                    ec_gain, ph_up, ph_down
                );

                let _ = insert_system_event(
                    &app_state.pg_pool,
                    &NewSystemEventRecord {
                        device_id: device_id.clone(),
                        level: "info".to_string(),
                        category: "calibration".to_string(),
                        title: "Runtime Calibration Tự Động (EMA)".to_string(),
                        message: msg,
                        reason: None,
                        metadata: Some(json.clone()), // Lưu nguyên JSON để sau này tiện debug
                        timestamp: chrono::Utc::now().timestamp_millis(),
                    },
                )
                .await;

                // (Optional) Gửi alert để UI hiện popup
                let alert = AlertMessage {
                    level: "info".to_string(),
                    title: "Cập nhật hệ số Calibration".to_string(),
                    message: format!(
                        "Hệ thống vừa cập nhật tự động (EMA) hệ số châm phân cho thiết bị {}.",
                        device_id
                    ),
                    device_id: device_id.clone(),
                    timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    reason: None,
                    metadata: Some(json.clone()),
                };
                let _ = app_state.alert_sender.send(alert);
            } else {
                warn!(
                    "⚠️ [EMA CALIBRATION] Không tìm thấy bản ghi dosing_calibration nào cho {}",
                    device_id
                );
            }
        }
        Err(e) => {
            error!("❌ [EMA CALIBRATION] Lỗi khi cập nhật Database: {:?}", e);
        }
    }
}
