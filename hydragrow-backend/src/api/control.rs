use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use hydragrow_shared::topics::topic_controller_command;
use hydragrow_shared::{MqttCommandOut, MqttCommandParams};
use rumqttc::QoS;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use std::time::{Duration, Instant};
use tracing::{error, info, instrument, warn};

use crate::api::middleware::auth::AuthContext;
use crate::api::mqtt_utils::publish_command;
use crate::db::postgres::{NewSystemEventRecord, insert_system_event};
use crate::models::config::{DosingCalibration, SafetyConfig};
use crate::{AppState, CommandRateEntry};
use hydragrow_shared::events::AppEvent;

#[derive(Debug, Deserialize)]
pub struct PumpControlReq {
    pub target: Option<String>,
    pub pump: Option<String>,      // legacy
    pub action: String,            // "on", "off", "reset_fault", "set_pwm"
    pub duration_sec: Option<u64>, // legacy
    pub pwm: Option<u32>,          // legacy
    pub params: Option<PumpControlParams>,
    #[serde(default, alias = "max_allowed_ml", alias = "manual_max_dose_per_cycle")]
    pub manual_max_allowed_ml: Option<f32>,
    pub command_metadata: Option<ControlCommandMetadata>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ControlCommandMetadata {
    pub action: String,
    pub pump_id: Option<String>,
    pub duration_sec: Option<u64>,
    pub pwm: Option<u32>,
    #[serde(default)]
    pub dangerous: bool,
}

#[derive(Debug, Deserialize)]
pub struct PumpControlParams {
    pub pump_id: Option<String>,
    pub duration_sec: Option<u64>,
    pub pwm: Option<u32>,
    pub state: Option<bool>,
}

// #[derive(Debug, Serialize)]
// struct MqttCommandOut {
//     pub target: String,
//     pub action: String,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub params: Option<MqttCommandParams>,
// }

// #[derive(Debug, Serialize)]
// struct MqttCommandParams {
//     pub pump_id: String,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub duration_sec: Option<u64>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub pwm: Option<u32>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub state: Option<bool>,
// }

/// POST /api/devices/{device_id}/control
#[instrument(skip(app_state, req))]
pub async fn control_pump(
    path: web::Path<String>,
    http_req: HttpRequest,
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

    let pump_name = req_data
        .params
        .as_ref()
        .and_then(|p| p.pump_id.clone())
        .or_else(|| req_data.pump.clone())
        .unwrap_or_else(|| "ALL".to_string());
    let duration_sec = req_data
        .params
        .as_ref()
        .and_then(|p| p.duration_sec)
        .or(req_data.duration_sec);
    let pwm = req_data
        .params
        .as_ref()
        .and_then(|p| p.pwm)
        .or(req_data.pwm);
    let explicit_state = req_data.params.as_ref().and_then(|p| p.state);
    let target = req_data.target.clone();
    let target = resolve_control_target(target);

    if !valid_pumps.contains(&pump_name.as_str()) {
        warn!("Từ chối lệnh: Tên bơm/van không hợp lệ ({})", pump_name);
        return HttpResponse::BadRequest().json(json!({"error": "Invalid pump name"}));
    }

    let valid_actions = ["on", "off", "reset_fault", "set_pwm", "force_on"];
    if !valid_actions.contains(&req_data.action.as_str()) {
        warn!("Từ chối lệnh: Hành động không hợp lệ ({})", req_data.action);
        return HttpResponse::BadRequest()
            .json(json!({"error": "Action must be 'on', 'off', 'reset_fault', or 'set_pwm'"}));
    }

    let auth = http_req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    let required_scope = required_control_scope(&req_data.action, pwm, &pump_name);
    if !auth.has_scope(required_scope) {
        audit_control_command(
            &app_state,
            &device_id,
            &auth,
            &req_data.action,
            &pump_name,
            "denied_missing_scope",
            Some(required_scope),
            duration_sec,
            pwm,
        )
        .await;
        return HttpResponse::Forbidden().json(json!({
            "error": "Missing required scope",
            "required_scope": required_scope
        }));
    }

    if is_dangerous_control(&req_data.action, pwm, &pump_name)
        && !has_dangerous_confirmation(&http_req)
    {
        audit_control_command(
            &app_state,
            &device_id,
            &auth,
            &req_data.action,
            &pump_name,
            "denied_confirmation_required",
            None,
            duration_sec,
            pwm,
        )
        .await;
        return HttpResponse::Forbidden().json(json!({
            "error": "Dangerous command requires user confirmation or elevated token",
            "required_confirmation": "X-User-Confirmed: true",
            "alternative": "X-Elevated-Token with a short-lived backend-issued token"
        }));
    }

    if let (Some(pwm), Some(duration_sec)) = (pwm, duration_sec) {
        if let Err(resp) = validate_manual_dose_safety(
            &app_state.pg_pool,
            &device_id,
            &pump_name,
            pwm,
            duration_sec,
            req_data.manual_max_allowed_ml,
        )
        .await
        {
            audit_control_command(
                &app_state,
                &device_id,
                &auth,
                &req_data.action,
                &pump_name,
                "denied_safety_limit",
                None,
                Some(duration_sec),
                Some(pwm),
            )
            .await;
            return resp;
        }
    }

    let mqtt_action = match req_data.action.as_str() {
        "on" => {
            if pwm.is_some() {
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

    let command = MqttCommandOut {
        target,
        action: mqtt_action.to_string(),
        params: Some(MqttCommandParams {
            pump_id: Some(pump_name.clone()),
            duration_sec,
            pwm,
            state: explicit_state,
            ota_url: None,
        }),
        ts: None,
        nonce: None,
        signature: None,
    };

    if let Err(e) = publish_command(&app_state, &device_id, &command).await {
        error!("Lỗi gửi lệnh qua MQTT: {:?}", e);
        audit_control_command(
            &app_state,
            &device_id,
            &auth,
            &req_data.action,
            &pump_name,
            "publish_failed",
            None,
            duration_sec,
            pwm,
        )
        .await;
        return HttpResponse::InternalServerError()
            .json(json!({"error": "Không thể gửi lệnh xuống thiết bị"}));
    }

    info!(
        "📡 Đã xuất lệnh MQTT [{}] -> Bơm: {} | PWM: {:?}% | Timeout: {:?}s | (Thiết bị: {})",
        mqtt_action, pump_name, pwm, duration_sec, device_id
    );

    let action_vn = match req_data.action.as_str() {
        "on" => "BẬT",
        "off" => "TẮT",
        "force_on" => "BẬT CƯỠNG CHẾ",
        "set_pwm" => "ĐỔI CÔNG SUẤT",
        "reset_fault" => "RESET LỖI",
        _ => "ĐIỀU KHIỂN",
    };

    audit_control_command(
        &app_state,
        &device_id,
        &auth,
        &req_data.action,
        &pump_name,
        "published",
        None,
        duration_sec,
        pwm,
    )
    .await;

    let timestamp = chrono::Utc::now().timestamp_millis() as u64;
    let metadata = json!({
        "event_type": "manual_control",
        "action": req_data.action.clone(),
        "pump": pump_name.clone(),
        "duration_sec": duration_sec,
        "pwm": pwm,
    });
    let alert_msg = crate::models::alert::AlertMessage {
        level: control_event_level(&req_data.action).to_string(),
        category: "user_action".to_string(),
        title: "Can Thiệp Thủ Công".to_string(),
        message: format!(
            "Lệnh: {} thiết bị [{}]\nBởi: Người dùng / Ứng dụng",
            action_vn, pump_name
        ),
        device_id: device_id.clone(),
        reason: None,
        metadata: Some(metadata.clone()),
        timestamp,
    };

    let _ = insert_system_event(
        &app_state.pg_pool,
        &NewSystemEventRecord {
            device_id: device_id.clone(),
            level: alert_msg.level.clone(),
            category: alert_msg.category.clone(),
            title: alert_msg.title.clone(),
            message: alert_msg.message.clone(),
            reason: alert_msg.reason.clone(),
            metadata: Some(metadata),
            timestamp: timestamp as i64,
        },
    )
    .await;
    let _ = app_state.event_bus.send(AppEvent::SystemAlert(alert_msg));

    HttpResponse::Ok().json(json!({
        "status": "success",
        "message": "Command published to MQTT",
        "device_id": device_id,
        "target": command.target,
        "action": command.action,
        "pump": pump_name,
        "duration_sec": duration_sec,
        "pwm": pwm,
        "published_at": timestamp
    }))
}

async fn audit_control_command(
    app_state: &web::Data<AppState>,
    device_id: &str,
    auth: &AuthContext,
    action: &str,
    pump: &str,
    result: &str,
    required_scope: Option<&str>,
    duration_sec: Option<u64>,
    pwm: Option<u32>,
) {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let metadata = json!({
        "event_type": "control_audit",
        "user": auth.user_id.as_deref().unwrap_or("unknown"),
        "session": auth.session_id.as_deref().unwrap_or("unknown"),
        "device": device_id,
        "action": action,
        "pump_id": pump,
        "duration_sec": duration_sec,
        "pwm": pwm,
        "result": result,
        "required_scope": required_scope,
        "scopes": auth.scopes,
    });

    let _ = insert_system_event(
        &app_state.pg_pool,
        &NewSystemEventRecord {
            device_id: device_id.to_string(),
            level: if result == "published" {
                "info"
            } else {
                "warning"
            }
            .to_string(),
            category: "audit".to_string(),
            title: "Control Command Audit".to_string(),
            message: format!(
                "user={} session={} device={} action={} result={}",
                auth.user_id.as_deref().unwrap_or("unknown"),
                auth.session_id.as_deref().unwrap_or("unknown"),
                device_id,
                action,
                result
            ),
            reason: required_scope.map(ToString::to_string),
            metadata: Some(metadata),
            timestamp,
        },
    )
    .await;
}

fn required_control_scope(action: &str, pwm: Option<u32>, pump: &str) -> &'static str {
    if action == "reset_fault" || action == "force_on" {
        return "control:emergency";
    }

    if action == "set_pwm" || pwm.is_some() || normalize_dosing_pump_name(pump).is_some() {
        return "control:pump";
    }

    "control:pump"
}

fn is_dangerous_control(action: &str, pwm: Option<u32>, pump: &str) -> bool {
    action == "force_on"
        || action == "reset_fault"
        || action == "set_pwm"
        || pwm.is_some()
        || normalize_dosing_pump_name(pump).is_some()
}

fn has_dangerous_confirmation(req: &HttpRequest) -> bool {
    let confirmed = req
        .headers()
        .get("X-User-Confirmed")
        .and_then(|hv| hv.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false);

    let elevated = std::env::var("ELEVATED_CONTROL_TOKEN")
        .ok()
        .and_then(|expected| {
            req.headers()
                .get("X-Elevated-Token")
                .and_then(|hv| hv.to_str().ok())
                .map(|actual| actual == expected)
        })
        .unwrap_or(false);

    confirmed || elevated
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

pub async fn request_device_sync(
    path: web::Path<String>,
    app_state: web::Data<crate::AppState>,
) -> impl Responder {
    let device_id = path.into_inner();

    // Gửi lệnh "SYNC" xuống topic điều khiển của ESP32
    let topic = topic_controller_command(&device_id);
    let payload = json!({
        "action": "SYNC_STATUS",
        "value": 0
    });

    let payload = match crate::api::mqtt_utils::sign_command_value(&device_id, payload) {
        Ok(payload) => payload,
        Err(_) => return HttpResponse::InternalServerError().json(json!({"error": "Sign failed"})),
    };

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

pub async fn get_control_state(
    path: web::Path<String>,
    app_state: web::Data<crate::AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let states = app_state.device_states.read().await;
    let cached = states
        .get(&device_id)
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok());

    let data = cached.unwrap_or_else(|| {
        json!({
            "device_id": device_id,
            "fsm_state": "Unknown",
            "pump_status": {
                "pump_a": false,
                "pump_b": false,
                "ph_up": false,
                "ph_down": false,
                "osaka_pump": false,
                "mist_valve": false,
                "water_pump_in": false,
                "water_pump_out": false
            }
        })
    });

    HttpResponse::Ok().json(json!({ "status": "success", "data": data }))
}

fn resolve_control_target(target: Option<String>) -> String {
    target
        .unwrap_or_else(|| "all".to_string())
        .trim()
        .to_ascii_lowercase()
}

fn control_event_level(action: &str) -> &'static str {
    match action {
        "force_on" => "warning",
        "reset_fault" => "success",
        _ => "info",
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/control", web::post().to(control_pump))
        .route("/control/sync", web::post().to(request_device_sync))
        .route("/control/state", web::get().to(get_control_state));
}

#[cfg(test)]
mod tests {
    use super::{
        control_event_level, is_dangerous_control, required_control_scope, resolve_control_target,
    };

    #[test]
    fn control_target_defaults_to_all_for_controller_commands() {
        assert_eq!(resolve_control_target(None), "all");
    }

    #[test]
    fn control_target_normalizes_explicit_all() {
        assert_eq!(resolve_control_target(Some("ALL".to_string())), "all");
    }

    #[test]
    fn normal_manual_pump_actions_are_logged_as_info() {
        assert_eq!(control_event_level("on"), "info");
        assert_eq!(control_event_level("off"), "info");
        assert_eq!(control_event_level("set_pwm"), "info");
    }

    #[test]
    fn force_and_fault_reset_actions_keep_attention_levels() {
        assert_eq!(control_event_level("force_on"), "warning");
        assert_eq!(control_event_level("reset_fault"), "success");
    }

    #[test]
    fn emergency_commands_require_emergency_scope() {
        assert_eq!(
            required_control_scope("force_on", None, "OSAKA"),
            "control:emergency"
        );
        assert_eq!(
            required_control_scope("reset_fault", None, "ALL"),
            "control:emergency"
        );
    }

    #[test]
    fn pwm_and_dosing_commands_are_dangerous() {
        assert!(is_dangerous_control("set_pwm", Some(40), "OSAKA"));
        assert!(is_dangerous_control("on", Some(80), "PUMP_A"));
        assert!(is_dangerous_control("on", None, "PH_UP"));
    }
}
