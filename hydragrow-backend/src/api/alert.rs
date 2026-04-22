use crate::{AppState, db::postgres::get_system_events};
use actix_web::{HttpResponse, Responder, web};
use serde_json::json;

pub async fn fetch_events(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();

    match get_system_events(&app_state.pg_pool, &device_id, 50).await {
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
