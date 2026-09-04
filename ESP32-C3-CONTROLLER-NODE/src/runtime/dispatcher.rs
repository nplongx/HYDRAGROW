// src/runtime/dispatcher.rs
//! EventDispatcher — Thực thi toàn bộ side-effects (Hardware, Flash, MQTT).

use esp_idf_svc::nvs::EspDefaultNvs;
use hydragrow_controller_core::WaterDirection;
use hydragrow_shared::ControllerConfig;
use hydragrow_shared::fsm::FaultCode;
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
        let mut first_fault = None;
        for event in events {
            if let Some(fault) = Self::handle_event(event.clone(), dc) {
                if first_fault.is_none() {
                    first_fault = Some(fault);
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
                std::thread::spawn(move || {
                    if let Err(e) = crate::hw::ota::perform_ota_update(&device_id, Some(mqtt_tx)) {
                        log::error!("❌ [DISPATCHER] Lỗi trong quá trình OTA: {:?}", e);
                        // Cân nhắc gửi một MQTT message báo lỗi ở đây
                    }
                });
            }
            OrchestratorEvent::UpdateWifiList { list } => {
                if let Some(flash) = dc.nvs.as_mut() {
                    match crate::hw::save_wifi_list(flash, &list) {
                        Ok(()) => {
                            let payload = serde_json::json!({
                                "type": "system_alert", "device_id": dc.device_id, "level": "Success",
                                "category": "system", "title": "Đã lưu danh sách WiFi mới",
                                "message": format!("{} SSID đã lưu; áp dụng sau lần khởi động tiếp theo.", list.sorted_valid().len()),
                                "timestamp_ms": dc.now_sec * 1000,
                            });
                            let _ = dc.mqtt_tx.send(payload.to_string());
                        }
                        Err(error) => warn!("⚠️ [DISPATCHER] Cannot save wifi_list: {:?}", error),
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
                log::warn!("⚠️ [DISPATCHER] Factory Reset: xoá NVS và reboot...");
                if let Some(nvs) = dc.nvs.as_mut() {
                    let empty = hydragrow_shared::WifiCredentialList::default();
                    let _ = crate::hw::save_wifi_list(nvs, &empty);
                    let _ = nvs.remove("active_recipe");
                    let _ = nvs.remove("safety_budget");
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
