//! MqttTelemetryObserver — Gửi telemetry lên MQTT sau mỗi cycle hoàn tất.
//!
//! Subscribe: PublishFsmState, PublishCalibrationUpdate
//! Output: JSON payload lên topic fsm_state và calibration

use super::ObserverContext;
use crate::{core::fsm::events::OrchestratorEvent, runtime::build_status_msg};
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

        let payload = oc.ctx.tuner.to_mqtt_payload(
            &oc.config.device_id,
            oc.config,
            oc.ctx.diagnostic.adaptive_mixing_sec,
            oc.ctx.diagnostic.adaptive_stabilize_sec,
            oc.now_ms,
        );

        let topic = topic_calibration(&oc.config.device_id);
        if oc.mqtt_tx.send(payload).is_err() {
            warn!("⚠️ [TELEMETRY] MQTT channel full, dropped calibration publish");
        }
        let _ = topic;

        self.publish_count = self.publish_count.saturating_add(1);
        self.last_publish_ms = oc.now_ms;
    }
}
