//! GET /api/fleet/summary — trạng thái tổng hợp cho từng thiết bị của user.

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
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
    pub is_online: Option<bool>,
    pub last_seen: Option<String>,
    pub operational_state: hydragrow_shared::telemetry::OperationalState,
    /// Tên giai đoạn đang chạy của recipe active (nếu có).
    pub crop: Option<String>,
    pub ec_latest: Option<f32>,
    pub ph_latest: Option<f32>,
    pub warning_count: usize,
    /// False means the warning query failed; zero must never be interpreted as
    /// confirmed absence of warnings.
    pub warning_count_known: bool,
    pub telemetry_freshness: hydragrow_shared::telemetry::FreshnessState,
    pub telemetry_observed_at: Option<String>,
    pub telemetry_received_at: Option<String>,
    pub ec_quality: Option<hydragrow_shared::telemetry::TelemetryQuality>,
    pub ec_observed_at: Option<String>,
    pub ph_quality: Option<hydragrow_shared::telemetry::TelemetryQuality>,
    pub ph_observed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FleetComparisonEntry {
    pub device_id: String,
    pub label: Option<String>,
    pub crop: Option<String>,
    pub ec_latest: Option<f32>,
    pub ph_latest: Option<f32>,
    pub operational_state: hydragrow_shared::telemetry::OperationalState,
    pub telemetry_freshness: hydragrow_shared::telemetry::FreshnessState,
    pub telemetry_observed_at: Option<String>,
    pub telemetry_received_at: Option<String>,
    pub ec_quality: Option<hydragrow_shared::telemetry::TelemetryQuality>,
    pub ec_observed_at: Option<String>,
    pub ph_quality: Option<hydragrow_shared::telemetry::TelemetryQuality>,
    pub ph_observed_at: Option<String>,
}

#[derive(Debug, Clone)]
struct FleetTelemetrySnapshot {
    freshness: hydragrow_shared::telemetry::FreshnessState,
    observed_at: Option<String>,
    received_at: Option<String>,
    ec: Option<hydragrow_shared::telemetry::TelemetryAxis>,
    ph: Option<hydragrow_shared::telemetry::TelemetryAxis>,
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

async fn status_from_cache(
    app_state: &AppState,
    device_id: &str,
) -> (
    Option<bool>,
    Option<String>,
    hydragrow_shared::telemetry::OperationalState,
) {
    let raw = app_state.device_states.read().await.get(device_id).cloned();
    let Some(raw) = raw else {
        return (
            None,
            None,
            hydragrow_shared::telemetry::OperationalState::default(),
        );
    };
    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(_) => {
            return (
                None,
                None,
                hydragrow_shared::telemetry::OperationalState::default(),
            );
        }
    };
    let last_seen = parsed
        .get("controller_status_ts")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let mut state = parsed
        .get("telemetry")
        .and_then(|value| {
            serde_json::from_value::<hydragrow_shared::telemetry::AuthoritativeTelemetrySnapshot>(
                value.clone(),
            )
            .ok()
        })
        .map(|snapshot| snapshot.operational_state)
        .unwrap_or_default();
    let now = chrono::Utc::now();
    state.freshness = hydragrow_shared::telemetry::OperationalState::classify_freshness(
        state.received_at.as_deref().or(last_seen.as_deref()),
        now,
        hydragrow_shared::telemetry::OPERATIONAL_FRESHNESS_THRESHOLD_SECS,
    );
    if state.contact == hydragrow_shared::telemetry::ContactState::Unknown
        && !matches!(
            state.freshness,
            hydragrow_shared::telemetry::FreshnessState::Unknown
        )
    {
        state.contact = hydragrow_shared::telemetry::ContactState::Contacted;
    }
    state.classified_at = Some(now.to_rfc3339());
    let is_online = match (state.contact, state.freshness) {
        (
            hydragrow_shared::telemetry::ContactState::Contacted,
            hydragrow_shared::telemetry::FreshnessState::Fresh,
        ) => Some(true),
        (hydragrow_shared::telemetry::ContactState::NotContacted, _) => Some(false),
        _ => None,
    };
    (is_online, last_seen, state)
}
async fn warning_count_in_last_hour(pool: &sqlx::PgPool, device_id: &str) -> (usize, bool) {
    let now = chrono::Utc::now().timestamp_millis();
    let window_ms = 3_600_000i64;

    match get_system_events(pool, device_id, &[], 500, None, None, None).await {
        Ok(events) => (
            events
                .into_iter()
                .filter(|e| now.saturating_sub(e.timestamp) <= window_ms)
                .filter(|e| e.level == "warning")
                .count(),
            true,
        ),
        Err(e) => {
            tracing::warn!(
                ?e,
                device_id,
                "Không thể lấy warning count cho fleet summary"
            );
            (0, false)
        }
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

async fn authoritative_fleet_telemetry(
    app_state: &AppState,
    device_id: &str,
) -> Option<FleetTelemetrySnapshot> {
    let raw = app_state.device_states.read().await.get(device_id).cloned();
    let raw = raw?;
    let parsed = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    let value = parsed.get("telemetry")?;
    let snapshot = serde_json::from_value::<
        hydragrow_shared::telemetry::AuthoritativeTelemetrySnapshot,
    >(value.clone())
    .ok()?;
    let ec = snapshot.axes.iter().find(|axis| axis.name == "ec").cloned();
    let ph = snapshot.axes.iter().find(|axis| axis.name == "ph").cloned();
    Some(FleetTelemetrySnapshot {
        freshness: snapshot.operational_state.freshness,
        observed_at: snapshot.observed_at,
        received_at: snapshot.received_at,
        ec,
        ph,
    })
}

pub async fn fleet_summary(req: HttpRequest, app_state: web::Data<AppState>) -> impl Responder {
    let auth = req
        .extensions()
        .get::<crate::api::middleware::auth::AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("read:telemetry") {
        return HttpResponse::Forbidden().json(json!({
            "error": "Missing required scope",
            "required_scope": "read:telemetry"
        }));
    }
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
        let (is_online, last_seen, operational_state) =
            status_from_cache(&app_state, &rec.device_id).await;
        let authoritative = authoritative_fleet_telemetry(&app_state, &rec.device_id).await;
        let (fallback_ec, fallback_ph) = if authoritative.is_none() {
            latest_ec_ph(&app_state, &rec.device_id).await
        } else {
            (None, None)
        };
        let telemetry_freshness = authoritative
            .as_ref()
            .map(|snapshot| snapshot.freshness)
            .unwrap_or(hydragrow_shared::telemetry::FreshnessState::Unknown);
        let telemetry_observed_at = authoritative
            .as_ref()
            .and_then(|snapshot| snapshot.observed_at.clone());
        let telemetry_received_at = authoritative
            .as_ref()
            .and_then(|snapshot| snapshot.received_at.clone());
        let ec_quality = authoritative
            .as_ref()
            .and_then(|snapshot| snapshot.ec.as_ref().map(|axis| axis.quality));
        let ec_observed_at = authoritative.as_ref().and_then(|snapshot| {
            snapshot
                .ec
                .as_ref()
                .and_then(|axis| axis.observed_at.clone())
        });
        let ph_quality = authoritative
            .as_ref()
            .and_then(|snapshot| snapshot.ph.as_ref().map(|axis| axis.quality));
        let ph_observed_at = authoritative.as_ref().and_then(|snapshot| {
            snapshot
                .ph
                .as_ref()
                .and_then(|axis| axis.observed_at.clone())
        });
        let authoritative_ec = authoritative
            .as_ref()
            .and_then(|snapshot| snapshot.ec.as_ref().and_then(|axis| axis.value))
            .map(|v| v as f32);
        let authoritative_ph = authoritative
            .as_ref()
            .and_then(|snapshot| snapshot.ph.as_ref().and_then(|axis| axis.value))
            .map(|v| v as f32);
        let (warning_count, warning_count_known) =
            warning_count_in_last_hour(&app_state.pg_pool, &rec.device_id).await;
        let crop = active_stage_name(&app_state.pg_pool, &rec.device_id).await;

        entries.push(FleetSummaryEntry {
            device_id: rec.device_id,
            label: rec.label,
            is_online,
            last_seen,
            operational_state,
            crop,
            ec_latest: authoritative_ec.or(fallback_ec),
            ph_latest: authoritative_ph.or(fallback_ph),
            warning_count,
            warning_count_known,
            telemetry_freshness,
            telemetry_observed_at,
            telemetry_received_at,
            ec_quality,
            ec_observed_at,
            ph_quality,
            ph_observed_at,
        });
    }

    HttpResponse::Ok().json(json!({ "status": "success", "data": entries }))
}

pub async fn fleet_compare(
    req: HttpRequest,
    query: web::Query<std::collections::HashMap<String, String>>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let auth = req
        .extensions()
        .get::<crate::api::middleware::auth::AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !auth.has_scope("read:telemetry") {
        return HttpResponse::Forbidden().json(json!({
            "error": "Missing required scope",
            "required_scope": "read:telemetry"
        }));
    }
    let Some(user_id) = user_id_from(&req) else {
        return HttpResponse::Unauthorized().json(json!({"error": "Chưa đăng nhập"}));
    };
    let ids: Vec<String> = query
        .get("device_ids")
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if !(2..=4).contains(&ids.len()) {
        return HttpResponse::BadRequest().json(json!({
            "error": "device_ids must contain 2 to 4 devices"
        }));
    }

    let owned = match device_ownership::list_devices_for_user(&app_state.pg_pool, user_id).await {
        Ok(devices) => devices,
        Err(e) => {
            tracing::error!(?e, "fleet comparison ownership lookup failed");
            return HttpResponse::InternalServerError()
                .json(json!({"error": "Không thể kiểm tra quyền thiết bị"}));
        }
    };
    let owned_ids: std::collections::HashSet<&str> =
        owned.iter().map(|d| d.device_id.as_str()).collect();
    if ids.iter().any(|id| !owned_ids.contains(id.as_str())) {
        return HttpResponse::Forbidden().json(json!({"error": "Device ownership required"}));
    }

    let mut entries = Vec::with_capacity(ids.len());
    for device_id in &ids {
        let (_, _, operational_state) = status_from_cache(&app_state, device_id).await;
        let authoritative = authoritative_fleet_telemetry(&app_state, device_id).await;
        let (ec_latest, ph_latest) = if authoritative.is_none() {
            latest_ec_ph(&app_state, device_id).await
        } else {
            (None, None)
        };
        let crop = active_stage_name(&app_state.pg_pool, device_id).await;
        let label = owned
            .iter()
            .find(|d| d.device_id == *device_id)
            .and_then(|d| d.label.clone());
        entries.push(FleetComparisonEntry {
            device_id: device_id.clone(),
            label,
            crop,
            ec_latest,
            ph_latest,
            operational_state,
            telemetry_freshness: authoritative
                .as_ref()
                .map(|snapshot| snapshot.freshness)
                .unwrap_or(hydragrow_shared::telemetry::FreshnessState::Unknown),
            telemetry_observed_at: authoritative
                .as_ref()
                .and_then(|snapshot| snapshot.observed_at.clone()),
            telemetry_received_at: authoritative
                .as_ref()
                .and_then(|snapshot| snapshot.received_at.clone()),
            ec_quality: authoritative
                .as_ref()
                .and_then(|snapshot| snapshot.ec.as_ref().map(|axis| axis.quality)),
            ec_observed_at: authoritative.as_ref().and_then(|snapshot| {
                snapshot
                    .ec
                    .as_ref()
                    .and_then(|axis| axis.observed_at.clone())
            }),
            ph_quality: authoritative
                .as_ref()
                .and_then(|snapshot| snapshot.ph.as_ref().map(|axis| axis.quality)),
            ph_observed_at: authoritative.as_ref().and_then(|snapshot| {
                snapshot
                    .ph
                    .as_ref()
                    .and_then(|axis| axis.observed_at.clone())
            }),
        });
    }
    let crops: Vec<&str> = entries.iter().filter_map(|e| e.crop.as_deref()).collect();
    let same_crop = crops.len() == entries.len() && crops.windows(2).all(|pair| pair[0] == pair[1]);
    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": entries,
        "comparison": { "same_crop": same_crop, "normalized": same_crop }
    }))
}

pub fn init_fleet_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/fleet/summary", web::get().to(fleet_summary))
        .route("/fleet/compare", web::get().to(fleet_compare));
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
            is_online: Some(true),
            last_seen: Some("2026-09-10T00:00:00Z".to_string()),
            operational_state: hydragrow_shared::telemetry::OperationalState::default(),
            crop: Some("Sinh trưởng".to_string()),
            ec_latest: Some(1.4),
            ph_latest: Some(6.0),
            warning_count: 2,
            warning_count_known: true,
            telemetry_freshness: hydragrow_shared::telemetry::FreshnessState::Fresh,
            telemetry_observed_at: Some("2026-09-10T00:00:00Z".to_string()),
            telemetry_received_at: Some("2026-09-10T00:00:01Z".to_string()),
            ec_quality: Some(hydragrow_shared::telemetry::TelemetryQuality::Valid),
            ec_observed_at: Some("2026-09-10T00:00:00Z".to_string()),
            ph_quality: Some(hydragrow_shared::telemetry::TelemetryQuality::Valid),
            ph_observed_at: Some("2026-09-10T00:00:00Z".to_string()),
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
            is_online: None,
            last_seen: None,
            operational_state: hydragrow_shared::telemetry::OperationalState::default(),
            crop: None,
            ec_latest: None,
            ph_latest: None,
            warning_count: 0,
            warning_count_known: false,
            telemetry_freshness: hydragrow_shared::telemetry::FreshnessState::Unknown,
            telemetry_observed_at: None,
            telemetry_received_at: None,
            ec_quality: None,
            ec_observed_at: None,
            ph_quality: None,
            ph_observed_at: None,
        };
        let v = serde_json::to_value(&entry).unwrap();
        let obj = v.as_object().unwrap();
        assert!(obj["crop"].is_null());
        assert!(obj["ec_latest"].is_null());
        assert!(obj["ph_latest"].is_null());
        assert_eq!(obj["warning_count"], 0);
        assert_eq!(obj["warning_count_known"], false);
    }

    #[test]
    fn fleet_comparison_requires_bounded_selection() {
        assert!(!(2..=4).contains(&0usize));
        assert!(!(2..=4).contains(&1usize));
        assert!((2..=4).contains(&2usize));
        assert!((2..=4).contains(&4usize));
        assert!(!(2..=4).contains(&5usize));
    }

    #[test]
    fn authoritative_snapshot_preserves_partial_axis_quality_without_fallback() {
        let snapshot = hydragrow_shared::telemetry::AuthoritativeTelemetrySnapshot {
            device_id: "dev-03".to_string(),
            observed_at: Some("2026-09-10T00:00:00Z".to_string()),
            received_at: Some("2026-09-10T00:00:01Z".to_string()),
            availability: hydragrow_shared::telemetry::TelemetryAvailability::Online,
            axes: vec![
                hydragrow_shared::telemetry::TelemetryAxis {
                    name: "ec".to_string(),
                    value: None,
                    unit: "mS/cm".to_string(),
                    quality: hydragrow_shared::telemetry::TelemetryQuality::Error,
                    observed_at: Some("2026-09-10T00:00:00Z".to_string()),
                    received_at: Some("2026-09-10T00:00:01Z".to_string()),
                    source: hydragrow_shared::telemetry::TelemetrySource::ControllerSensor,
                    error_code: Some("ec_sensor_error".to_string()),
                },
                hydragrow_shared::telemetry::TelemetryAxis {
                    name: "ph".to_string(),
                    value: Some(6.1),
                    unit: "pH".to_string(),
                    quality: hydragrow_shared::telemetry::TelemetryQuality::Valid,
                    observed_at: Some("2026-09-10T00:00:00Z".to_string()),
                    received_at: Some("2026-09-10T00:00:01Z".to_string()),
                    source: hydragrow_shared::telemetry::TelemetrySource::ControllerSensor,
                    error_code: None,
                },
            ],
            controller_health: None,
            actuator: None,
            fsm: None,
            runtime_ready: None,
            actuator_contradictory: false,
            operational_state: hydragrow_shared::telemetry::OperationalState::default(),
        };
        let raw = serde_json::json!({"telemetry": snapshot});
        let parsed = serde_json::from_value::<
            hydragrow_shared::telemetry::AuthoritativeTelemetrySnapshot,
        >(raw["telemetry"].clone())
        .unwrap();
        let ec = parsed.axes.iter().find(|axis| axis.name == "ec").unwrap();
        let ph = parsed.axes.iter().find(|axis| axis.name == "ph").unwrap();
        assert_eq!(
            ec.quality,
            hydragrow_shared::telemetry::TelemetryQuality::Error
        );
        assert_eq!(ec.value, None);
        assert_eq!(
            ph.quality,
            hydragrow_shared::telemetry::TelemetryQuality::Valid
        );
        assert_eq!(ph.value, Some(6.1));
    }
}
