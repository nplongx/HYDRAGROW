use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;
use serde_json::json;
use tracing::{error, instrument};

use crate::AppState;
use crate::db::influx::get_latest_sensor_data;
use crate::models::sensor::SensorDataRow;

#[derive(Deserialize, Debug)]
pub struct HistoryQuery {
    pub range: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub resolution: Option<String>, // Ví dụ: "5m", "30m", "1h"
}

#[instrument(skip(app_state))]
pub async fn get_latest(path: web::Path<String>, app_state: web::Data<AppState>) -> impl Responder {
    let device_id = path.into_inner();

    match get_latest_sensor_data(
        &app_state.influx_client,
        &app_state.influx_bucket,
        &device_id,
    )
    .await
    {
        Ok(data) => {
            let mut json_data = json!(data);

            let states = app_state.device_states.read().await;
            if let Some(cached_str) = states.get(&device_id)
                && let Ok(cached_json) = serde_json::from_str::<serde_json::Value>(cached_str)
            {
                if let Some(ps) = cached_json.get("pump_status") {
                    json_data["pump_status"] = ps.clone();
                }
                if let Some(ph_voltage_mv) = cached_json.get("ph_voltage_mv") {
                    json_data["ph_voltage_mv"] = ph_voltage_mv.clone();
                }
                if let Some(fsm_state) = cached_json.get("fsm_state") {
                    json_data["fsm_state"] = fsm_state.clone();
                }
                if let Some(budgets) = cached_json.get("budgets") {
                    json_data["budgets"] = budgets.clone();
                }
            }

            HttpResponse::Ok().json(json!({ "status": "success", "data": json_data }))
        }
        Err(e) => {
            error!(
                "Lỗi khi lấy dữ liệu sensor mới nhất cho {}: {:?}",
                device_id, e
            );
            HttpResponse::NotFound().json(json!({
                "error": "Not Found",
                "message": "Không tìm thấy dữ liệu cho thiết bị này"
            }))
        }
    }
}

fn build_range_clause(query: &HistoryQuery) -> String {
    if let (Some(start), Some(end)) = (&query.start, &query.end) {
        format!("start: time(v: \"{}\"), stop: time(v: \"{}\")", start, end)
    } else if let Some(start) = &query.start {
        format!("start: time(v: \"{}\")", start)
    } else {
        let range_val = query.range.as_deref().unwrap_or("24h");
        format!("start: -{}", range_val)
    }
}

#[instrument(skip(app_state))]
pub async fn get_history(
    path: web::Path<String>,
    query: web::Query<HistoryQuery>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();

    // Xây dựng range clause
    let range_clause = build_range_clause(&query);

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
            bucket = app_state.influx_bucket,
            range = range_clause,
            device = device_id,
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
            bucket = app_state.influx_bucket,
            range = range_clause,
            device = device_id
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
            tracing::info!("Query thành công! Trả về {} bản ghi.", tables.len());
            // Nếu dùng sort desc, ta có thể đảo ngược lại để frontend nhận theo thứ tự thời gian tăng dần
            // Nhưng SensorDataRow có thể không có thứ tự, tạm thời trả về như cũ.
            HttpResponse::Ok().json(json!({ "status": "success", "data": tables }))
        }
        Err(e) => {
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
    query: web::Query<RangeStatsQuery>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let allowed_fields = ["ec", "ph", "temp", "water_level"];
    if !allowed_fields.contains(&query.field.as_str()) {
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
    let range_clause = build_range_clause(&history_query);

    let flux_query = format!(
        r#"
        from(bucket: "{bucket}")
        |> range({range})
        |> filter(fn: (r) => r["_measurement"] == "sensor_data")
        |> filter(fn: (r) => r.device_id == "{device}")
        |> sort(columns: ["_time"], desc: false)
        |> limit(n: 5000)
        "#,
        bucket = app_state.influx_bucket,
        range = range_clause,
        device = device_id
    );

    let query_obj = influxdb2::models::Query::new(flux_query);
    match app_state
        .influx_client
        .query::<SensorDataRow>(Some(query_obj))
        .await
    {
        Ok(rows) => {
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
        let clause = build_range_clause(&query);
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
        let clause = build_range_clause(&query);
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
        let clause = build_range_clause(&query);
        assert_eq!(clause, "start: -24h");
    }
}
