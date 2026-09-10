use actix_web::{HttpResponse, Responder, web};
use serde_json::json;

use crate::AppState;
use crate::services::analytics::extract_kalman_from_payload;

#[derive(serde::Deserialize)]
pub struct DosingHistoryRangeQuery {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize)]
pub struct DeviceHealthResponse {
    pub free_heap_bytes: Option<i64>,
    pub wifi_rssi_dbm: Option<i64>,
    pub uptime_seconds: Option<i64>,
    pub backend_process_cpu_percent: Option<f64>,
    pub last_updated_at: chrono::DateTime<chrono::Utc>,
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

async fn get_device_health(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();

    let mut heap: Option<i64> = {
        let val = crate::metrics::CONTROLLER_FREE_HEAP_BYTES
            .with_label_values(&[&device_id])
            .get();
        if val > 0 { Some(val) } else { None }
    };

    let mut rssi: Option<i64> = {
        let val = crate::metrics::CONTROLLER_WIFI_RSSI_DBM
            .with_label_values(&[&device_id])
            .get();
        if val != 0 { Some(val) } else { None }
    };

    let mut uptime: Option<i64> = {
        let val = crate::metrics::CONTROLLER_UPTIME_SECONDS
            .with_label_values(&[&device_id])
            .get();
        if val > 0 { Some(val) } else { None }
    };

    if heap.is_none() || rssi.is_none() || uptime.is_none() {
        let cached_json = app_state
            .device_states
            .read()
            .await
            .get(&device_id)
            .and_then(|r| serde_json::from_str::<serde_json::Value>(r).ok());

        if let Some(val) = cached_json {
            if heap.is_none() {
                heap = val.get("free_heap").and_then(|v| v.as_i64());
            }
            if rssi.is_none() {
                rssi = val.get("rssi").and_then(|v| v.as_i64());
            }
            if uptime.is_none() {
                uptime = val.get("uptime_sec").and_then(|v| v.as_i64());
            }
        }
    }

    let health = DeviceHealthResponse {
        free_heap_bytes: heap,
        wifi_rssi_dbm: rssi,
        uptime_seconds: uptime,
        backend_process_cpu_percent: None,
        last_updated_at: chrono::Utc::now(),
    };

    HttpResponse::Ok().json(json!({ "status": "success", "data": health }))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/analytics/dosing-history",
        web::get().to(get_dosing_history_range),
    );
    cfg.route("/analytics/health", web::get().to(get_device_health));
}
