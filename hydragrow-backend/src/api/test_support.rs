//! Shared builders for API handler tests (compiled under `cfg(test)` only).
//!
//! Handlers under test take a full [`crate::AppState`]; the pool below is
//! lazy (never connects unless a handler reaches the DB layer) and the MQTT
//! client is disconnected, so scope/ownership rejections can be asserted
//! without any live infrastructure.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Hand-build an `AppState` for handler tests.
///
/// Keep the field list in sync with [`crate::AppState`].
#[allow(clippy::unwrap_used)]
pub(crate) fn test_app_state() -> crate::AppState {
    use rumqttc::{AsyncClient, MqttOptions};
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
            crate::services::firebase_auth::FirebaseAuthVerifier::new("test-project".to_string()),
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

/// Build an `HttpRequest` carrying `scopes` (and optionally `user_id`) in its
/// extensions, exactly as `ApiKeyAuthMiddleware` does — no fake auth path.
pub(crate) fn authed_request(scopes: &[&str], user_id: Option<&str>) -> actix_web::HttpRequest {
    use actix_web::HttpMessage;

    let req = actix_web::test::TestRequest::put()
        .uri("/test")
        .to_http_request();
    req.extensions_mut()
        .insert(crate::api::middleware::auth::AuthContext {
            scopes: scopes.iter().map(|s| s.to_string()).collect(),
            user_id: user_id.map(|s| s.to_string()),
            session_id: None,
            service_key_label: None,
        });
    req
}
