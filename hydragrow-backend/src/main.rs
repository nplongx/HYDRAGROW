use actix_web::{App, HttpServer, web};
use anyhow::Context;
use dotenvy::dotenv;
use hydragrow_shared::{events::AppEvent, topics::AGITECH_PREFIX};
use influxdb2::Client as InfluxClient;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use std::{
    collections::{HashMap, VecDeque},
    env,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{
    RwLock,
    broadcast::{self},
};
use tracing::{error, info};
use tracing_subscriber::filter::filter_fn;

use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

use crate::{
    models::alert::AlertMessage, mqtt::process_message, services::solana::SolanaTraceability,
};

pub mod api;
pub mod db;
pub mod metrics;
pub mod models;
pub mod mqtt;
pub mod services;

#[derive(Debug, Clone)]
pub struct PhVoltageSample {
    pub voltage_mv: f64,
    pub observed_at: chrono::DateTime<chrono::Utc>,
    pub received_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhCalibrationMode {
    TwoPoint,
    ThreePoint,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhCapturedPoint {
    pub point: i32,
    pub voltage_mv: f64,
    pub sample_count: usize,
    pub captured_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct PhCalibrationSession {
    pub mode: PhCalibrationMode,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub captured_points: HashMap<i32, PhCapturedPoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DosingLearningSample {
    pub before_ec: Option<f32>,
    pub after_ec: Option<f32>,
    pub stabilized_ec: Option<f32>,
    pub before_ph: Option<f32>,
    pub after_ph: Option<f32>,
    pub stabilized_ph: Option<f32>,
    pub stabilized_window_sec: Option<u32>,
    pub reported_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct CommandRateEntry {
    pub count: u32,
    pub window_start: Instant,
}

pub struct DosingDynamicState {
    pub base_ec_gain_per_ml: f32,
    pub dynamic_ec_gain_per_ml: f32,
    pub confidence: f32,
    pub sample_count: u32,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub samples: VecDeque<DosingLearningSample>,
}

pub struct AppState {
    // Persistence
    pub pg_pool: sqlx::PgPool,
    pub influx_client: InfluxClient,
    pub influx_bucket: String,

    // Messaging
    pub mqtt_client: AsyncClient,

    // Auth
    pub api_key: String,
    pub firebase_auth: std::sync::Arc<crate::services::firebase_auth::FirebaseAuthVerifier>,

    // Event bus — tất cả side-effect đi qua đây
    pub event_bus: broadcast::Sender<AppEvent>,

    // FCM alerts — chỉ critical/warning từ system_log handler
    pub alert_sender: broadcast::Sender<AlertMessage>,

    // Device state cache — in-memory, keyed by device_id
    pub device_states: Arc<RwLock<HashMap<String, String>>>,

    /// Last firmware version reported by each controller health snapshot.
    pub device_firmware: Arc<RwLock<HashMap<String, String>>>,

    // Solana
    pub solana_traceability: SolanaTraceability,

    // FCM tokens — keyed by device_id, each device has its own token set
    pub fcm_tokens: Arc<Mutex<HashMap<String, Vec<String>>>>,

    // pH Calibration session state
    pub ph_calibration_sessions: Arc<RwLock<HashMap<String, PhCalibrationSession>>>,
    pub ph_voltage_samples: Arc<RwLock<HashMap<String, VecDeque<PhVoltageSample>>>>,

    // Dosing dynamic learning (in-memory)
    pub dosing_dynamic_states: Arc<RwLock<HashMap<String, DosingDynamicState>>>,

    // Manual command rate limits keyed by api_key + device_id + action.
    pub command_rate_limits: Arc<Mutex<HashMap<String, CommandRateEntry>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    crate::metrics::register_metrics();

    // =========================================================================
    // 1. KHỞI TẠO LOKI LOGGING PIPELINE
    // =========================================================================
    let loki_url_str = env::var("LOKI_URL").unwrap_or_else(|_| "http://localhost:3100".to_string());
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string());

    // Khởi tạo 2 bộ (Layer, Task) riêng biệt
    let (backend_loki, esp_loki) = if let Ok(loki_url) = Url::parse(&loki_url_str) {
        let b = tracing_loki::builder()
            .label("service", "hydragrow-backend")
            .unwrap() // startup: acceptable to panic — builder label creation
            .extra_field("environment", environment.clone())
            .unwrap() // startup: acceptable to panic — builder field configuration
            .build_url(loki_url.clone())
            .expect("Lỗi cấu hình Loki Layer cho Backend"); // startup: acceptable to panic — fail fast if Loki layer config is invalid

        let e = tracing_loki::builder()
            .label("service", "hydragrow-controller")
            .unwrap() // startup: acceptable to panic — builder label creation
            .extra_field("environment", environment)
            .unwrap() // startup: acceptable to panic — builder field configuration
            .build_url(loki_url)
            .expect("Lỗi cấu hình Loki Layer cho ESP32"); // startup: acceptable to panic — fail fast if Loki layer config is invalid

        (Some(b), Some(e))
    } else {
        (None, None)
    };

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hydragrow_backend=debug,actix_web=info"));

    // Phân loại luồng Log
    let is_esp_log = filter_fn(|meta| meta.target().contains("esp32_device"));
    let is_backend_log = filter_fn(|meta| !meta.target().contains("esp32_device"));

    // Gom Layer và Kích hoạt
    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    if let (Some((b_layer, b_task)), Some((e_layer, e_task))) = (backend_loki, esp_loki) {
        // Áp dụng filter cho từng ống dẫn
        registry
            .with(b_layer.with_filter(is_backend_log))
            .with(e_layer.with_filter(is_esp_log))
            .init();

        // [VÁ BUG LỖI E0277]: Chỉ spawn 1 lần duy nhất bằng cách move quyền sở hữu (ownership)
        tokio::spawn(b_task);
        tokio::spawn(e_task);
    } else {
        registry.init();
    }

    info!("🚀 Khởi động hệ thống IoT Hydroponics Backend với Loki Collector...");
    let database_url = env::var("DATABASE_URL").expect("Thiếu biến DATABASE_URL"); // startup: acceptable to panic — fail fast if DATABASE_URL is missing
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let influx_url = env::var("INFLUX_URL").expect("Thiếu biến INFLUX_URL"); // startup: acceptable to panic — fail fast if INFLUX_URL is missing
    let influx_org = env::var("INFLUX_ORG").expect("Thiếu biến INFLUX_ORG"); // startup: acceptable to panic — fail fast if INFLUX_ORG is missing
    let influx_token = env::var("INFLUX_TOKEN").expect("Thiếu biến INFLUX_TOKEN"); // startup: acceptable to panic — fail fast if INFLUX_TOKEN is missing
    let influx_bucket = env::var("INFLUX_BUCKET").expect("Thiếu biến INFLUX_BUCKET"); // startup: acceptable to panic — fail fast if INFLUX_BUCKET is missing
    let influx_client = InfluxClient::new(influx_url, influx_org, influx_token);
    info!("Đã khởi tạo client InfluxDB Cloud (v2 API)");

    let mqtt_host = env::var("MQTT_HOST").unwrap_or_else(|_| "localhost".to_string());
    let mqtt_port: u16 = env::var("MQTT_PORT")
        .unwrap_or_else(|_| "1883".to_string())
        .parse()?;
    let mqtt_client_id =
        env::var("MQTT_CLIENT_ID").unwrap_or_else(|_| "rust_backend_server".to_string());

    let mut mqttoptions = MqttOptions::new(mqtt_client_id, mqtt_host, mqtt_port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_clean_session(false);

    let mqtt_user = env::var("MQTT_USER").unwrap_or_default();
    let mqtt_pass = env::var("MQTT_PASSWORD").unwrap_or_default();
    if !mqtt_user.is_empty() && !mqtt_pass.is_empty() {
        mqttoptions.set_credentials(&mqtt_user, mqtt_pass);
        info!("Đã cấu hình xác thực MQTT với user: {}", mqtt_user);
    }

    let (mqtt_client, mut eventloop) = AsyncClient::new(mqttoptions, 50);

    let private_key = match env::var("SOLANA_PRIVATE_KEY") {
        Ok(encoded_key) => match bs58::decode(encoded_key).into_vec() {
            Ok(key) => Some(key),
            Err(error) => {
                error!(
                    ?error,
                    "SOLANA_PRIVATE_KEY is not valid base58; Solana traceability disabled"
                );
                None
            }
        },
        Err(_) => {
            error!("SOLANA_PRIVATE_KEY is not configured; Solana traceability disabled");
            None
        }
    };
    let solana_service =
        SolanaTraceability::new("https://api.devnet.solana.com", private_key.as_deref());

    let (alert_sender, _) = broadcast::channel(100);
    let (event_bus, _) = broadcast::channel(256);
    let api_key = std::env::var("API_KEY").context("API_KEY must be set in .env")?;
    let firebase_project_id =
        std::env::var("FIREBASE_PROJECT_ID").context("FIREBASE_PROJECT_ID must be set in .env")?;
    let firebase_auth = std::sync::Arc::new(
        crate::services::firebase_auth::FirebaseAuthVerifier::new(firebase_project_id),
    );
    let device_states = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let device_firmware = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

    // Spawn retention task: xóa system_events cũ hơn 90 ngày, mỗi 24h
    crate::services::retention::spawn(pg_pool.clone());

    let app_state = web::Data::new(AppState {
        pg_pool,
        influx_client,
        influx_bucket,
        mqtt_client: mqtt_client.clone(),
        alert_sender,
        api_key,
        firebase_auth,
        device_states,
        device_firmware,
        solana_traceability: solana_service,
        fcm_tokens: Arc::new(Mutex::new(HashMap::new())),
        event_bus: event_bus.clone(),
        ph_calibration_sessions: Arc::new(RwLock::new(HashMap::new())),
        ph_voltage_samples: Arc::new(RwLock::new(HashMap::new())),
        dosing_dynamic_states: Arc::new(RwLock::new(HashMap::new())),
        command_rate_limits: Arc::new(Mutex::new(HashMap::new())),
    });

    let app_state_for_mqtt = app_state.clone();
    tokio::spawn(async move {
        info!("Bắt đầu vòng lặp sự kiện MQTT dưới nền...");
        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                    process_message(publish, app_state_for_mqtt.clone()).await;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("Mất kết nối MQTT, thử lại sau 5 giây... Lỗi: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "sensors"),
            QoS::AtMostOnce,
        )
        .await
        .expect("Lỗi sub"); // startup: acceptable to panic — fail fast if initial MQTT subscription fails
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "status"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub"); // startup: acceptable to panic — fail fast if initial MQTT subscription fails
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "sensor/status"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub"); // startup: acceptable to panic — fail fast if initial MQTT subscription fails
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "fsm/state"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub"); // startup: acceptable to panic — fail fast if initial MQTT subscription fails
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "fsm/events"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub"); // startup: acceptable to panic — fail fast if initial MQTT subscription fails
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "system_log"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub"); // startup: acceptable to panic — fail fast if initial MQTT subscription fails
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "calibration"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub"); // startup: acceptable to panic — fail fast if initial MQTT subscription fails
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "controller/status"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub"); // startup: acceptable to panic — fail fast if initial MQTT subscription fails
    // Trong hydragrow-backend/src/main.rs, thêm sau các subscribe hiện tại:
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "fsm/transition"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub fsm/transition"); // startup: acceptable to panic — fail fast if initial MQTT subscription fails

    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "dosing_cycle"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub dosing_cycle"); // startup: acceptable to panic — fail fast if initial MQTT subscription fails

    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "water_cycle"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub water_cycle"); // startup: acceptable to panic — fail fast if initial MQTT subscription fails

    let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let server_port: u16 = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()?;

    info!(
        "🚀 API Server đang khởi chạy tại http://{}:{}",
        server_host, server_port
    );

    // main.rs

    let allowed_origins_env = env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "".to_string());
    let allowed_origins: Vec<String> = allowed_origins_env
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    HttpServer::new(move || {
        let auth_middleware = api::middleware::auth::ApiKeyAuth::new();
        let rate_limit_middleware = api::middleware::rate_limit::RateLimiter::new(60, 60);

        let allowed_origins = allowed_origins.clone();
        let cors = actix_cors::Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                if allowed_origins.is_empty() {
                    return false; // If no origins configured, deny all cross-origin requests
                }
                let origin_str = origin.as_bytes();
                allowed_origins
                    .iter()
                    .any(|allowed| allowed.as_bytes() == origin_str)
            })
            .allow_any_method()
            .allow_any_header();

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .route("/metrics", web::get().to(api::metrics::metrics_handler)) // 🟢 1. Chỉ bắt đúng 1 endpoint WebSocket, KHÔNG dùng web::scope chiếm toàn bộ đường dẫn
            .service(
                web::resource("/api/devices/{device_id}/ws")
                    .route(web::get().to(api::ws::ws_handler)),
            )
            // 🟢 2. Toàn bộ REST APIs sẽ đi vào đây bình thường
            .service(
                web::scope("/api")
                    .wrap(auth_middleware)
                    .wrap(rate_limit_middleware)
                    .configure(api::notification::init_routes)
                    .configure(api::solana::init_routes)
                    .configure(api::recipe::init_routes)
                    .configure(api::admin_users::init_routes)
                    .configure(api::device_pairing::init_routes)
                    .service(
                        web::scope("/devices/{device_id}")
                            .configure(api::control::init_routes)
                            .service(
                                web::scope("/admin").configure(api::config_backup::init_routes),
                            )
                            .configure(api::device_admin::init_routes)
                            .configure(api::sensor::init_routes)
                            .configure(api::config::init_routes)
                            .configure(api::calibration::init_routes)
                            .configure(api::crop_season::init_routes)
                            .configure(api::alert::init_routes),
                    ),
            )
    })
    .bind((server_host, server_port))?
    .run()
    .await?;

    Ok(())
}
