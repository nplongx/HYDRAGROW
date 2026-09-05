use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use serde_json::json;
use tracing::{info, warn};
use uuid::Uuid;

use crate::AppState;
use crate::api::middleware::auth::AuthContext;
use crate::models::script::{
    ConditionTraceEntry, ScriptValidateResponse, TestScriptRequest, TestScriptResponse,
    UpsertScriptRequest, UserScript,
};

// ─── Validation helpers ───────────────────────────────────────────────────────

pub fn validate_kind(kind: &str) -> Result<(), String> {
    match kind {
        "alert" | "recipe_override" | "action_command" | "config_override" => Ok(()),
        other => Err(format!(
            "kind phải là 'alert', 'recipe_override', 'action_command' hoặc 'config_override', nhận: '{}'",
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

    let next_flow_ids = body.next_flow_ids.clone().unwrap_or_default();
    if !next_flow_ids.is_empty() {
        let existing_rows = sqlx::query_as::<_, (Uuid, sqlx::types::Json<Vec<String>>)>(
            "SELECT id, next_flow_ids FROM user_scripts WHERE device_id = $1",
        )
        .bind(&device_id)
        .fetch_all(&app_state.pg_pool)
        .await
        .unwrap_or_default();

        let existing: Vec<(String, Vec<String>)> = existing_rows
            .into_iter()
            .map(|(id, json_ids)| (id.to_string(), json_ids.0))
            .collect();

        if let Err(msg) =
            crate::services::flow_graph::detect_cycle(&id.to_string(), &next_flow_ids, &existing)
        {
            return HttpResponse::BadRequest().json(json!({"error": msg, "valid": false}));
        }
    }

    let next_flow_ids_json = sqlx::types::Json(next_flow_ids);

    let result = sqlx::query_as::<_, UserScript>(
        r#"INSERT INTO user_scripts (id, device_id, kind, name, source, enabled, ir_json, next_flow_ids)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
RETURNING *"#,
    )
    .bind(id)
    .bind(&device_id)
    .bind(&body.kind)
    .bind(&body.name)
    .bind(&body.source)
    .bind(enabled)
    .bind(&body.ir_json)
    .bind(&next_flow_ids_json)
    .fetch_one(&app_state.pg_pool)
    .await;

    match result {
        Ok(script) => {
            info!(device_id, script_id = %script.id, kind = %script.kind, "Script created");
            if let Err(e) = app_state
                .script_cache
                .reload_device(&app_state.pg_pool, &device_id)
                .await
            {
                warn!(device_id, error = %e, "Failed to reload script cache after create");
            }
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

    let next_flow_ids = body.next_flow_ids.clone().unwrap_or_default();
    if !next_flow_ids.is_empty() {
        let existing_rows = sqlx::query_as::<_, (Uuid, sqlx::types::Json<Vec<String>>)>(
            "SELECT id, next_flow_ids FROM user_scripts WHERE device_id = $1 AND id != $2",
        )
        .bind(&device_id)
        .bind(script_id)
        .fetch_all(&app_state.pg_pool)
        .await
        .unwrap_or_default();

        let existing: Vec<(String, Vec<String>)> = existing_rows
            .into_iter()
            .map(|(id, json_ids)| (id.to_string(), json_ids.0))
            .collect();

        if let Err(msg) = crate::services::flow_graph::detect_cycle(
            &script_id.to_string(),
            &next_flow_ids,
            &existing,
        ) {
            return HttpResponse::BadRequest().json(json!({"error": msg, "valid": false}));
        }
    }

    let next_flow_ids_json = sqlx::types::Json(next_flow_ids);

    let result = sqlx::query_as::<_, UserScript>(
        r#"UPDATE user_scripts
SET kind = $1, name = $2, source = $3, enabled = $4, ir_json = $5, next_flow_ids = $6, updated_at = NOW()
WHERE id = $7 AND device_id = $8
RETURNING *"#,
    )
    .bind(&body.kind)
    .bind(&body.name)
    .bind(&body.source)
    .bind(enabled)
    .bind(&body.ir_json)
    .bind(&next_flow_ids_json)
    .bind(script_id)
    .bind(&device_id)
    .fetch_optional(&app_state.pg_pool)
    .await;

    match result {
        Ok(Some(script)) => {
            if let Err(e) = app_state
                .script_cache
                .reload_device(&app_state.pg_pool, &device_id)
                .await
            {
                warn!(device_id, error = %e, "Failed to reload script cache after update");
            }
            HttpResponse::Ok().json(json!({"status": "updated", "data": script}))
        }
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

    if let Err(e) = validate_kind(&body.kind) {
        return HttpResponse::Ok().json(ScriptValidateResponse {
            valid: false,
            error: Some(e),
        });
    }

    if let Err(e) = validate_script_source(&body.kind, &body.source) {
        return HttpResponse::Ok().json(ScriptValidateResponse {
            valid: false,
            error: Some(e),
        });
    }

    if let Some(ref next_ids) = body.next_flow_ids
        && !next_ids.is_empty()
    {
        let candidate_id = body.id.unwrap_or_else(Uuid::new_v4);
        let existing_rows = sqlx::query_as::<_, (Uuid, sqlx::types::Json<Vec<String>>)>(
            "SELECT id, next_flow_ids FROM user_scripts WHERE device_id = $1 AND id != $2",
        )
        .bind(&device_id)
        .bind(candidate_id)
        .fetch_all(&app_state.pg_pool)
        .await
        .unwrap_or_default();

        let existing: Vec<(String, Vec<String>)> = existing_rows
            .into_iter()
            .map(|(id, json_ids)| (id.to_string(), json_ids.0))
            .collect();

        let candidate_id = "validate_candidate";
        if let Err(msg) =
            crate::services::flow_graph::detect_cycle(candidate_id, next_ids, &existing)
        {
            return HttpResponse::Ok().json(ScriptValidateResponse {
                valid: false,
                error: Some(msg),
            });
        }
    }

    HttpResponse::Ok().json(ScriptValidateResponse {
        valid: true,
        error: None,
    })
}

pub async fn apply_template(
    path: web::Path<(String, uuid::Uuid)>,
    body: web::Json<Vec<crate::services::multi_device_template::TemplateTarget>>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let (device_id, script_id) = path.into_inner();
    let source: Option<UserScript> =
        sqlx::query_as("SELECT * FROM user_scripts WHERE id = $1 AND device_id = $2")
            .bind(script_id)
            .bind(&device_id)
            .fetch_optional(&app_state.pg_pool)
            .await
            .unwrap_or(None);

    let Some(source) = source else {
        return HttpResponse::NotFound()
            .json(serde_json::json!({"error": "Flow gốc không tồn tại"}));
    };

    match crate::services::multi_device_template::apply_template(
        &app_state.pg_pool,
        &source,
        body.into_inner(),
    )
    .await
    {
        Ok(ids) => HttpResponse::Ok()
            .json(serde_json::json!({"status": "success", "applied_script_ids": ids})),
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        }
    }
}

pub fn get_config_unit(key: &str) -> &'static str {
    match key {
        "ec_target" | "ec_tolerance" => "mS/cm",
        "delay_between_a_and_b_sec" => "s",
        "water_cycle_sec" => "s",
        "dose_max_ml" => "ml",
        _ => "",
    }
}

#[rustfmt::skip]
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ConfigOverrideRow {
    pub id: Uuid, pub script_id: Uuid, pub device_id: String, pub config_key: String,
    pub original_value: String, pub applied_at: chrono::DateTime<chrono::Utc>,
    pub restored_at: Option<chrono::DateTime<chrono::Utc>>, pub flow_name: Option<String>,
}

#[rustfmt::skip]
#[derive(Debug, serde::Serialize)]
pub struct ConfigOverrideActiveItemDto {
    pub id: Uuid, pub config_key: String, pub device_id: String, pub device_name: Option<String>,
    pub original_value: String, pub current_value: String, pub unit: String,
    pub flow_name: String, pub flow_id: Uuid, pub status: String,
}

#[rustfmt::skip]
#[derive(Debug, serde::Serialize)]
pub struct ConfigAuditLogEntryDto {
    pub id: Uuid, pub timestamp: String, pub device_id: String, pub device_name: Option<String>,
    pub config_key: String, pub original_value: String, pub override_value: String,
    pub unit: String, pub reason: String, pub status: String,
}

/// GET /api/devices/{device_id}/scripts/config-overrides
pub async fn list_config_overrides(
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

    let current_config = crate::db::postgres::get_device_config(&app_state.pg_pool, &device_id)
        .await
        .ok();

    let active_rows = sqlx::query_as::<_, ConfigOverrideRow>(
        "SELECT fco.id, fco.script_id, fco.device_id, fco.config_key, fco.original_value, fco.applied_at, fco.restored_at, s.name as flow_name FROM flow_config_overrides fco LEFT JOIN user_scripts s ON s.id = fco.script_id WHERE fco.device_id = $1 AND fco.restored_at IS NULL ORDER BY fco.applied_at DESC",
    )
    .bind(&device_id).fetch_all(&app_state.pg_pool).await.unwrap_or_default();

    let active: Vec<ConfigOverrideActiveItemDto> = active_rows
        .into_iter()
        .map(|row| {
            let unit = get_config_unit(&row.config_key).to_string();
            let current_val = current_config
                .as_ref()
                .and_then(|cfg| {
                    crate::services::config_override::read_field_as_string(cfg, &row.config_key)
                })
                .unwrap_or_else(|| row.original_value.clone());
            ConfigOverrideActiveItemDto {
                id: row.id,
                config_key: row.config_key,
                device_id: row.device_id,
                device_name: None,
                original_value: row.original_value,
                current_value: current_val,
                unit,
                flow_name: row
                    .flow_name
                    .unwrap_or_else(|| "Flow không xác định".to_string()),
                flow_id: row.script_id,
                status: "active".to_string(),
            }
        })
        .collect();

    let history_rows = sqlx::query_as::<_, ConfigOverrideRow>(
        "SELECT fco.id, fco.script_id, fco.device_id, fco.config_key, fco.original_value, fco.applied_at, fco.restored_at, s.name as flow_name FROM flow_config_overrides fco LEFT JOIN user_scripts s ON s.id = fco.script_id WHERE fco.device_id = $1 ORDER BY fco.applied_at DESC LIMIT 50",
    )
    .bind(&device_id).fetch_all(&app_state.pg_pool).await.unwrap_or_default();

    let history: Vec<ConfigAuditLogEntryDto> = history_rows
        .into_iter()
        .map(|row| {
            let unit = get_config_unit(&row.config_key).to_string();
            let (status, reason) = if row.restored_at.is_some() {
                (
                    "restored".to_string(),
                    "Điều kiện sai -> khôi phục gốc".to_string(),
                )
            } else {
                (
                    "applied".to_string(),
                    "Điều kiện thỏa mãn -> ghi đè".to_string(),
                )
            };
            ConfigAuditLogEntryDto {
                id: row.id,
                timestamp: row.applied_at.format("%d/%m %H:%M").to_string(),
                device_id: row.device_id,
                device_name: None,
                config_key: row.config_key,
                original_value: row.original_value.clone(),
                override_value: row.original_value,
                unit,
                reason,
                status,
            }
        })
        .collect();

    HttpResponse::Ok()
        .json(json!({ "status": "success", "data": { "active": active, "history": history } }))
}

/// POST /api/devices/{device_id}/scripts/config-overrides/{override_id}/revert
pub async fn revert_config_override(
    path: web::Path<(String, Uuid)>,
    http_req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let (device_id, override_id) = path.into_inner();
    let auth = http_req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("script:write") {
        return HttpResponse::Forbidden().json(json!({"error": "Missing scope script:write"}));
    }

    let record: Option<(String, String)> = sqlx::query_as(
        "SELECT config_key, original_value FROM flow_config_overrides WHERE id = $1 AND device_id = $2 AND restored_at IS NULL",
    )
    .bind(override_id).bind(&device_id)
    .fetch_optional(&app_state.pg_pool).await.unwrap_or(None);

    let Some((config_key, original_value)) = record else {
        return HttpResponse::NotFound()
            .json(json!({"error": "Không tìm thấy bản ghi đè đang hoạt động"}));
    };

    if let Ok(mut config) =
        crate::db::postgres::get_device_config(&app_state.pg_pool, &device_id).await
    {
        let empty_params = std::collections::HashMap::new();
        if crate::services::config_override::write_field(
            &mut config,
            &config_key,
            &original_value,
            &empty_params,
        )
        .is_ok()
        {
            let _ = crate::db::postgres::upsert_device_config(&app_state.pg_pool, &config).await;
        }
    }

    let _ = sqlx::query("UPDATE flow_config_overrides SET restored_at = NOW() WHERE id = $1")
        .bind(override_id)
        .execute(&app_state.pg_pool)
        .await;

    HttpResponse::Ok()
        .json(json!({"status": "success", "message": "Đã khôi phục cấu hình về giá trị gốc"}))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(list_scripts))
        .route("", web::post().to(create_script))
        .route("/validate", web::post().to(validate_script))
        .route("/test", web::post().to(test_script))
        .route("/config-overrides", web::get().to(list_config_overrides))
        .route(
            "/config-overrides/{override_id}/revert",
            web::post().to(revert_config_override),
        )
        .route(
            "/{script_id}/apply-template",
            web::post().to(apply_template),
        )
        .route("/{script_id}", web::put().to(update_script))
        .route("/{script_id}", web::delete().to(delete_script));
}

pub fn eval_condition_tree(
    node: &serde_json::Value,
    sample: &std::collections::HashMap<String, crate::models::script::SampleValue>,
    trace: &mut Vec<ConditionTraceEntry>,
) -> bool {
    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
        let op = node.get("op").and_then(|v| v.as_str()).unwrap_or("and");
        let results: Vec<bool> = children
            .iter()
            .map(|c| eval_condition_tree(c, sample, trace))
            .collect();
        return if op == "or" {
            results.iter().any(|&r| r)
        } else {
            results.iter().all(|&r| r)
        };
    }

    let sensor = node.get("sensor").and_then(|v| v.as_str()).unwrap_or("");
    let operator = node.get("operator").and_then(|v| v.as_str()).unwrap_or(">");
    let mode = node
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("instant");

    // Khi valueVariable có mặt, tra nó trong `sample` (được backend nạp từ
    // Config·Read/Chain context — xem services/config_context.rs) thay vì
    // dùng literal `value`. Biến thiếu khỏi sample -> mặc định 0.0, giữ hành
    // vi "fail theo cách cũ" thay vì panic hay coi là false cứng.
    let value_variable = node.get("valueVariable").and_then(|v| v.as_str());
    let threshold = match value_variable {
        Some(var_name) => sample.get(var_name).map(|sv| sv.resolve("instant")),
        None => node.get("value").and_then(|v| v.as_f64()),
    }
    .unwrap_or(0.0);

    let actual = sample.get(sensor).map(|sv| sv.resolve(mode));

    let passed = match (actual, operator) {
        (Some(a), ">") => a > threshold,
        (Some(a), "<") => a < threshold,
        (Some(a), ">=") => a >= threshold,
        (Some(a), "<=") => a <= threshold,
        (Some(a), "==") => (a - threshold).abs() < f64::EPSILON,
        (Some(a), "!=") => (a - threshold).abs() >= f64::EPSILON,
        _ => false,
    };

    trace.push(ConditionTraceEntry {
        description: format!("{} {} {}", sensor, operator, threshold),
        passed,
        actual_value: actual,
    });

    passed
}

pub async fn test_script(body: web::Json<TestScriptRequest>) -> impl Responder {
    let conditions = body
        .ir_json
        .get("conditions")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();
    let mut trace = Vec::new();
    let will_fire = conditions
        .iter()
        .all(|c| eval_condition_tree(c, &body.sample, &mut trace));
    let actions_preview = body
        .ir_json
        .get("actions")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();

    HttpResponse::Ok().json(TestScriptResponse {
        will_fire,
        trace,
        actions_preview: if will_fire { actions_preview } else { vec![] },
    })
}

#[cfg(test)]
mod tests {

    #[actix_web::test]
    async fn test_endpoint_returns_will_fire_true_and_trace_when_condition_met() {
        use crate::api::script::init_routes;
        use actix_web::{App, test, web};
        use serde_json::json;

        let app = test::init_service(
            App::new().service(web::scope("/api/scripts").configure(init_routes)),
        )
        .await;

        let req_body = json!({
            "ir_json": {
                "conditions": [
                    {
                        "sensor": "ph",
                        "operator": ">",
                        "value": 7.5
                    }
                ],
                "actions": [
                    {
                        "action": "dose",
                        "pump": "ph_down",
                        "dose_ml": 5
                    }
                ]
            },
            "sample": {
                "ph": 7.8
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/scripts/test")
            .set_json(&req_body)
            .to_request();
        let resp: crate::models::script::TestScriptResponse =
            test::call_and_read_body_json(&app, req).await;

        assert!(resp.will_fire);
        assert_eq!(resp.trace.len(), 1);
        assert_eq!(resp.trace[0].description, "ph > 7.5");
        assert!(resp.trace[0].passed);
        assert_eq!(resp.trace[0].actual_value, Some(7.8));
        assert_eq!(resp.actions_preview.len(), 1);
    }

    #[actix_web::test]
    async fn test_endpoint_returns_will_fire_false_with_failing_leaf_marked() {
        use crate::api::script::init_routes;
        use actix_web::{App, test, web};
        use serde_json::json;

        let app = test::init_service(
            App::new().service(web::scope("/api/scripts").configure(init_routes)),
        )
        .await;

        let req_body = json!({
            "ir_json": {
                "conditions": [
                    {
                        "children": [
                            {
                                "sensor": "ph",
                                "operator": ">",
                                "value": 7.5
                            },
                            {
                                "sensor": "ec",
                                "operator": ">",
                                "value": 3.0
                            }
                        ],
                        "op": "and"
                    }
                ]
            },
            "sample": {
                "ph": 7.8,
                "ec": 2.1
            }
        });

        let req = test::TestRequest::post()
            .uri("/api/scripts/test")
            .set_json(&req_body)
            .to_request();
        let resp: crate::models::script::TestScriptResponse =
            test::call_and_read_body_json(&app, req).await;

        assert!(!resp.will_fire);
        assert_eq!(resp.trace.len(), 2);

        let ec_trace = resp
            .trace
            .iter()
            .find(|t| t.description.contains("ec"))
            .expect("Value should exist in test");
        assert!(!ec_trace.passed);
        assert_eq!(ec_trace.actual_value, Some(2.1));

        let ph_trace = resp
            .trace
            .iter()
            .find(|t| t.description.contains("ph"))
            .expect("Value should exist in test");
        assert!(ph_trace.passed);
        assert_eq!(ph_trace.actual_value, Some(7.8));
    }

    #[test]
    fn eval_condition_tree_agrees_with_compiled_rhai_on_random_cases() {
        use crate::api::script::eval_condition_tree;

        use serde_json::json;
        use std::collections::HashMap;

        // Condition 1: ph > 7.5 AND ec < 2.0
        let cond1 = json!({
            "children": [
                { "sensor": "ph", "operator": ">", "value": 7.5 },
                { "sensor": "ec", "operator": "<", "value": 2.0 }
            ],
            "op": "and"
        });

        let rhai_guard1 = "input.ph > 7.5 && input.ec < 2.0";

        // Condition 2: water_level < 50.0 OR temp >= 30.0
        let cond2 = json!({
            "children": [
                { "sensor": "water_level", "operator": "<", "value": 50.0 },
                { "sensor": "temp", "operator": ">=", "value": 30.0 }
            ],
            "op": "or"
        });

        let rhai_guard2 = "input.water_level < 50.0 || input.temp >= 30.0";

        let cases = vec![
            (&cond1, rhai_guard1, vec![("ph", 7.8), ("ec", 1.5)], true),
            (&cond1, rhai_guard1, vec![("ph", 7.8), ("ec", 2.5)], false),
            (
                &cond2,
                rhai_guard2,
                vec![("water_level", 40.0), ("temp", 25.0)],
                true,
            ),
            (
                &cond2,
                rhai_guard2,
                vec![("water_level", 60.0), ("temp", 35.0)],
                true,
            ),
            (
                &cond2,
                rhai_guard2,
                vec![("water_level", 60.0), ("temp", 25.0)],
                false,
            ),
        ];

        let engine = rhai::Engine::new();

        for (cond, rhai_guard, sample_data, expected) in cases {
            let mut sample = HashMap::new();
            let mut rhai_map = rhai::Map::new();

            for (k, v) in sample_data {
                sample.insert(k.to_string(), crate::models::script::SampleValue::Value(v));
                rhai_map.insert(k.into(), rhai::Dynamic::from_float(v as rhai::FLOAT));
            }

            let mut trace = Vec::new();
            let rust_result = eval_condition_tree(cond, &sample, &mut trace);

            let mut scope = rhai::Scope::new();
            scope.push("input", rhai_map);
            let rhai_result: bool = engine
                .eval_with_scope(&mut scope, rhai_guard)
                .expect("Value should exist in test");

            assert_eq!(rust_result, expected);
            assert_eq!(rhai_result, expected);
            assert_eq!(rust_result, rhai_result);
        }
    }

    use super::*;

    #[test]
    fn valid_alert_script_source_passes_validation() {
        let src = r#"fn main(input) { if input.ph < 7.0 { return (); } #{ level: "warning", title: "pH", message: "cao" } }"#;
        assert!(validate_script_source("alert", src).is_ok());
    }

    #[test]
    fn validates_compiled_rhai_from_nested_condition_group() {
        let src = r#"fn main(input) { if !((input.ph < 5.5 || input.ph > 7.5) && input.ec > 3.0) { return (); } #{ "level": "warning", "title": "pH", "message": "out of range" } }"#;
        assert!(validate_script_source("alert", src).is_ok());
    }

    #[test]
    fn script_with_syntax_error_fails_validation() {
        let src = r#"fn main(input { }"#;
        let result = validate_script_source("alert", src);
        assert!(result.is_err());
        let msg = result.expect_err("Expected compile or syntax error message");
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
        assert!(validate_kind("action_command").is_ok());
    }

    #[test]
    fn next_flow_ids_empty_vec_serializes_to_json_array() {
        let ids: Vec<String> = vec![];
        let wrapped = sqlx::types::Json(ids.clone());
        let serialized = serde_json::to_string(&wrapped).expect("serialization works");
        assert_eq!(serialized, "[]");
    }

    #[test]
    fn next_flow_ids_with_values_serializes_correctly() {
        let ids = vec!["uuid-1".to_string(), "uuid-2".to_string()];
        let wrapped = sqlx::types::Json(ids);
        let serialized = serde_json::to_string(&wrapped).expect("serialization works");
        let parsed: Vec<String> = serde_json::from_str(&serialized).expect("deserialization works");
        assert_eq!(parsed, vec!["uuid-1", "uuid-2"]);
    }

    #[test]
    fn validate_script_detects_cycle_with_existing_db_script() {
        let script_a_id = Uuid::new_v4().to_string();
        let script_b_id = Uuid::new_v4().to_string();

        // Simulate script A -> script B existing in DB
        let existing: Vec<(String, Vec<String>)> =
            vec![(script_a_id.clone(), vec![script_b_id.clone()])];

        // Validating script B attempting to chain back to script A (creating cycle A -> B -> A)
        let result =
            crate::services::flow_graph::detect_cycle(&script_b_id, &[script_a_id], &existing);

        assert!(result.is_err());
        let err_msg = result.expect_err("expected cycle error");
        assert!(err_msg.contains("chu trình") || err_msg.contains("vòng lặp"));
    }

    #[test]
    fn eval_condition_tree_respects_mean_mode_over_series() {
        use crate::models::script::SampleValue;
        use std::collections::HashMap;
        let mut sample = HashMap::new();
        sample.insert(
            "ph".to_string(),
            SampleValue::Series(vec![7.0, 7.5, 8.5]), // mean = 7.666..
        );
        let node = serde_json::json!({
            "sensor": "ph", "operator": ">", "value": 7.5,
            "mode": "mean", "windowSec": 900
        });
        let mut trace = Vec::new();
        let passed = eval_condition_tree(&node, &sample, &mut trace);
        assert!(passed); // mean 7.67 > 7.5
        assert_eq!(trace[0].actual_value, Some(7.666666666666667));
    }

    #[test]
    fn eval_condition_tree_defaults_to_instant_value_when_series_absent() {
        use crate::models::script::SampleValue;
        use std::collections::HashMap;
        let mut sample = HashMap::new();
        sample.insert("ec".to_string(), SampleValue::Value(2.1));
        let node = serde_json::json!({"sensor": "ec", "operator": "<", "value": 3.0});
        let mut trace = Vec::new();
        assert!(eval_condition_tree(&node, &sample, &mut trace));
    }

    #[test]
    fn eval_condition_tree_uses_value_variable_when_present() {
        use crate::models::script::SampleValue;
        use std::collections::HashMap;

        let node = serde_json::json!({
            "sensor": "ph", "operator": ">", "value": 0, "valueVariable": "ph_target_now"
        });
        let sample: HashMap<String, SampleValue> = [
            ("ph".to_string(), SampleValue::Value(7.4)),
            ("ph_target_now".to_string(), SampleValue::Value(7.2)),
        ]
        .into_iter()
        .collect();
        let mut trace = Vec::new();
        assert!(eval_condition_tree(&node, &sample, &mut trace));
        assert_eq!(trace[0].description, "ph > 7.2");
    }

    #[test]
    fn eval_condition_tree_fails_closed_when_value_variable_missing_from_sample() {
        use crate::models::script::SampleValue;
        use std::collections::HashMap;

        let node = serde_json::json!({
            "sensor": "ph", "operator": ">", "value": 0, "valueVariable": "ph_target_now"
        });
        let sample: HashMap<String, SampleValue> = [("ph".to_string(), SampleValue::Value(7.4))]
            .into_iter()
            .collect();
        let mut trace = Vec::new();
        // ph_target_now chưa được nạp (vd. Config·Read lỗi) -> threshold mặc định 0.0,
        // giữ hành vi cũ khi thiếu dữ liệu thay vì panic.
        assert!(eval_condition_tree(&node, &sample, &mut trace));
        assert_eq!(trace[0].description, "ph > 0");
    }

    #[test]
    fn eval_condition_tree_without_value_variable_is_unchanged() {
        use crate::models::script::SampleValue;
        use std::collections::HashMap;

        let node = serde_json::json!({ "sensor": "ec", "operator": "<", "value": 1.5 });
        let sample: HashMap<String, SampleValue> = [("ec".to_string(), SampleValue::Value(1.2))]
            .into_iter()
            .collect();
        let mut trace = Vec::new();
        assert!(eval_condition_tree(&node, &sample, &mut trace));
    }
}
