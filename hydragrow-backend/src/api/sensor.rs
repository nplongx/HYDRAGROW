use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use serde::Deserialize;
use serde_json::json;
use tracing::instrument;

use crate::AppState;
use crate::metrics::FLUX_QUERY_TOTAL;
use crate::models::sensor::SensorDataRow;

#[derive(Deserialize, Debug)]
pub struct HistoryQuery {
    pub range: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub resolution: Option<String>, // Ví dụ: "5m", "30m", "1h"
}

#[instrument(skip(app_state))]
pub async fn get_latest(
    path: web::Path<String>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let auth = req
        .extensions()
        .get::<crate::api::middleware::auth::AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("read:telemetry") {
        return HttpResponse::Forbidden()
            .json(json!({"error": "Missing required scope", "required_scope": "read:telemetry"}));
    }
    let states = app_state.device_states.read().await;
    let Some(cached_str) = states.get(&device_id) else {
        return HttpResponse::ServiceUnavailable().json(json!({
            "error": "Unavailable",
            "message": "Chưa có authoritative current state cho thiết bị"
        }));
    };
    let Ok(cached_json) = serde_json::from_str::<serde_json::Value>(cached_str) else {
        return HttpResponse::ServiceUnavailable().json(json!({
            "error": "Unavailable",
            "message": "Authoritative current state không hợp lệ"
        }));
    };
    let Some(telemetry) = cached_json.get("telemetry") else {
        return HttpResponse::ServiceUnavailable().json(json!({
            "error": "Unavailable",
            "message": "Chưa có authoritative telemetry hiện tại cho thiết bị"
        }));
    };

    let mut telemetry_value = telemetry.clone();
    if let Ok(mut snapshot) = serde_json::from_value::<
        hydragrow_shared::telemetry::AuthoritativeTelemetrySnapshot,
    >(telemetry.clone())
    {
        snapshot.refresh_operational_state(chrono::Utc::now());
        telemetry_value = serde_json::to_value(snapshot).unwrap_or(telemetry_value);
    }

    HttpResponse::Ok().json(json!({ "status": "success", "data": telemetry_value }))
}

fn build_range_clause(query: &HistoryQuery) -> Result<String, String> {
    if query.start.is_none() && query.end.is_some() {
        return Err("end requires start".to_string());
    }
    if let (Some(start), Some(end)) = (&query.start, &query.end) {
        crate::db::influx::validate_absolute_range(start, Some(end)).map_err(|e| e.to_string())?;
        Ok(format!(
            "start: time(v: \"{}\"), stop: time(v: \"{}\")",
            crate::db::influx::quote_flux_string(start),
            crate::db::influx::quote_flux_string(end)
        ))
    } else if let Some(start) = &query.start {
        crate::db::influx::validate_absolute_range(start, None).map_err(|e| e.to_string())?;
        Ok(format!(
            "start: time(v: \"{}\")",
            crate::db::influx::quote_flux_string(start)
        ))
    } else {
        let range_val = query.range.as_deref().unwrap_or("24h");
        crate::db::influx::validate_relative_range(range_val).map_err(|e| e.to_string())?;
        Ok(format!("start: -{}", range_val))
    }
}

#[instrument(skip(app_state))]
pub async fn get_history(
    path: web::Path<String>,
    req: HttpRequest,
    query: web::Query<HistoryQuery>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let auth = req
        .extensions()
        .get::<crate::api::middleware::auth::AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("read:telemetry") {
        return HttpResponse::Forbidden()
            .json(json!({"error": "Missing required scope", "required_scope": "read:telemetry"}));
    }

    if let Err(e) = crate::db::influx::validate_device_id(&device_id) {
        FLUX_QUERY_TOTAL
            .with_label_values(&["history", "rejected"])
            .inc();
        return HttpResponse::BadRequest()
            .json(json!({ "error": "Bad Request", "message": e.to_string() }));
    }
    let range_clause = match build_range_clause(&query) {
        Ok(clause) => clause,
        Err(message) => {
            FLUX_QUERY_TOTAL
                .with_label_values(&["history", "rejected"])
                .inc();
            return HttpResponse::BadRequest()
                .json(json!({ "error": "Bad Request", "message": message }));
        }
    };
    if let Some(resolution) = &query.resolution
        && let Err(e) = crate::db::influx::validate_resolution(resolution)
    {
        FLUX_QUERY_TOTAL
            .with_label_values(&["history", "rejected"])
            .inc();
        return HttpResponse::BadRequest()
            .json(json!({ "error": "Bad Request", "message": e.to_string() }));
    }

    // Lựa chọn chiến lược truy vấn
    let flux_query = if let Some(resolution) = &query.resolution {
        // Có resolution: aggregateWindow trên các cột số
        format!(
            r#"
            from(bucket: "{bucket}")
            |> range({range})
            |> filter(fn: (r) => r["_measurement"] == "sensor_data")
            |> filter(fn: (r) => r.device_id == "{device}")
            |> filter(fn: (r) => r._field == "ec" or r._field == "ph" or r._field == "temp" or r._field == "water_level")
            |> map(fn: (r) => ({{ r with _value: float(v: r._value) }}))
            |> aggregateWindow(every: {res}, fn: mean, createEmpty: false)
            |> sort(columns: ["_time"], desc: false)
            |> limit(n: 2000)
            "#,
            bucket = crate::db::influx::quote_flux_string(&app_state.influx_bucket),
            range = range_clause,
            device = crate::db::influx::quote_flux_string(&device_id),
            res = resolution
        )
    } else {
        // Không có resolution: lấy dữ liệu gốc, giới hạn 2000 điểm
        format!(
            r#"
            from(bucket: "{bucket}")
            |> range({range})
            |> filter(fn: (r) => r["_measurement"] == "sensor_data")
            |> filter(fn: (r) => r.device_id == "{device}")
            |> filter(fn: (r) => r._field == "ec" or r._field == "ph" or r._field == "temp" or r._field == "water_level")
            |> sort(columns: ["_time"], desc: false)
            |> limit(n: 2000)
            "#,
            bucket = crate::db::influx::quote_flux_string(&app_state.influx_bucket),
            range = range_clause,
            device = crate::db::influx::quote_flux_string(&device_id)
        )
    };

    tracing::info!("Câu lệnh Flux Query:\n{}", flux_query);
    let query_obj = influxdb2::models::Query::new(flux_query.clone());

    match app_state
        .influx_client
        .query::<SensorDataRow>(Some(query_obj))
        .await
    {
        Ok(tables) => {
            FLUX_QUERY_TOTAL
                .with_label_values(&["history", "success"])
                .inc();
            tracing::info!("Query thành công! Trả về {} bản ghi.", tables.len());
            // Nếu dùng sort desc, ta có thể đảo ngược lại để frontend nhận theo thứ tự thời gian tăng dần
            // Nhưng SensorDataRow có thể không có thứ tự, tạm thời trả về như cũ.
            HttpResponse::Ok().json(json!({ "status": "success", "data": tables }))
        }
        Err(e) => {
            FLUX_QUERY_TOTAL
                .with_label_values(&["history", "failure"])
                .inc();
            tracing::error!("Lỗi khi query từ InfluxDB Cloud cho {}: {:?}", device_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Database Error",
                "message": "Không thể truy xuất dữ liệu lịch sử"
            }))
        }
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct RangeStatsQuery {
    /// "ec" | "ph" | "temp" | "water_level"
    pub field: String,
    pub range: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
}

#[instrument(skip(app_state))]
pub async fn get_range_stats(
    path: web::Path<String>,
    req: HttpRequest,
    query: web::Query<RangeStatsQuery>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let auth = req
        .extensions()
        .get::<crate::api::middleware::auth::AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("read:telemetry") {
        return HttpResponse::Forbidden()
            .json(json!({"error": "Missing required scope", "required_scope": "read:telemetry"}));
    }
    if let Err(e) = crate::db::influx::validate_device_id(&device_id) {
        FLUX_QUERY_TOTAL
            .with_label_values(&["range_stats", "rejected"])
            .inc();
        return HttpResponse::BadRequest()
            .json(json!({ "error": "Bad Request", "message": e.to_string() }));
    }
    let allowed_fields = ["ec", "ph", "temp", "water_level"];
    if !allowed_fields.contains(&query.field.as_str()) {
        FLUX_QUERY_TOTAL
            .with_label_values(&["range_stats", "rejected"])
            .inc();
        return HttpResponse::BadRequest().json(json!({
            "error": "Bad Request",
            "message": format!("field phải là một trong {:?}", allowed_fields)
        }));
    }

    let history_query = HistoryQuery {
        range: query.range.clone(),
        start: query.start.clone(),
        end: query.end.clone(),
        resolution: None, // range-stats luôn dùng dữ liệu gốc, không windowing
    };
    let range_clause = match build_range_clause(&history_query) {
        Ok(clause) => clause,
        Err(message) => {
            FLUX_QUERY_TOTAL
                .with_label_values(&["range_stats", "rejected"])
                .inc();
            return HttpResponse::BadRequest()
                .json(json!({ "error": "Bad Request", "message": message }));
        }
    };

    let flux_query = format!(
        r#"
        from(bucket: "{bucket}")
        |> range({range})
        |> filter(fn: (r) => r["_measurement"] == "sensor_data")
        |> filter(fn: (r) => r.device_id == "{device}")
        |> filter(fn: (r) => r["_field"] == "{field}")
        |> sort(columns: ["_time"], desc: false)
        |> limit(n: 5000)
        "#,
        bucket = crate::db::influx::quote_flux_string(&app_state.influx_bucket),
        range = range_clause,
        device = crate::db::influx::quote_flux_string(&device_id),
        field = crate::db::influx::quote_flux_string(&query.field)
    );

    let query_obj = influxdb2::models::Query::new(flux_query);
    match app_state
        .influx_client
        .query::<SensorDataRow>(Some(query_obj))
        .await
    {
        Ok(rows) => {
            FLUX_QUERY_TOTAL
                .with_label_values(&["range_stats", "success"])
                .inc();
            let values: Vec<f64> = rows
                .iter()
                .map(|r| match query.field.as_str() {
                    "ec" => r.ec,
                    "ph" => r.ph,
                    "temp" => r.temp,
                    _ => r.water_level,
                })
                .collect();
            match crate::services::analytics::compute_range_stats(&values) {
                Some(stats) => {
                    HttpResponse::Ok().json(json!({ "status": "success", "data": stats }))
                }
                None => HttpResponse::Ok().json(json!({
                    "status": "success",
                    "data": serde_json::Value::Null,
                    "message": "Không có dữ liệu trong khoảng thời gian này"
                })),
            }
        }
        Err(e) => {
            FLUX_QUERY_TOTAL
                .with_label_values(&["range_stats", "failure"])
                .inc();
            tracing::error!("Lỗi range-stats query cho {}: {:?}", device_id, e);
            HttpResponse::InternalServerError().json(json!({
                "error": "Database Error",
                "message": "Không thể truy xuất dữ liệu range-stats"
            }))
        }
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/sensors/latest", web::get().to(get_latest))
        .route("/sensors/history", web::get().to(get_history))
        .route("/sensors/range-stats", web::get().to(get_range_stats));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_clause_uses_both_start_and_end_when_given() {
        let query = HistoryQuery {
            range: None,
            start: Some("2026-08-01T00:00:00Z".to_string()),
            end: Some("2026-08-02T00:00:00Z".to_string()),
            resolution: None,
        };
        let clause = build_range_clause(&query).expect("valid start/end range");
        assert!(clause.contains("start: time(v: \"2026-08-01T00:00:00Z\")"));
        assert!(clause.contains("stop: time(v: \"2026-08-02T00:00:00Z\")"));
    }

    #[test]
    fn range_clause_falls_back_to_relative_range_when_no_dates_given() {
        let query = HistoryQuery {
            range: Some("6h".to_string()),
            start: None,
            end: None,
            resolution: None,
        };
        let clause = build_range_clause(&query).expect("valid relative range");
        assert_eq!(clause, "start: -6h");
    }

    #[test]
    fn range_clause_defaults_to_24h_when_nothing_given() {
        let query = HistoryQuery {
            range: None,
            start: None,
            end: None,
            resolution: None,
        };
        let clause = build_range_clause(&query).expect("default range is valid");
        assert_eq!(clause, "start: -24h");
    }

    #[test]
    fn range_clause_rejects_injection_like_range_and_dates() {
        let query = HistoryQuery {
            range: Some("24h) |> drop()".to_string()),
            start: None,
            end: None,
            resolution: None,
        };
        assert!(build_range_clause(&query).is_err());

        let query = HistoryQuery {
            range: None,
            start: Some("2026-09-01T00:00:00Z\") |> drop()".to_string()),
            end: None,
            resolution: None,
        };
        assert!(build_range_clause(&query).is_err());
    }

    #[test]
    fn range_clause_rejects_excessive_ranges() {
        let query = HistoryQuery {
            range: Some("31d".to_string()),
            start: None,
            end: None,
            resolution: None,
        };
        assert!(build_range_clause(&query).is_err());
    }

    #[test]
    fn resolution_rejects_unbounded_flux_syntax() {
        assert!(crate::db::influx::validate_resolution("5m").is_ok());
        assert!(crate::db::influx::validate_resolution("5m) |> drop() //").is_err());
    }
}
