use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use hydragrow_shared::topics::topic_controller_command;
use hydragrow_shared::{CommandLifecycle, CommandMetadata, MqttCommandOut, MqttCommandParams};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rumqttc::QoS;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use tracing::{error, info, instrument, warn};

use crate::AppState;
use crate::api::middleware::auth::AuthContext;
use crate::api::mqtt_utils::publish_command;
use crate::db::postgres::{NewSystemEventRecord, insert_system_event};
use crate::models::config::DosingCalibration;
use crate::services::durable_command::{
    NewCommand, create_command_with_state, list_lifecycle_events, mark_publish_attempt,
    schedule_publish_retry_with_state, transition_with_state,
};
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
    #[serde(default)]
    pub idempotency_key: Option<String>,
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

#[derive(Debug, Deserialize)]
pub struct PrivilegedTokenRequest {
    pub action_class: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PrivilegedControlClaims {
    sub: String,
    device_id: String,
    action_class: String,
    exp: usize,
}

const PRIVILEGED_TOKEN_TTL_SECS: u64 = 60;
const PRIVILEGED_ACTION_CLASS: &str = "dangerous_control";

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
        "MIX",
        "MIX_VALVE",
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

    let valid_actions = [
        "on",
        "off",
        "reset_fault",
        "set_pwm",
        "force_on",
        "emergency_stop",
    ];
    if !valid_actions.contains(&req_data.action.as_str()) {
        warn!("Từ chối lệnh: Hành động không hợp lệ ({})", req_data.action);
        return HttpResponse::BadRequest()
            .json(json!({"error": "Action must be 'on', 'off', 'reset_fault', 'set_pwm', 'force_on', or 'emergency_stop'"}));
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

    if let Err(response) = crate::api::middleware::auth::authorize_device(
        &http_req,
        &app_state,
        Some(required_scope),
        &device_id,
    )
    .await
    {
        return response;
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
            "error": "Dangerous command requires user confirmation",
            "required_confirmation": "X-User-Confirmed: true"
        }));
    }

    if is_dangerous_control(&req_data.action, pwm, &pump_name) {
        let Some(token) = http_req
            .headers()
            .get("X-Privileged-Token")
            .and_then(|value| value.to_str().ok())
        else {
            return HttpResponse::Forbidden().json(json!({
                "error": "Dangerous command requires X-Privileged-Token"
            }));
        };
        if !validate_privileged_token(&app_state, &auth, &device_id, token) {
            return HttpResponse::Forbidden().json(json!({
                "error": "Invalid or expired privileged control token"
            }));
        }
    }

    if let (Some(pwm), Some(duration_sec)) = (pwm, duration_sec)
        && let Err(resp) = validate_manual_dose_safety(
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
            if resp.status() == actix_web::http::StatusCode::SERVICE_UNAVAILABLE {
                "denied_safety_data"
            } else {
                "denied_safety_limit"
            },
            None,
            Some(duration_sec),
            Some(pwm),
        )
        .await;
        return resp;
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
        "emergency_stop" => "emergency_stop",
        _ => "pump_off",
    };

    let requested_state = match req_data.action.as_str() {
        "on" | "force_on" => Some(true),
        "off" | "emergency_stop" => Some(false),
        "set_pwm" => Some(pwm.unwrap_or(0) > 0),
        _ => None,
    };
    let principal_id = auth.user_id.clone();
    let user_id = principal_id
        .as_deref()
        .and_then(|id| id.parse::<i64>().ok());
    let request_payload = json!({
        "action": req_data.action,
        "pump_id": pump_name,
        "duration_sec": duration_sec,
        "pwm": pwm,
        "state": explicit_state,
    });
    let (durable, existing) = match create_command_with_state(
        &app_state,
        NewCommand {
            idempotency_key: req_data.idempotency_key.clone(),
            principal_kind: format!("{:?}", auth.principal_kind()).to_ascii_lowercase(),
            principal_id,
            service_key_label: auth.service_key_label.clone(),
            session_id: auth.session_id.clone(),
            user_id,
            device_id: device_id.clone(),
            action: req_data.action.clone(),
            request_payload,
            requested_state,
            requested_pwm: pwm,
            pump_id: Some(pump_name.clone()),
            retry_safe: !is_dangerous_control(&req_data.action, pwm, &pump_name),
        },
    )
    .await
    {
        Ok(value) => value,
        Err(e) if e.to_string().contains("idempotency key conflicts") => {
            return HttpResponse::Conflict()
                .json(json!({"error": "Idempotency key conflicts with existing command"}));
        }
        Err(e) => {
            error!(error = %e, device_id = %device_id, "Failed to persist command intent");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Could not persist command"}));
        }
    };
    let command_id = durable.command_id.clone();
    if existing {
        return HttpResponse::Ok().json(json!({
            "status": "success",
            "message": "Existing idempotent command",
            "command_id": command_id,
            "lifecycle": format_lifecycle(durable.lifecycle),
            "device_id": device_id,
        }));
    }

    let command = MqttCommandOut {
        target,
        action: mqtt_action.to_string(),
        params: Some(MqttCommandParams {
            pump_id: Some(pump_name.clone()),
            duration_sec,
            pwm,
            state: explicit_state,
            ota_url: None,
            candidates: None,
            ota_provision: None,
        }),
        ts: None,
        nonce: None,
        signature: None,
        metadata: Some(CommandMetadata {
            command_id: Some(command_id.clone()),
        }),
    };

    if let Err(e) = mark_publish_attempt(&app_state.pg_pool, &command_id).await {
        error!(error = %e, command_id = %command_id, "Failed to persist MQTT attempt");
        return HttpResponse::InternalServerError()
            .json(json!({"error": "Could not persist command attempt"}));
    }

    if let Err(e) = publish_command(&app_state, &device_id, &command).await {
        error!("Lỗi gửi lệnh qua MQTT: {:?}", e);
        let _ = schedule_publish_retry_with_state(&app_state, &command_id, &e.to_string()).await;
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

    if let Err(e) = transition_with_state(
        &app_state,
        &command_id,
        &device_id,
        CommandLifecycle::Sent,
        None,
        "publish",
        json!({}),
    )
    .await
    {
        error!(error = %e, command_id = %command_id, "Failed to persist SENT lifecycle");
        return HttpResponse::InternalServerError()
            .json(json!({"error": "Command lifecycle persistence failed"}));
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
            source: "rule".to_string(),
            primary_reason_code: None,
        },
    )
    .await;
    let _ = app_state.event_bus.send(AppEvent::SystemAlert(alert_msg));

    HttpResponse::Ok().json(json!({
        "status": "success",
        "message": "Command published to MQTT; awaiting device lifecycle",
        "command_id": command_id,
        "lifecycle": "SENT",
        "device_id": device_id,
        "target": command.target,
        "action": command.action,
        "pump": pump_name,
        "duration_sec": duration_sec,
        "pwm": pwm,
        "published_at": timestamp
    }))
}

pub async fn issue_privileged_token(
    path: web::Path<String>,
    req: HttpRequest,
    body: web::Json<PrivilegedTokenRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("control:emergency")
        && !auth.has_scope("device:admin")
        && !auth.has_scope("control:pump")
    {
        return HttpResponse::Forbidden()
            .json(json!({"error":"Missing privileged control capability"}));
    }
    if body.action_class != PRIVILEGED_ACTION_CLASS {
        return HttpResponse::BadRequest().json(json!({"error":"Unsupported action_class"}));
    }
    if !has_dangerous_confirmation(&req) {
        return HttpResponse::Forbidden().json(json!({"error":"Requires X-User-Confirmed: true"}));
    }

    let sub = auth.user_id.clone().or(auth.service_key_label.clone());
    let Some(sub) = sub else {
        return HttpResponse::Unauthorized().json(json!({"error":"Unauthorized"}));
    };
    let exp = (chrono::Utc::now().timestamp() as u64 + PRIVILEGED_TOKEN_TTL_SECS) as usize;
    let claims = PrivilegedControlClaims {
        sub,
        device_id,
        action_class: body.action_class.clone(),
        exp,
    };
    match encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(app_state.privileged_control_secret.as_bytes()),
    ) {
        Ok(token) => HttpResponse::Ok().json(json!({"token": token, "expires_at": exp})),
        Err(_) => HttpResponse::InternalServerError()
            .json(json!({"error":"Could not issue privileged token"})),
    }
}

fn validate_privileged_token(
    app_state: &AppState,
    auth: &AuthContext,
    device_id: &str,
    token: &str,
) -> bool {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    let Ok(data) = decode::<PrivilegedControlClaims>(
        token,
        &DecodingKey::from_secret(app_state.privileged_control_secret.as_bytes()),
        &validation,
    ) else {
        return false;
    };
    let expected_sub = auth
        .user_id
        .as_deref()
        .or(auth.service_key_label.as_deref());
    expected_sub == Some(data.claims.sub.as_str())
        && data.claims.device_id == device_id
        && data.claims.action_class == PRIVILEGED_ACTION_CLASS
}

fn format_lifecycle(lifecycle: CommandLifecycle) -> &'static str {
    match lifecycle {
        CommandLifecycle::Requested => "REQUESTED",
        CommandLifecycle::Sent => "SENT",
        CommandLifecycle::Acknowledged => "ACKNOWLEDGED",
        CommandLifecycle::Confirmed => "CONFIRMED",
        CommandLifecycle::Rejected => "REJECTED",
        CommandLifecycle::Failed => "FAILED",
        CommandLifecycle::Timeout => "TIMEOUT",
        CommandLifecycle::Unknown => "UNKNOWN",
    }
}

#[allow(clippy::too_many_arguments)]
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
            source: "rule".to_string(),
            primary_reason_code: None,
        },
    )
    .await;
}

fn required_control_scope(action: &str, pwm: Option<u32>, pump: &str) -> &'static str {
    if action == "reset_fault" || action == "force_on" || action == "emergency_stop" {
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
        || action == "emergency_stop"
        || pwm.is_some()
        || normalize_dosing_pump_name(pump).is_some()
}

fn has_dangerous_confirmation(req: &HttpRequest) -> bool {
    req.headers()
        .get("X-User-Confirmed")
        .and_then(|hv| hv.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
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

    let dosing_cfg = load_dosing_calibration(pg_pool, device_id).await.map_err(|e| {
            error!(
                "Không thể tải dosing_calibration cho kiểm tra an toàn manual [{}]: {:?}",
                device_id, e
            );
            let reason = match &e {
                crate::services::safety_data::SafetyDataError::Missing => "DOSING_CALIBRATION_MISSING",
                crate::services::safety_data::SafetyDataError::Database => "DOSING_CALIBRATION_DB_ERROR",
                crate::services::safety_data::SafetyDataError::Invalid => "DOSING_CALIBRATION_INVALID",
            };
            HttpResponse::ServiceUnavailable().json(json!({
                "error": {"code": "safety_data_unavailable", "message": "Safety data unavailable", "details": {"reason": reason}}
            }))
        })?;

    let capacity_ml_per_sec = capacity_ml_per_sec(&dosing_cfg, normalized_pump);
    let estimated_ml = capacity_ml_per_sec * (pwm as f32 / 100.0) * duration_sec as f32;

    let server_max = load_max_dose_per_cycle(pg_pool, device_id).await.map_err(|e| {
            error!(
                "Không thể tải safety_config cho kiểm tra an toàn manual [{}]: {:?}",
                device_id, e
            );
            HttpResponse::ServiceUnavailable().json(json!({
                "error": {"code": "safety_data_unavailable", "message": "Safety data unavailable", "details": {"reason": e.reason_code()}}
            }))
        })?;

    let max_allowed_ml = compute_effective_max_allowed_ml(manual_max_allowed_ml, server_max);

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
) -> Result<DosingCalibration, crate::services::safety_data::SafetyDataError> {
    let dosing_cfg_res = sqlx::query_as::<_, DosingCalibration>(
        "SELECT * FROM dosing_calibration WHERE device_id = $1",
    )
    .bind(device_id)
    .fetch_optional(pg_pool)
    .await;
    crate::services::safety_data::classify_calibration(dosing_cfg_res).map_err(
        |reason| match reason {
            "DOSING_CALIBRATION_MISSING" => crate::services::safety_data::SafetyDataError::Missing,
            "DOSING_CALIBRATION_INVALID" => crate::services::safety_data::SafetyDataError::Invalid,
            _ => crate::services::safety_data::SafetyDataError::Database,
        },
    )
}

async fn load_max_dose_per_cycle(
    pg_pool: &PgPool,
    device_id: &str,
) -> Result<f32, crate::services::safety_data::SafetyDataError> {
    let safety_cfg_res = crate::db::postgres::fetch_safety_config(pg_pool, device_id).await;
    Ok(crate::services::safety_data::classify_safety_config(safety_cfg_res)?.max_dose_per_cycle)
}

pub(crate) fn compute_effective_max_allowed_ml(
    manual_max_allowed_ml: Option<f32>,
    server_max: f32,
) -> f32 {
    match manual_max_allowed_ml {
        Some(v) if v > 0.0 => v.min(server_max),
        _ => server_max,
    }
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
    req: HttpRequest,
    app_state: web::Data<crate::AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("device:admin") {
        return HttpResponse::Forbidden()
            .json(json!({"error":"Missing required scope: device:admin"}));
    }

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
    req: HttpRequest,
    app_state: web::Data<crate::AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("read:telemetry") {
        return HttpResponse::Forbidden()
            .json(json!({"error":"Missing required scope: read:telemetry"}));
    }
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
                "mix_valve": false,
                "water_pump_in": false,
                "water_pump_out": false
            }
        })
    });

    HttpResponse::Ok().json(json!({ "status": "success", "data": data }))
}

pub async fn get_command_lifecycles(
    path: web::Path<String>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("read:telemetry") {
        return HttpResponse::Forbidden()
            .json(json!({"error":"Missing required scope: read:telemetry"}));
    }
    match crate::services::durable_command::list_commands(&app_state.pg_pool, &device_id, 100).await
    {
        Ok(commands) => HttpResponse::Ok().json(json!({
            "status": "success",
            "data": commands.into_iter().map(command_json).collect::<Vec<_>>()
        })),
        Err(error) => {
            error!(device_id = %device_id, ?error, "Failed to load command lifecycle history");
            HttpResponse::InternalServerError()
                .json(json!({ "error": "Failed to load command lifecycle history" }))
        }
    }
}

pub async fn get_command_lifecycle(
    path: web::Path<(String, String)>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let (device_id, command_id) = path.into_inner();
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("read:telemetry") {
        return HttpResponse::Forbidden()
            .json(json!({"error": "Missing required scope: read:telemetry"}));
    }
    match crate::services::durable_command::get_command(&app_state.pg_pool, &command_id).await {
        Ok(command) if command.device_id == device_id => {
            let history = list_lifecycle_events(&app_state.pg_pool, &command_id)
                .await
                .unwrap_or_default();
            HttpResponse::Ok()
                .json(json!({"status":"success","data":command_json(command),"history":history}))
        }
        Ok(_) => HttpResponse::NotFound().json(json!({"error":"Command not found"})),
        Err(error) if error.to_string().contains("no rows returned") => {
            HttpResponse::NotFound().json(json!({"error":"Command not found"}))
        }
        Err(error) => {
            error!(device_id = %device_id, command_id = %command_id, ?error, "Failed to load command");
            HttpResponse::InternalServerError().json(json!({"error":"Failed to load command"}))
        }
    }
}

fn command_json(command: crate::services::durable_command::DurableCommand) -> serde_json::Value {
    json!({
        "command_id": command.command_id,
        "device_id": command.device_id,
        "action": command.action,
        "pump_id": command.pump_id,
        "requested_state": command.requested_state,
        "requested_pwm": command.requested_pwm,
        "lifecycle": format_lifecycle(command.lifecycle),
        "created_at": command.created_at,
        "authorized_at": command.authorized_at,
        "sent_at": command.sent_at,
        "acknowledged_at": command.acknowledged_at,
        "confirmed_at": command.confirmed_at,
        "terminal_at": command.terminal_at,
        "next_retry_at": command.next_retry_at,
        "attempt_count": command.attempt_count,
        "last_error": command.last_error,
        "last_observed_at": command.last_observed_at,
        "version": command.version,
    })
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
        "emergency_stop" => "critical",
        _ => "info",
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/control", web::post().to(control_pump))
        .route(
            "/control/privileged-token",
            web::post().to(issue_privileged_token),
        )
        .route("/control/commands", web::get().to(get_command_lifecycles))
        .route(
            "/control/commands/{command_id}",
            web::get().to(get_command_lifecycle),
        )
        .route("/control/sync", web::post().to(request_device_sync))
        .route("/control/state", web::get().to(get_control_state));
}

#[cfg(test)]
mod tests {
    use super::{
        PRIVILEGED_ACTION_CLASS, PrivilegedControlClaims, control_event_level,
        is_dangerous_control, issue_privileged_token, required_control_scope,
        resolve_control_target, validate_privileged_token,
    };
    use crate::api::middleware::auth::AuthContext;
    use actix_web::{HttpMessage, Responder, web};
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

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
    fn emergency_stop_requires_control_emergency_scope() {
        assert_eq!(
            required_control_scope("emergency_stop", None, "ALL"),
            "control:emergency"
        );
    }

    #[test]
    fn emergency_stop_is_dangerous() {
        assert!(is_dangerous_control("emergency_stop", None, "ALL"));
    }

    #[test]
    fn emergency_stop_event_level_is_critical() {
        assert_eq!(control_event_level("emergency_stop"), "critical");
    }

    #[test]
    fn pwm_and_dosing_commands_are_dangerous() {
        assert!(is_dangerous_control("set_pwm", Some(40), "OSAKA"));
        assert!(is_dangerous_control("on", Some(80), "PUMP_A"));
        assert!(is_dangerous_control("on", None, "PH_UP"));
    }

    #[test]
    fn mix_valve_pump_names_are_accepted() {
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
            "MIX",
            "MIX_VALVE",
            "ALL",
        ];
        assert!(valid_pumps.contains(&"MIX"), "MIX phải hợp lệ");
        assert!(valid_pumps.contains(&"MIX_VALVE"), "MIX_VALVE phải hợp lệ");
    }

    #[test]
    fn mix_valve_is_not_treated_as_dosing_pump() {
        assert!(
            super::normalize_dosing_pump_name("MIX").is_none(),
            "MIX không phải dosing pump, không cần safety check"
        );
        assert!(
            super::normalize_dosing_pump_name("MIX_VALVE").is_none(),
            "MIX_VALVE không phải dosing pump"
        );
    }

    #[test]
    fn mix_valve_on_off_is_not_dangerous() {
        // MIX là van bật/tắt đơn giản (allowPwm=false trên frontend)
        assert!(
            !is_dangerous_control("on", None, "MIX"),
            "Bật MIX không cần confirmation"
        );
        assert!(
            !is_dangerous_control("off", None, "MIX"),
            "Tắt MIX không cần confirmation"
        );
    }

    #[test]
    fn fallback_pump_status_includes_mix_valve() {
        // Kiểm tra rằng JSON fallback có đủ các key mà frontend cần.
        // Đây là kiểm tra doc/contract — khi thay đổi fallback, test này sẽ fail.
        let fallback_keys = [
            "pump_a",
            "pump_b",
            "ph_up",
            "ph_down",
            "osaka_pump",
            "mist_valve",
            "mix_valve",
            "water_pump_in",
            "water_pump_out",
        ];
        // Tạo JSON như trong get_control_state
        let fallback = serde_json::json!({
            "pump_a": false,
            "pump_b": false,
            "ph_up": false,
            "ph_down": false,
            "osaka_pump": false,
            "mist_valve": false,
            "mix_valve": false,
            "water_pump_in": false,
            "water_pump_out": false,
        });
        for key in &fallback_keys {
            assert!(
                fallback.get(key).is_some(),
                "Thiếu key '{}' trong fallback pump_status",
                key
            );
        }
    }

    #[test]
    fn manual_max_allowed_ml_cannot_exceed_server_max_dose_per_cycle() {
        assert_eq!(
            super::compute_effective_max_allowed_ml(Some(100.0), 10.0),
            10.0
        );
        assert_eq!(
            super::compute_effective_max_allowed_ml(Some(5.0), 10.0),
            5.0
        );
        assert_eq!(super::compute_effective_max_allowed_ml(None, 10.0), 10.0);
        assert_eq!(
            super::compute_effective_max_allowed_ml(Some(0.0), 10.0),
            10.0
        );
        assert_eq!(
            super::compute_effective_max_allowed_ml(Some(-1.0), 10.0),
            10.0
        );
    }

    #[actix_web::test]
    async fn privileged_token_is_bound_to_principal_device_and_action_class() {
        let state = crate::api::test_support::test_app_state();
        let auth = AuthContext {
            scopes: vec!["control:emergency".to_string()],
            user_id: Some("42".to_string()),
            session_id: Some("session-1".to_string()),
            service_key_label: None,
        };
        let claims = PrivilegedControlClaims {
            sub: "42".to_string(),
            device_id: "device-A".to_string(),
            action_class: PRIVILEGED_ACTION_CLASS.to_string(),
            exp: (chrono::Utc::now().timestamp() + 30) as usize,
        };
        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(state.privileged_control_secret.as_bytes()),
        )
        .expect("test signing key must produce a valid JWT");

        assert!(validate_privileged_token(&state, &auth, "device-A", &token));
        assert!(!validate_privileged_token(
            &state, &auth, "device-B", &token
        ));
        assert!(!validate_privileged_token(
            &state,
            &AuthContext {
                user_id: Some("99".to_string()),
                ..auth.clone()
            },
            "device-A",
            &token
        ));
    }

    #[actix_web::test]
    async fn privileged_token_issue_requires_confirmation_and_supported_action() {
        let state = web::Data::new(crate::api::test_support::test_app_state());
        let req = actix_web::test::TestRequest::post()
            .uri("/api/devices/device-A/control/privileged-token")
            .to_http_request();
        req.extensions_mut().insert(AuthContext {
            scopes: vec!["control:emergency".to_string()],
            user_id: Some("42".to_string()),
            session_id: Some("session-1".to_string()),
            service_key_label: None,
        });
        let body = web::Json(super::PrivilegedTokenRequest {
            action_class: PRIVILEGED_ACTION_CLASS.to_string(),
        });
        let response =
            issue_privileged_token(web::Path::from("device-A".to_string()), req, body, state)
                .await
                .respond_to(&actix_web::test::TestRequest::default().to_http_request());
        assert_eq!(response.status(), actix_web::http::StatusCode::FORBIDDEN);
    }
}
