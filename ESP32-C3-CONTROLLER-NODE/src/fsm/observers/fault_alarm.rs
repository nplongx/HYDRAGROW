//! FaultAlarmObserver — Phát hiện pattern lỗi từ event stream và phát alarm.
//!
//! Subscribe: SetDosingPump, PublishSystemLog (critical), SaveNvsSnapshot
//! Output: Alert JSON → mqtt_tx khi phát hiện anomaly pattern

use super::ObserverContext;
use crate::fsm::events::OrchestratorEvent;
use hydragrow_shared::log::{
    BasicSystemLogMetadata, LogCategory, SystemLogEvent, UnifiedSystemLog,
};
use log::warn;

pub struct FaultAlarmObserver {
    /// Số lần bơm bật mà chưa có cycle hoàn tất (NvsSnapshot = cycle hoàn tất)
    pub dosing_without_completion: u32,
    /// Ngưỡng cảnh báo: nếu bơm bật > N lần mà không có cycle hoàn tất
    pub pump_threshold: u32,
    /// Timestamp lần alarm cuối — tránh spam
    pub last_alarm_ms: u64,
    /// Khoảng cách tối thiểu giữa 2 alarm (ms)
    pub alarm_cooldown_ms: u64,
    /// Đếm số critical log liên tiếp
    pub consecutive_critical_count: u32,
}

impl FaultAlarmObserver {
    pub fn new() -> Self {
        Self {
            dosing_without_completion: 0,
            pump_threshold: 10,
            last_alarm_ms: 0,
            alarm_cooldown_ms: 300_000, // 5 phút giữa 2 alarm
            consecutive_critical_count: 0,
        }
    }

    pub fn on_event(&mut self, event: &OrchestratorEvent, oc: &ObserverContext<'_>) {
        match event {
            // Bơm bật → tăng counter chờ completion
            OrchestratorEvent::SetDosingPump { on: true, .. } => {
                self.dosing_without_completion = self.dosing_without_completion.saturating_add(1);

                if self.dosing_without_completion > self.pump_threshold {
                    self.fire_alarm(
                        oc,
                        "DOSING_CYCLE_STALL",
                        format!(
                            "Bơm đã bật {} lần nhưng chưa có chu kỳ nào hoàn tất. Khả năng FSM bị kẹt pha.",
                            self.dosing_without_completion
                        ),
                    );
                }
            }

            // Cycle hoàn tất (NVS save = cycle kết thúc bình thường) → reset counter
            OrchestratorEvent::SaveNvsSnapshot => {
                self.dosing_without_completion = 0;
                self.consecutive_critical_count = 0;
            }

            // Critical log liên tiếp → alarm
            OrchestratorEvent::PublishSystemLog { payload_json } => {
                // Nếu payload là critical level, tăng counter
                if payload_json.contains("\"level\":\"critical\"")
                    || payload_json.contains("\"level\":\"Critical\"")
                {
                    self.consecutive_critical_count =
                        self.consecutive_critical_count.saturating_add(1);

                    if self.consecutive_critical_count >= 3 {
                        self.fire_alarm(
                            oc,
                            "CONSECUTIVE_CRITICAL_LOGS",
                            format!(
                                "{} critical log liên tiếp. Kiểm tra hardware ngay.",
                                self.consecutive_critical_count
                            ),
                        );
                    }
                } else {
                    // Non-critical log → reset streak
                    self.consecutive_critical_count = 0;
                }
            }

            _ => {}
        }
    }

    fn fire_alarm(&mut self, oc: &ObserverContext<'_>, alarm_code: &str, message: String) {
        // Throttle alarm
        if oc.now_ms.saturating_sub(self.last_alarm_ms) < self.alarm_cooldown_ms {
            return;
        }
        self.last_alarm_ms = oc.now_ms;

        warn!("🚨 [ALARM] {} — {}", alarm_code, message);

        let event = SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
            source: alarm_code.to_string(),
            message,
            skip_reason: None,
            cycle_id: None,
        });

        let log = UnifiedSystemLog::critical(
            &oc.config.device_id,
            LogCategory::System,
            alarm_code,
            event,
            oc.now_ms,
        );

        if let Ok(json) = serde_json::to_string(&log) {
            if oc.mqtt_tx.send(json).is_err() {
                warn!(
                    "⚠️ [ALARM] MQTT channel full, alarm dropped: {}",
                    alarm_code
                );
            }
        }
    }
}
