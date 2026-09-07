use hydragrow_controller_core::core::fsm::context::SystemContext;
use hydragrow_controller_core::core::fsm::events::OrchestratorEvent;
use hydragrow_shared::SensorData;
use hydragrow_shared::fsm::{FsmBudgets, FsmSnapshot};
use hydragrow_shared::topics;
use rumqttc::{Client, MqttOptions, QoS};
use std::time::Duration;

pub struct MqttBridge {
    device_id: String,
    client: Client,
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

        // Spawn background thread to poll connection
        std::thread::spawn(move || {
            for notification in connection.iter() {
                if let Err(e) = notification {
                    println!("MqttBridge Connection Error: {:?}", e);
                    // Connection iterator usually yields the error and then terminates or reconnects
                }
            }
        });

        Self {
            device_id: device_id.to_string(),
            client,
        }
    }

    pub fn publish_sensors(&mut self, data: &SensorData) {
        let topic = topics::topic_sensors(&self.device_id);
        let payload = serde_json::to_string(data).unwrap();
        let _ = self.client.publish(topic, QoS::AtLeastOnce, false, payload);
    }

    pub fn publish_event(&mut self, event: &OrchestratorEvent, ctx: &SystemContext, uptime_ms: u64) {
        if let OrchestratorEvent::PublishFsmState = event {
            let snapshot = build_fsm_snapshot(ctx, uptime_ms);
            let topic = topics::topic_fsm_state(&self.device_id);
            let payload = serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".to_string());
            let _ = self.client.publish(topic, QoS::AtLeastOnce, false, payload);
        }
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
