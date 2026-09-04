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

async fn receive_webhook_flow_event(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
    payload: web::Json<serde_json::Value>,
) -> HttpResponse {
    let device_id = path.into_inner();

    let is_authorized_webhook = extract_webhook_auth(&req, &app_state.pg_pool, &device_id).await;
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

    let Some(body) = payload.as_object() else {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Body phải là JSON object"
        }));
    };

    let alert_scripts = app_state.script_cache.get_alert_scripts(&device_id).await;
    let action_scripts = app_state
        .script_cache
        .get_action_command_scripts(&device_id)
        .await;

    let all_cached: Vec<_> = alert_scripts.into_iter().chain(action_scripts).collect();

    let all_chain_nodes: Vec<crate::mqtt::handlers::script_eval::WebhookChainNode> = all_cached
        .iter()
        .map(|s| crate::mqtt::handlers::script_eval::WebhookChainNode {
            id: s.id,
            kind: match s.kind.as_str() {
                "alert" => crate::models::script::ScriptKind::Alert,
                "action_command" => crate::models::script::ScriptKind::ActionCommand,
                _ => crate::models::script::ScriptKind::Alert,
            },
            next_flow_ids: s.next_flow_ids.clone(),
            ast: s.ast.clone(),
        })
        .collect();

    let mut webhook_scripts = Vec::new();
    for s in &all_cached {
        if let Some(ir) = s.ir_json.clone() {
            let mode = ir
                .get("trigger")
                .and_then(|t| t.get("mode"))
                .and_then(|m| m.as_str())
                .unwrap_or("flow");
            let is_webhook = ir
                .get("trigger")
                .and_then(|t| t.get("type"))
                .and_then(|t| t.as_str())
                == Some("webhook");
            if is_webhook && mode == "flow" {
                webhook_scripts.push((s.clone(), ir));
            }
        }
    }

    let engine = std::sync::Arc::new(crate::services::script_engine::ScriptEngine::new());
    let mut fired_total = 0usize;

    for (_script, ir) in webhook_scripts {
        let mut mapped = serde_json::Map::new();
        if let Some(mappings) = ir
            .get("trigger")
            .and_then(|t| t.get("fieldMappings"))
            .and_then(|m| m.as_array())
        {
            for m in mappings {
                let body_path = m.get("bodyPath").and_then(|p| p.as_str()).unwrap_or("");
                let target_field = m.get("targetField").and_then(|f| f.as_str()).unwrap_or("");
                if !body_path.is_empty() && !target_field.is_empty() {
                    let pointer_path = format!("/{}", body_path.replace('.', "/"));
                    let val = payload.pointer(&pointer_path).or_else(|| body.get(body_path));
                    if let Some(v) = val {
                        mapped.insert(target_field.to_string(), v.clone());
                    }
                }
            }
        } else {
            for (k, v) in body {
                mapped.insert(k.clone(), v.clone());
            }
        }

        let results = crate::mqtt::handlers::script_eval::eval_webhook_chain(
            &engine,
            &all_chain_nodes,
            &mapped,
        );
        fired_total += results.len();

        for (_id, res) in results {
            match res {
                crate::mqtt::handlers::script_eval::ChainFireResult::ActionCommand(cmd) => {
                    let safety_config =
                        crate::db::postgres::get_safety_config(&app_state.pg_pool, &device_id)
                            .await
                            .ok();
                    if let Some(cfg) = safety_config {
                        let limits = hydragrow_shared::safety::DoseSafetyLimits {
                            max_dose_per_cycle_ml: cfg.max_dose_per_cycle,
                            max_dose_per_hour_ml: cfg.max_dose_per_hour,
                            cooldown_sec: cfg.cooldown_sec as u64,
                        };
                        let calibration = crate::db::postgres::fetch_dosing_calibration(
                            &app_state.pg_pool,
                            &device_id,
                        )
                        .await
                        .unwrap_or(None);
                        let hourly_history_ml = crate::db::postgres::get_dosing_history_last_hour(
                            &app_state.pg_pool,
                            &device_id,
                        )
                        .await
                        .unwrap_or_default();
                        let last_dose_at_sec =
                            crate::db::postgres::get_last_dose_at(&app_state.pg_pool, &device_id)
                                .await
                                .unwrap_or(None);
                        let now_sec = (chrono::Utc::now().timestamp_millis() / 1000) as u64;

                        let _ = crate::services::action_dispatch::dispatch_action_command(
                            &app_state,
                            &device_id,
                            cmd,
                            &limits,
                            &hourly_history_ml,
                            now_sec,
                            last_dose_at_sec,
                            calibration.as_ref(),
                        )
                        .await;
                    }
                }
                crate::mqtt::handlers::script_eval::ChainFireResult::Alert(alert) => {
                    let alert_msg =
                        crate::mqtt::handlers::script_eval::alert_output_to_system_alert(
                            alert,
                            &device_id,
                            chrono::Utc::now().timestamp_millis(),
                        );
                    let db_record = crate::db::postgres::NewSystemEventRecord {
                        device_id: device_id.clone(),
                        level: alert_msg.level.clone(),
                        category: alert_msg.category.clone(),
                        title: alert_msg.title.clone(),
                        message: alert_msg.message.clone(),
                        reason: alert_msg.reason.clone(),
                        metadata: alert_msg.metadata.clone(),
                        timestamp: alert_msg.timestamp as i64,
                    };
                    let _ =
                        crate::db::postgres::insert_system_event(&app_state.pg_pool, &db_record)
                            .await;
                }
                _ => {}
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({ "status": "success", "fired_count": fired_total }))
}

pub fn routes() -> Scope {
    web::scope("/devices/{device_id}/webhook")
        .route("/action", web::post().to(receive_webhook_action))
        .route("/flow-event", web::post().to(receive_webhook_flow_event))
}
