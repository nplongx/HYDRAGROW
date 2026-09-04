// src/hw/mqtt_client.rs
//! Client MQTT kết nối với Broker và xử lý Packet cảm biến/lệnh.

use esp_idf_svc::mqtt::client::{
    EspMqttClient, EventPayload, LwtConfiguration, MqttClientConfiguration, QoS,
};
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};
use esp_idf_sys::{
    esp_get_free_heap_size, esp_timer_get_time, esp_wifi_sta_get_ap_info, wifi_ap_record_t,
};
use hydragrow_controller_core::core::security::verify_signed_json_payload;
use hydragrow_shared::topics::{
    topic_controller_command, topic_controller_config, topic_controller_recipe,
    topic_recipe_events, topic_recipe_set, topic_sensors, topic_status,
};
use hydragrow_shared::{ControllerConfig, MqttCommandIn, PumpStatus, RecipeSetCommand, SensorData};
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::sync::{mpsc::Sender, Arc, RwLock};

use crate::config::SharedConfig;
use hydragrow_controller_core::utils::{build_recipe_event, validate_recipe};
use hydragrow_shared::recipe::CropRecipe;

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
    #[serde(alias = "tds")]
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
    #[serde(alias = "err_tds")]
    pub err_ec: Option<bool>,
}

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
        controller_received_ms: None,
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

pub fn get_free_heap() -> u32 {
    unsafe { esp_get_free_heap_size() as u32 }
}

pub fn get_uptime_sec() -> u64 {
    (unsafe { esp_timer_get_time() } / 1_000_000) as u64
}

pub fn get_uptime_ms() -> u64 {
    (unsafe { esp_timer_get_time() } / 1_000) as u64
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

#[allow(clippy::too_many_arguments)] // TODO(follow-up): group broker/auth args into an MqttClientConfig struct
pub fn init_mqtt_client(
    broker_url: &str,
    mqtt_user: &str,
    mqtt_password: &str,
    mqtt_command_secret: &str,
    shared_config: SharedConfig,
    shared_sensor_data: SharedSensorData,
    cmd_tx: Sender<MqttCommandIn>,
    conn_tx: Sender<ConnectionState>,
    recipe_event_tx: Sender<String>,
    nvs_partition: EspDefaultNvsPartition,
) -> anyhow::Result<EspMqttClient<'static>> {
    info!("🚀 Initializing MQTT client...");
    info!("Broker: {}", broker_url);

    let device_id = shared_config
        .read()
        .unwrap()
        .effective_config
        .device_id
        .to_string();
    let topic_config = topic_controller_config(&device_id);
    let topic_command = topic_controller_command(&device_id);
    let topic_recipe = topic_controller_recipe(&device_id);
    let topic_sensors_top = topic_sensors(&device_id);

    let topic_config_cb = topic_config.clone();
    let topic_command_cb = topic_command.clone();
    let topic_recipe_cb = topic_recipe.clone();
    let topic_sensors_cb = topic_sensors_top.clone();
    let recipe_event_topic = topic_recipe_events(&device_id);

    let lwt_topic = topic_status(&device_id);
    let lwt_payload = r#"{"online": false, "status": "disconnected"}"#.as_bytes();
    let lwt_config = LwtConfiguration {
        topic: &lwt_topic,
        payload: lwt_payload,
        qos: QoS::AtLeastOnce,
        retain: true,
    };

    let topic_recipe_set = topic_recipe_set(&device_id);
    let topic_recipe_set_cb = topic_recipe_set.clone();
    let command_secret_cb = mqtt_command_secret.to_string();

    let mqtt_config = MqttClientConfiguration {
        buffer_size: 4096,
        keep_alive_interval: Some(std::time::Duration::from_secs(15)),
        password: Some(mqtt_password),
        username: Some(mqtt_user),
        lwt: Some(lwt_config),
        ..Default::default()
    };

    std::thread::sleep(std::time::Duration::from_secs(3));

    let client = EspMqttClient::new_cb(broker_url, &mqtt_config, move |event| {
        debug!("📩 MQTT Event Received");
        match event.payload() {
            EventPayload::Connected(_) => {
                info!("🟢 MQTT Broker Callback: Connected");
                let _ = conn_tx.send(ConnectionState::MqttConnected);
            }
            EventPayload::Disconnected => {
                warn!("🔴 MQTT Broker Callback: Disconnected");
                let _ = conn_tx.send(ConnectionState::MqttDisconnected);
            }
            EventPayload::Received { topic, data, .. } => {
                let topic_str = topic.unwrap_or("");

                // 1. Update Config
                if topic_str == topic_config_cb {
                    match serde_json::from_slice::<ControllerConfig>(data) {
                        Ok(new_config) => {
                            if let Err(errors) = new_config.validate() {
                                error!(
                                    "❌ Config validation failed ({} errors): {:?}",
                                    errors.len(),
                                    errors
                                );
                            } else {
                                info!("✅ New config received & applied: {}", new_config.device_id);
                                if let Ok(mut config) = shared_config.write() {
                                    config.set_base_config(new_config);
                                }
                            }
                        }
                        Err(e) => error!("❌ Config JSON parse error: {:?}", e),
                    }
                }
                // 2. Update Crop Recipe
                else if topic_str == topic_recipe_cb {
                    match serde_json::from_slice::<CropRecipe>(data) {
                        Ok(recipe) => {
                            let config = shared_config.read().unwrap().clone();
                            let mut nvs = EspNvs::new(nvs_partition.clone(), "agitech", true).ok();
                            let current_revision = nvs
                                .as_mut()
                                .and_then(|nvs| nvs.get_u64("recipe_rev").ok().flatten());

                            match validate_recipe(
                                &recipe,
                                &config.effective_config,
                                &device_id,
                                current_revision,
                            ) {
                                Ok(()) => {
                                    if let Some(nvs) = nvs.as_mut() {
                                        if let Ok(serialized) = serde_json::to_string(&recipe) {
                                            let _ = nvs.set_str("crop_recipe", &serialized);
                                            let _ = nvs.set_u64("recipe_rev", recipe.revision);
                                        }
                                    }
                                    let event = serde_json::json!({
                                        "_mqtt_topic_override": recipe_event_topic.clone(),
                                        "_payload": serde_json::from_str::<serde_json::Value>(&build_recipe_event(&device_id, "accepted", recipe.revision, None)).unwrap_or_default()
                                    });
                                    let _ = recipe_event_tx.send(event.to_string());
                                }
                                Err(e) => {
                                    let reason = e.to_string();
                                    warn!("❌ Recipe validation failed: {}", reason);
                                    let event = serde_json::json!({
                                        "_mqtt_topic_override": recipe_event_topic.clone(),
                                        "_payload": serde_json::from_str::<serde_json::Value>(&build_recipe_event(&device_id, "rejected", recipe.revision, Some(&reason))).unwrap_or_default()
                                    });
                                    let _ = recipe_event_tx.send(event.to_string());
                                }
                            }
                        }
                        Err(e) => {
                            let reason = format!("invalid_json: {:?}", e);
                            error!("❌ Recipe JSON parse error: {}", reason);
                            let event = serde_json::json!({
                                "_mqtt_topic_override": recipe_event_topic.clone(),
                                "_payload": serde_json::from_str::<serde_json::Value>(&build_recipe_event(&device_id, "rejected", 0u64, Some(&reason))).unwrap_or_default()
                            });
                            let _ = recipe_event_tx.send(event.to_string());
                        }
                    }
                }
                // 3. Received Command — HMAC-verified before parsing.
                else if topic_str == topic_command_cb {
                    match verify_signed_json_payload(&device_id, data, &command_secret_cb)
                        .and_then(|payload| Ok(serde_json::from_value::<MqttCommandIn>(payload)?))
                    {
                        Ok(cmd) => {
                            info!("📥 Command received: {:?}", cmd);
                            let _ = cmd_tx.send(cmd);
                        }
                        Err(e) => warn!("⛔ Rejected unsigned/invalid command payload: {:?}", e),
                    }
                }
                // 3. Sensor Data Packet
                else if topic_str == topic_sensors_cb {
                    if let Ok(payload) = serde_json::from_slice::<IncomingSensorPayload>(data) {
                        if let Ok(mut sensors) = shared_sensor_data.write() {
                            sensors.controller_received_ms = Some(get_uptime_ms());
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
                            if let Some(mv) = payload.ph_voltage_mv {
                                sensors.ph_voltage_mv = Some(mv as f64);
                            }
                            if let Some(cont) = payload.is_continuous {
                                sensors.is_continuous = Some(cont);
                            }
                            sensors.err_water = payload.err_water;
                            sensors.err_temp = payload.err_temp;
                            sensors.err_ec = payload.err_ec;
                            sensors.err_ph = payload.err_ph;
                            sensors.rssi = payload.rssi;
                            sensors.free_heap = payload.free_heap;
                            sensors.uptime = payload.uptime;
                            if let Some(time) = payload.time {
                                sensors.time = time;
                            }
                        }
                    }
                } else if topic_str == topic_recipe_set_cb {
                    match verify_signed_json_payload(&device_id, data, &command_secret_cb)
                        .and_then(|payload| {
                            Ok(serde_json::from_value::<RecipeSetCommand>(payload)?)
                        })
                        .and_then(|cmd| Ok(serde_json::from_value::<CropRecipe>(cmd.recipe)?))
                    {
                        Ok(recipe) => {
                            info!("🌱 Signed crop recipe received: {:?}", recipe);
                            // TODO: apply recipe to runtime recipe store once CropRecipe has a concrete FSM target.
                        }
                        Err(e) => warn!("⛔ Rejected unsigned/invalid recipe payload: {:?}", e),
                    }
                }
            }
            _ => {}
        }
    })?;

    Ok(client)
}
