use actix_web::web;
use rumqttc::Publish;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info, instrument, warn};

use crate::AppState;
use crate::db::influx::write_sensor_data;
use crate::models::alert::AlertMessage;
use crate::models::sensor::{PumpStatus, SensorData};

#[derive(Debug, Deserialize, Serialize)]
pub struct DosingReportPayload {
    pub start_ec: f32,
    pub start_ph: f32,
    pub pump_a_ml: f32,
    pub pump_b_ml: f32,
    pub ph_up_ml: f32,
    pub ph_down_ml: f32,
    pub target_ec: f32,
    pub target_ph: f32,
}

#[derive(Deserialize)]
struct DeviceStatusPayload {
    pub online: bool,
}

#[derive(Debug, Deserialize)]
pub struct IncomingSensorPayload {
    pub temp: Option<f64>,
    pub ec: Option<f64>,
    pub ph: Option<f64>,
    pub water_level: Option<f64>,
    #[serde(rename = "last_update_ms")]
    pub timestamp_ms: Option<u64>,
    pub pump_status: Option<PumpStatus>,

    pub rssi: Option<i32>,
    pub free_heap: Option<u32>,
    pub uptime: Option<u32>,

    pub err_water: Option<bool>,
    pub err_temp: Option<bool>,
    pub err_ph: Option<bool>,
    pub err_ec: Option<bool>,

    pub is_continuous: Option<bool>,
}

/// Extracts (device_id, suffix) from a topic like "AGITECH/device_001/sensors"
/// Returns None if the topic doesn't start with "AGITECH/".
fn parse_agitech_topic(topic: &str) -> Option<(String, String)> {
    let prefix = "AGITECH/";
    if !topic.starts_with(prefix) {
        return None;
    }
    let rest = &topic[prefix.len()..];
    // rest is now "device_001/sensors" or similar
    let slash = rest.find('/')?;
    let device_id = rest[..slash].to_string();
    let suffix = rest[slash..].to_string(); // includes leading slash, e.g. "/sensors"
    Some((device_id, suffix))
}

#[instrument(skip(app_state, publish))]
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
            // LWT của Controller Node
            handle_device_status(device_id, "Trạm Điều Khiển", &payload_bytes, app_state).await;
        }
        "/sensor/status" => {
            // LWT của Sensor Node
            handle_device_status(device_id, "Mạch Cảm Biến", &payload_bytes, app_state).await;
        }
        "/fsm" => {
            handle_fsm_state(device_id, &payload_bytes, app_state).await;
        }
        "/dosing_report" => {
            handle_dosing_report(device_id, &payload_bytes, app_state).await;
        }
        "/controller/status" => {
            // Periodic health heartbeat from the Controller Node
            if let Ok(payload_json) = serde_json::from_slice::<serde_json::Value>(&payload_bytes) {
                // 1. CẬP NHẬT CACHE TRONG RAM ĐỂ API /sensors/latest LẤY ĐƯỢC DỮ LIỆU MỚI NHẤT
                // Merge toàn bộ heartbeat payload (không chỉ pump_status) để giữ snapshot cảm biến đầy đủ.
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

                if let Ok(updated_str) = serde_json::to_string(&merged) {
                    states.insert(device_id.clone(), updated_str);
                }

                // 2. Đẩy qua WebSocket (code cũ)
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

async fn handle_sensor_data(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let incoming: IncomingSensorPayload = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!(
                "Lỗi parse JSON SensorData từ thiết bị {}: {:?}",
                device_id, e
            );
            return;
        }
    };

    let time = incoming
        .timestamp_ms
        .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64))
        .unwrap_or_else(|| chrono::Utc::now())
        .to_rfc3339();

    let sensor_data = SensorData {
        device_id: device_id.clone(),
        temp_value: incoming.temp.unwrap_or(0.0),
        ec_value: incoming.ec.unwrap_or(0.0),
        ph_value: incoming.ph.unwrap_or(0.0),
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
    };

    debug!(
        "Nhận dữ liệu cảm biến từ {}: ph={:.2}, ec={:.2}",
        device_id, sensor_data.ph_value, sensor_data.ec_value
    );

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
        error!("Lỗi lưu SensorData vào InfluxDB ({}): {:?}", device_id, e);
    }

    let _ = app_state.sensor_sender.send(sensor_data);
}

/// Handles LWT-style status messages from either the Controller or Sensor node.
///
/// We send the status via TWO channels so the frontend can always receive it:
///   1. `alert_sender`  — goes through the "alert" WS message path (legacy support)
///   2. `health_sender` — goes through the "device_status" WS message path (preferred)
///
/// The frontend's DeviceContext handles both paths and calls `resetControllerTimeout()`
/// on either, so there is no race between the two.
async fn handle_device_status(
    device_id: String,
    node_type: &str,
    payload: &[u8],
    app_state: web::Data<AppState>,
) {
    let status: DeviceStatusPayload = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!(
                "Lỗi parse DeviceStatus từ {} ({}): {:?}",
                device_id, node_type, e
            );
            return;
        }
    };

    let is_online = status.online;
    let now_iso = chrono::Utc::now().to_rfc3339();

    info!(
        "[{}] {} trạng thái: {}",
        device_id,
        node_type,
        if is_online { "ONLINE" } else { "OFFLINE (LWT)" }
    );

    // ── 1. Send via alert_sender (renders in SystemLog, legacy WS path) ───────
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

    // ── 2. Send via health_sender as a proper device_status packet ────────────
    // The WS handler (ws.rs) checks for `_msg_type == "device_status"` and
    // re-wraps this as { type: "device_status", payload: { is_online, last_seen } }
    // which the frontend DeviceContext handles in the `data.type === 'device_status'` branch.
    let status_payload = serde_json::json!({
        "_msg_type": "device_status",
        "is_online": is_online,
        "last_seen": now_iso
    });
    let _ = app_state.health_sender.send(status_payload);
}

async fn handle_fsm_state(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let raw_payload = std::str::from_utf8(payload).unwrap_or("Lỗi UTF-8");
    info!("📥 [MQTT-FSM] {} gửi gói tin: {}", device_id, raw_payload);

    match serde_json::from_slice::<serde_json::Value>(payload) {
        Ok(json) => {
            if let Some(state) = json["current_state"].as_str() {
                // Always broadcast FSM state to frontend first
                let fsm_sync_msg = AlertMessage {
                    level: "FSM_UPDATE".to_string(),
                    title: "FSM_SYNC".to_string(),
                    message: state.to_string(),
                    device_id: device_id.clone(),
                    timestamp: chrono::Utc::now().timestamp_millis() as u64,
                    reason: None,
                    metadata: None,
                };
                let _ = app_state.alert_sender.send(fsm_sync_msg);

                let mut alert: Option<AlertMessage> = None;

                let current_sensors = app_state.device_states.read().await;
                let metadata_json = if let Some(sensor_str) = current_sensors.get(&device_id) {
                    serde_json::from_str::<serde_json::Value>(sensor_str).ok()
                } else {
                    None
                };
                let alert_metadata =
                    build_sensor_snapshot(&device_id, state, metadata_json.clone());

                match state {
                    "SystemBooting" => {
                        alert = Some(AlertMessage {
                            level: "success".to_string(),
                            title: "Khởi Động Hệ Thống".to_string(),
                            message: "Trạm điều khiển vừa được cấp nguồn và đang hoạt động."
                                .to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    "ManualMode" => {
                        alert = Some(AlertMessage {
                            level: "info".to_string(),
                            title: "Điều Khiển Thủ Công".to_string(),
                            message: "Đang ở chế độ Manual (Thủ công). Hệ thống tắt tự động hóa."
                                .to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    "CleaningMode" => {
                        alert = Some(AlertMessage {
                            level: "info".to_string(),
                            title: "Chế Độ Súc Rửa".to_string(),
                            message: "Đang chạy chu trình súc rửa bồn chứa.".to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    "SensorCalibrating" => {
                        alert = Some(AlertMessage {
                            level: "info".to_string(),
                            title: "Hiệu Chuẩn Cảm Biến".to_string(),
                            message: "Hệ thống đang ở chế độ hiệu chuẩn đầu dò cảm biến."
                                .to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    "DosingCycleComplete" => {
                        alert = Some(AlertMessage {
                            level: "success".to_string(),
                            title: "Hoàn Tất Chu Trình".to_string(),
                            message: "Chu trình châm phân & điều chỉnh pH đã hoàn thành."
                                .to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    s if s.starts_with("Warning:") => {
                        let reason_str = s.replace("Warning:", "");
                        alert = Some(AlertMessage {
                            level: "warning".to_string(),
                            title: "Cảnh Báo Hệ Thống".to_string(),
                            message: format!("Phát hiện cảnh báo: {}", reason_str),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: Some(reason_str),
                            metadata: alert_metadata.clone(),
                        });
                    }
                    s if s.starts_with("LogInfo:") => {
                        let msg = s.replace("LogInfo:", "");
                        alert = Some(AlertMessage {
                            level: "info".to_string(),
                            title: "Nhật Ký (Log)".to_string(),
                            message: msg,
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    s if s.starts_with("EmergencyStop:") => {
                        let reason_str = s.replace("EmergencyStop:", "");
                        alert = Some(AlertMessage {
                            level: "critical".to_string(),
                            title: "Dừng Khẩn Cấp!".to_string(),
                            message: format!("Hệ thống bị ngắt khẩn cấp. Lý do: {}", reason_str),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: Some(reason_str),
                            metadata: alert_metadata.clone(),
                        });
                    }
                    "EmergencyStop" => {
                        alert = Some(AlertMessage {
                            level: "critical".to_string(),
                            title: "Dừng Khẩn Cấp!".to_string(),
                            message: "Hệ thống đã bị ngắt khẩn cấp do vi phạm ngưỡng an toàn."
                                .to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: alert_metadata.clone(),
                        });
                    }
                    s if s.starts_with("SystemFault:") => {
                        let reason_str = s.replace("SystemFault:", "");
                        alert = Some(AlertMessage {
                            level: "critical".to_string(),
                            title: "Lỗi Hệ Thống!".to_string(),
                            message: format!(
                                "Phát hiện lỗi phần cứng: {}. Vui lòng kiểm tra!",
                                reason_str
                            ),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: Some(reason_str),
                            metadata: alert_metadata.clone(),
                        });
                    }
                    "WaterRefilling" => {
                        alert = Some(AlertMessage {
                            level: "info".to_string(),
                            title: "Cấp Nước".to_string(),
                            message: "Hệ thống đang tiến hành bơm cấp nước vào bồn.".to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    "WaterDraining" => {
                        alert = Some(AlertMessage {
                            level: "info".to_string(),
                            title: "Xả Nước".to_string(),
                            message: "Hệ thống đang xả bớt nước trong bồn.".to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    "DosingPumpA" => {
                        alert = Some(AlertMessage {
                            level: "info".to_string(),
                            title: "Châm Phân".to_string(),
                            message: "Đang tiến hành châm phân bón Dinh Dưỡng A.".to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    "DosingPumpB" => {
                        alert = Some(AlertMessage {
                            level: "info".to_string(),
                            title: "Châm Phân".to_string(),
                            message: "Đang tiến hành châm phân bón Dinh Dưỡng B.".to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    "DosingPH" => {
                        alert = Some(AlertMessage {
                            level: "info".to_string(),
                            title: "Điều Chỉnh pH".to_string(),
                            message: "Đang tiến hành bơm dung dịch điều chỉnh pH.".to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    "ActiveMixing" => {
                        alert = Some(AlertMessage {
                            level: "info".to_string(),
                            title: "Sục Trộn Dinh Dưỡng".to_string(),
                            message: "Đang trộn đều dung dịch trong bồn (Jet Mixing).".to_string(),
                            device_id: device_id.clone(),
                            timestamp: chrono::Utc::now().timestamp_millis() as u64,
                            reason: None,
                            metadata: None,
                        });
                    }
                    "StartingOsakaPump" => {
                        debug!("Bắt đầu khởi động bơm trung tâm (Osaka).");
                    }
                    "WaitingBetweenDose" => {
                        debug!("Đang chờ hòa tan giữa 2 lần châm A và B.");
                    }
                    "Stabilizing" => {
                        debug!("Đang chờ cảm biến đọc số liệu ổn định.");
                    }
                    "Monitoring" => {
                        debug!("Hệ thống đang ở trạng thái giám sát (Monitoring).");
                    }
                    _ => {
                        debug!("Trạng thái FSM khác: {}", state);
                    }
                }

                if let Some(alert_msg) = alert {
                    if alert_msg.level == "critical" || alert_msg.level == "warning" {
                        info!("🚨 KÍCH HOẠT BÁO ĐỘNG: {}", alert_msg.title);
                    } else {
                        info!("ℹ️ THAY ĐỔI TRẠNG THÁI: {}", alert_msg.title);
                    }

                    let _ = app_state.alert_sender.send(alert_msg.clone());

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
            } else {
                error!("❌ [MQTT-FSM] JSON hợp lệ nhưng bị thiếu trường 'current_state'!");
            }
        }
        Err(e) => {
            error!("❌ [MQTT-FSM] Cấu trúc JSON bị sai định dạng: {:?}", e);
        }
    }
}

fn build_sensor_snapshot(
    device_id: &str,
    fsm_state: &str,
    metadata_json: Option<serde_json::Value>,
) -> Option<serde_json::Value> {
    match metadata_json {
        Some(sensor_snapshot) => Some(json!({
            "captured_at": chrono::Utc::now().to_rfc3339(),
            "fsm_state": fsm_state,
            "snapshot_source": "device_state_cache",
            "sensor_snapshot": sensor_snapshot
        })),
        None => {
            warn!(
                "Không tìm thấy sensor cache cho device_id={} khi nhận FSM state={}",
                device_id, fsm_state
            );
            Some(json!({
                "captured_at": chrono::Utc::now().to_rfc3339(),
                "fsm_state": fsm_state,
                "snapshot_source": "fsm_fallback",
                "sensor_snapshot": {
                    "device_id": device_id
                }
            }))
        }
    }
}

async fn handle_dosing_report(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let report: DosingReportPayload = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!("Lỗi parse DosingReport từ {}: {:?}", device_id, e);
            return;
        }
    };

    info!(
        "🌿 [{}] Báo cáo châm phân: A: {:.2}ml, B: {:.2}ml. Đang ghi lên Blockchain...",
        device_id, report.pump_a_ml, report.pump_b_ml
    );

    let season_id_str =
        match crate::db::postgres::get_active_crop_season(&app_state.pg_pool, &device_id).await {
            Ok(Some(season)) => season.id.to_string(),
            _ => "".to_string(),
        };

    let blockchain_payload = json!({
        "device_id": device_id,
        "season_id": season_id_str,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "dosing_data": report
    });

    let payload_str = blockchain_payload.to_string();

    match app_state
        .solana_traceability
        .record_dosing_history(&payload_str)
        .await
    {
        Ok(tx_id) => {
            info!("✅ Đã ghi lên Solana thành công! TxID: {}", tx_id);

            let action_str = format!(
                "Châm phân tự động: A({:.1}ml), B({:.1}ml)",
                report.pump_a_ml, report.pump_b_ml
            );

            if let Err(db_err) = crate::db::postgres::insert_blockchain_tx(
                &app_state.pg_pool,
                &device_id,
                &season_id_str,
                &action_str,
                &tx_id,
            )
            .await
            {
                error!("❌ Lỗi lưu TxID vào Database: {:?}", db_err);
            }

            let alert = AlertMessage {
                level: "success".to_string(),
                title: "Ghi Blockchain Thành Công".to_string(),
                message: format!(
                    "Đã bơm: Phân A: {:.1}ml | Phân B: {:.1}ml | pH Up: {:.1}ml | pH Down: {:.1}ml\nTxID Solana: {}",
                    report.pump_a_ml, report.pump_b_ml, report.ph_up_ml, report.ph_down_ml, tx_id
                ),
                device_id: device_id.clone(),
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                reason: None,
                metadata: None,
            };
            let _ = app_state.alert_sender.send(alert);
        }
        Err(e) => {
            error!("❌ Lỗi ghi Blockchain cho {}: {:?}", device_id, e);

            let alert = AlertMessage {
                level: "warning".to_string(),
                title: "Lỗi Ghi Blockchain".to_string(),
                message: format!(
                    "Mẻ phân bón hoàn tất nhưng không thể đồng bộ Solana. Lỗi: {:?}",
                    e
                ),
                device_id: device_id.clone(),
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                reason: Some(e.to_string()),
                metadata: None,
            };
            let _ = app_state.alert_sender.send(alert);
        }
    }
}
