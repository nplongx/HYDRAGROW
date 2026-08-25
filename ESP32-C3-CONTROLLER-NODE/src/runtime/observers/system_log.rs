//! SystemLogObserver — Tự động ghi structured log cho hardware events và phase transitions.
//!
//! Subscribe: PublishSystemLog (explicit), SetDosingPump (implicit pump log), phase change events
//! Output: UnifiedSystemLog JSON → mqtt_tx

use super::ObserverContext;
use hydragrow_controller_core::core::fsm::events::{DosingPumpTarget, OrchestratorEvent};
use hydragrow_shared::log::{
    emit_basic_system_log, emit_system_log_json, LogCategory, LogLevel, SystemLogRecord,
    UnifiedSystemLog,
};

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
                let _ = oc.mqtt_tx.send(payload_json.clone());
                emit_system_log_json(payload_json);
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
                use crate::hw::WaterDirection;
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
        let log_payload = UnifiedSystemLog::build_basic_log_json_with_ts(
            &oc.config.device_id,
            level.clone(),
            category.clone(),
            title,
            message.clone(),
            None,
            "system_log_observer",
            oc.now_ms,
        );
        let _ = oc.mqtt_tx.send(log_payload);

        emit_basic_system_log(SystemLogRecord {
            device_id: &oc.config.device_id,
            level,
            category,
            title,
            source: "system_log_observer",
            message: &message,
            cycle_id: None,
            timestamp_ms: oc.now_ms,
        });
    }
}
