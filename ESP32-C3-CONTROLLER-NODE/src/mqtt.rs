use crate::config::SharedConfig;
use esp_idf_svc::mqtt::client::{
    EspMqttClient, EventPayload, LwtConfiguration, MqttClientConfiguration, QoS,
};
use hydragrow_shared::topics::{
    topic_controller_command, topic_controller_config, topic_sensors, topic_status,
};
use hydragrow_shared::{ControllerConfig, ControllerHealthPayload, MqttCommandIn, PumpStatus, SensorData};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::{mpsc::Sender, Arc, RwLock};

use esp_idf_sys::{
    esp_get_free_heap_size, esp_timer_get_time, esp_wifi_sta_get_ap_info, wifi_ap_record_t,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionState {
    WifiConnected,
    WifiDisconnected,
    MqttConnected,
    MqttDisconnected,
}

#[derive(Debug, Deserialize)]
pub struct IncomingSensorPayload {
    pub temp: Option<f32>,
    pub ec: Option<f32>,
    pub ph: Option<f32>,
    pub water_level: Option<f32>,
    pub ph_voltage_mv: Option<f32>,
    pub time: Option<String>,

    pub rssi: Option<i32>,
    pub free_heap: Option<u32>,
    pub uptime: Option<u32>,
    pub is_continuous: Option<bool>,
    pub err_water: Option<bool>,
    pub err_temp: Option<bool>,
    pub err_ph: Option<bool>,
    pub err_ec: Option<bool>,
}

// XÓA BỎ RuntimeSensorData VÀ CHUYỂN SANG DÙNG SensorData TỪ shared_library
pub type SharedSensorData = Arc<RwLock<SensorData>>;

pub fn create_shared_sensor_data(device_id: &str) -> SharedSensorData {
    Arc::new(RwLock::new(SensorData {
        device_id: device_id.to_string(),
        ec: 0.0,
        ph: 7.0,
        temp: 25.0,
        water_level: 20.0,
        pump_status: PumpStatus::default(),
        time: String::new(),
        rssi: None,
        free_heap: None,
        uptime: None,
        err_water: None,
        err_temp: None,
        err_ph: None,
        err_ec: None,
        is_continuous: None,
        ph_voltage_mv: None,
    }))
}

#[derive(Debug, Serialize)]
pub fn get_free_heap() -> u32 {
    unsafe { esp_get_free_heap_size() as u32 }
}

pub fn get_uptime_sec() -> u64 {
    (unsafe { esp_timer_get_time() } / 1_000_000) as u64
}

pub fn get_wifi_rssi() -> i8 {
    let mut ap_info: wifi_ap_record_t = Default::default();
    let result = unsafe { esp_wifi_sta_get_ap_info(&mut ap_info) };
    if result == 0 {
        ap_info.rssi
    } else {
        0
    }
}

pub fn init_mqtt_client(
    broker_url: &str,
    shared_config: SharedConfig,
    shared_sensor_data: SharedSensorData,
    cmd_tx: Sender<MqttCommandIn>,
    conn_tx: Sender<ConnectionState>,
) -> anyhow::Result<EspMqttClient<'static>> {
    info!("🚀 Initializing MQTT client...");
    info!("Broker: {}", broker_url);

    let device_id = shared_config.read().unwrap().device_id.to_string();

    let topic_config = topic_controller_config(&device_id);
    let topic_command = topic_controller_command(&device_id);
    let topic_sensors = topic_sensors(&device_id);

    info!("Subscribing topics:");
    info!("Config: {}", topic_config);
    info!("Command: {}", topic_command);
    info!("Sensors: {}", topic_sensors);

    let topic_config_cb = topic_config.clone();
    let topic_command_cb = topic_command.clone();
    let topic_sensors_cb = topic_sensors.clone();

    let lwt_topic = topic_status(&device_id);
    let lwt_payload = r#"{"online": false, "status": "disconnected"}"#.as_bytes();
    let lwt_config = LwtConfiguration {
        topic: &lwt_topic,
        payload: lwt_payload,
        qos: QoS::AtLeastOnce,
        retain: true,
    };

    let mqtt_config = MqttClientConfiguration {
        buffer_size: 4096,
        keep_alive_interval: Some(std::time::Duration::from_secs(15)),
        password: Some("s7cjsq7bmxd7v4hlrf9idtwv6983rf3i"),
        username: Some("long"),
        lwt: Some(lwt_config),
        ..Default::default()
    };

    std::thread::sleep(std::time::Duration::from_secs(3));

    let client = EspMqttClient::new_cb(broker_url, &mqtt_config, move |event| {
        debug!("📩 MQTT Event Received");

        match event.payload() {
            EventPayload::Connected(_) => {
                info!("✅ MQTT Broker Callback: Connected");
                if let Err(e) = conn_tx.send(ConnectionState::MqttConnected) {
                    error!("Failed to send MQTT connected state: {:?}", e);
                }
            }

            EventPayload::Disconnected => {
                warn!("⚠️ MQTT Broker Callback: Disconnected");
                if let Err(e) = conn_tx.send(ConnectionState::MqttDisconnected) {
                    error!("Failed to send MQTT disconnected state: {:?}", e);
                }
            }

            EventPayload::Received { topic, data, .. } => {
                let topic_str = topic.unwrap_or("");

                // CONFIG UPDATE
                if topic_str == topic_config_cb {
                    debug!("⚙️ Processing CONFIG update");
                    match serde_json::from_slice::<ControllerConfig>(data) {
                        Ok(new_config) => {
                            info!("📦 New config received: {:?}", new_config);
                            if let Ok(mut config) = shared_config.write() {
                                *config = new_config;
                                info!("✅ Device config updated");
                            } else {
                                error!("❌ Failed to acquire config write lock");
                            }
                        }
                        Err(e) => error!("❌ Config JSON parse error: {:?}", e),
                    }
                }
                // COMMAND
                else if topic_str == topic_command_cb {
                    debug!("🎮 Processing COMMAND");
                    match serde_json::from_slice::<MqttCommandIn>(data) {
                        Ok(cmd) => {
                            info!("🎯 Command received: {:?}", cmd);
                            if let Err(e) = cmd_tx.send(cmd) {
                                error!("❌ Failed to forward command: {:?}", e);
                            }
                        }
                        Err(e) => error!("❌ Command JSON parse error: {:?}", e),
                    }
                }
                // SENSOR DATA
                else if topic_str == topic_sensors_cb {
                    debug!("📊 Processing SENSOR data snapshot");
                    match serde_json::from_slice::<IncomingSensorPayload>(data) {
                        Ok(payload) => {
                            if let Ok(mut sensors) = shared_sensor_data.write() {
                                if let Some(t) = payload.temp {
                                    sensors.temp = t;
                                }
                                if let Some(e) = payload.ec {
                                    sensors.ec = e;
                                }
                                if let Some(p) = payload.ph {
                                    sensors.ph = p;
                                }
                                if let Some(w) = payload.water_level {
                                    sensors.water_level = w;
                                }

                                if let Some(ph_voltage_mv) = payload.ph_voltage_mv {
                                    sensors.ph_voltage_mv = Some(ph_voltage_mv as f64);
                                }
                                if let Some(is_continuous) = payload.is_continuous {
                                    sensors.is_continuous = Some(is_continuous);
                                }
                                if let Some(err) = payload.err_water {
                                    sensors.err_water = Some(err);
                                }
                                if let Some(err) = payload.err_temp {
                                    sensors.err_temp = Some(err);
                                }
                                if let Some(err) = payload.err_ec {
                                    sensors.err_ec = Some(err);
                                }
                                if let Some(err) = payload.err_ph {
                                    sensors.err_ph = Some(err);
                                }
                                sensors.rssi = payload.rssi;
                                sensors.free_heap = payload.free_heap;
                                sensors.uptime = payload.uptime;
                                if let Some(time) = payload.time {
                                    sensors.time = time;
                                }

                                info!(
                                    "🌱 CẢM BIẾN | T: {:.1}°C | EC: {:.2} | pH: {:.2} | Lv: {:.1}cm | Sóng: {:?}dBm | Lỗi nước: {:?}",
                                    sensors.temp, sensors.ec, sensors.ph, sensors.water_level, sensors.rssi,
                                    sensors.err_water
                                );
                            } else {
                                error!("❌ Failed to acquire sensor write lock");
                            }
                        }
                        Err(e) => {
                            error!("❌ Sensor JSON parse error: {:?}", e);
                        }
                    }
                }
            }
            _ => {}
        }
    })?;

    info!("✅ MQTT client initialized with LWT configured");
    Ok(client)
}
