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
use hydragrow_shared::{
    ControllerConfig, IncomingSensorPayload, MqttCommandIn, PumpStatus, RecipeSetCommand, SensorData,
};
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
                // 2. Update Crop Recipe (controller/recipe or signed recipe/set)
                else if topic_str == topic_recipe_cb || topic_str == topic_recipe_set_cb {
                    let maybe_recipe: Result<Option<CropRecipe>, String> = if topic_str == topic_recipe_set_cb {
                        match verify_signed_json_payload(&device_id, data, &command_secret_cb) {
                            Ok(payload) => {
                                if let Some(action) = payload.get("action").and_then(|a| a.as_str()) {
                                    if action == "clear" {
                                        Ok(None)
                                    } else {
                                        match serde_json::from_value::<RecipeSetCommand>(payload.clone())
                                            .and_then(|cmd| Ok(serde_json::from_value::<CropRecipe>(cmd.recipe)?))
                                        {
                                            Ok(recipe) => Ok(Some(recipe)),
                                            Err(e) => Err(format!("invalid_recipe_set_payload: {:?}", e)),
                                        }
                                    }
                                } else {
                                    match serde_json::from_value::<RecipeSetCommand>(payload.clone())
                                        .and_then(|cmd| Ok(serde_json::from_value::<CropRecipe>(cmd.recipe)?))
                                    {
                                        Ok(recipe) => Ok(Some(recipe)),
                                        Err(e) => Err(format!("invalid_recipe_set_payload: {:?}", e)),
                                    }
                                }
                            }
                            Err(e) => Err(format!("signature_verification_failed: {:?}", e)),
                        }
                    } else {
                        serde_json::from_slice::<CropRecipe>(data)
                            .map(Some)
                            .map_err(|e| format!("invalid_json: {:?}", e))
                    };

                    match maybe_recipe {
                        Ok(Some(recipe)) => {
                            let config = shared_config.read().unwrap().clone();
                            let mut nvs_store = NvsStore::new(nvs_partition.clone());
                            let current_revision = nvs_store
                                .load_active_recipe()
                                .ok()
                                .flatten()
                                .map(|r| r.revision);

                            match validate_recipe(
                                &recipe,
                                &config.effective_config,
                                &device_id,
                                current_revision,
                            ) {
                                Ok(()) => {
                                    if let Err(e) = nvs_store.save_active_recipe(&recipe) {
                                        warn!("⚠️ Failed to persist active recipe to NVS: {:?}", e);
                                    }
                                    if let Ok(mut state) = shared_config.write() {
                                        state.activate_recipe(recipe.clone());
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
                        Ok(None) => {
                            // Clear recipe request
                            info!("🗑️ Received recipe clear command. Removing recipe...");
                            let mut nvs_store = NvsStore::new(nvs_partition.clone());
                            let _ = nvs_store.clear_active_recipe();
                            if let Ok(mut state) = shared_config.write() {
                                state.clear_recipe();
                            }
                            let event = serde_json::json!({
                                "_mqtt_topic_override": recipe_event_topic.clone(),
                                "_payload": serde_json::from_str::<serde_json::Value>(&build_recipe_event(&device_id, "cleared", 0u64, None)).unwrap_or_default()
                            });
                            let _ = recipe_event_tx.send(event.to_string());
                        }
                        Err(reason) => {
                            warn!("❌ Recipe payload rejected: {}", reason);
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
                // 4. Sensor Data Packet
                else if topic_str == topic_sensors_cb {
                    if let Ok(payload) = serde_json::from_slice::<IncomingSensorPayload>(data) {
                        if let Ok(mut sensors) = shared_sensor_data.write() {
                            let uptime_ms = get_uptime_ms();
                            if !sensors.merge_incoming_payload(&payload, uptime_ms) {
                                warn!("⚠️ [SENSORS] Bỏ qua gói tin cảm biến rỗng hoặc không hợp lệ");
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    })?;

    Ok(client)
}
