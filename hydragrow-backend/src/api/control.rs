use actix_web::{HttpResponse, Responder, web};
use rumqttc::QoS;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tracing::{error, info, instrument, warn};

use crate::AppState;
use crate::models::config::{DosingCalibration, SafetyConfig};
use crate::services::tuya; // Giả sử file tuya nằm trong folder services

#[derive(Debug, Deserialize)]
pub struct PumpControlReq {
    pub pump: String, // "A", "B", "PH_UP", "PH_DOWN", "OSAKA_PUMP", "MIST_VALVE", "WATER_PUMP", "DRAIN_PUMP", "CIRCULATION_PUMP", "ALL"
    pub action: String, // "on", "off", "reset_fault", "set_pwm"
    pub duration_sec: Option<u64>,
    pub pwm: Option<u32>,
    #[serde(default, alias = "max_allowed_ml", alias = "manual_max_dose_per_cycle")]
    pub manual_max_allowed_ml: Option<f32>,
}

#[derive(Debug, Serialize)]
struct MqttCommandPayload {
    pub action: String,
    pub pump: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_sec: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pwm: Option<u32>,
}

/// POST /api/devices/{device_id}/control
#[instrument(skip(app_state, req))]
pub async fn control_pump(
    path: web::Path<String>,
    req: web::Json<PumpControlReq>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let req_data = req.into_inner();

    let valid_pumps = [
        "A",
        "PUMP_A",
        "B",
        "PUMP_B",
        "PH_UP",
        "PH_DOWN",
        "OSAKA",
        "OSAKA_PUMP",
        "MIST",
        "MIST_VALVE",
        "WATER_PUMP_IN",
        "WATER_PUMP",
        "PUMP_IN",
        "WATER_PUMP_OUT",
        "DRAIN_PUMP",
        "PUMP_OUT",
        "ALL",
    ];

    if !valid_pumps.contains(&req_data.pump.as_str()) {
        warn!("Từ chối lệnh: Tên bơm/van không hợp lệ ({})", req_data.pump);
        return HttpResponse::BadRequest().json(json!({"error": "Invalid pump name"}));
    }

    let valid_actions = ["on", "off", "reset_fault", "set_pwm", "force_on"];
    if !valid_actions.contains(&req_data.action.as_str()) {
        warn!("Từ chối lệnh: Hành động không hợp lệ ({})", req_data.action);
        return HttpResponse::BadRequest()
            .json(json!({"error": "Action must be 'on', 'off', 'reset_fault', or 'set_pwm'"}));
    }

    if let (Some(pwm), Some(duration_sec)) = (req_data.pwm, req_data.duration_sec) {
        if let Err(resp) = validate_manual_dose_safety(
            &app_state.pg_pool,
            &device_id,
            &req_data.pump,
            pwm,
            duration_sec,
            req_data.manual_max_allowed_ml,
        )
        .await
        {
            return resp;
        }
    }

    // 🟢 Xử lý riêng cho CIRCULATION_PUMP qua Tuya Cloud
    // if req_data.pump == "CIRCULATION_PUMP" {
    //     ... (Giữ nguyên code cũ của bạn)
    // }

    // 🟢 ĐỊNH TUYẾN LỆNH THÔNG MINH (SMART ROUTING)
    // Nếu Frontend gửi action="on" nhưng có kèm theo giá trị PWM,
    // Backend sẽ tự dịch thành "set_pwm" để ESP32 hiểu đúng ý đồ chỉnh tốc độ.
    let mqtt_action = match req_data.action.as_str() {
        "on" => {
            if req_data.pwm.is_some() {
                "set_pwm"
            } else {
                "pump_on"
            }
        }
        "off" => "pump_off",
        "reset_fault" => "reset_fault",
        "set_pwm" => "set_pwm",
        "force_on" => "force_on",
        _ => "pump_off",
    };

    let command = MqttCommandPayload {
        action: mqtt_action.to_string(),
        pump: req_data.pump.clone(),
        duration_sec: req_data.duration_sec,
        pwm: req_data.pwm,
    };

    if let Err(e) = publish_command(&app_state, &device_id, &command).await {
        error!("Lỗi gửi lệnh qua MQTT: {:?}", e);
        return HttpResponse::InternalServerError()
            .json(json!({"error": "Không thể gửi lệnh xuống thiết bị"}));
    }

    // Nâng cấp Log để dễ theo dõi các lệnh Smart Dosing
    info!(
        "📡 Đã xuất lệnh MQTT [{}] -> Bơm: {} | PWM: {:?}% | Timeout: {:?}s | (Thiết bị: {})",
        mqtt_action, req_data.pump, req_data.pwm, req_data.duration_sec, device_id
    );

    let action_vn = match req_data.action.as_str() {
        "on" => "BẬT",
        "off" => "TẮT",
        "force_on" => "BẬT CƯỠNG CHẾ",
        "set_pwm" => "ĐỔI CÔNG SUẤT",
        "reset_fault" => "RESET LỖI",
        _ => "ĐIỀU KHIỂN",
    };

    let alert_msg = crate::models::alert::AlertMessage {
        level: "warning".to_string(), // Dùng màu Vàng (Warning) cho thao tác can thiệp thủ công
        title: "Can Thiệp Thủ Công".to_string(),
        message: format!(
            "Lệnh: {} thiết bị [{}]\nBởi: Người dùng / Ứng dụng",
            action_vn, req_data.pump
        ),
        device_id: device_id.clone(),
        reason: Some(format!("Người dùng bấm nút điều khiển qua Web/App")), // 🟢 Bổ sung reason
        metadata: None, // 🟢 Bổ sung metadata (để None vì không cần lưu chi tiết sensor lúc người dùng bấm)
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let _ = app_state.alert_sender.send(alert_msg);

    HttpResponse::Ok().json(json!({"status": "success", "message": "Command sent"}))
}

async fn validate_manual_dose_safety(
    pg_pool: &PgPool,
    device_id: &str,
    pump: &str,
    pwm: u32,
    duration_sec: u64,
    manual_max_allowed_ml: Option<f32>,
) -> Result<(), HttpResponse> {
    let normalized_pump = normalize_dosing_pump_name(pump);
    let Some(normalized_pump) = normalized_pump else {
        return Ok(());
    };

    let dosing_cfg = load_dosing_calibration(pg_pool, device_id)
        .await
        .map_err(|e| {
            error!(
                "Không thể tải dosing_calibration cho kiểm tra an toàn manual [{}]: {:?}",
                device_id, e
            );
            HttpResponse::InternalServerError().json(json!({"error": "DB Error"}))
        })?;

    let capacity_ml_per_sec = capacity_ml_per_sec(&dosing_cfg, normalized_pump);
    let estimated_ml = capacity_ml_per_sec * (pwm as f32 / 100.0) * duration_sec as f32;

    let max_allowed_ml = match manual_max_allowed_ml {
        Some(v) if v > 0.0 => v,
        _ => load_max_dose_per_cycle(pg_pool, device_id)
            .await
            .map_err(|e| {
                error!(
                    "Không thể tải safety_config cho kiểm tra an toàn manual [{}]: {:?}",
                    device_id, e
                );
                HttpResponse::InternalServerError().json(json!({"error": "DB Error"}))
            })?,
    };

    if estimated_ml > max_allowed_ml {
        warn!(
            "Chặn lệnh manual vượt ngưỡng an toàn: device={} pump={} normalized={} pwm={} duration={}s estimated_ml={:.3} max_allowed_ml={:.3}",
            device_id, pump, normalized_pump, pwm, duration_sec, estimated_ml, max_allowed_ml
        );
        return Err(HttpResponse::BadRequest().json(json!({
            "error": "Manual dose exceeds safe limit",
            "estimated_ml": estimated_ml,
            "max_allowed_ml": max_allowed_ml,
            "pump": normalized_pump,
            "pwm": pwm,
            "duration_sec": duration_sec
        })));
    }

    Ok(())
}

async fn load_dosing_calibration(
    pg_pool: &PgPool,
    device_id: &str,
) -> anyhow::Result<DosingCalibration> {
    let dosing_cfg_res = sqlx::query_as::<_, DosingCalibration>(
        "SELECT * FROM dosing_calibration WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_optional(pg_pool)
    .await?;

    dosing_cfg_res.ok_or_else(|| anyhow::anyhow!("Dosing calibration not found for {}", device_id))
}

async fn load_max_dose_per_cycle(pg_pool: &PgPool, device_id: &str) -> anyhow::Result<f32> {
    let safety_cfg_res =
        sqlx::query_as::<_, SafetyConfig>("SELECT * FROM safety_config WHERE device_id = $1")
            .bind(device_id)
            .fetch_optional(pg_pool)
            .await?;

    Ok(safety_cfg_res
        .unwrap_or_else(|| SafetyConfig {
            device_id: device_id.to_string(),
            ..Default::default()
        })
        .max_dose_per_cycle)
}

fn normalize_dosing_pump_name(pump: &str) -> Option<&'static str> {
    match pump {
        "A" | "PUMP_A" => Some("PUMP_A"),
        "B" | "PUMP_B" => Some("PUMP_B"),
        "PH_UP" => Some("PH_UP"),
        "PH_DOWN" => Some("PH_DOWN"),
        _ => None,
    }
}

fn capacity_ml_per_sec(dosing_cfg: &DosingCalibration, normalized_pump: &str) -> f32 {
    match normalized_pump {
        "PUMP_A" => dosing_cfg.pump_a_capacity_ml_per_sec,
        "PUMP_B" => dosing_cfg.pump_b_capacity_ml_per_sec,
        "PH_UP" => dosing_cfg.pump_ph_up_capacity_ml_per_sec,
        "PH_DOWN" => dosing_cfg.pump_ph_down_capacity_ml_per_sec,
        _ => 0.0,
    }
}

async fn publish_command(
    app_state: &AppState,
    device_id: &str,
    payload: &MqttCommandPayload,
) -> anyhow::Result<()> {
    let topic = format!("AGITECH/{}/controller/command", device_id);
    let payload_bytes = serde_json::to_vec(payload)?;

    app_state
        .mqtt_client
        .publish(topic, QoS::AtLeastOnce, false, payload_bytes)
        .await?;

    Ok(())
}

pub async fn request_device_sync(
    path: web::Path<String>,
    app_state: web::Data<crate::AppState>,
) -> impl Responder {
    let device_id = path.into_inner();

    // Gửi lệnh "SYNC" xuống topic điều khiển của ESP32
    let topic = format!("AGITECH/{}/controller/command", device_id);
    let payload = json!({
        "action": "SYNC_STATUS",
        "value": 0
    });

    match serde_json::to_vec(&payload) {
        Ok(mqtt_bytes) => {
            let res = app_state
                .mqtt_client
                .publish(&topic, QoS::AtLeastOnce, false, mqtt_bytes)
                .await;

            if res.is_ok() {
                HttpResponse::Ok().json(json!({"status": "sync_requested"}))
            } else {
                HttpResponse::InternalServerError().json(json!({"error": "Failed to publish"}))
            }
        }
        Err(_) => HttpResponse::InternalServerError().json(json!({"error": "Serialize failed"})),
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/control", web::post().to(control_pump))
        .route("/control/sync", web::post().to(request_device_sync));
}
