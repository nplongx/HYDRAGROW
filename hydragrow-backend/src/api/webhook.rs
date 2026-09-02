use actix_web::{HttpRequest, HttpResponse, Scope, web};
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::AppState;
use crate::api::webhook_tokens::{find_by_token_hash, sha256_hex};
use crate::models::script::ActionCommandOutput;

async fn extract_webhook_auth(req: &HttpRequest, pool: &PgPool, device_id: &str) -> bool {
    if let Some(token) = req
        .headers()
        .get("X-Webhook-Token")
        .and_then(|h| h.to_str().ok())
    {
        let hash = sha256_hex(token);
        return find_by_token_hash(pool, &hash)
            .await
            .is_some_and(|t| t.device_id == device_id && t.is_active);
    }
    false // caller fallback về X-API-Key nếu cần
}

async fn receive_webhook_action(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
    payload: web::Json<ActionCommandOutput>,
) -> HttpResponse {
    let device_id = path.into_inner();

    // Verify authentication
    let is_authorized_webhook = extract_webhook_auth(&req, &app_state.pg_pool, &device_id).await;

    // Fallback to X-API-Key if no valid webhook token was provided
    let is_authorized_api_key = if !is_authorized_webhook {
        let expected_api_key = &app_state.api_key;
        let header_key = req
            .headers()
            .get("X-API-Key")
            .and_then(|hv| hv.to_str().ok());

        header_key.is_some_and(|key| key == expected_api_key)
    } else {
        false
    };

    if !is_authorized_webhook && !is_authorized_api_key {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Unauthorized: Invalid or missing Webhook Token or API Key"
        }));
    }

    let output = payload.into_inner();

    // Update last_used_at for token
    if is_authorized_webhook
        && let Some(token) = req
            .headers()
            .get("X-Webhook-Token")
            .and_then(|h| h.to_str().ok())
    {
        let hash = sha256_hex(token);
        let _ = sqlx::query("UPDATE webhook_tokens SET last_used_at = NOW() WHERE token_hash = $1")
            .bind(hash)
            .execute(&app_state.pg_pool)
            .await;
    }

    let safety_config =
        match crate::db::postgres::get_safety_config(&app_state.pg_pool, &device_id).await {
            Ok(cfg) => cfg,
            Err(e) => {
                error!("Failed to fetch safety config for webhook: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Internal server error"
                }));
            }
        };

    let limits = hydragrow_shared::safety::DoseSafetyLimits {
        max_dose_per_cycle_ml: safety_config.max_dose_per_cycle,
        max_dose_per_hour_ml: safety_config.max_dose_per_hour,
        cooldown_sec: safety_config.cooldown_sec as u64,
    };

    let calibration = crate::db::postgres::fetch_dosing_calibration(&app_state.pg_pool, &device_id)
        .await
        .unwrap_or(None);

    let hourly_history_ml =
        crate::db::postgres::get_dosing_history_last_hour(&app_state.pg_pool, &device_id)
            .await
            .unwrap_or_default();
    let last_dose_at_sec = crate::db::postgres::get_last_dose_at(&app_state.pg_pool, &device_id)
        .await
        .unwrap_or(None);
    let now_sec = (chrono::Utc::now().timestamp_millis() / 1000) as u64;

    match crate::services::action_dispatch::dispatch_action_command(
        &app_state,
        &device_id,
        output,
        &limits,
        &hourly_history_ml,
        now_sec,
        last_dose_at_sec,
        calibration.as_ref(),
    )
    .await
    {
        Ok(()) => {
            info!(
                "Successfully dispatched webhook action for device {}",
                device_id
            );
            HttpResponse::Ok().json(serde_json::json!({ "success": true }))
        }
        Err(crate::services::action_dispatch::ActionDispatchError::Safety(violation)) => {
            warn!(
                "Webhook action blocked by safety constraints: {:?}",
                violation
            );
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Safety violation",
                "details": format!("{:?}", violation)
            }))
        }
        Err(e) => {
            error!("Webhook action dispatch failed: {:?}", e);
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Dispatch failed",
                "details": format!("{:?}", e)
            }))
        }
    }
}

pub fn routes() -> Scope {
    web::scope("/devices/{device_id}/webhook")
        .route("/action", web::post().to(receive_webhook_action))
}
