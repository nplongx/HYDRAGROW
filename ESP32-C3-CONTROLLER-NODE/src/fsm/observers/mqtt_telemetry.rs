//! MqttTelemetryObserver — Gửi telemetry lên MQTT sau mỗi cycle hoàn tất.
//!
//! Subscribe: PublishFsmState, PublishCalibrationUpdate
//! Output: JSON payload lên topic fsm_state và calibration

use super::ObserverContext;
use crate::fsm::{build_status_msg, events::OrchestratorEvent};
use hydragrow_shared::topics::{topic_calibration, topic_fsm_state};
use log::warn;

pub struct MqttTelemetryObserver {
    /// Đếm số lần telemetry đã publish kể từ boot
    pub publish_count: u64,
    /// Timestamp lần publish cuối (ms)
    pub last_publish_ms: u64,
    /// Throttle tối thiểu giữa hai lần publish cùng loại (ms)
    pub min_interval_ms: u64,
}

impl MqttTelemetryObserver {
    pub fn new() -> Self {
        Self {
            publish_count: 0,
            last_publish_ms: 0,
            min_interval_ms: 1_000, // Tối thiểu 1 giây giữa hai publish
        }
    }

    pub fn on_event(&mut self, event: &OrchestratorEvent, oc: &ObserverContext<'_>) {
        match event {
            OrchestratorEvent::PublishFsmState => {
                self.publish_fsm_state(oc);
            }
            OrchestratorEvent::PublishCalibrationUpdate => {
                self.publish_calibration(oc);
            }
            // Các event khác bỏ qua
            _ => {}
        }
    }

    fn publish_fsm_state(&mut self, oc: &ObserverContext<'_>) {
        if oc.now_ms.saturating_sub(self.last_publish_ms) < self.min_interval_ms {
            return; // Throttle
        }

        let now_sec = oc.now_ms / 1000;

        let payload = build_status_msg(oc.ctx, now_sec);

        // let sum_ml = |pump_name: &str| -> f32 {
        //     oc.ctx
        //         .safety
        //         .hourly_doses()
        //         .get(pump_name)
        //         .map(|hist| {
        //             hist.iter()
        //                 .filter(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600)
        //                 .map(|(_, ml)| *ml)
        //                 .sum()
        //         })
        //         .unwrap_or(0.0)
        // };
        //
        // let refill_count = oc
        //     .ctx
        //     .safety
        //     .refill_history()
        //     .iter()
        //     .filter(|ts| now_sec.saturating_sub(**ts) <= 3600)
        //     .count();
        //
        // let drain_count = oc
        //     .ctx
        //     .safety
        //     .drain_history()
        //     .iter()
        //     .filter(|ts| now_sec.saturating_sub(**ts) <= 3600)
        //     .count();
        //
        // let payload = serde_json::json!({
        //     "online": true,
        //     "current_state": match &oc.ctx.phase {
        //         SystemPhase::Fault(code) => {
        //             format!("Fault:{}", code.as_str())
        //         }
        //         SystemPhase::EmergencyStop(reason) => {
        //             format!("EmergencyStop:{}", reason)
        //         }
        //         p => p.as_str().to_string(),
        //     },
        //     "pump_status": oc.ctx.peripherals.pump_status,
        //     "budgets": {
        //         "ec_ml": sum_ml("NutrientA") + sum_ml("NutrientB"),
        //         "ph_ml": sum_ml("PhUp") + sum_ml("PhDown"),
        //         "refill_count": refill_count,
        //         "drain_count": drain_count,
        //     },
        //     "log_drop_count": crate::fsm::utils::get_log_drop_count(),
        //     "diagnostics": oc.ctx.diagnostic.to_telemetry_json(),
        // })
        // .to_string();

        let topic = topic_fsm_state(&oc.config.device_id);
        if oc.mqtt_tx.send(payload).is_err() {
            warn!("⚠️ [TELEMETRY] MQTT channel full, dropped fsm_state publish");
        }
        let _ = topic; // topic dùng trong future khi có multi-topic support

        self.publish_count = self.publish_count.saturating_add(1);
        self.last_publish_ms = oc.now_ms;
    }

    fn publish_calibration(&mut self, oc: &ObserverContext<'_>) {
        log::debug!(
            "📡 [TELEMETRY] Publishing calibration: matrix_warm={}, updates={}",
            oc.ctx.tuner.matrix_is_warm,
            oc.ctx.tuner.matrix_update_count
        );

        let payload = oc
            .ctx
            .tuner
            .to_mqtt_payload(&oc.config.device_id, oc.config, oc.now_ms);

        let topic = topic_calibration(&oc.config.device_id);
        if oc.mqtt_tx.send(payload).is_err() {
            warn!("⚠️ [TELEMETRY] MQTT channel full, dropped calibration publish");
        }
        let _ = topic;

        self.publish_count = self.publish_count.saturating_add(1);
        self.last_publish_ms = oc.now_ms;
    }
}
