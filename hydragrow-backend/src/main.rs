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
    env, fs,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{
    RwLock,
    broadcast::{self, Receiver},
};
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;

use crate::{
    models::{alert::AlertMessage, sensor::SensorData},
    mqtt::process_message,
    services::solana::SolanaTraceability,
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

    // Event bus — tất cả side-effect đi qua đây
    pub event_bus: broadcast::Sender<AppEvent>,

    // FCM alerts — chỉ critical/warning từ system_log handler
    pub alert_sender: broadcast::Sender<AlertMessage>,

    // Device state cache — in-memory, keyed by device_id
    pub device_states: Arc<RwLock<HashMap<String, String>>>,

    // Solana
    pub solana_traceability: SolanaTraceability,

    // FCM tokens
    pub fcm_tokens: Arc<Mutex<Vec<String>>>,

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

    let (loki_layer, loki_task) = if let Ok(loki_url) = Url::parse(&loki_url_str) {
        // Gắn nhãn tĩnh cho instance Backend
        let (layer, task) = tracing_loki::builder()
            .label("service", "hydragrow-backend")
            .unwrap()
            .extra_field("environment", environment)
            .unwrap()
            .build_url(loki_url)
            .expect("Lỗi cấu hình Loki Layer");
        (Some(layer), Some(task))
    } else {
        (None, None)
    };

    // Spawn background task để đẩy log HTTP batch về Loki (không block server)
    if let Some(task) = loki_task {
        tokio::spawn(task);
    }

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hydragrow_backend=debug,actix_web=info"));

    // Đăng ký toàn bộ layers vào registry
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(loki_layer)
        .init();

    info!("🚀 Khởi động hệ thống IoT Hydroponics Backend với Loki Collector...");
    let database_url = env::var("DATABASE_URL").expect("Thiếu biến DATABASE_URL");
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let influx_url = env::var("INFLUX_URL").expect("Thiếu biến INFLUX_URL");
    let influx_org = env::var("INFLUX_ORG").expect("Thiếu biến INFLUX_ORG");
    let influx_token = env::var("INFLUX_TOKEN").expect("Thiếu biến INFLUX_TOKEN");
    let influx_bucket = env::var("INFLUX_BUCKET").expect("Thiếu biến INFLUX_BUCKET");
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

    let (mqtt_client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    let wallet_data =
        fs::read_to_string("server_wallet.json").expect("Không tìm thấy server_wallet.json");
    let private_key: Vec<u8> = serde_json::from_str(&wallet_data).unwrap();
    let solana_service = SolanaTraceability::new("https://api.devnet.solana.com", &private_key);

    let (alert_sender, _) = broadcast::channel(100);
    let (event_bus, _) = broadcast::channel(256);
    let api_key = std::env::var("API_KEY").context("API_KEY must be set in .env")?;
    let device_states = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

    // Spawn retention task: xóa system_events cũ hơn 90 ngày, mỗi 24h
    crate::services::retention::spawn(pg_pool.clone());

    let mut fcm_rx = alert_sender.subscribe();
    let app_state = web::Data::new(AppState {
        pg_pool,
        influx_client,
        influx_bucket,
        mqtt_client: mqtt_client.clone(),
        alert_sender,
        api_key,
        device_states,
        solana_traceability: solana_service,
        fcm_tokens: Arc::new(Mutex::new(Vec::new())),
        event_bus: event_bus.clone(),
        ph_calibration_sessions: Arc::new(RwLock::new(HashMap::new())),
        ph_voltage_samples: Arc::new(RwLock::new(HashMap::new())),
        dosing_dynamic_states: Arc::new(RwLock::new(HashMap::new())),
        command_rate_limits: Arc::new(Mutex::new(HashMap::new())),
    });

    let fcm_tokens_clone = app_state.fcm_tokens.clone();
    tokio::spawn(async move {
        while let Ok(alert) = fcm_rx.recv().await {
            if alert.level != "critical" && alert.level != "warning" {
                continue;
            }
            let tokens = fcm_tokens_clone.lock().unwrap().clone();
            if !tokens.is_empty() {
                crate::services::fcm::send_push_notification(&alert.title, &alert.message, tokens)
                    .await;
            }
        }
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
        .expect("Lỗi sub");
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "status"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub");
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "sensor/status"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub");
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "fsm/state"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub");
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "fsm/events"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub");
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "system_log"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub");
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "calibration"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub");
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "controller/status"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub");
    // Trong hydragrow-backend/src/main.rs, thêm sau các subscribe hiện tại:
    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "fsm/transition"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub fsm/transition");

    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "dosing_cycle"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub dosing_cycle");

    mqtt_client
        .subscribe(
            &format!("{}/+/{}", AGITECH_PREFIX, "water_cycle"),
            QoS::AtLeastOnce,
        )
        .await
        .expect("Lỗi sub water_cycle");

    let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let server_port: u16 = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()?;

    info!(
        "🚀 API Server đang khởi chạy tại http://{}:{}",
        server_host, server_port
    );

    // main.rs

    HttpServer::new(move || {
        let auth_middleware = api::middleware::auth::ApiKeyAuth::new();
        let rate_limit_middleware = api::middleware::rate_limit::RateLimiter::new(60, 60);

        let cors = actix_cors::Cors::default()
            .allow_any_origin()
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
                    .service(
                        web::scope("/devices/{device_id}")
                            .configure(api::control::init_routes)
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
