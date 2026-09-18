use hydragrow_controller_core::core::fsm::context::SystemContext;
use hydragrow_controller_core::core::fsm::events::OrchestratorEvent;
use hydragrow_controller_core::core::security::verify_signed_json_payload;
use hydragrow_shared::SensorData;
use hydragrow_shared::fsm::{FsmBudgets, FsmSnapshot};
use hydragrow_shared::topics;
use hydragrow_shared::{CommandLifecycle, CommandLifecycleEvent, ControllerConfig, MqttCommandIn};
use rumqttc::{Client, MqttOptions, QoS, SubscribeFilter};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboundMqttMessage {
    pub topic: String,
    pub payload: Vec<u8>,
}

pub struct MqttBridge {
    device_id: String,
    client: Client,
    inbound_rx: Receiver<InboundMqttMessage>,
    connected: Arc<AtomicBool>,
}

/// Builds the real FSM status payload for the `fsm/state` MQTT topic.
/// Mirrors `ESP32-C3-CONTROLLER-NODE/src/runtime/health.rs::build_status_msg`
/// field-for-field, but is implemented independently here because the
/// simulator crate must not depend on the ESP32 crate (module-rules/simulator.md).
pub fn build_fsm_snapshot(ctx: &SystemContext, uptime_ms: u64) -> FsmSnapshot {
    let uptime_sec = uptime_ms / 1000;

    let sum_ml = |pump_name: &str| -> f32 {
        ctx.safety
            .hourly_doses()
            .get(pump_name)
            .map(|hist| {
                hist.iter()
                    .filter(|(ts, _)| uptime_sec.saturating_sub(*ts) <= 3600)
                    .map(|(_, ml)| ml)
                    .sum()
            })
            .unwrap_or(0.0)
    };

    let refill_count = ctx
        .safety
        .refill_history()
        .iter()
        .filter(|ts| uptime_sec.saturating_sub(**ts) <= 3600)
        .count() as u32;

    let drain_count = ctx
        .safety
        .drain_history()
        .iter()
        .filter(|ts| uptime_sec.saturating_sub(**ts) <= 3600)
        .count() as u32;

    FsmSnapshot {
        online: true,
        current_phase: ctx.phase.clone(),
        previous_phase: ctx.previous_phase.clone(),
        pump_status: ctx.peripherals.pump_status.clone(),
        budgets: FsmBudgets {
            ec_ml: sum_ml("NutrientA") + sum_ml("NutrientB"),
            ph_ml: sum_ml("PhUp") + sum_ml("PhDown"),
            refill_count,
            drain_count,
        },
        diagnostics: Some(ctx.diagnostic.clone()),
    }
}

impl MqttBridge {
    pub fn new(device_id: &str, broker_uri: &str) -> Self {
        Self::new_with_subscriptions(device_id, broker_uri, true)
    }

    #[doc(hidden)]
    pub fn new_without_subscriptions(device_id: &str, broker_uri: &str) -> Self {
        Self::new_with_subscriptions(device_id, broker_uri, false)
    }

    fn new_with_subscriptions(
        device_id: &str,
        broker_uri: &str,
        enable_subscriptions: bool,
    ) -> Self {
        // Strip mqtt:// and parse host/port
        let uri = broker_uri.trim_start_matches("mqtt://");
        let mut parts = uri.split(':');
        let host = parts.next().unwrap_or("localhost");
        let port = parts
            .next()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(1883);
        let mut mqttoptions = MqttOptions::new(format!("sim-{}", device_id), host, port);
        mqttoptions.set_keep_alive(Duration::from_secs(5));

        let (client, mut connection) = Client::new(mqttoptions, 10);
        let (inbound_tx, inbound_rx) = mpsc::channel();
        let connected = Arc::new(AtomicBool::new(false));
        let connected_thread = Arc::clone(&connected);
        let subscription_topics = enable_subscriptions.then(|| {
            [
                topics::topic_controller_config(device_id),
                topics::topic_controller_command(device_id),
                topics::topic_controller_recipe(device_id),
                topics::topic_recipe_set(device_id),
                topics::topic_recipe_clear(device_id),
            ]
        });
        let mut subscription_client = client.clone();
        let device_id_for_thread = device_id.to_string();

        // Spawn background thread to poll connection
        std::thread::spawn(move || {
            for notification in connection.iter() {
                match notification {
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                        connected_thread.store(true, Ordering::Release);
                        let Some(subscription_topics) = subscription_topics.as_ref() else {
                            continue;
                        };
                        let subscriptions = subscription_topics
                            .iter()
                            .cloned()
                            .map(|topic| SubscribeFilter {
                                path: topic,
                                qos: QoS::AtLeastOnce,
                            })
                            .collect::<Vec<_>>();
                        if let Err(e) = subscription_client.subscribe_many(subscriptions) {
                            tracing::warn!(?e, device_id = %device_id_for_thread, "MqttBridge subscribe error");
                        }
                    }
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::Disconnect)) => {
                        connected_thread.store(false, Ordering::Release);
                    }
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                        tracing::debug!(device_id = %device_id_for_thread, topic = %publish.topic, "Twin received MQTT publish");
                        let _ = inbound_tx.send(InboundMqttMessage {
                            topic: publish.topic,
                            payload: publish.payload.to_vec(),
                        });
                    }
                    Ok(_) => {}
                    Err(e) => {
                        connected_thread.store(false, Ordering::Release);
                        tracing::warn!(?e, device_id = %device_id_for_thread, "MqttBridge connection error");
                    }
                }
            }
        });

        Self {
            device_id: device_id.to_string(),
            client,
            inbound_rx,
            connected,
        }
    }

    pub fn try_recv(&self) -> Result<InboundMqttMessage, TryRecvError> {
        self.inbound_rx.try_recv()
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    /// Decode the same signed command envelope accepted by the physical controller.
    /// No unsigned-command bypass is provided for the Twin.
    pub fn decode_command(&self, payload: &[u8]) -> anyhow::Result<MqttCommandIn> {
        let secret = std::env::var(format!(
            "MQTT_COMMAND_SECRET_{}",
            self.device_id.to_ascii_uppercase().replace('-', "_")
        ))
        .or_else(|_| std::env::var("MQTT_COMMAND_SECRET"))
        .map_err(|_| anyhow::anyhow!("missing MQTT command signing secret"))?;
        let value = verify_signed_json_payload(&self.device_id, payload, &secret)
            .map_err(|e| anyhow::anyhow!("command signature verification failed: {:?}", e))?;
        Ok(serde_json::from_value(value)?)
    }

    pub fn decode_config(&self, payload: &[u8]) -> anyhow::Result<ControllerConfig> {
        Ok(serde_json::from_slice(payload)?)
    }

    pub fn publish_sensors(&mut self, data: &SensorData) {
        let topic = topics::topic_sensors(&self.device_id);
        let payload = serde_json::to_string(data).unwrap();
        let _ = self.client.publish(topic, QoS::AtLeastOnce, false, payload);
    }

    pub fn publish_event(
        &mut self,
        event: &OrchestratorEvent,
        ctx: &SystemContext,
        uptime_ms: u64,
    ) {
        if let OrchestratorEvent::PublishFsmState = event {
            let snapshot = build_fsm_snapshot(ctx, uptime_ms);
            let topic = topics::topic_fsm_state(&self.device_id);
            let payload = serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".to_string());
            let _ = self.client.publish(topic, QoS::AtLeastOnce, false, payload);
        }
    }

    pub fn publish_command_lifecycle(
        &mut self,
        command_id: String,
        lifecycle: CommandLifecycle,
        reason: Option<String>,
        timestamp_ms: u64,
    ) {
        let event = CommandLifecycleEvent {
            command_id,
            device_id: self.device_id.clone(),
            lifecycle,
            reason,
            timestamp_ms: timestamp_ms as i64,
        };
        let topic = topics::topic_command_lifecycle(&self.device_id);
        if let Ok(payload) = serde_json::to_vec(&event) {
            let _ = self.client.publish(topic, QoS::AtLeastOnce, false, payload);
        }
    }

    /// Publish the same authoritative controller/status contract used by the
    /// physical controller. Extra additive fields are tolerated by the shared
    /// health snapshot decoder and carry the actuator observation used by backend
    /// command reconciliation.
    pub fn publish_raw_controller_status(&mut self, payload: Vec<u8>) {
        let topic = topics::topic_controller_status(&self.device_id);
        let _ = self.client.publish(topic, QoS::AtLeastOnce, false, payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_init() {
        let _bridge = MqttBridge::new("test-device", "mqtt://localhost:1883");
    }

    #[test]
    fn test_topic_generation() {
        assert_eq!(
            hydragrow_shared::topics::topic_sensors("sim-01"),
            "AGITECH/sim-01/sensors"
        );
    }

    #[test]
    fn build_fsm_snapshot_reports_real_phase_and_trailing_hour_budget() {
        use hydragrow_controller_core::core::fsm::context::SystemContext;
        use hydragrow_shared::fsm::SystemPhase;

        let mut ctx = SystemContext {
            phase: SystemPhase::MimoDosing,
            ..Default::default()
        };
        ctx.safety.commit_hourly_dose("NutrientA", 3600, 2.5);
        // Older than the trailing hour relative to now_sec=7200 below -> must be excluded.
        ctx.safety.commit_hourly_dose("NutrientA", 0, 999.0);

        let snapshot = build_fsm_snapshot(&ctx, 7_200_000);

        assert_eq!(snapshot.current_phase, SystemPhase::MimoDosing);
        assert_eq!(snapshot.budgets.ec_ml, 2.5);
        assert!(snapshot.online);
    }
}
