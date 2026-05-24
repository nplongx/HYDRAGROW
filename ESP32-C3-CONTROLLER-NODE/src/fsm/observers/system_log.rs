//! SystemLogObserver — Tự động ghi structured log cho hardware events và phase transitions.
//!
//! Subscribe: PublishSystemLog (explicit), SetDosingPump (implicit pump log), phase change events
//! Output: UnifiedSystemLog JSON → mqtt_tx

use super::ObserverContext;
use crate::fsm::events::{DosingPumpTarget, OrchestratorEvent};
use hydragrow_shared::log::{
    BasicSystemLogMetadata, LogCategory, LogLevel, SystemLogEvent, UnifiedSystemLog,
};
use log::warn;

pub struct SystemLogObserver {
    /// Đếm số pump-on events kể từ boot (dùng để correlate log)
    pub pump_on_count: u32,
}

impl SystemLogObserver {
    pub fn new() -> Self {
        Self { pump_on_count: 0 }
    }

    pub fn on_event(&mut self, event: &OrchestratorEvent, oc: &ObserverContext<'_>) {
        match event {
            // Pass-through: orchestrator đã build log payload đầy đủ
            OrchestratorEvent::PublishSystemLog { payload_json } => {
                if oc.mqtt_tx.send(payload_json.clone()).is_err() {
                    warn!("⚠️ [SYSLOG] MQTT channel full, dropped system log");
                }
            }

            // Implicit log: bơm dosing bật → ghi log tự động
            OrchestratorEvent::SetDosingPump {
                pump,
                on: true,
                pwm_percent,
            } => {
                self.pump_on_count = self.pump_on_count.saturating_add(1);
                let pump_name = match pump {
                    DosingPumpTarget::NutrientA => "Nutrient-A",
                    DosingPumpTarget::NutrientB => "Nutrient-B",
                    DosingPumpTarget::PhUp => "pH-Up",
                    DosingPumpTarget::PhDown => "pH-Down",
                };
                self.send_log(
                    oc,
                    LogLevel::Info,
                    LogCategory::Dosing,
                    "Bơm định lượng bật",
                    format!(
                        "Bơm {} bật ở {}% PWM. Chu kỳ #{}",
                        pump_name, pwm_percent, self.pump_on_count
                    ),
                );
            }

            // Implicit log: bơm nước bật
            OrchestratorEvent::SetWaterPump { direction } => {
                use crate::pump::WaterDirection;
                match direction {
                    WaterDirection::In => {
                        self.send_log(
                            oc,
                            LogLevel::Info,
                            LogCategory::System,
                            "Bơm cấp nước vào",
                            "Bắt đầu cấp nước vào bồn.".to_string(),
                        );
                    }
                    WaterDirection::Out => {
                        self.send_log(
                            oc,
                            LogLevel::Info,
                            LogCategory::System,
                            "Bơm xả nước ra",
                            "Bắt đầu xả nước khỏi bồn.".to_string(),
                        );
                    }
                    WaterDirection::Stop => {} // Không cần log stop thường xuyên
                }
            }

            // Implicit log: save NVS snapshot
            OrchestratorEvent::SaveNvsSnapshot => {
                self.send_log(
                    oc,
                    LogLevel::Info,
                    LogCategory::System,
                    "Lưu trạng thái Flash NVS",
                    format!(
                        "Runtime snapshot đã được lưu. Matrix updates: {}, Warm: {}",
                        oc.ctx.tuner.matrix_update_count, oc.ctx.tuner.matrix_is_warm,
                    ),
                );
            }

            _ => {}
        }
    }

    fn send_log(
        &self,
        oc: &ObserverContext<'_>,
        level: LogLevel,
        category: LogCategory,
        title: &str,
        message: String,
    ) {
        let ts = oc.now_ms;
        let event = SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
            source: "system_log_observer".to_string(),
            message,
            skip_reason: None,
            cycle_id: None,
        });

        let log = match level {
            LogLevel::Info => {
                UnifiedSystemLog::info(&oc.config.device_id, category, title, event, ts)
            }
            LogLevel::Warning => {
                UnifiedSystemLog::warning(&oc.config.device_id, category, title, event, ts)
            }
            LogLevel::Critical => {
                UnifiedSystemLog::critical(&oc.config.device_id, category, title, event, ts)
            }
            LogLevel::Success => {
                UnifiedSystemLog::success(&oc.config.device_id, category, title, event, ts)
            }
        };

        if let Ok(json) = serde_json::to_string(&log) {
            if oc.mqtt_tx.send(json).is_err() {
                warn!("⚠️ [SYSLOG] MQTT channel full in observer");
            }
        }
    }
}
