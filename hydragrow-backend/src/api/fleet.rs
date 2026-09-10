//! GET /api/fleet/summary — trạng thái tổng hợp cho từng thiết bị của user.

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde::Serialize;
use serde_json::json;

use crate::AppState;
use crate::api::device_pairing::user_id_from;
use crate::db::device_ownership;
use crate::db::influx::get_latest_sensor_data;
use crate::db::postgres::get_system_events;

#[derive(Debug, Serialize)]
pub struct FleetSummaryEntry {
    pub device_id: String,
    pub label: Option<String>,
    pub is_online: bool,
    pub last_seen: Option<String>,
    /// Tên giai đoạn đang chạy của recipe active (nếu có).
    pub crop: Option<String>,
    pub ec_latest: Option<f32>,
    pub ph_latest: Option<f32>,
    pub warning_count: usize,
}

async fn active_stage_name(pool: &sqlx::PgPool, device_id: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT s.name
        FROM device_active_recipes dar
        JOIN crop_recipe_stages s
            ON s.id = dar.current_stage_id AND s.recipe_id = dar.recipe_id
        WHERE dar.device_id = $1
        "#,
    )
    .bind(device_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

async fn status_from_cache(app_state: &AppState, device_id: &str) -> (bool, Option<String>) {
    let raw = app_state.device_states.read().await.get(device_id).cloned();

    let Some(raw) = raw else {
        return (false, None);
    };

    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap_or_default();
    let ts = parsed
        .get("controller_status_ts")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let is_online = ts
        .as_ref()
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|dt| chrono::Utc::now().signed_duration_since(dt).num_seconds() < 30)
        .unwrap_or(false);

    (is_online, ts)
}

async fn warning_count_in_last_hour(pool: &sqlx::PgPool, device_id: &str) -> usize {
    let now = chrono::Utc::now().timestamp_millis();
    let window_ms = 3_600_000i64;

    match get_system_events(pool, device_id, &[], 500, None, None, None).await {
        Ok(events) => events
            .into_iter()
            .filter(|e| now.saturating_sub(e.timestamp) <= window_ms)
            .filter(|e| e.level == "warning")
            .count(),
        Err(_) => 0,
    }
}

async fn latest_ec_ph(app_state: &AppState, device_id: &str) -> (Option<f32>, Option<f32>) {
    match get_latest_sensor_data(
        &app_state.influx_client,
        &app_state.influx_bucket,
        device_id,
    )
    .await
    {
        Ok(data) => (Some(data.ec), Some(data.ph)),
        Err(_) => (None, None),
    }
}

pub async fn fleet_summary(req: HttpRequest, app_state: web::Data<AppState>) -> impl Responder {
    let Some(user_id) = user_id_from(&req) else {
        return HttpResponse::Unauthorized().json(json!({"error": "Chưa đăng nhập"}));
    };

    let devices = match device_ownership::list_devices_for_user(&app_state.pg_pool, user_id).await {
        Ok(devices) => devices,
        Err(e) => {
            tracing::error!(?e, "Lỗi lấy danh sách thiết bị cho fleet summary");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Không thể lấy danh sách thiết bị"}));
        }
    };

    let mut entries = Vec::with_capacity(devices.len());
    for rec in devices {
        let (is_online, last_seen) = status_from_cache(&app_state, &rec.device_id).await;
        let (ec_latest, ph_latest) = latest_ec_ph(&app_state, &rec.device_id).await;
        let warning_count = warning_count_in_last_hour(&app_state.pg_pool, &rec.device_id).await;
        let crop = active_stage_name(&app_state.pg_pool, &rec.device_id).await;

        entries.push(FleetSummaryEntry {
            device_id: rec.device_id,
            label: rec.label,
            is_online,
            last_seen,
            crop,
            ec_latest,
            ph_latest,
            warning_count,
        });
    }

    HttpResponse::Ok().json(json!({ "status": "success", "data": entries }))
}

pub fn init_fleet_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/fleet/summary", web::get().to(fleet_summary));
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn fleet_summary_entry_serializes_expected_shape() {
        let entry = FleetSummaryEntry {
            device_id: "dev-01".to_string(),
            label: Some("Trạm 1".to_string()),
            is_online: true,
            last_seen: Some("2026-09-10T00:00:00Z".to_string()),
            crop: Some("Sinh trưởng".to_string()),
            ec_latest: Some(1.4),
            ph_latest: Some(6.0),
            warning_count: 2,
        };
        let v = serde_json::to_value(&entry).unwrap();
        let obj = v.as_object().unwrap();
        assert_eq!(obj["device_id"], "dev-01");
        assert_eq!(obj["label"], "Trạm 1");
        assert_eq!(obj["is_online"], true);
        assert_eq!(obj["crop"], "Sinh trưởng");
        assert!((obj["ec_latest"].as_f64().unwrap() - 1.4_f64).abs() < 1e-6);
        assert!((obj["ph_latest"].as_f64().unwrap() - 6.0_f64).abs() < 1e-6);
        assert_eq!(obj["warning_count"], 2);
        assert!(obj.contains_key("last_seen"));
    }

    #[test]
    fn fleet_summary_entry_allows_missing_sensor_values() {
        let entry = FleetSummaryEntry {
            device_id: "dev-02".to_string(),
            label: None,
            is_online: false,
            last_seen: None,
            crop: None,
            ec_latest: None,
            ph_latest: None,
            warning_count: 0,
        };
        let v = serde_json::to_value(&entry).unwrap();
        let obj = v.as_object().unwrap();
        assert!(obj["crop"].is_null());
        assert!(obj["ec_latest"].is_null());
        assert!(obj["ph_latest"].is_null());
        assert_eq!(obj["warning_count"], 0);
    }
}
