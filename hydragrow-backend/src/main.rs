use actix_web::{App, HttpServer, web};
use anyhow::Context;
use dotenvy::dotenv;
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

use crate::{
    models::{alert::AlertMessage, sensor::SensorData},
    services::solana::SolanaTraceability,
};

pub mod api;
pub mod db;
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
pub struct DosingDynamicState {
    pub base_ec_gain_per_ml: f32,
    pub dynamic_ec_gain_per_ml: f32,
    pub confidence: f32,
    pub sample_count: u32,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub samples: VecDeque<DosingLearningSample>,
}

pub struct AppState {
    pub pg_pool: sqlx::PgPool,
    pub influx_client: InfluxClient,
    pub influx_bucket: String,
    pub mqtt_client: AsyncClient,
    pub api_key: String,
    pub alert_sender: broadcast::Sender<AlertMessage>,
    pub device_states: std::sync::Arc<RwLock<HashMap<String, String>>>,
    pub solana_traceability: SolanaTraceability,
    pub sensor_sender: broadcast::Sender<SensorData>,
    pub health_sender: broadcast::Sender<serde_json::Value>,
    pub fcm_tokens: Mutex<Vec<String>>,
    pub ph_calibration_sessions: std::sync::Arc<RwLock<HashMap<String, PhCalibrationSession>>>,
    pub ph_voltage_samples: std::sync::Arc<RwLock<HashMap<String, VecDeque<PhVoltageSample>>>>,
    pub dosing_dynamic_states: std::sync::Arc<RwLock<HashMap<String, DosingDynamicState>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    env_logger::init();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Lỗi khởi tạo tracing");
    info!("Bắt đầu khởi động hệ thống IoT Hydroponics...");

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
    let mut alert_rx_for_db: Receiver<AlertMessage> = alert_sender.subscribe();

    let (health_tx, _) = broadcast::channel(100);

    let db_pool_clone = pg_pool.clone();

    tokio::spawn(async move {
        while let Ok(alert) = alert_rx_for_db.recv().await {
            // Bỏ qua FSM_UPDATE thuần (chỉ dùng để đồng bộ badge trên FE)
            if alert.level == "FSM_UPDATE" {
                continue;
            }

            // Phân loại category dựa theo nội dung alert
            let category = if alert.level == "critical" || alert.level == "warning" {
                "alert".to_string()
            } else if alert.title.contains("Châm Phân")
                || alert.title.contains("pH")
                || alert.title.contains("Blockchain")
            {
                "dosing".to_string()
            } else if alert.title.contains("Trạng thái") || alert.title.contains("Kết Nối") {
                "system".to_string()
            } else {
                "system".to_string()
            };

            let record = crate::db::postgres::NewSystemEventRecord {
                category,
                device_id: alert.device_id.clone(),
                level: alert.level.clone(),
                title: alert.title.clone(),
                message: alert.message.clone(),
                timestamp: alert.timestamp as i64,
                reason: alert.reason.clone(),
                metadata: alert.metadata.clone(),
            };

            if let Err(e) = crate::db::postgres::insert_system_event(&db_pool_clone, &record).await
            {
                tracing::error!("Lỗi ghi System Event vào DB: {:?}", e);
            }
        }
    });

    let (sensor_sender, _) = broadcast::channel(100);
    let api_key = std::env::var("API_KEY").context("API_KEY must be set in .env")?;
    let device_states = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

    let app_state = web::Data::new(AppState {
        pg_pool,
        influx_client,
        influx_bucket,
        mqtt_client: mqtt_client.clone(),
        alert_sender,
        api_key,
        device_states,
        solana_traceability: solana_service,
        sensor_sender,
        fcm_tokens: Mutex::new(Vec::new()),
        health_sender: health_tx,
        ph_calibration_sessions: Arc::new(RwLock::new(HashMap::new())),
        ph_voltage_samples: Arc::new(RwLock::new(HashMap::new())),
        dosing_dynamic_states: Arc::new(RwLock::new(HashMap::new())),
    });

    mqtt_client
        .subscribe("AGITECH/+/sensors", QoS::AtMostOnce)
        .await
        .expect("Lỗi sub");
    mqtt_client
        .subscribe("AGITECH/+/status", QoS::AtLeastOnce)
        .await
        .expect("Lỗi sub");
    mqtt_client
        .subscribe("AGITECH/+/sensor/status", QoS::AtLeastOnce)
        .await
        .expect("Lỗi sub");
    mqtt_client
        .subscribe("AGITECH/+/fsm", QoS::AtLeastOnce)
        .await
        .expect("Lỗi sub");
    mqtt_client
        .subscribe("AGITECH/+/controller/status", QoS::AtLeastOnce)
        .await
        .expect("Lỗi sub");
    mqtt_client
        .subscribe("AGITECH/+/dosing_report", QoS::AtLeastOnce)
        .await
        .expect("Lỗi sub");

    let app_state_for_mqtt = app_state.clone();
    tokio::spawn(async move {
        info!("Bắt đầu vòng lặp sự kiện MQTT dưới nền...");
        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                    mqtt::handler::process_message(publish, app_state_for_mqtt.clone()).await;
                }
                Ok(_) => {}
                Err(e) => {
                    error!("Mất kết nối MQTT, thử lại sau 5 giây... Lỗi: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    });

    let server_host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let server_port: u16 = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()?;

    info!(
        "🚀 API Server đang khởi chạy tại http://{}:{}",
        server_host, server_port
    );

    HttpServer::new(move || {
        let auth_middleware = api::middleware::auth::ApiKeyAuth::new();
        let rate_limit_middleware = api::middleware::rate_limit::RateLimiter::new(60, 60);

        // Khai báo CORS Middleware
        let cors = actix_cors::Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .app_data(app_state.clone())
            // CHÚ Ý 1: CORS bọc ngoài cùng (khai báo cuối cùng của App)
            .wrap(cors)
            // CHÚ Ý 2: Đưa Auth và Rate Limit vào bên TRONG scope /api
            .service(
                web::scope("/api")
                    // Middleware nội bộ của /api.
                    // Chạy sau CORS, auth chạy trước rate limit.
                    .wrap(auth_middleware)
                    .wrap(rate_limit_middleware)
                    // Các routes
                    .configure(api::notification::init_routes)
                    .configure(api::solana::init_routes)
                    .service(
                        web::scope("/devices/{device_id}")
                            .configure(api::control::init_routes)
                            .configure(api::sensor::init_routes)
                            .configure(api::ws::init_routes)
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
