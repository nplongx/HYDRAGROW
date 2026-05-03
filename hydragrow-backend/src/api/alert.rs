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
}
