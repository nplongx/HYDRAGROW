use actix_web::web;
use serde_json::json;
use tracing::{debug, error, instrument};

use crate::AppState;
use crate::db::influx::write_sensor_data;
use crate::models::sensor::SensorData;
use hydragrow_shared::events::AppEvent;
use hydragrow_shared::sensors::IncomingSensorPayload;
use hydragrow_shared::telemetry::AuthoritativeTelemetrySnapshot;

#[instrument(skip(app_state, payload), fields(device_id = %device_id))]
pub async fn handle(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let incoming: IncomingSensorPayload = match serde_json::from_slice(payload) {
        Ok(data) => data,
        Err(e) => {
            error!(error = ?e, "Lỗi parse JSON SensorData");
            return;
        }
    };

    if !incoming.is_valid() {
        error!(device_id = %device_id, "Bỏ qua payload sensor không hợp lệ hoặc không có measurement");
        return;
    }

    let received_at = chrono::Utc::now().to_rfc3339();
    let authoritative = AuthoritativeTelemetrySnapshot::from_incoming_payload(
        &device_id,
        &incoming,
        Some(received_at),
    );

    if let Some(ph_voltage_mv) = incoming.ph_voltage_mv {
        let Some(observed_at) = incoming
            .time
            .as_deref()
            .and_then(|time| chrono::DateTime::parse_from_rfc3339(time).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
        else {
            return;
        };

        let mut sample_map = app_state.ph_voltage_samples.write().await;
        let samples = sample_map.entry(device_id.clone()).or_default();
        samples.push_back(crate::PhVoltageSample {
            voltage_mv: ph_voltage_mv as f64,
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
    let existing_authoritative = cached_state
        .as_ref()
        .and_then(|cached| cached.get("telemetry"))
        .and_then(|value| {
            serde_json::from_value::<AuthoritativeTelemetrySnapshot>(value.clone()).ok()
        });
    let mut merged_authoritative = authoritative.merge_into(existing_authoritative.as_ref());
    merged_authoritative.refresh_operational_state(chrono::Utc::now());

    let mut merged_state = cached_state
        .clone()
        .unwrap_or_else(|| json!({ "device_id": device_id.clone() }));
    if let Some(object) = merged_state.as_object_mut() {
        object.insert("device_id".into(), json!(device_id.clone()));
        object.insert(
            "telemetry".into(),
            serde_json::to_value(&merged_authoritative).unwrap_or_else(|_| json!({})),
        );
        if let Some(value) = incoming.ph {
            object.insert("ph".into(), json!(value));
        }
        if let Some(value) = incoming.ec {
            object.insert("ec".into(), json!(value));
        }
        if let Some(value) = incoming.temp {
            object.insert("temp".into(), json!(value));
        }
        if let Some(value) = incoming.water_level {
            object.insert("water_level".into(), json!(value));
        }
        if let Some(value) = incoming.time.as_ref() {
            object.insert("time".into(), json!(value));
        }
        if let Some(value) = incoming.err_ph {
            object.insert("err_ph".into(), json!(value));
        }
        if let Some(value) = incoming.err_ec {
            object.insert("err_ec".into(), json!(value));
        }
        if let Some(value) = incoming.err_temp {
            object.insert("err_temp".into(), json!(value));
        }
        if let Some(value) = incoming.err_water {
            object.insert("err_water".into(), json!(value));
        }
        if let Some(value) = incoming.ph_voltage_mv {
            object.insert("ph_voltage_mv".into(), json!(value));
        }
    }
    if let Ok(json_str) = serde_json::to_string(&merged_state) {
        let mut states = app_state.device_states.write().await;
        states.insert(device_id.clone(), json_str);
    }

    let has_full_measurement = incoming.temp.is_some()
        && incoming.ec.is_some()
        && incoming.ph.is_some()
        && incoming.water_level.is_some();
    let legacy_sensor = has_full_measurement
        .then(|| serde_json::from_value::<SensorData>(merged_state.clone()).ok())
        .flatten();
    if let Some(sensor_data) = legacy_sensor {
        debug!(
            "Nhận dữ liệu cảm biến: ph={:.2}, ec={:.2}",
            sensor_data.ph, sensor_data.ec
        );
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
    }

    let _ = app_state
        .event_bus
        .send(AppEvent::TelemetrySnapshot(Box::new(merged_authoritative)));

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
        let Some((ph, ec, temp, water_level)) = incoming
            .ph
            .zip(incoming.ec)
            .zip(incoming.temp)
            .zip(incoming.water_level)
            .map(|(((ph, ec), temp), water_level)| (ph, ec, temp, water_level))
        else {
            return;
        };
        let snapshot = crate::models::script::SensorSnapshot {
            ph,
            ec,
            temp,
            water_level,
            phase: current_phase,
            device_id: device_id.clone(),
            timestamp_ms,
            err_ph: incoming.err_ph,
            err_tds: incoming.err_ec,
            err_temperature: incoming.err_temp,
            err_water_level: incoming.err_water,
        };

        let mut chain_nodes: Vec<crate::mqtt::handlers::script_eval::ChainNode> = Vec::new();
        let mut script_names: std::collections::HashMap<uuid::Uuid, String> =
            std::collections::HashMap::new();
        for s in alert_scripts {
            script_names.insert(s.id, s.name.clone());
            chain_nodes.push(crate::mqtt::handlers::script_eval::ChainNode {
                id: s.id,
                name: s.name,
                kind: crate::models::script::ScriptKind::Alert,
                next_flow_ids: s.next_flow_ids,
                ast: s.ast,
                ir_json: s.ir_json,
            });
        }
        for s in action_scripts {
            script_names.insert(s.id, s.name.clone());
            chain_nodes.push(crate::mqtt::handlers::script_eval::ChainNode {
                id: s.id,
                name: s.name,
                kind: crate::models::script::ScriptKind::ActionCommand,
                next_flow_ids: s.next_flow_ids,
                ast: s.ast,
                ir_json: s.ir_json,
            });
        }

        let chain_nodes =
            crate::mqtt::handlers::script_eval::filter_chain_nodes_for_sensor_path(chain_nodes);
        if chain_nodes.is_empty() {
            // Không còn Flow nào thuộc đường sensor trên thiết bị này (toàn bộ
            // là cron/webhook/fsm) — chúng chạy từ đường riêng của chúng.
            return;
        }

        let engine = std::sync::Arc::new(crate::services::script_engine::ScriptEngine::new());
        let results = crate::mqtt::handlers::script_eval::eval_flow_chain(
            &engine,
            &chain_nodes,
            &snapshot,
            &device_id,
            &app_state.influx_client,
            &app_state.influx_bucket,
            &app_state.pg_pool,
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
                match crate::services::safety_data::classify_safety_config(
                    crate::db::postgres::fetch_safety_config(&app_state.pg_pool, &device_id).await,
                ) {
                    Ok(safety_config) => {
                        let calibration = match crate::services::safety_data::classify_calibration(
                            crate::db::postgres::fetch_dosing_calibration(
                                &app_state.pg_pool,
                                &device_id,
                            )
                            .await,
                        ) {
                            Ok(calibration) => calibration,
                            Err(reason) => {
                                crate::metrics::SAFETY_DECISIONS_TOTAL
                                    .with_label_values(&[reason, "denied"])
                                    .inc();
                                tracing::warn!(device_id = %device_id, reason_code = reason, "sensor action denied: dosing calibration unavailable");
                                return;
                            }
                        };
                        let limits = hydragrow_shared::safety::DoseSafetyLimits {
                            max_dose_per_cycle_ml: safety_config.max_dose_per_cycle,
                            max_dose_per_hour_ml: safety_config.max_dose_per_hour,
                            cooldown_sec: safety_config.cooldown_sec as u64,
                        };
                        let now_sec = (timestamp_ms / 1000) as u64;
                        let hourly_history_ml = match crate::services::safety_data::classify_history(
                            crate::db::postgres::get_dosing_history_last_hour(
                                &app_state.pg_pool,
                                &device_id,
                            )
                            .await,
                        ) {
                            Ok(history) => history,
                            Err(reason) => {
                                crate::metrics::SAFETY_DECISIONS_TOTAL
                                    .with_label_values(&[reason, "denied"])
                                    .inc();
                                tracing::warn!(device_id = %device_id, reason_code = reason, "sensor action denied: safety history unavailable");
                                return;
                            }
                        };
                        let last_dose_at_sec =
                            match crate::services::safety_data::classify_last_dose(
                                crate::db::postgres::get_last_dose_at(
                                    &app_state.pg_pool,
                                    &device_id,
                                )
                                .await,
                            ) {
                                Ok(last_dose) => last_dose,
                                Err(reason) => {
                                    crate::metrics::SAFETY_DECISIONS_TOTAL
                                        .with_label_values(&[reason, "denied"])
                                        .inc();
                                    tracing::warn!(device_id = %device_id, reason_code = reason, "sensor action denied: last-dose unavailable");
                                    return;
                                }
                            };
                        Some((
                            limits,
                            Some(calibration),
                            hourly_history_ml,
                            now_sec,
                            last_dose_at_sec,
                        ))
                    }
                    Err(error) => {
                        let reason = error.reason_code();
                        crate::metrics::SAFETY_DECISIONS_TOTAL
                            .with_label_values(&[reason, "denied"])
                            .inc();
                        tracing::warn!(device_id = %device_id, reason_code = reason, "sensor action denied: safety data unavailable");
                        return;
                    }
                }
            } else {
                None
            };

            for (script_id, res) in results {
                crate::db::postgres::touch_script_last_run(&app_state.pg_pool, &script_id).await;
                match res {
                    crate::mqtt::handlers::script_eval::ChainFireResult::Alert(alert) => {
                        let script_name = script_names
                            .get(&script_id)
                            .map(|s| s.as_str())
                            .unwrap_or("unknown");
                        crate::mqtt::handlers::script_eval::handle_fired_alert(
                            &app_state,
                            &script_id,
                            script_name,
                            alert,
                            &device_id,
                            timestamp_ms,
                        )
                        .await;
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn alert_output_to_system_alert_sets_correct_category() {
        use crate::models::script::AlertOutput;
        use crate::mqtt::handlers::script_eval::alert_output_to_system_alert;

        let alert = AlertOutput {
            level: "warning".to_string(),
            title: "pH cao".to_string(),
            message: "pH = 8.5".to_string(),
            notify_fcm: None,
        };
        let script_id = uuid::Uuid::new_v4();
        let msg =
            alert_output_to_system_alert(alert, &script_id, "ph_rule", "device_001", 1234567890);
        assert_eq!(msg.category, "automation");
        assert_eq!(msg.device_id, "device_001");
        assert_eq!(msg.level, "warning");
        let meta = msg.metadata.expect("metadata should be populated");
        assert_eq!(meta["script_id"], script_id.to_string());
        assert_eq!(meta["script_name"], "ph_rule");
    }

    #[test]
    fn sensor_snapshot_carries_err_flags_when_set() {
        let snapshot = crate::models::script::SensorSnapshot {
            ph: 7.0,
            ec: 1.5,
            temp: 25.0,
            water_level: 80.0,
            phase: "Monitoring".to_string(),
            device_id: "test".to_string(),
            timestamp_ms: 0,
            err_ph: Some(true),
            err_tds: None,
            err_temperature: Some(false),
            err_water_level: Some(true),
        };
        assert_eq!(snapshot.err_ph, Some(true));
        assert!(snapshot.err_tds.is_none());
        assert_eq!(snapshot.err_temperature, Some(false));
        assert_eq!(snapshot.err_water_level, Some(true));
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
    fn partial_sensor_payload_does_not_require_or_invent_unrelated_values() {
        let existing = json!({
            "device_id": "device_001",
            "fsm_state": "Monitoring",
            "budgets": { "ec_ml": 2.0, "ph_ml": 1.0 },
            "pump_status": { "pump_a": true, "pump_b": false }
        });
        let payload = serde_json::json!({ "ph": 6.2 });
        assert!(existing.get("pump_status").is_some());
        assert_eq!(payload.get("ec"), None);
        assert_eq!(payload.get("temp"), None);
        assert_eq!(payload.get("water_level"), None);
    }
}
