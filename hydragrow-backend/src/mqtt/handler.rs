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
    #[serde(default)]
    pub before_ec: Option<f32>,
    #[serde(default)]
    pub after_ec: Option<f32>,
    #[serde(default)]
    pub stabilized_ec: Option<f32>,
    #[serde(default)]
    pub before_ph: Option<f32>,
    #[serde(default)]
    pub after_ph: Option<f32>,
    #[serde(default)]
    pub stabilized_ph: Option<f32>,
    #[serde(default)]
    pub stabilized_window_sec: Option<u32>,
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

                    // Lấy trạng thái misting trước đó từ cache
                    let prev_mist = states
                        .get(&device_id)
                        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                        .and_then(|v| v.get("pump_status").cloned())
                        .and_then(|ps| ps.get("mist_valve").cloned())
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // Chỉ gửi alert khi trạng thái thay đổi
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
        "Nhận dữ liệu cảm biến từ {}: ph={:.2}, ec={:.2}",
        device_id, sensor_data.ph, sensor_data.ec
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
        error!("Lỗi lưu SensorData vào InfluxDB ({}): {:?}", device_id, e);
    }

    let _ = app_state.sensor_sender.send(sensor_data);
}

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

/// Map trạng thái FSM sang AlertMessage phù hợp để gửi WebSocket & lưu log.
/// Trả về None nếu trạng thái chỉ cần debug, không cần thông báo.
fn fsm_state_to_alert(
    state: &str,
    device_id: &str,
    alert_metadata: Option<serde_json::Value>,
) -> Option<AlertMessage> {
    let ts = chrono::Utc::now().timestamp_millis() as u64;

    // Helper closure
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

    // --- Trạng thái có prefix động ---
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

    // --- Trạng thái cố định ---
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

        "DosingPumpA" => make(
            "info",
            "Châm Phân A",
            "Đang tiến hành châm phân bón Dinh Dưỡng A.",
        ),

        "DosingPumpB" => make(
            "info",
            "Châm Phân B",
            "Đang tiến hành châm phân bón Dinh Dưỡng B.",
        ),

        "DosingPH" => make("info", "Điều Chỉnh pH", "Đang bơm dung dịch điều chỉnh pH."),

        "ActiveMixing" => make(
            "info",
            "Sục Trộn Dinh Dưỡng",
            "Đang trộn đều dung dịch trong bồn (Jet Mixing).",
        ),

        // "cleaningmode" => make(
        //     "info",
        //     "Chế Độ Súc Rửa",
        //     "Đang chạy chu trình súc rửa bồn chứa.",
        // ),

        // Trạng thái chỉ cần debug, không cần thông báo lên UI
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

async fn handle_fsm_state(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let raw_payload = std::str::from_utf8(payload).unwrap_or("Lỗi UTF-8");
    info!("📥 [MQTT-FSM] {} gửi gói tin: {}", device_id, raw_payload);

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
            return; // Đã xử lý xong, thoát hàm để không chạy logic current_state bên dưới
        }
    }

    let state = match json["current_state"].as_str() {
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

    // Xây dựng metadata snapshot cảm biến
    let metadata_json = {
        let states = app_state.device_states.read().await;
        states
            .get(&device_id)
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
    };
    // MỚI
    let alert_metadata = {
        let states = app_state.device_states.read().await;
        let cache = states
            .get(&device_id)
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());
        build_relevant_metadata(&state, cache.as_ref())
    };

    // Tạo alert (nếu trạng thái cần thông báo)
    if let Some(alert_msg) = fsm_state_to_alert(&state, &device_id, alert_metadata) {
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

/// Chỉ trích xuất trường cảm biến thực sự liên quan đến từng loại trạng thái FSM.
fn build_relevant_metadata(
    state: &str,
    cache: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let cache = cache?;

    let pick = |keys: &[&str]| -> serde_json::Value {
        let mut m = serde_json::Map::new();
        for k in keys {
            if let Some(v) = cache.get(k) {
                m.insert(k.to_string(), v.clone());
            }
        }
        serde_json::Value::Object(m)
    };

    if state.starts_with("EmergencyStop") || state.starts_with("SystemFault") {
        return Some(pick(&[
            "ec",
            "ph",
            "temp",
            "water_level",
            "err_ec",
            "err_ph",
            "err_temp",
            "err_water",
            "time",
        ]));
    }
    if matches!(
        state,
        "DosingPumpA" | "DosingPumpB" | "DosingPH" | "StartingOsakaPump"
    ) {
        return Some(pick(&["ec", "ph", "time"]));
    }
    if matches!(state, "DosingCycleComplete" | "Stabilizing") {
        return Some(pick(&["ec", "ph", "temp", "time"]));
    }
    if matches!(state, "WaterRefilling" | "WaterDraining") {
        return Some(pick(&["water_level", "ec", "time"]));
    }
    if state.starts_with("SensorCalibration") {
        return Some(pick(&["ph", "ph_voltage_mv", "time"]));
    }
    None
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

    update_dosing_dynamic_learning(&device_id, &report, &app_state).await;

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

            let season_id_opt =
                match crate::db::postgres::get_active_crop_season(&app_state.pg_pool, &device_id)
                    .await
                {
                    Ok(Some(season)) => Some(season.id.to_string()),
                    _ => None,
                };

            if let Err(db_err) = crate::db::postgres::insert_blockchain_tx(
                &app_state.pg_pool,
                &device_id,
                season_id_opt.as_deref(),
                &action_str,
                &tx_id,
            )
            .await
            {
                error!("❌ Lỗi lưu TxID vào Database: {:?}", db_err);
            }

            let alert_msg_text = format!(
                "Đã bơm: Phân A: {:.1}ml | Phân B: {:.1}ml | pH Up: {:.1}ml | pH Down: {:.1}ml\nTxID Solana: {}",
                report.pump_a_ml, report.pump_b_ml, report.ph_up_ml, report.ph_down_ml, tx_id
            );

            let _ = crate::db::postgres::insert_system_event(
                &app_state.pg_pool,
                &crate::db::postgres::NewSystemEventRecord {
                    device_id: device_id.clone(),
                    level: "success".to_string(),
                    category: "dosing".to_string(),
                    title: "Ghi Blockchain Thành Công".to_string(),
                    message: alert_msg_text.clone(),
                    reason: None,
                    metadata: Some(json!({"tx_id": tx_id, "dosing_report": report})),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                },
            )
            .await;

            let alert = AlertMessage {
                level: "success".to_string(),
                title: "Ghi Blockchain Thành Công".to_string(),
                message: alert_msg_text,
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
    let ec_gain = coeffs.get("ec_gain_per_ml").and_then(|v| v.as_f64());
    let ph_up = coeffs.get("ph_shift_up_per_ml").and_then(|v| v.as_f64());
    let ph_down = coeffs.get("ph_shift_down_per_ml").and_then(|v| v.as_f64());

    if ec_gain.is_none() && ph_up.is_none() && ph_down.is_none() {
        debug!("ℹ️ [EMA CALIBRATION] Không có hệ số nào mới để cập nhật.");
        return;
    }

    // Câu lệnh SQL linh hoạt: Chỉ update các cột có giá trị (khác NULL).
    // Nếu controller gửi lên NULL (vì chưa tính được) -> Giữ nguyên (COALESCE).
    let query = r#"
        UPDATE dosing_calibration
        SET
            ec_gain_per_ml = COALESCE($1, ec_gain_per_ml),
            ph_shift_up_per_ml = COALESCE($2, ph_shift_up_per_ml),
            ph_shift_down_per_ml = COALESCE($3, ph_shift_down_per_ml),
            updated_at = NOW()
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
