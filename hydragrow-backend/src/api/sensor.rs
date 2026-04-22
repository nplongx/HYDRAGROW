use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;
use serde_json::json;
use tracing::{error, instrument};

use crate::AppState;
use crate::db::influx::get_latest_sensor_data;
use crate::models::sensor::SensorDataRow;

// 🟢 BƯỚC 1: Bổ sung `start` và `end` vào cấu trúc nhận Query Params
#[derive(Deserialize, Debug)]
pub struct HistoryQuery {
    pub range: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
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
        Ok(data) => HttpResponse::Ok().json(json!({ "status": "success", "data": data })),
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

#[instrument(skip(app_state))]
pub async fn get_history(
    path: web::Path<String>,
    query: web::Query<HistoryQuery>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();

    // 🟢 BƯỚC 2: Xử lý logic thời gian linh hoạt (Tuyệt đối vs Tương đối)
    let range_clause = if let (Some(start), Some(end)) = (&query.start, &query.end) {
        // Nếu Frontend gửi lên chuỗi ISO (VD: 2023-10-05T14:48:00.000Z)
        format!("start: time(v: \"{}\"), stop: time(v: \"{}\")", start, end)
    } else if let Some(start) = &query.start {
        format!("start: time(v: \"{}\")", start)
    } else {
        // Fallback về cách cũ nếu không có start/end
        let range_val = query.range.as_deref().unwrap_or("24h");
        format!("start: -{}", range_val)
    };

    // 🟢 BƯỚC 3: Cập nhật Flux Query
    // 1. Dùng range_clause động
    // 2. Thêm thực sự lệnh `pivot` (để gộp các trường ec, ph, temp vào cùng 1 dòng time)
    // 3. Tăng limit lên 1000 để chứa đủ dữ liệu nhiều ngày
    // 4. sort desc: false (từ cũ đến mới) để biểu đồ Recharts vẽ đúng chiều
    let flux_query = format!(
        r#"
        from(bucket: "{}")
        |> range({}) 
        |> filter(fn: (r) => r["_measurement"] == "sensor_data")
        |> filter(fn: (r) => r.device_id == "{}")
        |> sort(columns: ["_time"], desc: false)
        |> limit(n: 1000)
        "#,
        app_state.influx_bucket, range_clause, device_id
    );

    tracing::info!("Câu lệnh Flux Query:\n{}", flux_query);
    let query_obj = influxdb2::models::Query::new(flux_query.clone());

    match app_state
        .influx_client
        .query::<SensorDataRow>(Some(query_obj))
        .await
    {
        Ok(tables) => {
            tracing::info!("Query thành công! Trả về {} bản ghi.", tables.len());
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

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/sensors/latest", web::get().to(get_latest))
        .route("/sensors/history", web::get().to(get_history));
}

