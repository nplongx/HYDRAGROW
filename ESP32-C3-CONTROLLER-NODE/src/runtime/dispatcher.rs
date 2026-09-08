// src/runtime/dispatcher.rs
//! EventDispatcher — Thực thi toàn bộ side-effects (Hardware, Flash, MQTT).

use esp_idf_svc::nvs::EspDefaultNvs;
use hydragrow_controller_core::WaterDirection;
use hydragrow_shared::fsm::FaultCode;
use hydragrow_shared::ControllerConfig;
use std::sync::mpsc::Sender;
use tracing::warn;

use crate::hw::pump_controller::{PumpController, PumpType};
use crate::runtime::observers::{ObserverContext, ObserverSet};
use hydragrow_controller_core::core::fsm::context::{NvsSnapshot, SystemContext};
use hydragrow_controller_core::core::fsm::events::OrchestratorEvent;

pub struct DispatchContext<'a, 'd> {
    pub pumps: &'a mut PumpController<'d>,
    pub nvs: &'a mut Option<EspDefaultNvs>,
    pub mqtt_tx: &'a Sender<String>,
    pub dosing_report_tx: &'a Sender<String>,
    pub sensor_cmd_tx: &'a Sender<String>,
    pub ctx: &'a SystemContext,
    pub now_sec: u64,
    pub device_id: &'a str,
    pub config: &'a ControllerConfig,
    pub observers: &'a mut ObserverSet,
}

pub struct EventDispatcher;

impl EventDispatcher {
    pub fn dispatch(
        events: Vec<OrchestratorEvent>,
        dc: &mut DispatchContext<'_, '_>,
    ) -> Option<FaultCode> {
        let has_terminal = events.iter().any(|e| {
            matches!(
                e,
                OrchestratorEvent::RebootDevice | OrchestratorEvent::FactoryReset
            )
        });
        let mut first_fault = None;

        for event in events {
            if let Some(fault) = Self::handle_event(event.clone(), dc) {
                warn!(
                    "🚨 [DISPATCHER] Actuator command failed with fault {:?}, aborting remaining events",
                    fault
                );
                if first_fault.is_none() {
                    first_fault = Some(fault);
                }
                if !has_terminal {
                    return Some(fault);
                }
            }

            let oc = ObserverContext {
                ctx: dc.ctx,
                config: dc.config,
                now_ms: dc.now_sec * 1000,
                mqtt_tx: dc.mqtt_tx,
                dosing_report_tx: dc.dosing_report_tx,
            };
            dc.observers.notify_all(&event, &oc);
        }
        first_fault
    }

    pub fn dispatch_best_effort_all_off(
        events: Vec<OrchestratorEvent>,
        dc: &mut DispatchContext<'_, '_>,
    ) -> Option<FaultCode> {
        dc.pumps.invalidate_water_direction_cache();
        let mut shutdown_fault = None;
        for event in events {
            if let Some(fault) = Self::handle_event(event.clone(), dc) {
                warn!(
                    "🚨 [DISPATCHER] Secondary actuator fault during emergency ALL-OFF: {:?}",
                    fault
                );
                if shutdown_fault.is_none() {
                    shutdown_fault = Some(fault);
                }
            }

            let oc = ObserverContext {
                ctx: dc.ctx,
                config: dc.config,
                now_ms: dc.now_sec * 1000,
                mqtt_tx: dc.mqtt_tx,
                dosing_report_tx: dc.dosing_report_tx,
            };
            dc.observers.notify_all(&event, &oc);
        }
        shutdown_fault
    }

    fn handle_event(
        event: OrchestratorEvent,
        dc: &mut DispatchContext<'_, '_>,
    ) -> Option<FaultCode> {
        match event {
            OrchestratorEvent::SetDosingPump {
                pump,
                on,
                pwm_percent,
            } => {
                let pump_type: PumpType = pump.into();
                let res = if pwm_percent == 100 {
                    dc.pumps.set_pump_state(pump_type, on)
                } else {
                    dc.pumps.set_dosing_pump_pwm(pump_type, on, pwm_percent)
                };
                if let Err(e) = res {
                    warn!("⚠️ [DISPATCHER] SetDosingPump error: {:?}", e);
                    let fault = match pump_type {
                        PumpType::NutrientA | PumpType::NutrientB => FaultCode::EcDosingFailed,
                        PumpType::PhUp | PumpType::PhDown => FaultCode::PhDosingFailed,
                    };
                    return Some(fault);
                }
            }
            OrchestratorEvent::SetWaterPump { direction } => {
                if let Err(e) = dc.pumps.set_water_pump(direction) {
                    warn!("⚠️ [DISPATCHER] SetWaterPump error: {:?}", e);
                    dc.pumps.invalidate_water_direction_cache();
                    let fault = match direction {
                        WaterDirection::In => FaultCode::WaterRefillFailed,
                        WaterDirection::Out => FaultCode::WaterDrainFailed,
                        WaterDirection::Stop => FaultCode::EmergencyStop,
                    };
                    return Some(fault);
                }
            }
            OrchestratorEvent::SetMistValve { on } => {
                if let Err(e) = dc.pumps.set_mist_valve(on) {
                    warn!("⚠️ [DISPATCHER] SetMistValve error: {:?}", e);
                    return Some(FaultCode::EmergencyStop);
                }
            }
            OrchestratorEvent::SetMixValve { on } => {
                if let Err(e) = dc.pumps.set_mix_valve(on) {
                    warn!("⚠️ [DISPATCHER] SetMixValve error: {:?}", e);
                    return Some(FaultCode::EmergencyStop);
                }
            }
            OrchestratorEvent::SetOsakaPump { pwm_percent } => {
                if let Err(e) = dc.pumps.set_osaka_pump_pwm(pwm_percent) {
                    warn!("⚠️ [DISPATCHER] SetOsakaPump error: {:?}", e);
                    return Some(FaultCode::EmergencyStop);
                }
            }
            OrchestratorEvent::StartOsakaSoft { target_pwm_percent } => {
                if let Err(e) = dc.pumps.start_osaka_pump_soft(target_pwm_percent) {
                    warn!("⚠️ [DISPATCHER] StartOsakaSoft error: {:?}", e);
                    return Some(FaultCode::EmergencyStop);
                }
            }
            OrchestratorEvent::SaveNvsSnapshot => {
                if let Some(flash) = dc.nvs.as_mut() {
                    let snapshot = NvsSnapshot::from_context(dc.ctx, dc.now_sec);
                    if let Ok(serialized) = serde_json::to_string(&snapshot) {
                        let _ = flash.set_str("runtime_snap", &serialized);
                    }
                }
            }
            OrchestratorEvent::SaveLastWaterChange { timestamp_sec } => {
                if let Some(flash) = dc.nvs.as_mut() {
                    let _ = flash.set_u64("last_w_change", timestamp_sec);
                }
            }
            OrchestratorEvent::SaveCurrentStageIndex { stage_index } => {
                if let Some(flash) = dc.nvs.as_mut() {
                    match stage_index {
                        Some(idx) => {
                            let _ = flash.set_u64("current_stage", idx as u64);
                        }
                        None => {
                            let _ = flash.set_u64("current_stage", u64::MAX);
                        }
                    }
                    let snapshot = NvsSnapshot::from_context(dc.ctx, dc.now_sec);
                    if let Ok(serialized) = serde_json::to_string(&snapshot) {
                        let _ = flash.set_str("runtime_snap", &serialized);
                    }
                }
            }
            OrchestratorEvent::PublishDosingReport { report_json } => {
                let _ = dc.dosing_report_tx.send(report_json);
            }
            OrchestratorEvent::PublishRecipeStageChanged { payload_json } => {
                let _ = dc.mqtt_tx.send(payload_json);
            }
            OrchestratorEvent::PublishCommandRejected { reason, requested } => {
                let wrapper = serde_json::json!({
                    "_mqtt_topic_override": hydragrow_shared::topics::topic_status_suffix(dc.device_id, "osaka_rejected"),
                    "_payload": {
                        "reason": reason,
                        "requested": requested
                    }
                });
                let _ = dc.mqtt_tx.send(wrapper.to_string());
            }
            OrchestratorEvent::RequestSensorForcePublish => {
                let _ = dc.sensor_cmd_tx.send(
                    r#"{"target":"sensor","action":"force_publish","params":{}}"#.to_string(),
                );
            }
            OrchestratorEvent::SetSensorContinuousMode { enabled } => {
                let _ = dc.sensor_cmd_tx.send(format!(
                    r#"{{"target":"sensor","action":"set_continuous","params":{{"state":{}}}}}"#,
                    enabled
                ));
            }
            OrchestratorEvent::PublishFsmTransition {
                from_phase,
                to_phase,
                reason,
                phase_duration_ms,
            } => {
                use hydragrow_shared::telemetry::transition::FsmTransitionEvent;
                use hydragrow_shared::topics::topic_fsm_transition;

                let mut builder = FsmTransitionEvent::builder()
                    .device_id(dc.device_id)
                    .from(from_phase)
                    .to(to_phase)
                    .reason(reason)
                    .timestamp_ms(dc.now_sec * 1000);

                if let Some(dur) = phase_duration_ms {
                    builder = builder.phase_duration_ms(dur);
                }

                if let Ok(transition_event) = builder.try_build() {
                    let wrapper = serde_json::json!({
                        "_mqtt_topic_override": topic_fsm_transition(dc.device_id),
                        "_payload": serde_json::to_value(&transition_event).unwrap_or_default()
                    });
                    let _ = dc.mqtt_tx.send(wrapper.to_string());
                }
            }
            OrchestratorEvent::PublishDosingCycle { cycle_json } => {
                let wrapper = serde_json::json!({
                    "_mqtt_topic_override": hydragrow_shared::topics::topic_dosing_cycle(dc.device_id),
                    "_payload": serde_json::from_str::<serde_json::Value>(&cycle_json).unwrap_or_default()
                });
                let _ = dc.dosing_report_tx.send(wrapper.to_string());
            }
            OrchestratorEvent::TriggerOtaUpdate => {
                let device_id = dc.device_id.to_string();
                let mqtt_tx = dc.mqtt_tx.clone();
                std::thread::Builder::new()
                    .name("ota_thread".to_string())
                    .stack_size(16_000)
                    .spawn(move || {
                        if let Err(e) =
                            crate::hw::ota::perform_ota_update(&device_id, Some(mqtt_tx))
                        {
                            log::error!("❌ [DISPATCHER] Lỗi trong quá trình OTA: {:?}", e);
                        }
                    })
                    .expect("Không thể tạo OTA worker thread");
            }
            // Legacy network-only command, mapped onto the same
            // pending/active transaction primitives so older backends/UIs
            // cannot corrupt active WiFi state. Staged pending applies at
            // the next reboot; see PrepareWifiConfig for the full flow.
            OrchestratorEvent::UpdateWifiList { list } => {
                if let Some(flash) = dc.nvs.as_mut() {
                    let valid = list.sorted_valid();
                    if valid.is_empty() {
                        warn!("⚠️ [DISPATCHER] Ignoring legacy wifi list without a valid SSID.");
                    } else {
                        let staged = hydragrow_shared::WifiCredentialList { candidates: valid };
                        let count = staged.sorted_valid().len();
                        let version = crate::hw::get_active_wifi_version(flash) + 1;
                        match crate::hw::prepare_pending_wifi(flash, &staged, version) {
                            Ok(()) => {
                                let payload = serde_json::json!({
                                    "type": "system_alert", "device_id": dc.device_id, "level": "Success",
                                    "category": "system", "title": "Đã stage WiFi pending (legacy)",
                                    "message": format!("{} SSID staged as pending v{}; áp dụng sau lần khởi động tiếp theo.", count, version),
                                    "timestamp_ms": dc.now_sec * 1000,
                                });
                                let _ = dc.mqtt_tx.send(payload.to_string());
                            }
                            Err(error) => warn!(
                                "⚠️ [DISPATCHER] Cannot stage legacy pending wifi: {:?}",
                                error
                            ),
                        }
                    }
                }
            }
            // Transactional prepare: full validation (incl. version freshness)
            // with NVS, Keep resolution against active, then stage pending.
            // Active wifi_list is never touched here.
            OrchestratorEvent::PrepareWifiConfig { config, version } => {
                if let Some(flash) = dc.nvs.as_mut() {
                    let current = crate::hw::get_active_wifi_version(flash);
                    match hydragrow_shared::wifi_tx::validate_provision_config(&config, current) {
                        Err(reason) => {
                            warn!(
                                "⚠️ [DISPATCHER] Rejecting stale/invalid WiFi provision: {} (version={}).",
                                reason, version
                            );
                        }
                        Ok(()) => {
                            // Read active list for Keep resolution (passwords stay in NVS/RAM).
                            let active = crate::hw::load_active_wifi_list_from_nvs(flash);
                            match hydragrow_shared::wifi_tx::resolve_provision_credentials(
                                &config, &active,
                            ) {
                                Err(reason) => warn!(
                                    "⚠️ [DISPATCHER] Cannot resolve WiFi provision: {}.",
                                    reason
                                ),
                                Ok(list) => {
                                    let count = list.sorted_valid().len();
                                    match crate::hw::prepare_pending_wifi(flash, &list, version) {
                                        Ok(()) => {
                                            // Metadata only — never log credential payloads.
                                            let payload = serde_json::json!({
                                                "type": "system_alert", "device_id": dc.device_id, "level": "Success",
                                                "category": "system", "title": "Đã stage WiFi pending",
                                                "message": format!("{} SSID staged as pending; OTA must commit before boot applies them.", count),
                                                "timestamp_ms": dc.now_sec * 1000,
                                            });
                                            let _ = dc.mqtt_tx.send(payload.to_string());
                                        }
                                        Err(error) => warn!(
                                            "⚠️ [DISPATCHER] Cannot stage pending wifi: {:?}",
                                            error
                                        ),
                                    }
                                }
                            }
                        }
                    }
                }
            }
            OrchestratorEvent::CommitPendingWifiConfig => {
                if let Some(flash) = dc.nvs.as_mut() {
                    if let Err(error) = crate::hw::commit_pending_wifi(flash) {
                        warn!("⚠️ [DISPATCHER] Cannot commit pending wifi: {:?}", error);
                    }
                }
            }
            OrchestratorEvent::RollbackPendingWifiConfig => {
                if let Some(flash) = dc.nvs.as_mut() {
                    if let Err(error) = crate::hw::rollback_pending_wifi(flash) {
                        warn!("⚠️ [DISPATCHER] Cannot roll back pending wifi: {:?}", error);
                    }
                }
            }
            // Password-free provisioning result for the backend delivery table.
            OrchestratorEvent::PublishWifiConfigStatus { version, state } => {
                let ssid_count = dc
                    .nvs
                    .as_mut()
                    .map(crate::hw::count_active_ssids)
                    .unwrap_or_default();
                let status = hydragrow_shared::wifi_tx::WifiConfigStatus::new(
                    dc.device_id,
                    version,
                    &state,
                    ssid_count,
                );
                if let Ok(json) = serde_json::to_string(&status) {
                    let _ = dc.mqtt_tx.send(json);
                }
            }
            // Fleet claim: persist the provisioned logical id, then reboot so
            // all command/status topics use the stored runtime device_id.
            OrchestratorEvent::ProvisionDeviceId { device_id } => {
                let new_id = device_id.trim().to_string();
                if new_id.is_empty() || new_id.len() > 32 {
                    warn!("⚠️ [DISPATCHER] Rejecting invalid provisioned device id.");
                } else if new_id == dc.device_id {
                    log::info!("🆔 [DISPATCHER] Device id already provisioned; no change.");
                } else if let Some(nvs) = dc.nvs.as_mut() {
                    match nvs.set_str(crate::hw::DEVICE_ID_KEY, &new_id) {
                        Ok(()) => {
                            let payload = serde_json::json!({
                                "type": "system_alert", "device_id": dc.device_id, "level": "Success",
                                "category": "system", "title": "Đã gán device id mới",
                                "message": format!("Provisioned as {new_id}; rebooting..."),
                                "timestamp_ms": dc.now_sec * 1000,
                            });
                            let _ = dc.mqtt_tx.send(payload.to_string());
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            unsafe {
                                esp_idf_svc::sys::esp_restart();
                            }
                        }
                        Err(error) => warn!(
                            "⚠️ [DISPATCHER] Cannot persist provisioned device id: {:?}",
                            error
                        ),
                    }
                }
            }
            OrchestratorEvent::RebootDevice => {
                log::info!("🔄 [DISPATCHER] Thực hiện reboot...");
                std::thread::sleep(std::time::Duration::from_millis(200));
                unsafe {
                    esp_idf_svc::sys::esp_restart();
                }
            }
            OrchestratorEvent::FactoryReset => {
                log::warn!(
                    "⚠️ [DISPATCHER] Factory Reset: xoá toàn bộ runtime NVS keys và reboot..."
                );
                if let Some(nvs) = dc.nvs.as_mut() {
                    let empty = hydragrow_shared::WifiCredentialList::default();
                    let _ = crate::hw::save_wifi_list(nvs, &empty);
                    let _ = nvs.remove("active_recipe");
                    let _ = nvs.remove("runtime_snap");
                    let _ = nvs.remove("current_stage");
                    let _ = nvs.remove("last_w_change");
                    let _ = nvs.remove("safety_budget");
                    // Factory reset clears pending WiFi + transaction state too.
                    use crate::hw::wifi_store::{
                        WIFI_ACTIVE_VERSION_KEY, WIFI_PENDING_KEY, WIFI_PENDING_TARGET_KEY,
                        WIFI_PENDING_VERSION_KEY, WIFI_TRANSACTION_STATE_KEY,
                    };
                    let _ = nvs.remove(WIFI_PENDING_KEY);
                    let _ = nvs.remove(WIFI_PENDING_VERSION_KEY);
                    let _ = nvs.remove(WIFI_PENDING_TARGET_KEY);
                    let _ = nvs.remove(WIFI_ACTIVE_VERSION_KEY);
                    let _ = nvs.remove(WIFI_TRANSACTION_STATE_KEY);
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
                unsafe {
                    esp_idf_svc::sys::esp_restart();
                }
            }
            _ => {}
        }
        None
    }
}
