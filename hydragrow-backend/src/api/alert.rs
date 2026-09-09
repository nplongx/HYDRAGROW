// 1. 👇 SỬA IMPORT: Thêm get_events_by_cycle_id
use crate::{
    AppState,
    api::middleware::auth::AuthContext,
    db::postgres::{
        NewSystemEventRecord, get_events_by_cycle_id, get_system_events, insert_system_event,
    },
};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
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
    app_state: web::Data<AppState>,
) -> impl Responder {
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

pub async fn fetch_events(
    path: web::Path<String>,
    query: web::Query<EventsQuery>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let categories = normalize_categories(query.category.as_ref());

    match get_system_events(
        &app_state.pg_pool,
        &device_id,
        &categories,
        query.limit,
        query.before_timestamp,
        query.after_timestamp,
        query.level.clone(),
    )
    .await
    {
        Ok(events) => HttpResponse::Ok().json(json!({ "status": "success", "data": events })),
        Err(e) => {
            tracing::error!("Lỗi lấy system_events: {:?}", e);
            HttpResponse::InternalServerError().json(json!({ "error": "Database Error" }))
        }
    }
}

// 2. 👇 THÊM STRUCT & HÀM NÀY: Để xử lý endpoint /events/cycle/{cycle_id}
pub async fn get_cycle_timeline(
    path: web::Path<(String, String)>,
    app_state: web::Data<AppState>,
) -> impl Responder {
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

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    // Expose API cho Frontend
    cfg.route("/events", web::get().to(fetch_events));
    cfg.route("/health-summary", web::get().to(health_summary));

    // 3. 👇 ĐĂNG KÝ ROUTE MỚI
    cfg.route(
        "/events/cycle/{cycle_id}",
        web::get().to(get_cycle_timeline),
    );
    cfg.route("/events", web::post().to(create_event));
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
}
