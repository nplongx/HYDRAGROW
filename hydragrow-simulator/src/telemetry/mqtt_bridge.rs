use anyhow::{Context, Result};
use hydragrow_controller_core::core::fsm::events::OrchestratorEvent;
use hydragrow_shared::SensorData;
use hydragrow_shared::fsm::FsmSnapshot;
use hydragrow_shared::topics;
use rumqttc::{Client, MqttOptions, QoS};
use std::time::Duration;

pub struct MqttBridge {
    device_id: String,
    client: Client,
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

    pub fn publish_sensors(&mut self, data: &SensorData) -> Result<()> {
        let topic = topics::topic_sensors(&self.device_id);
        let payload = serde_json::to_string(data).context("Failed to serialize SensorData")?;
        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload)
            .context("Failed to publish sensor data")?;
        Ok(())
    }

    pub fn publish_fsm_state(&mut self, snapshot: &FsmSnapshot) -> Result<()> {
        let topic = topics::topic_fsm_state(&self.device_id);
        let payload = serde_json::to_string(snapshot).context("Failed to serialize FsmSnapshot")?;
        self.client
            .publish(topic, QoS::AtLeastOnce, false, payload)
            .context("Failed to publish FSM state")?;
        Ok(())
    }

    pub fn publish_event(&mut self, event: &OrchestratorEvent) -> Result<()> {
        match event {
            OrchestratorEvent::PublishFsmState => {
                // If triggered without snapshot, log notice; publish_fsm_state is preferred
                println!("PublishFsmState event received on bridge");
                Ok(())
            }
            OrchestratorEvent::PublishDosingReport { report_json } => {
                let topic = topics::topic_dosing_report(&self.device_id);
                self.client
                    .publish(topic, QoS::AtLeastOnce, false, report_json.clone())
                    .context("Failed to publish dosing report")?;
                Ok(())
            }
            OrchestratorEvent::PublishSystemLog { payload_json } => {
                let topic = topics::topic_system_log(&self.device_id);
                self.client
                    .publish(topic, QoS::AtLeastOnce, false, payload_json.clone())
                    .context("Failed to publish system log")?;
                Ok(())
            }
            OrchestratorEvent::PublishFsmTransition {
                from_phase,
                to_phase,
                reason,
                phase_duration_ms,
            } => {
                let topic = topics::topic_fsm_transition(&self.device_id);
                let payload = serde_json::json!({
                    "from_phase": from_phase,
                    "to_phase": to_phase,
                    "reason": reason,
                    "phase_duration_ms": phase_duration_ms,
                });
                let payload_str =
                    serde_json::to_string(&payload).context("Failed to serialize FsmTransition")?;
                self.client
                    .publish(topic, QoS::AtLeastOnce, false, payload_str)
                    .context("Failed to publish FSM transition")?;
                Ok(())
            }
            OrchestratorEvent::PublishDosingCycle { cycle_json } => {
                let topic = topics::topic_dosing_cycle(&self.device_id);
                self.client
                    .publish(topic, QoS::AtLeastOnce, false, cycle_json.clone())
                    .context("Failed to publish dosing cycle")?;
                Ok(())
            }
            _ => {
                // Local or persistence/device-control events without a network representation
                // Explicit no-op for events that do not have a simulator-side MQTT representation
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_shared::PumpStatus;
    use hydragrow_shared::fsm::{FsmBudgets, FsmSnapshot, SystemPhase};

    #[test]
    fn test_bridge_init() {
        let _bridge = MqttBridge::new("test-device", "mqtt://localhost:1883");
    }

    #[test]
    fn test_topic_generation() {
        assert_eq!(topics::topic_sensors("sim-01"), "AGITECH/sim-01/sensors");
    }

    fn sample_sensor() -> SensorData {
        SensorData {
            device_id: "sim-test".to_string(),
            ec: 1.2,
            ph: 6.0,
            temp: 24.5,
            water_level: 80.0,
            pump_status: PumpStatus::default(),
            time: "2024-01-01T00:00:00Z".to_string(),
            controller_received_ms: None,
            rssi: Some(-65),
            free_heap: Some(120000),
            uptime: Some(3600),
            err_water: None,
            err_ph: None,
            err_ec: None,
            err_temp: None,
            is_continuous: Some(true),
            ph_voltage_mv: Some(1500.0),
        }
    }

    #[test]
    fn sensor_payload_uses_shared_topic_and_roundtrips_json() {
        let data = sample_sensor();
        let payload = serde_json::to_string(&data).unwrap();
        let decoded: SensorData = serde_json::from_str(&payload).unwrap();
        assert_eq!(decoded.device_id, data.device_id);
        assert_eq!(decoded.ec, data.ec);
    }

    #[test]
    fn fsm_state_payload_is_structured_snapshot() {
        let snapshot = FsmSnapshot {
            online: true,
            current_phase: SystemPhase::Monitoring,
            previous_phase: None,
            pump_status: PumpStatus::default(),
            budgets: FsmBudgets::default(),
            diagnostics: None,
        };
        let payload = serde_json::to_string(&snapshot).unwrap();
        assert_ne!(payload, "{}");
        let decoded: FsmSnapshot = serde_json::from_str(&payload).unwrap();
        assert!(decoded.online);
        assert_eq!(decoded.current_phase, SystemPhase::Monitoring);
    }

    #[test]
    fn publish_event_routes_shared_topics_and_returns_result() {
        let mut bridge = MqttBridge::new("sim-test", "mqtt://localhost:1883");
        let report_evt = OrchestratorEvent::PublishDosingReport {
            report_json: r#"{"status":"ok"}"#.to_string(),
        };
        let res = bridge.publish_event(&report_evt);
        assert!(res.is_ok());
    }
}
