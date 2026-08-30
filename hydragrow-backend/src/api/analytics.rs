use actix_web::{HttpResponse, Responder, web};
use serde_json::json;

use crate::AppState;
use crate::services::analytics::extract_kalman_from_payload;

#[derive(serde::Deserialize)]
pub struct DosingHistoryRangeQuery {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

/// Trả lịch sử dosing (kèm Kalman gain nếu chu kỳ đó có bật adaptive learning)
/// trong khoảng thời gian. Nguồn: `dosing_reports.payload` (Postgres) — KHÔNG phải
/// InfluxDB (xem phần Grounding của Phase 4).
async fn get_dosing_history_range(
    path: web::Path<String>,
    query: web::Query<DosingHistoryRangeQuery>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    match crate::db::postgres::get_device_dosing_reports_in_range(
        &app_state.pg_pool,
        &device_id,
        query.start,
        query.end,
    )
    .await
    {
        Ok(reports) => {
            let data: Vec<_> = reports
                .iter()
                .map(|r| {
                    json!({
                        "created_at": r.created_at,
                        "pump_a_ml": r.pump_a_ml,
                        "pump_b_ml": r.pump_b_ml,
                        "ph_up_ml": r.ph_up_ml,
                        "ph_down_ml": r.ph_down_ml,
                        "kalman": extract_kalman_from_payload(&r.payload),
                    })
                })
                .collect();
            HttpResponse::Ok().json(json!({ "status": "success", "data": data }))
        }
        Err(e) => {
            tracing::error!("Lỗi get_dosing_history_range cho {}: {:?}", device_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Database Error",
                "message": "Không thể truy xuất lịch sử dosing"
            }))
        }
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/analytics/dosing-history",
        web::get().to(get_dosing_history_range),
    );
}
