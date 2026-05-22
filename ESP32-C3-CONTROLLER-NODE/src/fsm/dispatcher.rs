//! EventDispatcher — Layer duy nhất được phép chạm hardware, NVS, MQTT.
//! Nhận Vec<OrchestratorEvent> từ TickResult và execute tuần tự.

use std::sync::mpsc::Sender;

use super::events::OrchestratorEvent;
use super::system_context::{NvsSnapshot, SystemContext};
use crate::pump::{PumpController, PumpType, WaterDirection};
use esp_idf_svc::nvs::EspDefaultNvs;
use hydragrow_shared::{self, ControllerConfig};
use log::warn;

pub struct DispatchContext<'a> {
    pub pumps: &'a mut PumpController,
    pub nvs: &'a mut Option<EspDefaultNvs>,
    pub mqtt_tx: &'a Sender<String>,
    pub dosing_report_tx: &'a Sender<String>,
    pub sensor_cmd_tx: &'a Sender<String>,
    pub ctx: &'a SystemContext,
    pub now_sec: u64,
    pub device_id: &'a str,
    pub config: &'a ControllerConfig,
    pub sensors: &'a hydragrow_shared::SensorData,
    pub observers: &'a mut crate::fsm::observer_set::ObserverSet,
}

pub struct EventDispatcher;

impl EventDispatcher {
    /// Thực thi toàn bộ events từ một TickResult.
    /// Thứ tự thực thi: hardware first, persistence second, messaging last.
    pub fn dispatch(events: Vec<OrchestratorEvent>, dc: &mut DispatchContext<'_>) {
        for event in events {
            // 1. Execute hardware/persistence/messaging
            Self::handle_event(event.clone(), dc);

            // 2. Fan-out tới observers (read-only view, không gọi hardware)
            let oc = crate::fsm::observers::ObserverContext {
                ctx: dc.ctx,
                config: dc.config,
                sensors: dc.sensors,
                now_ms: dc.now_sec * 1000,
                mqtt_tx: dc.mqtt_tx,
                dosing_report_tx: dc.dosing_report_tx,
            };
            dc.observers.notify_all(&event, &oc);
        }
    }

    fn handle_event(event: OrchestratorEvent, dc: &mut DispatchContext<'_>) {
        match event {
            // --- HARDWARE: Bơm định lượng ---
            OrchestratorEvent::SetDosingPump {
                pump,
                on,
                pwm_percent,
            } => {
                let pump_type: PumpType = pump.into();
                let result = if pwm_percent == 100 {
                    dc.pumps.set_pump_state(pump_type, on)
                } else {
                    dc.pumps.set_dosing_pump_pwm(pump_type, on, pwm_percent)
                };
                if let Err(e) = result {
                    warn!("⚠️ [DISPATCHER] SetDosingPump error: {:?}", e);
                }
            }

            // --- HARDWARE: Bơm nước ---
            OrchestratorEvent::SetWaterPump { direction } => {
                if let Err(e) = dc.pumps.set_water_pump(direction) {
                    warn!("⚠️ [DISPATCHER] SetWaterPump error: {:?}", e);
                }
            }

            // --- HARDWARE: Van phun sương ---
            OrchestratorEvent::SetMistValve { on } => {
                if let Err(e) = dc.pumps.set_mist_valve(on) {
                    warn!("⚠️ [DISPATCHER] SetMistValve error: {:?}", e);
                }
            }

            // --- HARDWARE: Bơm Osaka ---
            OrchestratorEvent::SetOsakaPump { pwm_percent } => {
                if let Err(e) = dc.pumps.set_osaka_pump_pwm(pwm_percent) {
                    warn!("⚠️ [DISPATCHER] SetOsakaPump error: {:?}", e);
                }
            }
            OrchestratorEvent::StartOsakaSoft { target_pwm_percent } => {
                if let Err(e) = dc.pumps.start_osaka_pump_soft(target_pwm_percent) {
                    warn!("⚠️ [DISPATCHER] StartOsakaSoft error: {:?}", e);
                }
            }

            // --- PERSISTENCE: NVS ---
            OrchestratorEvent::SaveNvsSnapshot => {
                if let Some(flash) = dc.nvs.as_mut() {
                    let snapshot = NvsSnapshot::from_context(dc.ctx, dc.now_sec);
                    match serde_json::to_string(&snapshot) {
                        Ok(serialized) => {
                            let _ = flash.set_str("runtime_snap", &serialized);
                        }
                        Err(e) => warn!("⚠️ [DISPATCHER] NVS serialize error: {:?}", e),
                    }
                }
            }
            OrchestratorEvent::SaveLastWaterChange { timestamp_sec } => {
                if let Some(flash) = dc.nvs.as_mut() {
                    let _ = flash.set_u64("last_w_change", timestamp_sec);
                }
            }

            // --- MESSAGING: MQTT ---
            OrchestratorEvent::PublishFsmState => {}

            OrchestratorEvent::PublishCalibrationUpdate => {}
            OrchestratorEvent::PublishDosingReport { report_json } => {
                let _ = dc.dosing_report_tx.send(report_json);
            }
            OrchestratorEvent::PublishSystemLog { payload_json } => {}

            // --- CONTROL FLOW: Sensor node ---
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
        }
    }
}
// Thêm vào cuối file dispatcher.rs
impl super::system_context::AutoTuner {
    // Placeholder — sẽ xóa ở Task 7
    fn to_string_placeholder(&self) -> String {
        "device_placeholder".to_string()
    }
}
