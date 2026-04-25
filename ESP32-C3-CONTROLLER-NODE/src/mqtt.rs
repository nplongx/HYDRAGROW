use crate::config::{DeviceConfig, SharedConfig};
use esp_idf_svc::mqtt::client::{
    EspMqttClient, EventPayload, LwtConfiguration, MqttClientConfiguration, QoS,
};
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PumpStatus {
    pub pump_a: bool,
    pub pump_b: bool,
    pub ph_up: bool,
    pub ph_down: bool,
    pub osaka_pump: bool,
    pub mist_valve: bool,
    pub water_pump_in: bool,
    pub water_pump_out: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pump_a_pwm: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pump_b_pwm: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ph_up_pwm: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ph_down_pwm: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub osaka_pwm: Option<u32>,

    #[serde(default, skip_serializing_if = "is_false")]
    pub dosing_pulse_active: bool,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub dosing_pulse_count: u32,
}

impl Default for PumpStatus {
    fn default() -> Self {
        Self {
            pump_a: false,
            pump_b: false,
            ph_up: false,
            ph_down: false,
            osaka_pump: false,
            mist_valve: false,
            water_pump_in: false,
            water_pump_out: false,
            pump_a_pwm: Some(0),
            pump_b_pwm: Some(0),
            ph_up_pwm: Some(0),
            ph_down_pwm: Some(0),
            osaka_pwm: Some(0),
            dosing_pulse_active: false,
            dosing_pulse_count: 0,
        }
    }
}

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

fn is_false(value: &bool) -> bool {
    !*value
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorData {
    pub ec: f32,
    pub ph: f32,
    pub temp: f32,
    pub water_level: f32,
    pub ph_voltage_mv: Option<f32>,
    pub is_continuous: bool,
    pub rssi: Option<i32>,
    pub free_heap: Option<u32>,
    pub uptime: Option<u32>,
    pub time: Option<String>,
    pub last_update_ms: u64,
    #[serde(default)]
    pub pump_status: PumpStatus,

    #[serde(default)]
    pub err_water: bool,
    #[serde(default)]
    pub err_temp: bool,
    #[serde(default)]
    pub err_ph: bool,
    #[serde(default)]
    pub err_ec: bool,
}

impl Default for SensorData {
    fn default() -> Self {
        Self {
            ec: 0.0,
            ph: 7.0,
            temp: 25.0,
            water_level: 20.0,
            ph_voltage_mv: None,
            is_continuous: false,
            rssi: None,
            free_heap: None,
            uptime: None,
            time: None,
            last_update_ms: 0,
            pump_status: PumpStatus::default(),
            err_water: false,
            err_temp: false,
            err_ph: false,
            err_ec: false,
        }
    }
}

pub type SharedSensorData = Arc<RwLock<SensorData>>;

pub fn create_shared_sensor_data() -> SharedSensorData {
    Arc::new(RwLock::new(SensorData::default()))
}

#[derive(Debug, Deserialize, Clone)]
pub struct MqttCommandPayload {
    pub target: Option<String>,
    pub action: String,
    pub params: Option<MqttCommandParams>,
    pub pump: Option<String>,
    pub duration_sec: Option<u64>,
    pub pwm: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MqttCommandParams {
    pub pump_id: Option<String>,
    pub duration_sec: Option<u64>,
    pub pwm: Option<u32>,
    pub state: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ControllerHealthPayload {
    pub free_heap: u32,
    pub uptime_sec: u64,
    pub rssi: i8,
    pub pump_status: PumpStatus,
}

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
    cmd_tx: Sender<MqttCommandPayload>,
    conn_tx: Sender<ConnectionState>,
) -> anyhow::Result<EspMqttClient<'static>> {
    info!("🚀 Initializing MQTT client...");
    info!("Broker: {}", broker_url);

    let device_id = shared_config.read().unwrap().device_id.to_string();

    let topic_config = format!("AGITECH/{}/controller/config", device_id);
    let topic_command = format!("AGITECH/{}/controller/command", device_id);
    let topic_sensors = format!("AGITECH/{}/sensors", device_id);

    info!("Subscribing topics:");
    info!("Config: {}", topic_config);
    info!("Command: {}", topic_command);
    info!("Sensors: {}", topic_sensors);

    let topic_config_cb = topic_config.clone();
    let topic_command_cb = topic_command.clone();
    let topic_sensors_cb = topic_sensors.clone();

    let lwt_topic = format!("AGITECH/{}/status", device_id);
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
        password: Some("53zx37kxq3epbexgqt6rjlce1d0e0gwq"),
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
                    match serde_json::from_slice::<DeviceConfig>(data) {
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
                    match serde_json::from_slice::<MqttCommandPayload>(data) {
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
                                    sensors.ph_voltage_mv = Some(ph_voltage_mv);
                                }
                                if let Some(is_continuous) = payload.is_continuous {
                                    sensors.is_continuous = is_continuous;
                                }
                                if let Some(err) = payload.err_water {
                                    sensors.err_water = err;
                                }
                                if let Some(err) = payload.err_temp {
                                    sensors.err_temp = err;
                                }
                                if let Some(err) = payload.err_ec {
                                    sensors.err_ec = err;
                                }
                                if let Some(err) = payload.err_ph {
                                    sensors.err_ph = err;
                                }
                                sensors.rssi = payload.rssi;
                                sensors.free_heap = payload.free_heap;
                                sensors.uptime = payload.uptime;
                                if let Some(time) = payload.time {
                                    sensors.time = Some(time);
                                }

                                sensors.last_update_ms = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis()
                                    as u64;

                                info!(
                                    "🌱 CẢM BIẾN | T: {:.1}°C | EC: {:.2} | pH: {:.2} | Lv: {:.1}cm | Sóng: {:?}dBm | Lỗi nước: {:?}",
                                    sensors.temp, sensors.ec, sensors.ph, sensors.water_level, sensors.rssi, payload.err_water
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
