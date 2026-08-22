// 1. 👇 SỬA IMPORT: Thêm get_events_by_cycle_id
use crate::{
    AppState,
    db::postgres::{get_events_by_cycle_id, get_system_events},
};
use actix_web::{HttpResponse, Responder, web};
use serde_json::json;

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
            let critical_count = recent
                .iter()
                .filter(|e| e.level == "critical" || e.level == "error")
                .count();
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
}

#[cfg(test)]
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
}
