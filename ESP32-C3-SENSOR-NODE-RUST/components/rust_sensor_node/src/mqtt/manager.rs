use anyhow::Result;
use esp_idf_svc::mqtt::client::{EspMqttClient, EventPayload, MqttClientConfiguration, QoS};
use esp_idf_svc::tls::X509;
use log::{error, info, warn};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::AppConfig;
use crate::mqtt::command_security::CommandSecurity;
use crate::mqtt::payload::{build_sensor_payload, build_status_payload};
use crate::sensors::sensor_manager::SensorData;

const ROOT_CA_PEM: &str = concat!(include_str!("../../certs/root_ca.pem"), "\0");

pub struct MqttTopics {
    pub sensor: String,
    pub status: String,
    pub command: String,
    pub config: String,
}

impl MqttTopics {
    pub fn new(device_id: &str) -> Self {
        let prefix = format!("AGITECH/{}/", device_id);
        Self {
            sensor: format!("{}sensors", prefix),
            status: format!("{}sensor/status", prefix),
            command: format!("{}command", prefix),
            config: format!("{}sensors/config", prefix),
        }
    }
}

pub struct MqttConfig {
    pub broker_url: &'static str, // "mqtts://host:8883"
    pub client_id: &'static str,
    pub username: &'static str,
    pub password: &'static str,
    pub command_secret: &'static str,
}

/// MQTT Manager — TLS + subscribe + publish.
/// Thread-safe: SensorData được truyền qua Arc<Mutex<SensorData>>.
pub struct MqttManager {
    config: MqttConfig,
    topics: MqttTopics,
    shared_data: Arc<Mutex<SensorData>>,
    shared_app_config: Arc<Mutex<AppConfig>>,
    security: CommandSecurity,
}

impl MqttManager {
    pub fn new(
        config: MqttConfig,
        shared_data: Arc<Mutex<SensorData>>,
        shared_app_config: Arc<Mutex<AppConfig>>,
    ) -> Self {
        let topics = MqttTopics::new(config.client_id);
        let security = CommandSecurity::new(config.command_secret);
        Self {
            config,
            topics,
            shared_data,
            shared_app_config,
            security,
        }
    }

    /// Khởi chạy MQTT loop. Blocking — nên chạy trong FreeRTOS task riêng.
    pub fn run(&self) -> Result<()> {
        let cert_cstr = std::ffi::CStr::from_bytes_with_nul(ROOT_CA_PEM.as_bytes())
            .expect("Lỗi chứng chỉ: Không đúng chuẩn C string");

        let tls_config = MqttClientConfiguration {
            client_id: Some(self.config.client_id),
            username: Some(self.config.username),
            password: Some(self.config.password),
            server_certificate: Some(X509::pem(cert_cstr)),
            keep_alive_interval: Some(Duration::from_secs(30)),
            ..Default::default()
        };

        let topics = MqttTopics::new(self.config.client_id);
        let shared_data = Arc::clone(&self.shared_data);
        let shared_config = Arc::clone(&self.shared_app_config);
        let device_id = self.config.client_id.to_string();

        let (mut client, mut connection) = EspMqttClient::new(self.config.broker_url, &tls_config)?;

        info!("[MQTT] Kết nối đến {}", self.config.broker_url);

        // Subscribe sau khi connected (trong event loop)
        std::thread::spawn(move || loop {
            match connection.next() {
                Ok(event) => {
                    if let EventPayload::Connected(_) = event.payload() {
                        info!("[MQTT] Connected!");
                        let _ = client.subscribe(&topics.command, QoS::AtLeastOnce);
                        let _ = client.subscribe(&topics.config, QoS::AtLeastOnce);
                        // Publish online status
                        let status =
                            build_status_payload(&device_id, "online", "Sensor node connected");
                        let _ = client.publish(
                            &topics.status,
                            QoS::AtLeastOnce,
                            false,
                            status.to_string().as_bytes(),
                        );
                    }
                    if let EventPayload::Received { topic, data, .. } = event.payload() {
                        let topic = topic.unwrap_or("");
                        let payload_str = std::str::from_utf8(data).unwrap_or("");
                        if let Ok(doc) = serde_json::from_str::<Value>(payload_str) {
                            if topic == topics.command {
                                // Command handling (get_status, restart)
                                let cmd = doc["cmd"].as_str().unwrap_or("");
                                match cmd {
                                    "get_status" => {
                                        if let Ok(d) = shared_data.lock() {
                                            let payload =
                                                build_sensor_payload(&device_id, &d, "N/A", 0, 0);
                                            let _ = client.publish(
                                                &topics.sensor,
                                                QoS::AtLeastOnce,
                                                false,
                                                payload.to_string().as_bytes(),
                                            );
                                        }
                                    }
                                    "restart" => {
                                        warn!("[MQTT] Restart command received");
                                        unsafe {
                                            esp_idf_sys::esp_restart();
                                        }
                                    }
                                    _ => {}
                                }
                            } else if topic == topics.config {
                                if let Ok(mut cfg) = shared_config.lock() {
                                    cfg.apply_from_json(&doc);
                                    info!("[MQTT] Config applied");
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("[MQTT] Connection error: {:?}", e);
                    std::thread::sleep(Duration::from_secs(5));
                }
            }
        });

        Ok(())
    }

    /// Publish sensor data. Gọi theo publish_interval từ main loop.
    pub fn publish_sensor_data(
        client: &mut EspMqttClient,
        topic: &str,
        device_id: &str,
        data: &SensorData,
        timestamp_iso: &str,
        rssi: i32,
        free_heap: u32,
    ) -> Result<()> {
        let payload = build_sensor_payload(device_id, data, timestamp_iso, rssi, free_heap);
        client.publish(
            topic,
            QoS::AtLeastOnce,
            false,
            payload.to_string().as_bytes(),
        )?;
        Ok(())
    }
}
