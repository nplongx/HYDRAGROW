//! Privileged controller administration: OTA and WiFi provisioning.

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use hydragrow_shared::{
    MqttCommandOut, MqttCommandParams, OtaProvisionParams, WifiCandidate, WifiProvisionConfig,
};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::AppState;
use crate::api::middleware::auth::AuthContext;
use crate::api::mqtt_utils::publish_command;
use crate::api::mqtt_utils::publish_sensor_command;
use crate::db::device_wifi;

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
    body: Option<web::Json<TriggerOtaReq>>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();

    if !auth_from(&req).has_scope("device:ota") {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Missing required scope: device:ota"}));
    }

    // The browser's current passwords are used to build the MQTT payload
    // immediately below. They are never read back from Postgres.
    let wifi = body.and_then(|body| body.into_inner().wifi);

    if wifi.is_some() && !auth_from(&req).has_scope("device:network") {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Missing required scope: device:network"}));
    }

    if !has_dangerous_confirmation(&req) {
        return HttpResponse::Forbidden().json(
            serde_json::json!({"error": "Dangerous command requires X-User-Confirmed: true or X-Elevated-Token"}),
        );
    }

    if wifi.is_some() && !check_provision_throttle(&device_id) {
        warn!(%device_id, "Provision throttle exceeded");
        return HttpResponse::TooManyRequests().json(
            serde_json::json!({"error": "Too many provisioning attempts for this device; retry later"}),
        );
    }

    if let Some(config) = &wifi {
        if let Err(reason) = hydragrow_shared::wifi_tx::validate_provision_structure(config) {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"error": format!("Invalid wifi provision: {reason}")}));
        }

        match device_wifi::get_wifi_metadata(&app_state.pg_pool, &device_id).await {
            Ok((_, stored_version)) if config.config_version <= stored_version => {
                return HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Stale wifi config version",
                    "stored_version": stored_version,
                }));
            }
            Err(error) => {
                warn!(%device_id, ?error, "Failed to load stored wifi version");
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({"error": "Could not load wifi metadata"}));
            }
            _ => {}
        }
    }

    let command = build_update_firmware_command(wifi.clone());

    // Audit log uses the metadata-only summary — never the command itself.
    let audit = command_audit_summary(&command);

    match publish_command(&app_state, &device_id, &command).await {
        Ok(()) => {
            info!(%device_id, %audit, "OTA provision command sent");

            if let Some(config) = &wifi {
                // Persist SSID metadata only — passwords never cross this boundary.
                let metadata = device_wifi::metadata_from_provision(config);

                if let Err(error) = device_wifi::replace_wifi_metadata(
                    &app_state.pg_pool,
                    &device_id,
                    &metadata,
                    config.config_version,
                )
                .await
                {
                    warn!(
                        %device_id,
                        ?error,
                        "OTA sent but wifi metadata persist failed"
                    );

                    return HttpResponse::Accepted().json(serde_json::json!({
                        "status": "ota_provision_sent",
                        "device_id": device_id,
                        "config_version": config.config_version,
                        "warning": "metadata persist failed; device state is authoritative",
                    }));
                }

                if let Err(error) = device_wifi::set_delivery_state(
                    &app_state.pg_pool,
                    &device_id,
                    config.config_version,
                    device_wifi::delivery_state::PENDING,
                    None,
                )
                .await
                {
                    warn!(%device_id, ?error, "Failed to record wifi delivery state");
                }

                return HttpResponse::Accepted().json(serde_json::json!({
                    "status": "ota_provision_sent",
                    "device_id": device_id,
                    "config_version": config.config_version,
                }));
            }

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

/// Metadata-only audit summary for a provision command. Passwords never
/// appear here — log this instead of the command itself.
pub fn command_audit_summary(command: &MqttCommandOut) -> String {
    let (count, version) = command
        .params
        .as_ref()
        .and_then(|params| params.ota_provision.as_ref())
        .and_then(|provision| provision.wifi.as_ref())
        .map(|wifi| (wifi.entries.len(), wifi.config_version))
        .unwrap_or((0, 0));

    format!(
        "action={} wifi_entries={} config_version={}",
        command.action, count, version
    )
}

/// Explicit per-device provision throttle: at most PROVISION_MAX_ATTEMPTS
/// combined OTA+WiFi provisions per PROVISION_WINDOW. Pure decision helper
/// so the policy is unit-testable without the global map.
pub const PROVISION_MAX_ATTEMPTS: usize = 5;
pub const PROVISION_WINDOW_SECS: u64 = 300;

pub fn provision_allowed(attempts: &mut Vec<std::time::Instant>, now: std::time::Instant) -> bool {
    attempts.retain(|at| now.duration_since(*at).as_secs() < PROVISION_WINDOW_SECS);

    if attempts.len() >= PROVISION_MAX_ATTEMPTS {
        return false;
    }

    attempts.push(now);
    true
}

static PROVISION_THROTTLE: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, Vec<std::time::Instant>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

fn check_provision_throttle(device_id: &str) -> bool {
    let now = std::time::Instant::now();
    let mut map = PROVISION_THROTTLE.lock().unwrap_or_else(|e| e.into_inner());
    let attempts = map.entry(device_id.to_string()).or_default();
    provision_allowed(attempts, now)
}

/// Build the fleet `update_firmware` command. OTA-only when `wifi` is None;
/// combined OTA+WiFi otherwise. Passwords stay in the returned MQTT payload
/// (transient) and must never be persisted by callers.
pub fn build_update_firmware_command(wifi: Option<WifiProvisionConfig>) -> MqttCommandOut {
    MqttCommandOut {
        target: "all".to_string(),
        action: "update_firmware".to_string(),
        params: Some(MqttCommandParams {
            pump_id: None,
            duration_sec: None,
            pwm: None,
            state: None,
            ota_url: None,
            candidates: None,
            ota_provision: Some(OtaProvisionParams {
                firmware_release: None,
                wifi,
            }),
        }),
        ts: None,
        nonce: None,
        signature: None,
    }
}

#[derive(Debug, Deserialize)]
pub struct TriggerOtaReq {
    #[serde(default)]
    pub wifi: Option<WifiProvisionConfig>,
}

/// Password-blind desired-state view: SSIDs, version, delivery state. No secrets.
pub async fn get_wifi_config(
    path: web::Path<String>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();

    if !auth_from(&req).has_scope("device:network") {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Missing required scope: device:network"}));
    }

    match device_wifi::get_wifi_config_view(&app_state.pg_pool, &device_id).await {
        Ok(view) => HttpResponse::Ok().json(view),
        Err(error) => {
            warn!(%device_id, ?error, "Failed to load wifi config view");

            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Could not load wifi config"}))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateWifiListReq {
    pub candidates: Vec<WifiCandidate>,
}

/// LEGACY network-only provisioning path. Kept for migration; the UI must use
/// POST /ota/trigger with a wifi payload instead. Emits a metric-style log
/// (no secrets) when used so the rollout can track remaining callers.
///
/// Strips passwords out of a candidate list before it touches the database
/// — the DB schema has no column to put one in, but this keeps the
/// conversion itself unit-testable and explicit about the boundary.
pub fn candidates_to_ssid_entries(
    candidates: &[WifiCandidate],
) -> Vec<crate::db::device_wifi::WifiSsidEntry> {
    candidates
        .iter()
        .map(|c| crate::db::device_wifi::WifiSsidEntry {
            ssid: c.ssid.clone(),
            priority: c.priority as i16,
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct WifiConfigEntryResponse {
    pub ssid: String,
    pub priority: i16,
}

#[derive(Debug, Serialize)]
pub struct WifiConfigResponse {
    pub device_id: String,
    pub ssids: Vec<WifiConfigEntryResponse>,
    pub config_version: i64,
}

pub fn build_wifi_config_response(
    device_id: String,
    rows: Vec<crate::db::device_wifi::DeviceWifiConfigRow>,
) -> WifiConfigResponse {
    let config_version = rows.first().map(|r| r.config_version).unwrap_or(0);

    let ssids = rows
        .into_iter()
        .map(|r| WifiConfigEntryResponse {
            ssid: r.ssid,
            priority: r.priority,
        })
        .collect();

    WifiConfigResponse {
        device_id,
        ssids,
        config_version,
    }
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
        return HttpResponse::Forbidden().json(
            serde_json::json!({"error":"Dangerous command requires X-User-Confirmed: true or X-Elevated-Token"}),
        );
    }

    info!(%device_id, ssid_count = candidates.len(), "legacy POST /wifi used");

    let ssid_entries = candidates_to_ssid_entries(&candidates);

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
            ota_provision: None,
        }),
        ts: None,
        nonce: None,
        signature: None,
    };

    match publish_command(&app_state, &device_id, &command).await {
        Ok(()) => {
            if let Err(error) = crate::db::device_wifi::replace_device_wifi_config(
                &app_state.pg_pool,
                &device_id,
                &ssid_entries,
            )
            .await
            {
                warn!(
                    %device_id,
                    ?error,
                    "Failed to persist WiFi config metadata to DB"
                );
            }

            HttpResponse::Accepted()
                .json(serde_json::json!({"status":"wifi_list_sent", "device_id":device_id}))
        }
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
        params: None,
        ts: None,
        nonce: None,
        signature: None,
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
        params: None,
        ts: None,
        nonce: None,
        signature: None,
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

#[derive(Debug, Serialize)]
pub struct DeviceStatusResponse {
    pub is_online: bool,
    pub firmware_version: String,
    pub last_seen: Option<String>,
}

pub async fn get_device_status(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();

    let states = app_state.device_states.read().await;
    let raw = states.get(&device_id).cloned();
    drop(states);

    let (is_online, last_seen) = match raw {
        Some(s) => {
            let parsed: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();

            let ts = parsed
                .get("controller_status_ts")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Online if a heartbeat landed in the last 30s
            // (firmware publish cycle is 10s — see health.rs run_main_health_loop)
            let is_online = ts
                .as_ref()
                .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                .map(|dt| chrono::Utc::now().signed_duration_since(dt).num_seconds() < 30)
                .unwrap_or(false);

            (is_online, ts)
        }
        None => (false, None),
    };

    let firmware_version = app_state
        .device_firmware
        .read()
        .await
        .get(&device_id)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    HttpResponse::Ok().json(DeviceStatusResponse {
        is_online,
        firmware_version,
        last_seen,
    })
}

pub async fn trigger_sensor_ota(
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
        return HttpResponse::Forbidden().json(
            serde_json::json!({"error": "Dangerous command requires X-User-Confirmed:true or X-Elevated-Token"}),
        );
    }

    let command = MqttCommandOut {
        target: "all".to_string(),
        action: "trigger_ota".to_string(),
        params: None,
        ts: None,
        nonce: None,
        signature: None,
    };

    match publish_sensor_command(&app_state, &device_id, &command).await {
        Ok(()) => {
            info!(%device_id, "Sensor OTA command sent");
            HttpResponse::Accepted()
                .json(serde_json::json!({"status":"ota_triggered", "device_id":device_id}))
        }
        Err(error) => {
            warn!(%device_id, ?error, "Failed to send sensor OTA command");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"error":"Could not send sensor OTA command"}))
        }
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/status", web::get().to(get_device_status))
        .route("/ota/status", web::get().to(get_ota_status))
        .route("/ota/trigger", web::post().to(trigger_ota))
        .route("/sensor/ota/trigger", web::post().to(trigger_sensor_ota))
        .route("/wifi", web::get().to(get_wifi_config))
        .route("/wifi", web::post().to(update_wifi_list))
        .route("/reboot", web::post().to(reboot_device))
        .route("/factory-reset", web::post().to(factory_reset_device));
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use hydragrow_shared::{WifiProvisionEntry, WifiSecretAction};

    fn provision_config(version: i64) -> WifiProvisionConfig {
        WifiProvisionConfig {
            config_version: version,
            entries: vec![
                WifiProvisionEntry {
                    ssid: "Farm-A".into(),
                    priority: 0,
                    secret_action: WifiSecretAction::Set,
                    password: Some("super-secret".into()),
                },
                WifiProvisionEntry {
                    ssid: "Farm-Backup".into(),
                    priority: 1,
                    secret_action: WifiSecretAction::Keep,
                    password: None,
                },
            ],
        }
    }

    #[test]
    fn combined_ota_request_strips_password_for_db_metadata() {
        let config = provision_config(8);
        let metadata = device_wifi::metadata_from_provision(&config);

        assert_eq!(metadata.len(), 2);

        let json = serde_json::to_value(&metadata).unwrap();
        let serialized = json.to_string();

        assert!(!serialized.contains("super-secret"));
        assert!(!serialized.contains("password"));
        assert_eq!(json[0]["ssid"], "Farm-A");
    }

    #[test]
    fn ota_only_request_does_not_require_wifi_payload() {
        let command = build_update_firmware_command(None);

        assert_eq!(command.action, "update_firmware");

        let provision = command.params.unwrap().ota_provision.unwrap();
        assert!(provision.wifi.is_none());
    }

    #[test]
    fn combined_command_carries_transient_passwords_only_in_mqtt_payload() {
        let command = build_update_firmware_command(Some(provision_config(8)));

        let wifi = command.params.unwrap().ota_provision.unwrap().wifi.unwrap();

        assert_eq!(wifi.config_version, 8);
        assert_eq!(wifi.entries[0].password.as_deref(), Some("super-secret"));
    }

    #[test]
    fn keep_password_action_is_not_persisted() {
        let config = WifiProvisionConfig {
            config_version: 9,
            entries: vec![WifiProvisionEntry {
                ssid: "Farm-Backup".into(),
                priority: 1,
                secret_action: WifiSecretAction::Keep,
                password: None,
            }],
        };

        let metadata = device_wifi::metadata_from_provision(&config);

        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].ssid, "Farm-Backup");
        assert!(
            !serde_json::to_string(&metadata)
                .unwrap()
                .contains("password")
        );
    }

    #[test]
    fn update_wifi_db_row_has_no_password_field() {
        let entry = device_wifi::WifiSsidEntry {
            ssid: "Farm-A".into(),
            priority: 0,
        };

        let value = serde_json::to_value(&entry).unwrap();

        assert!(value.get("password").is_none());
        assert!(value.get("secret").is_none());
    }

    #[test]
    fn wifi_command_audit_output_does_not_include_password() {
        let secret = "DO_NOT_LOG_ME";

        let command = build_update_firmware_command(Some(WifiProvisionConfig {
            config_version: 8,
            entries: vec![WifiProvisionEntry {
                ssid: "Farm-A".into(),
                priority: 0,
                secret_action: WifiSecretAction::Set,
                password: Some(secret.into()),
            }],
        }));

        let audit = command_audit_summary(&command);

        assert!(!audit.contains(secret));
        assert!(!audit.contains("password"));
        assert!(audit.contains("config_version=8"));
    }

    #[test]
    fn provision_throttle_blocks_bursts() {
        let now = std::time::Instant::now();
        let mut attempts = Vec::new();

        for _ in 0..PROVISION_MAX_ATTEMPTS {
            assert!(provision_allowed(&mut attempts, now));
        }

        assert!(!provision_allowed(&mut attempts, now));

        // After the window passes, provisioning is allowed again.
        let later = now + std::time::Duration::from_secs(PROVISION_WINDOW_SECS + 1);
        assert!(provision_allowed(&mut attempts, later));
    }

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

    #[test]
    fn candidates_to_ssid_entries_strips_password() {
        let candidates = vec![WifiCandidate {
            ssid: "Home".to_string(),
            password: "supersecret".to_string(),
            priority: 0,
        }];

        let entries = candidates_to_ssid_entries(&candidates);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].ssid, "Home");
        assert_eq!(entries[0].priority, 0);
    }
}

#[cfg(test)]
mod status_tests {
    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn device_status_response_serializes_expected_shape() {
        let resp = DeviceStatusResponse {
            is_online: true,
            firmware_version: "1.2.3".to_string(),
            last_seen: Some("2026-08-23T10:00:00+00:00".to_string()),
        };

        let json = serde_json::to_value(&resp).unwrap();

        assert_eq!(json["is_online"], true);
        assert_eq!(json["firmware_version"], "1.2.3");
    }

    #[test]
    fn wifi_config_response_uses_first_rows_version_and_lists_all_ssids() {
        use chrono::Utc;

        let rows = vec![
            crate::db::device_wifi::DeviceWifiConfigRow {
                device_id: "esp-1".to_string(),
                ssid: "First".to_string(),
                priority: 0,
                config_version: 3,
                updated_at: Utc::now(),
            },
            crate::db::device_wifi::DeviceWifiConfigRow {
                device_id: "esp-1".to_string(),
                ssid: "Second".to_string(),
                priority: 1,
                config_version: 3,
                updated_at: Utc::now(),
            },
        ];

        let resp = build_wifi_config_response("esp-1".to_string(), rows);

        assert_eq!(resp.device_id, "esp-1");
        assert_eq!(resp.config_version, 3);
        assert_eq!(resp.ssids.len(), 2);
        assert_eq!(resp.ssids[0].ssid, "First");
    }

    #[test]
    fn wifi_config_response_defaults_version_to_zero_when_empty() {
        let resp = build_wifi_config_response("esp-empty".to_string(), vec![]);

        assert_eq!(resp.config_version, 0);
        assert!(resp.ssids.is_empty());
    }
}
