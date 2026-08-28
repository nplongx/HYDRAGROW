use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

use crate::AppState;
use crate::api::middleware::auth::AuthContext;
use crate::models::script::{ScriptValidateResponse, UpsertScriptRequest, UserScript};

// ─── Validation helpers ───────────────────────────────────────────────────────

pub fn validate_kind(kind: &str) -> Result<(), String> {
    match kind {
        "alert" | "recipe_override" => Ok(()),
        other => Err(format!(
            "kind phải là 'alert' hoặc 'recipe_override', nhận: '{}'",
            other
        )),
    }
}

/// Compile script và gọi thử fn main với dummy input để đảm bảo hàm tồn tại.
pub fn validate_script_source(_kind: &str, source: &str) -> Result<(), String> {
    let engine = rhai::Engine::new();
    let ast = engine
        .compile(source)
        .map_err(|e| format!("Lỗi compile: {}", e))?;

    use rhai::{Dynamic, Map, Scope};
    let dummy_map = Map::new();
    let result = engine.call_fn::<Dynamic>(
        &mut Scope::new(),
        &ast,
        "main",
        (Dynamic::from_map(dummy_map),),
    );

    match result {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("Function not found") => {
            Err("Script phải định nghĩa hàm `fn main(input)`".to_string())
        }
        Err(_) => Ok(()), // Runtime error với dummy input là OK — fn main tồn tại
    }
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// GET /api/devices/{device_id}/scripts
pub async fn list_scripts(
    path: web::Path<String>,
    http_req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let auth = http_req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("script:read") {
        return HttpResponse::Forbidden().json(json!({"error": "Missing scope script:read"}));
    }

    let rows = sqlx::query_as::<_, UserScript>(
        "SELECT * FROM user_scripts WHERE device_id = $1 ORDER BY created_at DESC",
    )
    .bind(&device_id)
    .fetch_all(&app_state.pg_pool)
    .await;

    match rows {
        Ok(scripts) => HttpResponse::Ok().json(json!({"status": "success", "data": scripts})),
        Err(e) => {
            warn!(device_id, error = %e, "Failed to list scripts");
            HttpResponse::InternalServerError().json(json!({"error": "DB error"}))
        }
    }
}

/// POST /api/devices/{device_id}/scripts
pub async fn create_script(
    path: web::Path<String>,
    http_req: HttpRequest,
    body: web::Json<UpsertScriptRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let auth = http_req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("script:write") {
        return HttpResponse::Forbidden().json(json!({"error": "Missing scope script:write"}));
    }

    if let Err(e) = validate_kind(&body.kind) {
        return HttpResponse::BadRequest().json(json!({"error": e}));
    }
    if let Err(e) = validate_script_source(&body.kind, &body.source) {
        return HttpResponse::BadRequest().json(json!({"error": e, "valid": false}));
    }

    let id = Uuid::new_v4();
    let enabled = body.enabled.unwrap_or(true);
    let result = sqlx::query_as::<_, UserScript>(
        r#"INSERT INTO user_scripts (id, device_id, kind, name, source, enabled)
VALUES ($1, $2, $3, $4, $5, $6)
RETURNING *"#,
    )
    .bind(id)
    .bind(&device_id)
    .bind(&body.kind)
    .bind(&body.name)
    .bind(&body.source)
    .bind(enabled)
    .fetch_one(&app_state.pg_pool)
    .await;

    match result {
        Ok(script) => {
            info!(device_id, script_id = %script.id, kind = %script.kind, "Script created");
            HttpResponse::Created().json(json!({"status": "created", "data": script}))
        }
        Err(e) => {
            warn!(device_id, error = %e, "Failed to insert script");
            HttpResponse::InternalServerError().json(json!({"error": "DB error"}))
        }
    }
}

/// PUT /api/devices/{device_id}/scripts/{script_id}
pub async fn update_script(
    path: web::Path<(String, Uuid)>,
    http_req: HttpRequest,
    body: web::Json<UpsertScriptRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let (device_id, script_id) = path.into_inner();
    let auth = http_req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("script:write") {
        return HttpResponse::Forbidden().json(json!({"error": "Missing scope script:write"}));
    }

    if let Err(e) = validate_kind(&body.kind) {
        return HttpResponse::BadRequest().json(json!({"error": e}));
    }
    if let Err(e) = validate_script_source(&body.kind, &body.source) {
        return HttpResponse::BadRequest().json(json!({"error": e, "valid": false}));
    }

    let enabled = body.enabled.unwrap_or(true);
    let result = sqlx::query_as::<_, UserScript>(
        r#"UPDATE user_scripts
SET kind = $1, name = $2, source = $3, enabled = $4, updated_at = NOW()
WHERE id = $5 AND device_id = $6
RETURNING *"#,
    )
    .bind(&body.kind)
    .bind(&body.name)
    .bind(&body.source)
    .bind(enabled)
    .bind(script_id)
    .bind(&device_id)
    .fetch_optional(&app_state.pg_pool)
    .await;

    match result {
        Ok(Some(script)) => HttpResponse::Ok().json(json!({"status": "updated", "data": script})),
        Ok(None) => HttpResponse::NotFound().json(json!({"error": "Script not found"})),
        Err(e) => {
            warn!(error = %e, "Failed to update script");
            HttpResponse::InternalServerError().json(json!({"error": "DB error"}))
        }
    }
}

/// DELETE /api/devices/{device_id}/scripts/{script_id}
pub async fn delete_script(
    path: web::Path<(String, Uuid)>,
    http_req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let (device_id, script_id) = path.into_inner();
    let auth = http_req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("script:write") {
        return HttpResponse::Forbidden().json(json!({"error": "Missing scope script:write"}));
    }

    let result = sqlx::query("DELETE FROM user_scripts WHERE id = $1 AND device_id = $2")
        .bind(script_id)
        .bind(&device_id)
        .execute(&app_state.pg_pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok().json(json!({"status": "deleted"})),
        Ok(_) => HttpResponse::NotFound().json(json!({"error": "Script not found"})),
        Err(e) => {
            warn!(error = %e, "Failed to delete script");
            HttpResponse::InternalServerError().json(json!({"error": "DB error"}))
        }
    }
}

/// POST /api/devices/{device_id}/scripts/validate — dry-run, không lưu DB
pub async fn validate_script(
    path: web::Path<String>,
    http_req: HttpRequest,
    body: web::Json<UpsertScriptRequest>,
) -> impl Responder {
    let _device_id = path.into_inner();
    let auth = http_req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("script:read") {
        return HttpResponse::Forbidden().json(json!({"error": "Missing scope script:read"}));
    }

    if let Err(e) = validate_kind(&body.kind) {
        return HttpResponse::Ok().json(ScriptValidateResponse {
            valid: false,
            error: Some(e),
        });
    }

    match validate_script_source(&body.kind, &body.source) {
        Ok(()) => HttpResponse::Ok().json(ScriptValidateResponse {
            valid: true,
            error: None,
        }),
        Err(e) => HttpResponse::Ok().json(ScriptValidateResponse {
            valid: false,
            error: Some(e),
        }),
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(list_scripts))
        .route("", web::post().to(create_script))
        .route("/validate", web::post().to(validate_script))
        .route("/{script_id}", web::put().to(update_script))
        .route("/{script_id}", web::delete().to(delete_script));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_alert_script_source_passes_validation() {
        let src = r#"fn main(input) { if input.ph < 7.0 { return (); } #{ level: "warning", title: "pH", message: "cao" } }"#;
        assert!(validate_script_source("alert", src).is_ok());
    }

    #[test]
    fn script_with_syntax_error_fails_validation() {
        let src = r#"fn main(input { }"#;
        let result = validate_script_source("alert", src);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("compile") || msg.contains("syntax") || msg.len() > 5);
    }

    #[test]
    fn script_without_main_function_fails_validation() {
        let src = r#"let x = 1;"#; // valid syntax nhưng không có fn main
        let result = validate_script_source("alert", src);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_kind_rejected() {
        let result = validate_kind("unknown_kind");
        assert!(result.is_err());
    }

    #[test]
    fn valid_kinds_accepted() {
        assert!(validate_kind("alert").is_ok());
        assert!(validate_kind("recipe_override").is_ok());
    }
}
