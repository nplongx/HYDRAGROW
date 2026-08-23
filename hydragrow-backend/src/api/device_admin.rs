//! Privileged controller administration: OTA and WiFi provisioning.

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use hydragrow_shared::{MqttCommandOut, MqttCommandParams, WifiCandidate};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::AppState;
use crate::api::middleware::auth::AuthContext;
use crate::api::mqtt_utils::publish_command;

#[derive(Debug, Serialize)]
pub struct OtaStatusResponse {
    pub device_id: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

pub fn build_ota_status_response(
    current_version: String,
    latest_version: Option<String>,
) -> OtaStatusResponse {
    let update_available = latest_version
        .as_ref()
        .is_some_and(|latest| current_version != "unknown" && latest != &current_version);
    OtaStatusResponse {
        device_id: String::new(),
        current_version,
        latest_version,
        update_available,
    }
}

fn auth_from(req: &HttpRequest) -> AuthContext {
    req.extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default()
}

/// Supports the existing confirmation header and the existing elevated token mechanism.
fn has_dangerous_confirmation(req: &HttpRequest) -> bool {
    req.headers()
        .get("X-User-Confirmed")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("true") || value == "1")
        || std::env::var("ELEVATED_CONTROL_TOKEN")
            .ok()
            .is_some_and(|expected| {
                req.headers()
                    .get("X-Elevated-Token")
                    .and_then(|value| value.to_str().ok())
                    == Some(expected.as_str())
            })
}

pub async fn get_ota_status(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let current_version = app_state
        .device_firmware
        .read()
        .await
        .get(&device_id)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    let mut response = build_ota_status_response(current_version, fetch_latest_release_tag().await);
    response.device_id = device_id;
    HttpResponse::Ok().json(response)
}

async fn fetch_latest_release_tag() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;
    let response = client
        .get("https://api.github.com/repos/nplongx/HYDRAGROW/releases/latest")
        .header("User-Agent", "Hydragrow-Backend")
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?;
    response
        .json::<serde_json::Value>()
        .await
        .ok()?
        .get("tag_name")?
        .as_str()
        .map(str::to_owned)
}

pub async fn trigger_ota(
    path: web::Path<String>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    if !auth_from(&req).has_scope("device:ota") {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Missing required scope: device:ota"}));
    }
    if !has_dangerous_confirmation(&req) {
        return HttpResponse::Forbidden().json(serde_json::json!({"error": "Dangerous command requires X-User-Confirmed: true or X-Elevated-Token"}));
    }
    let command = MqttCommandOut {
        target: "all".to_string(),
        action: "trigger_ota".to_string(),
        params: None,
        ts: None,
        nonce: None,
        signature: None,
    };
    match publish_command(&app_state, &device_id, &command).await {
        Ok(()) => {
            info!(%device_id, "OTA trigger command sent");
            HttpResponse::Accepted()
                .json(serde_json::json!({"status":"ota_triggered", "device_id":device_id}))
        }
        Err(error) => {
            warn!(%device_id, ?error, "Failed to send OTA command");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error":"Could not send OTA command"}))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateWifiListReq {
    pub candidates: Vec<WifiCandidate>,
}

pub async fn update_wifi_list(
    path: web::Path<String>,
    req: HttpRequest,
    body: web::Json<UpdateWifiListReq>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    if !auth_from(&req).has_scope("device:network") {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Missing required scope: device:network"}));
    }
    let candidates: Vec<_> = body
        .into_inner()
        .candidates
        .into_iter()
        .filter(|candidate| !candidate.ssid.trim().is_empty())
        .collect();
    if candidates.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error":"At least one non-empty SSID is required"}));
    }
    if !has_dangerous_confirmation(&req) {
        return HttpResponse::Forbidden().json(serde_json::json!({"error":"Dangerous command requires X-User-Confirmed: true or X-Elevated-Token"}));
    }
    let command = MqttCommandOut {
        target: "all".to_string(),
        action: "update_wifi_list".to_string(),
        params: Some(MqttCommandParams {
            pump_id: None,
            duration_sec: None,
            pwm: None,
            state: None,
            ota_url: None,
            candidates: Some(candidates),
        }),
        ts: None,
        nonce: None,
        signature: None,
    };
    match publish_command(&app_state, &device_id, &command).await {
        Ok(()) => HttpResponse::Accepted()
            .json(serde_json::json!({"status":"wifi_list_sent", "device_id":device_id})),
        Err(error) => {
            warn!(%device_id, ?error, "Failed to send WiFi provisioning command");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error":"Could not send WiFi list"}))
        }
    }
}

pub async fn reboot_device(
    path: web::Path<String>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    if !auth_from(&req).has_scope("device:admin") {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Missing scope: device:admin"}));
    }
    if !has_dangerous_confirmation(&req) {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Requires X-User-Confirmed: true"}));
    }
    let command = MqttCommandOut {
        target: "all".to_string(),
        action: "reboot_device".to_string(),
        params: None, ts: None, nonce: None, signature: None,
    };
    match publish_command(&app_state, &device_id, &command).await {
        Ok(()) => HttpResponse::Accepted()
            .json(serde_json::json!({"status": "reboot_triggered", "device_id": device_id})),
        Err(e) => {
            warn!(%device_id, ?e, "Failed to send reboot command");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Could not send reboot command"}))
        }
    }
}

pub async fn factory_reset_device(
    path: web::Path<String>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    if !auth_from(&req).has_scope("device:admin") {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Missing scope: device:admin"}));
    }
    if !has_dangerous_confirmation(&req) {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Requires X-User-Confirmed: true"}));
    }
    let command = MqttCommandOut {
        target: "all".to_string(),
        action: "factory_reset".to_string(),
        params: None, ts: None, nonce: None, signature: None,
    };
    match publish_command(&app_state, &device_id, &command).await {
        Ok(()) => HttpResponse::Accepted()
            .json(serde_json::json!({"status": "factory_reset_triggered", "device_id": device_id})),
        Err(e) => {
            warn!(%device_id, ?e, "Failed to send factory_reset command");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Could not send factory_reset"}))
        }
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/ota/status", web::get().to(get_ota_status))
        .route("/ota/trigger", web::post().to(trigger_ota))
        .route("/wifi", web::post().to(update_wifi_list))
        .route("/reboot", web::post().to(reboot_device))
        .route("/factory-reset", web::post().to(factory_reset_device));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ota_status_marks_version_difference_available() {
        assert!(build_ota_status_response("v1.2.0".into(), Some("v1.3.0".into())).update_available);
    }
    #[test]
    fn ota_status_does_not_mark_matching_version_available() {
        assert!(
            !build_ota_status_response("v1.3.0".into(), Some("v1.3.0".into())).update_available
        );
    }
    #[test]
    fn ota_status_does_not_claim_update_without_latest_version() {
        assert!(!build_ota_status_response("v1.2.0".into(), None).update_available);
    }
}
