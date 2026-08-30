use hydragrow_controller_core::core::fsm::events::OrchestratorEvent;
use hydragrow_shared::SensorData;
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

    pub fn publish_sensors(&mut self, data: &SensorData) {
        let topic = topics::topic_sensors(&self.device_id);
        let payload = serde_json::to_string(data).unwrap();
        let _ = self.client.publish(topic, QoS::AtLeastOnce, false, payload);
    }

    pub fn publish_event(&mut self, event: &OrchestratorEvent) {
        if let OrchestratorEvent::PublishFsmState = event {
            let topic = topics::topic_fsm_state(&self.device_id);
            let payload = serde_json::to_string(&serde_json::json!({})).unwrap();
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
}
