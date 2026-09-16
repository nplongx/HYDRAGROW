// 1. 👇 SỬA IMPORT: Thêm get_events_by_cycle_id
use crate::{
    AppState,
    api::middleware::auth::AuthContext,
    db::postgres::{
        NewSystemEventRecord, SystemEventRecord, get_events_by_cycle_id, get_system_events,
        get_system_events_export, get_system_events_filtered, insert_system_event,
        resolve_system_event,
    },
};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use base64::Engine;
use serde_json::json;

fn default_reason_codes() -> Vec<String> {
    Vec::new()
}

#[derive(serde::Deserialize)]
pub struct CreateEventRequest {
    pub level: String,
    pub category: String,
    pub title: String,
    pub message: String,
    #[serde(default = "default_reason_codes")]
    pub reason_codes: Vec<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub observations: Option<serde_json::Value>,
}

fn validate_reason_codes(codes: &[String]) -> Result<(), String> {
    let valid = hydragrow_shared::supervisor::SupervisorReasonCode::all_as_str();
    for code in codes {
        if !valid.contains(&code.as_str()) {
            return Err(format!("unknown reason_code: {code}"));
        }
    }
    Ok(())
}

pub async fn create_event(
    path: web::Path<String>,
    req: HttpRequest,
    body: web::Json<CreateEventRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("events:write") {
        return HttpResponse::Forbidden().json(json!({
            "error": "Missing required scope",
            "required_scope": "events:write"
        }));
    }
    if let Err(e) = validate_reason_codes(&body.reason_codes) {
        return HttpResponse::BadRequest().json(json!({ "error": e }));
    }
    let source = req
        .headers()
        .get("X-Supervisor-Source")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if source != "watchdog" && source != "ai_supervisor" {
        return HttpResponse::BadRequest().json(json!({ "error": "Invalid X-Supervisor-Source" }));
    }
    let device_id = path.into_inner();
    let now = chrono::Utc::now().timestamp_millis();
    let record = NewSystemEventRecord {
        device_id,
        level: body.level.clone(),
        category: body.category.clone(),
        title: body.title.clone(),
        message: body.message.clone(),
        reason: None,
        metadata: Some(json!({
            "reason_codes": body.reason_codes,
            "confidence": body.confidence,
            "observations": body.observations,
        })),
        timestamp: now,
        source: source.to_string(),
        primary_reason_code: body.reason_codes.first().cloned(),
    };
    match insert_system_event(&app_state.pg_pool, &record).await {
        Ok(()) => HttpResponse::Created().json(json!({ "status": "created", "timestamp": now })),
        Err(e) => {
            tracing::error!("Lỗi insert system_event: {:?}", e);
            HttpResponse::InternalServerError().json(json!({ "error": "Database Error" }))
        }
    }
}

#[derive(serde::Deserialize)]
pub struct EventsQuery {
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub before_timestamp: Option<i64>,
    #[serde(default)]
    pub after_timestamp: Option<i64>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub unresolved: bool,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

fn default_limit() -> i64 {
    200
}

fn normalize_categories(raw_categories: Option<&String>) -> Vec<String> {
    let mut categories = Vec::new();

    if let Some(raw) = raw_categories {
        for category in raw.split(',') {
            let category = category.trim();
            if category.is_empty() || categories.iter().any(|c| c == category) {
                continue;
            }
            categories.push(category.to_string());
        }
    }

    categories
}

#[derive(serde::Serialize)]
struct HealthSummary {
    window_seconds: i64,
    ec_dosing_count: usize,
    ph_dosing_count: usize,
    water_operation_count: usize,
    warning_count: usize,
    critical_count: usize,
    latest_ph_dosing_at: Option<i64>,
}

pub async fn health_summary(
    path: web::Path<String>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("health:read") {
        return HttpResponse::Forbidden()
            .json(json!({"error":"Missing required scope: health:read"}));
    }
    let device_id = path.into_inner();
    let now = chrono::Utc::now().timestamp_millis();
    let window_ms = 3_600_000i64;

    match get_system_events(&app_state.pg_pool, &device_id, &[], 500, None, None, None).await {
        Ok(events) => {
            let recent: Vec<_> = events
                .into_iter()
                .filter(|e| now.saturating_sub(e.timestamp) <= window_ms)
                .collect();

            let ec_dosing_count = recent
                .iter()
                .filter(|e| e.category == "dosing" && e.title.to_lowercase().contains("ec"))
                .count();
            let ph_dosing_count = recent
                .iter()
                .filter(|e| e.category == "dosing" && e.title.to_lowercase().contains("ph"))
                .count();
            let water_operation_count = recent.iter().filter(|e| e.category == "water").count();
            let warning_count = recent.iter().filter(|e| e.level == "warning").count();
            let critical_count = recent.iter().filter(|e| e.level == "critical").count();
            let latest_ph_dosing_at = recent
                .iter()
                .filter(|e| e.category == "dosing" && e.title.to_lowercase().contains("ph"))
                .map(|e| e.timestamp)
                .max();

            HttpResponse::Ok().json(json!({
                "status": "success",
                "data": HealthSummary {
                    window_seconds: 3600,
                    ec_dosing_count,
                    ph_dosing_count,
                    water_operation_count,
                    warning_count,
                    critical_count,
                    latest_ph_dosing_at,
                }
            }))
        }
        Err(e) => {
            tracing::error!("Lỗi tổng hợp health-summary: {:?}", e);
            HttpResponse::InternalServerError().json(json!({ "error": "Database Error" }))
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct EventCursor {
    timestamp: i64,
    id: i32,
}

fn decode_cursor(raw: Option<&str>) -> Result<Option<EventCursor>, String> {
    let Some(raw) = raw.filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(raw)
        .map_err(|_| "invalid cursor".to_string())?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "invalid cursor".to_string())
}
fn encode_cursor(event: &SystemEventRecord) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&EventCursor {
            timestamp: event.timestamp,
            id: event.id,
        })
        .unwrap_or_default(),
    )
}
fn parse_bound(raw: Option<&String>) -> Result<Option<i64>, String> {
    raw.map(|s| {
        s.parse::<i64>()
            .map_err(|_| "invalid timestamp bound".to_string())
    })
    .transpose()
}

pub async fn fetch_events(
    path: web::Path<String>,
    req: HttpRequest,
    query: web::Query<EventsQuery>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("read:telemetry") {
        return HttpResponse::Forbidden()
            .json(json!({"error":"Missing required scope: read:telemetry"}));
    }
    let cursor = match decode_cursor(query.cursor.as_deref()) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error":e})),
    };
    let from = match parse_bound(query.from.as_ref()) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error":e})),
    };
    let to = match parse_bound(query.to.as_ref()) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error":e})),
    };
    let categories = normalize_categories(query.category.as_ref());
    let limit = query.limit.clamp(1, 500);
    match get_system_events_filtered(
        &app_state.pg_pool,
        &path,
        &categories,
        limit + 1,
        cursor.as_ref().map(|c| c.timestamp),
        cursor.as_ref().map(|c| c.id),
        None,
        query.level.clone(),
        query.event_type.clone(),
        query.unresolved,
        query.search.clone(),
        from,
        to,
    )
    .await
    {
        Ok(mut events) => {
            let has_more = events.len() > limit as usize;
            if has_more {
                events.truncate(limit as usize);
            }
            let next_cursor = if has_more {
                events.last().map(encode_cursor)
            } else {
                None
            };
            HttpResponse::Ok()
                .json(json!({"status":"success","data":events,"next_cursor":next_cursor}))
        }
        Err(e) => {
            tracing::error!("Lỗi lấy system_events: {:?}", e);
            HttpResponse::InternalServerError().json(json!({"error":"Database Error"}))
        }
    }
}

fn csv_field(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
fn event_csv(events: &[SystemEventRecord]) -> String {
    let mut out = String::from(
        "id,occurred_at,received_at,device_id,category,level,event_type,source,actor_kind,actor_id,title,message,reason_code,resolved_at,resolved_by\n",
    );
    for e in events {
        let fields = [
            e.id.to_string(),
            e.timestamp.to_string(),
            e.received_at.to_rfc3339(),
            e.device_id.clone(),
            e.category.clone(),
            e.level.clone(),
            e.event_type.clone(),
            e.source.clone(),
            e.actor_kind.clone(),
            e.actor_id.clone().unwrap_or_default(),
            e.title.clone(),
            e.message.clone(),
            e.primary_reason_code.clone().unwrap_or_default(),
            e.resolved_at.map(|v| v.to_rfc3339()).unwrap_or_default(),
            e.resolved_by.clone().unwrap_or_default(),
        ];
        out.push_str(
            &fields
                .iter()
                .map(|v| csv_field(v))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }
    out
}

fn export_json_event(e: &SystemEventRecord) -> serde_json::Value {
    json!({
        "id": e.id, "occurred_at": e.timestamp, "received_at": e.received_at,
        "device_id": e.device_id, "category": e.category, "level": e.level,
        "event_type": e.event_type, "source": e.source,
        "actor": { "kind": e.actor_kind, "id": e.actor_id },
        "title": e.title, "message": e.message, "reason_code": e.primary_reason_code,
        "resolution": { "resolved_at": e.resolved_at, "resolved_by": e.resolved_by }
    })
}

pub async fn export_events(
    path: web::Path<String>,
    req: HttpRequest,
    query: web::Query<EventsQuery>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("read:telemetry") {
        return HttpResponse::Forbidden()
            .json(json!({"error":"Missing required scope: read:telemetry"}));
    }
    let format = query
        .format
        .as_deref()
        .unwrap_or("json")
        .to_ascii_lowercase();
    if format != "json" && format != "csv" {
        return HttpResponse::BadRequest().json(json!({"error":"format must be json or csv"}));
    }
    let from = match parse_bound(query.from.as_ref()) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error":e})),
    };
    let to = match parse_bound(query.to.as_ref()) {
        Ok(v) => v,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error":e})),
    };
    let categories = normalize_categories(query.category.as_ref());
    match get_system_events_export(&app_state.pg_pool, &path, &categories, query.level.clone(), query.event_type.clone(), query.unresolved, query.search.clone(), from, to, 1000).await {
        Ok(events) if format == "csv" => HttpResponse::Ok().content_type("text/csv; charset=utf-8").insert_header(("Content-Disposition", "attachment; filename=journal-events.csv")).body(event_csv(&events)),
        Ok(events) => HttpResponse::Ok().content_type("application/json; charset=utf-8").json(json!({"status":"success","data":events.iter().map(export_json_event).collect::<Vec<_>>(),"count":events.len(),"truncated":events.len() == 1000})),
        Err(e) => { tracing::error!("Lỗi export system_events: {:?}", e); HttpResponse::InternalServerError().json(json!({"error":"Database Error"})) }
    }
}

// 2. 👇 THÊM STRUCT & HÀM NÀY: Để xử lý endpoint /events/cycle/{cycle_id}
pub async fn get_cycle_timeline(
    path: web::Path<(String, String)>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("read:telemetry") {
        return HttpResponse::Forbidden()
            .json(json!({"error":"Missing required scope: read:telemetry"}));
    }
    let (device_id, cycle_id) = path.into_inner();

    match get_events_by_cycle_id(&app_state.pg_pool, &device_id, &cycle_id).await {
        Ok(events) => HttpResponse::Ok().json(json!({
            "status": "success",
            "data": events
        })),
        Err(e) => {
            tracing::error!("Lỗi lấy timeline cho cycle_id {}: {:?}", cycle_id, e);
            HttpResponse::InternalServerError().json(json!({ "error": "Database Error" }))
        }
    }
}

#[derive(serde::Deserialize)]
pub struct ResolveEventRequest {
    #[serde(default)]
    pub resolved: bool,
}

/// Đánh dấu đã xử lý / mở lại một sự kiện hệ thống.
pub async fn resolve_event(
    path: web::Path<(String, i32)>,
    req: HttpRequest,
    body: web::Json<ResolveEventRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("events:write") {
        return HttpResponse::Forbidden().json(json!({
            "error": "Missing required scope",
            "required_scope": "events:write"
        }));
    }

    let (device_id, event_id) = path.into_inner();
    match resolve_system_event(
        &app_state.pg_pool,
        &device_id,
        event_id,
        body.resolved,
        auth.user_id.as_deref(),
    )
    .await
    {
        Ok(()) => HttpResponse::Ok().json(json!({
            "status": "success",
            "data": { "id": event_id, "resolved": body.resolved }
        })),
        Err(e) => {
            tracing::error!("Lỗi resolve system_event {}: {:?}", event_id, e);
            HttpResponse::InternalServerError().json(json!({ "error": "Database Error" }))
        }
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    // Expose API cho Frontend
    cfg.route("/events", web::get().to(fetch_events));
    cfg.route("/events/export", web::get().to(export_events));
    cfg.route("/health-summary", web::get().to(health_summary));

    // 3. 👇 ĐĂNG KÝ ROUTE MỚI
    cfg.route(
        "/events/cycle/{cycle_id}",
        web::get().to(get_cycle_timeline),
    );
    cfg.route("/events", web::post().to(create_event));
    // 4. Đánh dấu đã xử lý
    cfg.route(
        "/events/{event_id}/acknowledge",
        web::put().to(resolve_event),
    );
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use actix_web::{App, HttpResponse, Responder, test, web};

    async fn dummy_cycle(path: web::Path<(String, String)>) -> impl Responder {
        let (device_id, cycle_id) = path.into_inner();
        HttpResponse::Ok().body(format!("{device_id}:{cycle_id}"))
    }

    #[actix_web::test]
    async fn nested_scope_path_extracts_device_and_cycle_ids() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api/devices/{device_id}")
                    .route("/events/cycle/{cycle_id}", web::get().to(dummy_cycle)),
            ),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/devices/dev-01/events/cycle/cycle-99")
            .to_request();
        let resp = test::call_and_read_body(&app, req).await;

        assert_eq!(resp, web::Bytes::from_static(b"dev-01:cycle-99"));
    }

    #[test]
    async fn events_query_deserializes_after_timestamp() {
        // Kiểm tra struct deserialization từ query string
        let qs = "after_timestamp=1717000000000&limit=50&category=dosing";
        let query: EventsQuery = serde_urlencoded::from_str(qs).unwrap();
        assert_eq!(query.after_timestamp, Some(1717000000000_i64));
        assert_eq!(query.limit, 50);
        assert_eq!(query.category.as_deref(), Some("dosing"));
    }

    #[test]
    async fn compound_cursor_round_trips_timestamp_and_id() {
        let event = SystemEventRecord {
            id: 77,
            device_id: "dev".into(),
            level: "info".into(),
            category: "system".into(),
            title: "t".into(),
            message: "m".into(),
            reason: None,
            metadata: None,
            timestamp: 1717000000123,
            source: "system".into(),
            primary_reason_code: None,
            resolved_at: None,
            event_type: "system.test".into(),
            actor_kind: "system".into(),
            actor_id: None,
            received_at: chrono::Utc::now(),
            resolved_by: None,
        };
        let cursor = encode_cursor(&event);
        let decoded = decode_cursor(Some(&cursor)).unwrap().unwrap();
        assert_eq!(decoded.timestamp, 1717000000123);
        assert_eq!(decoded.id, 77);
    }

    #[test]
    async fn events_query_without_after_timestamp_defaults_to_none() {
        let qs = "limit=100";
        let query: EventsQuery = serde_urlencoded::from_str(qs).unwrap();
        assert_eq!(query.after_timestamp, None);
        assert_eq!(query.before_timestamp, None);
    }

    #[actix_web::test]
    async fn deserializes_with_optional_reason_code() {
        let v: CreateEventRequest = serde_json::from_value(json!({
            "level": "warning", "category": "alert",
            "title": "t", "message": "m",
            "reason_codes": ["leak_suspected"], "confidence": 0.9
        }))
        .unwrap();
        assert_eq!(v.reason_codes, vec!["leak_suspected".to_string()]);
        assert_eq!(v.confidence, Some(0.9));
        assert!(v.observations.is_none());
    }

    #[actix_web::test]
    async fn rejects_unknown_reason_code_via_validation() {
        let err = validate_reason_codes(&["bogus_code".to_string()]).unwrap_err();
        assert_eq!(err, "unknown reason_code: bogus_code");
    }

    #[actix_web::test]
    async fn accepts_known_reason_code() {
        assert!(validate_reason_codes(&["leak_suspected".to_string()]).is_ok());
        assert!(validate_reason_codes(&[]).is_ok());
    }

    /// SEC-RECIPE-ALERT-SCOPE-001 / AC-1: resolving an event is a write and
    /// must require `events:write` — a read-only caller gets 403, before any
    /// DB access happens.
    #[actix_web::test]
    async fn resolve_event_rejects_read_only_scope() {
        use actix_web::Responder;

        let state = web::Data::new(crate::api::test_support::test_app_state());
        let req = crate::api::test_support::authed_request(&["read:telemetry"], Some("1"));
        let resp = resolve_event(
            web::Path::from(("dev-01".to_string(), 7)),
            req.clone(),
            web::Json(ResolveEventRequest { resolved: true }),
            state,
        )
        .await
        .respond_to(&req)
        .map_into_boxed_body();

        assert_eq!(resp.status(), actix_web::http::StatusCode::FORBIDDEN);
        let bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"], "Missing required scope");
        assert_eq!(v["required_scope"], "events:write");
    }

    /// SEC-RECIPE-ALERT-SCOPE-001 / AC-1: a caller WITH `events:write` passes
    /// the scope gate into the DB layer (offline this surfaces as a database
    /// error, never as 403 — the assertion is gate-passage, which holds with
    /// or without a live database).
    #[actix_web::test]
    async fn resolve_event_with_events_write_reaches_db_layer() {
        use actix_web::Responder;

        let state = web::Data::new(crate::api::test_support::test_app_state());
        let req = crate::api::test_support::authed_request(&["events:write"], Some("1"));
        let resp = resolve_event(
            web::Path::from(("dev-01".to_string(), 7)),
            req.clone(),
            web::Json(ResolveEventRequest { resolved: true }),
            state,
        )
        .await
        .respond_to(&req)
        .map_into_boxed_body();

        assert_ne!(
            resp.status(),
            actix_web::http::StatusCode::FORBIDDEN,
            "events:write must pass the scope gate"
        );
        let bytes = actix_web::body::to_bytes(resp.into_body()).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(
            v.get("required_scope").is_none(),
            "must not be a scope rejection, got: {v}"
        );
    }
}
