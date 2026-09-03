use actix_web::web;
use serde_json::json;
use tracing::{debug, error, instrument};

use crate::AppState;
use crate::db::influx::write_sensor_data;
use crate::models::sensor::{PumpStatus, SensorData};
use hydragrow_shared::events::AppEvent;

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let incoming: SensorData = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!(error = ?e, "Lỗi parse JSON SensorData");
            return;
        }
    };

    let time = incoming.time.clone();

    let mut sensor_data = SensorData {
        device_id: device_id.clone(),
        temp: incoming.temp,
        ec: incoming.ec,
        ph: incoming.ph,
        water_level: incoming.water_level,
        pump_status: incoming.pump_status,
        time,
        controller_received_ms: incoming.controller_received_ms,
        rssi: incoming.rssi,
        free_heap: incoming.free_heap,
        uptime: incoming.uptime,
        err_water: incoming.err_water,
        err_temp: incoming.err_temp,
        err_ph: incoming.err_ph,
        err_ec: incoming.err_ec,
        is_continuous: incoming.is_continuous,
        ph_voltage_mv: incoming.ph_voltage_mv,
    };

    debug!(
        "Nhận dữ liệu cảm biến: ph={:.2}, ec={:.2}",
        sensor_data.ph, sensor_data.ec
    );

    if let Some(ph_voltage_mv) = incoming.ph_voltage_mv {
        let observed_at = chrono::DateTime::parse_from_rfc3339(&sensor_data.time)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        let mut sample_map = app_state.ph_voltage_samples.write().await;
        let samples = sample_map.entry(device_id.clone()).or_default();
        samples.push_back(crate::PhVoltageSample {
            voltage_mv: ph_voltage_mv,
            observed_at,
            received_at: std::time::Instant::now(),
        });

        while samples
            .front()
            .is_some_and(|sample| sample.received_at.elapsed().as_secs() > 120)
        {
            samples.pop_front();
        }
    }

    let cached_state = {
        let states = app_state.device_states.read().await;
        states
            .get(&device_id)
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
    };
    if let Some(cached_pump_status) = cached_state
        .as_ref()
        .and_then(|cached| cached.get("pump_status"))
        .and_then(|value| serde_json::from_value::<PumpStatus>(value.clone()).ok())
    {
        sensor_data.pump_status = cached_pump_status;
    }
    let merged_state = merge_sensor_state_cache(cached_state.clone(), &sensor_data);
    if let Ok(json_str) = serde_json::to_string(&merged_state) {
        let mut states = app_state.device_states.write().await;
        states.insert(device_id.clone(), json_str);
    }

    if let Err(e) = write_sensor_data(
        &app_state.influx_client,
        &app_state.influx_bucket,
        &sensor_data,
    )
    .await
    {
        error!(error = ?e, "Lỗi lưu SensorData vào InfluxDB");
    }

    let _ = app_state
        .event_bus
        .send(AppEvent::SensorUpdate(sensor_data));

    // --- Rhai script eval (unified flow chain) ---
    let alert_scripts = app_state.script_cache.get_alert_scripts(&device_id).await;
    let action_scripts = app_state
        .script_cache
        .get_action_command_scripts(&device_id)
        .await;

    if !alert_scripts.is_empty() || !action_scripts.is_empty() {
        let current_phase = cached_state
            .as_ref()
            .and_then(|cached| {
                cached
                    .get("fsm_state")
                    .or_else(|| cached.get("fsm_phase"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("Monitoring")
            .to_string();

        let timestamp_ms = chrono::Utc::now().timestamp_millis();
        let snapshot = crate::models::script::SensorSnapshot {
            ph: incoming.ph,
            ec: incoming.ec,
            temp: incoming.temp,
            water_level: incoming.water_level,
            phase: current_phase,
            device_id: device_id.clone(),
            timestamp_ms,
        };

        let mut chain_nodes: Vec<crate::mqtt::handlers::script_eval::ChainNode> = Vec::new();
        for s in alert_scripts {
            chain_nodes.push(crate::mqtt::handlers::script_eval::ChainNode {
                id: s.id,
                kind: crate::models::script::ScriptKind::Alert,
                next_flow_ids: s.next_flow_ids,
                ast: s.ast,
                ir_json: s.ir_json,
            });
        }
        for s in action_scripts {
            chain_nodes.push(crate::mqtt::handlers::script_eval::ChainNode {
                id: s.id,
                kind: crate::models::script::ScriptKind::ActionCommand,
                next_flow_ids: s.next_flow_ids,
                ast: s.ast,
                ir_json: s.ir_json,
            });
        }

        let engine = std::sync::Arc::new(crate::services::script_engine::ScriptEngine::new());
        let results = crate::mqtt::handlers::script_eval::eval_flow_chain(
            &engine,
            &chain_nodes,
            &snapshot,
            &device_id,
            &app_state.influx_client,
            &app_state.influx_bucket,
        )
        .await;

        if !results.is_empty() {
            let has_action_commands = results.iter().any(|(_, r)| {
                matches!(
                    r,
                    crate::mqtt::handlers::script_eval::ChainFireResult::ActionCommand(_)
                )
            });

            let safety_ctx = if has_action_commands {
                if let Ok(safety_config) =
                    crate::db::postgres::get_safety_config(&app_state.pg_pool, &device_id).await
                {
                    let calibration = crate::db::postgres::fetch_dosing_calibration(
                        &app_state.pg_pool,
                        &device_id,
                    )
                    .await
                    .unwrap_or(None);
                    let limits = hydragrow_shared::safety::DoseSafetyLimits {
                        max_dose_per_cycle_ml: safety_config.max_dose_per_cycle,
                        max_dose_per_hour_ml: safety_config.max_dose_per_hour,
                        cooldown_sec: safety_config.cooldown_sec as u64,
                    };
                    let now_sec = (timestamp_ms / 1000) as u64;
                    let hourly_history_ml = crate::db::postgres::get_dosing_history_last_hour(
                        &app_state.pg_pool,
                        &device_id,
                    )
                    .await
                    .unwrap_or_default();
                    let last_dose_at_sec =
                        crate::db::postgres::get_last_dose_at(&app_state.pg_pool, &device_id)
                            .await
                            .unwrap_or(None);
                    Some((
                        limits,
                        calibration,
                        hourly_history_ml,
                        now_sec,
                        last_dose_at_sec,
                    ))
                } else {
                    None
                }
            } else {
                None
            };

            for (script_id, res) in results {
                match res {
                    crate::mqtt::handlers::script_eval::ChainFireResult::Alert(alert) => {
                        let alert_msg =
                            crate::mqtt::handlers::script_eval::alert_output_to_system_alert(
                                alert,
                                &device_id,
                                timestamp_ms,
                            );
                        let db_record = crate::db::postgres::NewSystemEventRecord {
                            device_id: device_id.clone(),
                            level: alert_msg.level.clone(),
                            category: alert_msg.category.clone(),
                            title: alert_msg.title.clone(),
                            message: alert_msg.message.clone(),
                            reason: alert_msg.reason.clone(),
                            metadata: alert_msg.metadata.clone(),
                            timestamp: alert_msg.timestamp as i64,
                        };
                        if let Err(e) =
                            crate::db::postgres::insert_system_event(&app_state.pg_pool, &db_record)
                                .await
                        {
                            tracing::error!(error = ?e, device_id = %device_id, "Lỗi persist script alert vào DB");
                        }
                        let _ = app_state
                            .event_bus
                            .send(AppEvent::SystemAlert(alert_msg.clone()));
                        let level_lower = alert_msg.level.to_lowercase();
                        if level_lower == "warning" || level_lower == "critical" {
                            let tokens = match app_state.fcm_tokens.lock() {
                                Ok(guard) => guard.get(&device_id).cloned().unwrap_or_default(),
                                Err(poisoned) => poisoned
                                    .into_inner()
                                    .get(&device_id)
                                    .cloned()
                                    .unwrap_or_default(),
                            };
                            if !tokens.is_empty() {
                                let title = alert_msg.title.clone();
                                let message = alert_msg.message.clone();
                                tokio::spawn(async move {
                                    crate::services::fcm::send_push_notification(
                                        &title, &message, tokens,
                                    )
                                    .await;
                                });
                            }
                        }
                    }
                    crate::mqtt::handlers::script_eval::ChainFireResult::ActionCommand(output) => {
                        if let Some((
                            ref limits,
                            ref calibration,
                            ref hourly_history_ml,
                            now_sec,
                            last_dose_at_sec,
                        )) = safety_ctx
                            && let Err(err) =
                                crate::services::action_dispatch::dispatch_action_command(
                                    &app_state,
                                    &device_id,
                                    output,
                                    limits,
                                    hourly_history_ml,
                                    now_sec,
                                    last_dose_at_sec,
                                    calibration.as_ref(),
                                )
                                .await
                        {
                            tracing::warn!(
                                script_id = %script_id, device_id = %device_id, error = ?err,
                                "action_command bị chặn hoặc lỗi khi dispatch"
                            );
                        }
                    }
                    crate::mqtt::handlers::script_eval::ChainFireResult::RecipeOverride(_) => {}
                }
            }
        }
    }
}

fn merge_sensor_state_cache(
    existing: Option<serde_json::Value>,
    sensor_data: &SensorData,
) -> serde_json::Value {
    let mut merged = existing.unwrap_or_else(|| json!({ "device_id": sensor_data.device_id }));
    let sensor_json = serde_json::to_value(sensor_data).unwrap_or_else(|_| json!({}));

    if let (Some(merged_obj), Some(sensor_obj)) = (merged.as_object_mut(), sensor_json.as_object())
    {
        for (key, value) in sensor_obj {
            if key == "pump_status" && merged_obj.contains_key("pump_status") {
                continue;
            }
            merged_obj.insert(key.clone(), value.clone());
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::merge_sensor_state_cache;
    use crate::models::sensor::{PumpStatus, SensorData};
    use serde_json::json;

    fn sensor_data() -> SensorData {
        SensorData {
            device_id: "device_001".to_string(),
            ec: 1.2,
            ph: 6.1,
            temp: 25.0,
            water_level: 80.0,
            pump_status: PumpStatus::default(),
            time: "2026-05-28T00:00:00Z".to_string(),
            controller_received_ms: None,
            rssi: None,
            free_heap: None,
            uptime: None,
            err_water: None,
            err_temp: None,
            err_ph: None,
            err_ec: None,
            is_continuous: None,
            ph_voltage_mv: Some(2450.0),
        }
    }

    #[test]
    fn alert_output_to_system_alert_sets_correct_category() {
        use crate::models::script::AlertOutput;
        use crate::mqtt::handlers::script_eval::alert_output_to_system_alert;

        let alert = AlertOutput {
            level: "warning".to_string(),
            title: "pH cao".to_string(),
            message: "pH = 8.5".to_string(),
        };
        let msg = alert_output_to_system_alert(alert, "device_001", 1234567890);
        assert_eq!(msg.category, "script_alert");
        assert_eq!(msg.device_id, "device_001");
        assert_eq!(msg.level, "warning");
    }

    #[test]
    fn sensors_handler_reads_fsm_phase_from_cache() {
        let existing = json!({
            "device_id": "device_001",
            "fsm_state": "Dosing",
            "fsm_phase": "Dosing"
        });

        let cached_state = Some(existing);
        let current_phase = cached_state
            .as_ref()
            .and_then(|cached| {
                cached
                    .get("fsm_state")
                    .or_else(|| cached.get("fsm_phase"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("Monitoring")
            .to_string();

        assert_eq!(current_phase, "Dosing");
    }

    #[test]
    fn sensor_update_preserves_fsm_pump_status_in_device_cache() {
        let existing = json!({
            "device_id": "device_001",
            "fsm_state": "Monitoring",
            "budgets": { "ec_ml": 2.0, "ph_ml": 1.0 },
            "pump_status": { "pump_a": true, "pump_b": false }
        });

        let merged = merge_sensor_state_cache(Some(existing), &sensor_data());

        assert_eq!(merged["pump_status"]["pump_a"], true);
        assert_eq!(merged["fsm_state"], "Monitoring");
        assert_eq!(merged["budgets"]["ph_ml"], 1.0);
        assert_eq!(merged["ph_voltage_mv"], 2450.0);
    }
}
