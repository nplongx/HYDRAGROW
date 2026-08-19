// src/runtime/health.rs
//! Health & Hestia Engine Bridge — Quản lý Snapshot sức khỏe thiết bị và Main Health Loop.

use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

use esp_idf_svc::mqtt::client::{EspMqttClient, QoS};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use hydragrow_shared::fsm::{FsmBudgets, FsmSnapshot};
use hydragrow_shared::hestia::{HestiaAction, HestiaContext, HestiaEngine};
use hydragrow_shared::telemetry::health::{DeviceHealthSnapshot, KalmanConfidence};
use hydragrow_shared::topics::{
    topic_calibration, topic_controller_command, topic_controller_config, topic_controller_recipe,
    topic_controller_status, topic_dosing_report, topic_fsm_events, topic_fsm_state,
    topic_sensor_command, topic_sensors, topic_status,
};
use hydragrow_shared::MqttCommandIn;
use log::{error, info, warn};

use crate::config::SharedConfig;
use crate::core::fsm::context::SystemContext;
use crate::hw::mqtt_client::{
    get_free_heap, get_uptime_sec, get_wifi_rssi, init_mqtt_client, ConnectionState,
    SharedSensorData,
};
use crate::utils::{get_current_time_sec, get_log_drop_count};

pub fn build_status_msg(ctx: &SystemContext, now_sec: u64) -> String {
    let sum_ml = |pump_name: &str| -> f32 {
        ctx.safety
            .hourly_doses()
            .get(pump_name)
            .map(|hist| {
                hist.iter()
                    .filter(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600)
                    .map(|(_, ml)| ml)
                    .sum()
            })
            .unwrap_or(0.0)
    };

    let refill_count = ctx
        .safety
        .refill_history()
        .iter()
        .filter(|ts| now_sec.saturating_sub(**ts) <= 3600)
        .count();

    let drain_count = ctx
        .safety
        .drain_history()
        .iter()
        .filter(|ts| now_sec.saturating_sub(**ts) <= 3600)
        .count();

    let mut diagnostics_snapshot = ctx.diagnostic.clone();
    diagnostics_snapshot.log_drop_count = get_log_drop_count();

    let payload = FsmSnapshot {
        online: true,
        current_phase: ctx.phase.clone(),
        previous_phase: ctx.previous_phase.clone(),
        pump_status: ctx.peripherals.pump_status.clone(),
        budgets: FsmBudgets {
            ec_ml: sum_ml("NutrientA") + sum_ml("NutrientB"),
            ph_ml: sum_ml("PhUp") + sum_ml("PhDown"),
            refill_count: refill_count as u32,
            drain_count: drain_count as u32,
        },
        diagnostics: Some(diagnostics_snapshot),
    };

    serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
}

pub fn hestia_action_from_phase(phase: &str) -> HestiaAction {
    match phase {
        "MimoDosing" => HestiaAction::EcDosing,
        "WaterRefilling" => HestiaAction::WaterRefill,
        "WaterDraining" => HestiaAction::WaterDrain,
        "ActiveMixing" => HestiaAction::Mixing,
        "Stabilizing" | "Cooldown" => HestiaAction::Manual,
        _ => HestiaAction::None,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_main_health_loop(
    mqtt_url: &str,
    mqtt_user: &str,
    mqtt_password: &str,
    shared_config: SharedConfig,
    shared_sensor_data: SharedSensorData,
    conn_rx: Receiver<ConnectionState>,
    conn_tx: Sender<ConnectionState>,
    cmd_tx: Sender<MqttCommandIn>,
    fsm_tx: Sender<String>,
    fsm_rx: Receiver<String>,
    dosing_report_rx: Receiver<String>,
    sensor_cmd_rx: Receiver<String>,
    nvs_partition: EspDefaultNvsPartition,
) -> anyhow::Result<()> {
    let mut mqtt_client: Option<EspMqttClient> = None;
    let mut is_mqtt_connected = false;

    info!("🔄 Đang chạy Main Event Loop...");

    let mut force_publish_next = false;
    let mut last_health_publish = std::time::Instant::now();
    let mut latest_fsm_snapshot: Option<serde_json::Value> = None;
    let mut latest_runtime_calibration: Option<serde_json::Value> = None;
    let mut previous_hestia_sensor: Option<hydragrow_shared::SensorData> = None;
    let mut previous_hestia_sensor_sec: Option<u64> = None;
    let mut last_hestia_action = HestiaAction::None;
    let mut last_hestia_intervention_sec: Option<u64> = None;

    loop {
        // XỬ LÝ TRẠNG THÁI KẾT NỐI
        if let Ok(state) = conn_rx.try_recv() {
            match state {
                ConnectionState::WifiConnected => {
                    info!("🛜 Đã kết nối WiFi. Tiến hành khởi tạo MQTT...");
                    if mqtt_client.is_none() {
                        match init_mqtt_client(
                            mqtt_url,
                            mqtt_user,
                            mqtt_password,
                            shared_config.clone(),
                            shared_sensor_data.clone(),
                            cmd_tx.clone(),
                            conn_tx.clone(),
                            fsm_tx.clone(),
                            nvs_partition.clone(),
                        ) {
                            Ok(client) => mqtt_client = Some(client),
                            Err(e) => error!("❌ Lỗi khởi tạo MQTT: {:?}", e),
                        }
                    }
                }
                ConnectionState::WifiDisconnected => {
                    warn!("⚠️ Rớt mạng WiFi!");
                    is_mqtt_connected = false;
                    mqtt_client = None;
                }
                ConnectionState::MqttConnected => {
                    info!("📡 MQTT Client: ĐÃ KẾT NỐI THÀNH CÔNG");
                    is_mqtt_connected = true;

                    if let Some(client) = mqtt_client.as_mut() {
                        let device_id = shared_config
                            .read()
                            .unwrap()
                            .effective_config
                            .device_id
                            .clone();
                        let topic_config = topic_controller_config(&device_id);
                        let topic_command = topic_controller_command(&device_id);
                        let topic_status = topic_status(&device_id);
                        let topic_sensors = topic_sensors(&device_id);
                        let topic_recipe = topic_controller_recipe(&device_id);

                        let payload = serde_json::json!({
                            "device_id": device_id,
                            "is_online": true,
                            "online": true
                        })
                        .to_string();
                        let _ = client.publish(
                            &topic_status,
                            QoS::AtLeastOnce,
                            true,
                            payload.as_bytes(),
                        );
                        let _ = client.subscribe(&topic_config, QoS::AtLeastOnce);
                        let _ = client.subscribe(&topic_command, QoS::AtLeastOnce);
                        let _ = client.subscribe(&topic_sensors, QoS::AtLeastOnce);
                        let _ = client.subscribe(&topic_recipe, QoS::AtLeastOnce);
                    }
                }
                ConnectionState::MqttDisconnected => {
                    warn!("📡 MQTT Client: MẤT KẾT NỐI");
                    is_mqtt_connected = false;
                }
            }
        }

        // XỬ LÝ PAYLOAD TỪ FSM
        if let Ok(payload) = fsm_rx.try_recv() {
            if is_mqtt_connected {
                if let Some(client) = mqtt_client.as_mut() {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&payload) {
                        if v.get("current_phase").is_some() {
                            latest_fsm_snapshot = Some(v.clone());
                        }
                        if v.get("type").and_then(|t| t.as_str())
                            == Some("runtime_calibration_update")
                        {
                            latest_runtime_calibration = Some(v.clone());
                        }

                        let device_id = shared_config
                            .read()
                            .unwrap()
                            .effective_config
                            .device_id
                            .clone();
                        let topic = if let Some(override_topic) =
                            v.get("_mqtt_topic_override").and_then(|t| t.as_str())
                        {
                            let actual_payload = v.get("_payload").cloned().unwrap_or(v.clone());
                            let actual_payload_str =
                                serde_json::to_string(&actual_payload).unwrap_or(payload.clone());
                            let _ = client.publish(
                                override_topic,
                                QoS::AtLeastOnce,
                                false,
                                actual_payload_str.as_bytes(),
                            );
                            continue;
                        } else {
                            match v.get("type").and_then(|t| t.as_str()) {
                                Some("water_event") | Some("system_alert")
                                | Some("dosing_cycle") => topic_fsm_events(&device_id),
                                Some("ema_update")
                                | Some("auto_tune")
                                | Some("runtime_calibration_update") => {
                                    topic_calibration(&device_id)
                                }
                                _ => topic_fsm_state(&device_id),
                            }
                        };
                        let _ = client.publish(&topic, QoS::AtLeastOnce, false, payload.as_bytes());
                    }
                }
            }
        }

        if let Ok(report_json) = dosing_report_rx.try_recv() {
            if is_mqtt_connected {
                if let Some(client) = mqtt_client.as_mut() {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&report_json) {
                        if let Some(override_topic) =
                            v.get("_mqtt_topic_override").and_then(|t| t.as_str())
                        {
                            let actual_payload = v.get("_payload").cloned().unwrap_or(v.clone());
                            let actual_str = serde_json::to_string(&actual_payload)
                                .unwrap_or_else(|_| report_json.clone());
                            let _ = client.publish(
                                override_topic,
                                QoS::AtLeastOnce,
                                false,
                                actual_str.as_bytes(),
                            );
                            continue;
                        }
                    }
                    let device_id = shared_config
                        .read()
                        .unwrap()
                        .effective_config
                        .device_id
                        .clone();
                    let topic = topic_dosing_report(&device_id);
                    let _ = client.publish(&topic, QoS::AtLeastOnce, false, report_json.as_bytes());
                }
            }
        }

        if let Ok(sensor_cmd_json) = sensor_cmd_rx.try_recv() {
            if sensor_cmd_json.contains("\"action\":\"force_publish\"") {
                force_publish_next = true;
            } else if is_mqtt_connected {
                if let Some(client) = mqtt_client.as_mut() {
                    let device_id = shared_config
                        .read()
                        .unwrap()
                        .effective_config
                        .device_id
                        .clone();
                    let topic_sensor_cmd = topic_sensor_command(&device_id);
                    let _ = client.publish(
                        &topic_sensor_cmd,
                        QoS::AtLeastOnce,
                        false,
                        sensor_cmd_json.as_bytes(),
                    );
                }
            }
        }

        if is_mqtt_connected
            && (force_publish_next || last_health_publish.elapsed().as_secs() >= 10)
        {
            last_health_publish = std::time::Instant::now();
            force_publish_next = false;

            if let Some(client) = mqtt_client.as_mut() {
                let diagnostics = latest_fsm_snapshot
                    .as_ref()
                    .and_then(|v| v.get("diagnostics"));
                let health_score_percent = diagnostics
                    .and_then(|v| v.get("health_score_percent"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100) as u32;
                let log_drop_count = diagnostics
                    .and_then(|v| v.get("log_drop_count"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let fsm_state_display = latest_fsm_snapshot
                    .as_ref()
                    .and_then(|v| v.get("current_phase"))
                    .map(|phase| {
                        phase
                            .as_str()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| phase.to_string().trim_matches('"').to_string())
                    })
                    .unwrap_or_else(|| "Unknown".to_string());

                let runtime_coefficients = latest_runtime_calibration
                    .as_ref()
                    .and_then(|v| v.get("runtime_coefficients"));
                let matrix_update_count = runtime_coefficients
                    .and_then(|v| v.get("matrix_update_count"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let matrix_is_warm = runtime_coefficients
                    .and_then(|v| v.get("matrix_is_warm"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let kalman_confidence = runtime_coefficients
                    .and_then(|v| v.get("kalman_confidence"))
                    .and_then(|v| v.as_array())
                    .filter(|items| items.len() == 8)
                    .map(|items| KalmanConfidence {
                        nutrient_a: items[0].as_f64().unwrap_or(0.0) as f32,
                        nutrient_b: items[1].as_f64().unwrap_or(0.0) as f32,
                        ph_up: items[2].as_f64().unwrap_or(0.0) as f32,
                        ph_down: items[3].as_f64().unwrap_or(0.0) as f32,
                        water_in: items[4].as_f64().unwrap_or(0.0) as f32,
                        water_out: items[5].as_f64().unwrap_or(0.0) as f32,
                        osaka_mixing: items[6].as_f64().unwrap_or(0.0) as f32,
                        misting: items[7].as_f64().unwrap_or(0.0) as f32,
                    });
                let mean_kalman_confidence = kalman_confidence.as_ref().map(|confidence| {
                    (confidence.nutrient_a
                        + confidence.nutrient_b
                        + confidence.ph_up
                        + confidence.ph_down
                        + confidence.water_in
                        + confidence.water_out
                        + confidence.osaka_mixing
                        + confidence.misting)
                        / 8.0
                });

                let now_sec = get_current_time_sec();
                let current_hestia_action = hestia_action_from_phase(&fsm_state_display);
                if current_hestia_action != HestiaAction::None {
                    last_hestia_action = current_hestia_action;
                    last_hestia_intervention_sec = Some(now_sec);
                }
                let current_sensor = shared_sensor_data.read().ok().map(|sensor| sensor.clone());
                let current_config = shared_config
                    .read()
                    .ok()
                    .map(|state| state.effective_config.clone());
                let hestia = match (current_sensor, current_config) {
                    (Some(sensor), Some(config)) => {
                        let context = HestiaContext {
                            previous: previous_hestia_sensor.clone(),
                            minutes_since_previous: previous_hestia_sensor_sec
                                .map(|ts| now_sec.saturating_sub(ts) as f32 / 60.0),
                            minutes_since_last_intervention: last_hestia_intervention_sec
                                .map(|ts| now_sec.saturating_sub(ts) as f32 / 60.0),
                            last_action: last_hestia_action,
                            matrix_is_warm,
                            mean_kalman_confidence,
                            phase: Some(fsm_state_display.clone()),
                        };
                        let assessment = HestiaEngine::evaluate(&sensor, &config, &context);
                        previous_hestia_sensor = Some(sensor);
                        previous_hestia_sensor_sec = Some(now_sec);
                        Some(assessment)
                    }
                    _ => None,
                };

                let device_id = shared_config
                    .read()
                    .unwrap()
                    .effective_config
                    .device_id
                    .clone();
                let health_payload = DeviceHealthSnapshot {
                    device_id: device_id.clone(),
                    free_heap: get_free_heap(),
                    uptime_sec: get_uptime_sec(),
                    rssi: get_wifi_rssi(),
                    health_score_percent,
                    fsm_state_display,
                    log_drop_count,
                    kalman_confidence,
                    matrix_update_count,
                    matrix_is_warm,
                    hestia,
                    timestamp_ms: now_sec * 1000,
                };

                if let Ok(json_string) = serde_json::to_string(&health_payload) {
                    let topic_health = topic_controller_status(&device_id);
                    let _ = client.publish(
                        &topic_health,
                        QoS::AtMostOnce,
                        false,
                        json_string.as_bytes(),
                    );
                }
            }
        }

        thread::sleep(Duration::from_millis(50));
    }
}
