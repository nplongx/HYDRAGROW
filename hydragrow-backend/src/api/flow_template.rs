use crate::models::script::{ApplyTemplateRequest, FlowTemplateOverride, UserScript};
use crate::api::middleware::auth::AuthContext;
use crate::AppState;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde_json::json;
use tracing::warn;
use uuid::Uuid;

/// POST /api/devices/{device_id}/scripts/{script_id}/apply-template
pub async fn apply_template(
    path: web::Path<(String, Uuid)>,
    http_req: HttpRequest,
    body: web::Json<ApplyTemplateRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let (source_device_id, source_script_id) = path.into_inner();
    let auth = http_req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("script:write") {
        return HttpResponse::Forbidden().json(json!({"error": "Missing scope script:write"}));
    }

    let source_script = match sqlx::query_as::<_, UserScript>(
        "SELECT * FROM user_scripts WHERE id = $1 AND device_id = $2",
    )
    .bind(source_script_id)
    .bind(&source_device_id)
    .fetch_optional(&app_state.pg_pool)
    .await
    {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Source script not found"})),
        Err(e) => {
            warn!(error = %e, "Failed to fetch source script");
            return HttpResponse::InternalServerError().json(json!({"error": "DB error"}));
        }
    };

    let mut applied_overrides = Vec::new();
    let mut overrides_ids = Vec::new();

    for target_device_id in body.target_device_ids.iter() {
        let new_script_id = Uuid::new_v4();
        let next_flow_ids_json = sqlx::types::Json(source_script.next_flow_ids.clone());

        // Create copied script
        let result = sqlx::query(
            r#"INSERT INTO user_scripts (id, device_id, kind, name, source, enabled, ir_json, next_flow_ids)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(new_script_id)
        .bind(target_device_id)
        .bind(&source_script.kind)
        .bind(&source_script.name)
        .bind(&source_script.source)
        .bind(source_script.enabled)
        .bind(&source_script.ir_json)
        .bind(&next_flow_ids_json)
        .execute(&app_state.pg_pool)
        .await;

        if let Err(e) = result {
            warn!(error = %e, "Failed to clone script for device");
            continue;
        }

        // Insert override record
        let empty_fields = sqlx::types::Json(Vec::<String>::new());
        let override_res = sqlx::query_as::<_, FlowTemplateOverride>(
            r#"INSERT INTO flow_template_overrides (source_script_id, target_device_id, override_script_id, overridden_fields)
               VALUES ($1, $2, $3, $4)
               RETURNING *"#,
        )
        .bind(source_script_id)
        .bind(target_device_id)
        .bind(new_script_id)
        .bind(&empty_fields)
        .fetch_one(&app_state.pg_pool)
        .await;

        match override_res {
            Ok(ovr) => {
                applied_overrides.push(ovr);
                overrides_ids.push(new_script_id);
                // Also trigger cache reload for target device
                let _ = app_state.script_cache.reload_device(&app_state.pg_pool, target_device_id).await;
            }
            Err(e) => {
                warn!(error = %e, "Failed to insert template override");
            }
        }
    }

    HttpResponse::Ok().json(json!({
        "status": "applied",
        "override_script_ids": overrides_ids,
        "overrides": applied_overrides
    }))
}

/// POST /api/devices/{device_id}/scripts/{script_id}/sync-template
pub async fn sync_template(
    path: web::Path<(String, Uuid)>,
    http_req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let (source_device_id, source_script_id) = path.into_inner();
    let auth = http_req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("script:write") {
        return HttpResponse::Forbidden().json(json!({"error": "Missing scope script:write"}));
    }

    let source_script = match sqlx::query_as::<_, UserScript>(
        "SELECT * FROM user_scripts WHERE id = $1 AND device_id = $2",
    )
    .bind(source_script_id)
    .bind(&source_device_id)
    .fetch_optional(&app_state.pg_pool)
    .await
    {
        Ok(Some(s)) => s,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Source script not found"})),
        Err(e) => {
            warn!(error = %e, "Failed to fetch source script");
            return HttpResponse::InternalServerError().json(json!({"error": "DB error"}));
        }
    };

    let overrides = match sqlx::query_as::<_, FlowTemplateOverride>(
        "SELECT * FROM flow_template_overrides WHERE source_script_id = $1",
    )
    .bind(source_script_id)
    .fetch_all(&app_state.pg_pool)
    .await
    {
        Ok(ovrs) => ovrs,
        Err(e) => {
            warn!(error = %e, "Failed to fetch template overrides");
            return HttpResponse::InternalServerError().json(json!({"error": "DB error"}));
        }
    };

    let mut synced_count = 0;

    for ovr in overrides {
        // fetch target script
        let target_script = match sqlx::query_as::<_, UserScript>(
            "SELECT * FROM user_scripts WHERE id = $1 AND device_id = $2",
        )
        .bind(ovr.override_script_id)
        .bind(&ovr.target_device_id)
        .fetch_optional(&app_state.pg_pool)
        .await
        {
            Ok(Some(s)) => s,
            _ => continue,
        };

        // Merge ir_json shallow
        let mut new_ir = source_script.ir_json.clone();
        if let Some(src_ir) = new_ir.as_mut()
            && let Some(src_obj) = src_ir.as_object_mut()
            && let Some(tgt_ir) = target_script.ir_json.as_ref()
            && let Some(tgt_obj) = tgt_ir.as_object()
        {
            for field in ovr.overridden_fields.iter() {
                if let Some(tgt_val) = tgt_obj.get(field) {
                    src_obj.insert(field.clone(), tgt_val.clone());
                }
            }
        }

        let next_flow_ids_json = sqlx::types::Json(source_script.next_flow_ids.clone());
        // Update target script
        let update_res = sqlx::query(
            r#"UPDATE user_scripts
               SET name = $1, source = $2, ir_json = $3, next_flow_ids = $4, updated_at = NOW()
               WHERE id = $5"#,
        )
        .bind(&source_script.name)
        .bind(&source_script.source)
        .bind(&new_ir)
        .bind(&next_flow_ids_json)
        .bind(ovr.override_script_id)
        .execute(&app_state.pg_pool)
        .await;

        if update_res.is_ok() {
            synced_count += 1;
            let _ = app_state.script_cache.reload_device(&app_state.pg_pool, &ovr.target_device_id).await;
        }
    }

    HttpResponse::Ok().json(json!({
        "status": "synced",
        "synced_devices_count": synced_count
    }))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/{script_id}/apply-template", web::post().to(apply_template))
        .route("/{script_id}/sync-template", web::post().to(sync_template));
}
