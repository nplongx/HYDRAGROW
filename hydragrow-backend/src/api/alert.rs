use crate::{AppState, db::postgres::get_system_events};
use actix_web::{HttpResponse, Responder, web};
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct EventsQuery {
    pub category: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    200
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

    match get_system_events(&app_state.pg_pool, &device_id, None, 500).await {
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

    match get_system_events(
        &app_state.pg_pool,
        &device_id,
        query.category.as_deref(),
        query.limit,
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

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    // Expose API cho Frontend
    cfg.route("/events", web::get().to(fetch_events));
    cfg.route("/health-summary", web::get().to(health_summary));
}
