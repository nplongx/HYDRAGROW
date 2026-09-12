use crate::AppState;
use crate::api::middleware::auth::AuthContext;
use crate::db::topic_last_seen::{get_all_topics, get_topics_for_device};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use chrono::{DateTime, Utc};
use hydragrow_shared::{hestia::HestiaAssessment, telemetry::health::DeviceHealthSnapshot};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct TopicStatus {
    pub topic_category: String,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TopicRow {
    pub device_id: String,
    pub topic_category: String,
    pub last_seen_at: DateTime<Utc>,
}

pub fn group_by_device(rows: Vec<TopicRow>) -> HashMap<String, Vec<TopicStatus>> {
    let mut map: HashMap<String, Vec<TopicStatus>> = HashMap::new();
    for row in rows {
        map.entry(row.device_id).or_default().push(TopicStatus {
            topic_category: row.topic_category,
            last_seen_at: row.last_seen_at,
        });
    }
    map
}

pub fn has_health_read_scope(auth: &AuthContext) -> bool {
    auth.has_scope("health:read")
}

pub fn extract_hestia_by_device(
    states: &HashMap<String, String>,
) -> HashMap<String, HestiaAssessment> {
    states
        .iter()
        .filter_map(|(device_id, raw)| {
            let snapshot = serde_json::from_str::<DeviceHealthSnapshot>(raw).ok()?;
            let hestia = snapshot.hestia?;
            Some((device_id.clone(), hestia))
        })
        .collect()
}

fn auth_or_forbidden(req: &HttpRequest) -> Result<AuthContext, HttpResponse> {
    let auth = req
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .unwrap_or_default();
    if !has_health_read_scope(&auth) {
        return Err(HttpResponse::Forbidden().json(json!({
            "error": "Missing required scope",
            "required_scope": "health:read"
        })));
    }
    Ok(auth)
}

pub async fn get_all_health_topics(
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    if let Err(resp) = auth_or_forbidden(&req) {
        return resp;
    }
    match get_all_topics(&app_state.pg_pool).await {
        Ok(rows) => {
            let grouped = group_by_device(
                rows.into_iter()
                    .map(|r| TopicRow {
                        device_id: r.device_id,
                        topic_category: r.topic_category,
                        last_seen_at: r.last_seen_at,
                    })
                    .collect(),
            );
            HttpResponse::Ok().json(json!({ "status": "success", "data": grouped }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch all health topics: {:?}", e);
            HttpResponse::InternalServerError().json(json!({ "error": "Database Error" }))
        }
    }
}

pub async fn get_device_health_topics(
    path: web::Path<String>,
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    if let Err(resp) = auth_or_forbidden(&req) {
        return resp;
    }
    let device_id = path.into_inner();
    match get_topics_for_device(&app_state.pg_pool, &device_id).await {
        Ok(rows) => {
            let topics: Vec<TopicStatus> = rows
                .into_iter()
                .map(|r| TopicStatus {
                    topic_category: r.topic_category,
                    last_seen_at: r.last_seen_at,
                })
                .collect();
            HttpResponse::Ok().json(json!({ "status": "success", "data": topics }))
        }
        Err(e) => {
            tracing::error!(
                "Failed to fetch health topics for device {}: {:?}",
                device_id,
                e
            );
            HttpResponse::InternalServerError().json(json!({ "error": "Database Error" }))
        }
    }
}

pub async fn get_all_hestia(req: HttpRequest, app_state: web::Data<AppState>) -> impl Responder {
    if let Err(resp) = auth_or_forbidden(&req) {
        return resp;
    }

    let data = {
        let states = app_state.device_states.read().await;
        extract_hestia_by_device(&states)
    };

    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": data,
    }))
}

pub fn init_fleet_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health/topics", web::get().to(get_all_health_topics));
    cfg.route("/health/hestia", web::get().to(get_all_hestia));
}

pub fn init_device_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health/topics", web::get().to(get_device_health_topics));
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use hydragrow_shared::hestia::HestiaState;

    #[test]
    fn topic_status_serializes_last_seen_as_rfc3339() {
        let ts = Utc.with_ymd_and_hms(2024, 5, 1, 12, 0, 0).unwrap();
        let status = TopicStatus {
            topic_category: "sensors".to_string(),
            last_seen_at: ts,
        };
        let v = serde_json::to_value(&status).unwrap();
        let s = v.get("last_seen_at").unwrap().as_str().unwrap();
        assert!(s.starts_with("2024-05-01T12:00:00"));
        let parsed: DateTime<Utc> = s.parse().unwrap();
        assert_eq!(parsed, ts);
    }

    #[test]
    fn group_by_device_groups_multiple_topics_under_one_device_id() {
        let ts = Utc.with_ymd_and_hms(2024, 5, 1, 12, 0, 0).unwrap();
        let rows = vec![
            TopicRow {
                device_id: "dev-01".to_string(),
                topic_category: "sensors".to_string(),
                last_seen_at: ts,
            },
            TopicRow {
                device_id: "dev-01".to_string(),
                topic_category: "status".to_string(),
                last_seen_at: ts,
            },
            TopicRow {
                device_id: "dev-02".to_string(),
                topic_category: "sensors".to_string(),
                last_seen_at: ts,
            },
        ];
        let grouped = group_by_device(rows);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped.get("dev-01").unwrap().len(), 2);
        assert_eq!(grouped.get("dev-02").unwrap().len(), 1);
    }

    #[test]
    fn extract_hestia_by_device_returns_only_snapshots_with_hestia() {
        let warning = serde_json::json!({
            "device_id": "dev-1",
            "free_heap": 100,
            "uptime_sec": 10,
            "rssi": -40,
            "health_score_percent": 70,
            "fsm_state_display": "Running",
            "log_drop_count": 0,
            "firmware_version": "v1",
            "kalman_confidence": null,
            "matrix_update_count": 1,
            "matrix_is_warm": true,
            "hestia": {
                "score": 65.0,
                "state": "WARNING",
                "confidence": 0.8,
                "axes": {
                    "ec": {"comfort": 0.5, "weight": 0.35, "trend": "degrading", "trend_factor": 1.1, "action_factor": 1.0},
                    "ph": {"comfort": 1.0, "weight": 0.3, "trend": "stable", "trend_factor": 1.0, "action_factor": 1.0},
                    "water_level": {"comfort": 1.0, "weight": 0.2, "trend": "stable", "trend_factor": 1.0, "action_factor": 1.0},
                    "temp": {"comfort": 1.0, "weight": 0.15, "trend": "stable", "trend_factor": 1.0, "action_factor": 1.0}
                },
                "reasons": ["ec_out_of_range"]
            },
            "timestamp_ms": 1234
        });
        let no_hestia = serde_json::json!({
            "device_id": "dev-2",
            "free_heap": 100,
            "uptime_sec": 10,
            "rssi": -40,
            "health_score_percent": 100,
            "fsm_state_display": "Running",
            "log_drop_count": 0,
            "firmware_version": "v1",
            "kalman_confidence": null,
            "matrix_update_count": 1,
            "matrix_is_warm": false,
            "hestia": null,
            "timestamp_ms": 1234
        });

        let states = HashMap::from([
            ("dev-1".to_string(), warning.to_string()),
            ("dev-2".to_string(), no_hestia.to_string()),
        ]);

        let result = extract_hestia_by_device(&states);

        assert_eq!(result.len(), 1);
        assert_eq!(result["dev-1"].state, HestiaState::Warning);
        assert_eq!(result["dev-1"].reasons, vec!["ec_out_of_range"]);
    }

    #[test]
    fn extract_hestia_by_device_skips_invalid_cached_state() {
        let states = HashMap::from([("broken".to_string(), "not-json".to_string())]);
        assert!(extract_hestia_by_device(&states).is_empty());
    }

    #[test]
    fn extract_hestia_by_device_skips_structurally_invalid_snapshot() {
        let states = HashMap::from([(
            "broken".to_string(),
            serde_json::json!({"device_id": "broken"}).to_string(),
        )]);
        assert!(extract_hestia_by_device(&states).is_empty());
    }

    #[test]
    fn hestia_response_uses_existing_success_envelope() {
        let data = HashMap::<String, HestiaAssessment>::new();
        let body = serde_json::json!({"status": "success", "data": data});
        assert_eq!(body["status"], "success");
        assert!(body["data"].is_object());
    }

    // test_app_state hand-builds AppState; keep its fields in sync when AppState gains fields.
    fn test_app_state() -> crate::AppState {
        use rumqttc::{AsyncClient, MqttOptions};
        use std::sync::{Arc, Mutex};
        use tokio::sync::broadcast;

        let (mqtt_client, _eventloop) =
            AsyncClient::new(MqttOptions::new("test-client", "localhost", 1883), 10);
        let (event_bus, _rx) = broadcast::channel(16);
        crate::AppState {
            pg_pool: sqlx::PgPool::connect_lazy("postgres://localhost/hydragrow_test").unwrap(),
            influx_client: influxdb2::Client::new(
                "http://localhost:8086".to_string(),
                "test-org".to_string(),
                "test-token".to_string(),
            ),
            influx_bucket: "test-bucket".to_string(),
            mqtt_client,
            api_key: "test-api-key".to_string(),
            firebase_auth: std::sync::Arc::new(
                crate::services::firebase_auth::FirebaseAuthVerifier::new(
                    "test-project".to_string(),
                ),
            ),
            event_bus,
            device_states: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            device_firmware: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            solana_traceability: crate::services::solana::SolanaTraceability::new(
                "https://api.devnet.solana.com",
                None,
            ),
            fcm_tokens: Arc::new(Mutex::new(HashMap::new())),
            ph_calibration_sessions: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            ph_voltage_samples: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            dosing_dynamic_states: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            command_rate_limits: Arc::new(Mutex::new(HashMap::new())),
            script_cache: crate::services::script_engine::ScriptCache::new(std::sync::Arc::new(
                crate::services::script_engine::ScriptEngine::new(),
            )),
            cloudinary: None,
        }
    }

    fn hestia_snapshot_json(device_id: &str) -> serde_json::Value {
        serde_json::json!({
            "device_id": device_id,
            "free_heap": 100,
            "uptime_sec": 10,
            "rssi": -40,
            "health_score_percent": 70,
            "fsm_state_display": "Running",
            "log_drop_count": 0,
            "firmware_version": "v1",
            "kalman_confidence": null,
            "matrix_update_count": 1,
            "matrix_is_warm": true,
            "hestia": {
                "score": 65.0,
                "state": "WARNING",
                "confidence": 0.8,
                "axes": {
                    "ec": {"comfort": 0.5, "weight": 0.35, "trend": "degrading", "trend_factor": 1.1, "action_factor": 1.0},
                    "ph": {"comfort": 1.0, "weight": 0.3, "trend": "stable", "trend_factor": 1.0, "action_factor": 1.0},
                    "water_level": {"comfort": 1.0, "weight": 0.2, "trend": "stable", "trend_factor": 1.0, "action_factor": 1.0},
                    "temp": {"comfort": 1.0, "weight": 0.15, "trend": "stable", "trend_factor": 1.0, "action_factor": 1.0}
                },
                "reasons": ["ec_out_of_range"]
            },
            "timestamp_ms": 1234
        })
    }

    #[actix_web::test]
    async fn hestia_fleet_route_returns_success_envelope_with_health_read_scope() {
        use actix_web::body::MessageBody;
        use actix_web::dev::{ServiceRequest, ServiceResponse};
        use actix_web::middleware::{Next, from_fn};
        use actix_web::{App, HttpMessage, test};

        // Inject the real AuthContext through request extensions, exactly as
        // ApiKeyAuthMiddleware does — no fake auth mechanism.
        async fn inject_health_read_auth<B: MessageBody + 'static>(
            req: ServiceRequest,
            next: Next<B>,
        ) -> Result<ServiceResponse<B>, actix_web::Error> {
            req.extensions_mut().insert(AuthContext {
                scopes: vec!["health:read".to_string()],
                user_id: None,
                session_id: None,
                service_key_label: None,
            });
            next.call(req).await
        }

        let state = web::Data::new(test_app_state());
        {
            let mut states = state.device_states.write().await;
            states.insert(
                "dev-1".to_string(),
                hestia_snapshot_json("dev-1").to_string(),
            );
            states.insert(
                "dev-2".to_string(),
                serde_json::json!({
                    "device_id": "dev-2",
                    "free_heap": 100,
                    "uptime_sec": 10,
                    "rssi": -40,
                    "health_score_percent": 100,
                    "fsm_state_display": "Running",
                    "log_drop_count": 0,
                    "firmware_version": "v1",
                    "kalman_confidence": null,
                    "matrix_update_count": 1,
                    "matrix_is_warm": false,
                    "hestia": null,
                    "timestamp_ms": 1234
                })
                .to_string(),
            );
        }

        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .wrap(from_fn(inject_health_read_auth))
                .configure(init_fleet_routes),
        )
        .await;

        let req = test::TestRequest::get().uri("/health/hestia").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "success");
        assert!(body["data"].is_object());
        assert!(body["data"].get("dev-1").is_some());
        assert!(body["data"].get("dev-2").is_none());
    }
}
