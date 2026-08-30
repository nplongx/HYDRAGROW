# Simulator Phase 4 - Digital-Twin MQTT Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow the simulator to act as a "fake device" by publishing its telemetry via MQTT, allowing the existing backend and frontend to observe it exactly like a real physical controller.

**Architecture:** Create an `MqttBridge` that converts `SensorData` and `OrchestratorEvent`s into properly typed JSON payloads, routing them to the correct MQTT topics specified by `hydragrow-shared::topics`. Update the simulator CI to run an actual Mosquitto broker using Docker for integration testing.

**Tech Stack:** Rust, `rumqttc`, `hydragrow-shared`, Mosquitto (Docker).

---

### Task 1: Add rumqttc dependency and Bridge skeleton

**Files:**
- Modify: `hydragrow-simulator/Cargo.toml`
- Create: `hydragrow-simulator/src/telemetry/mqtt_bridge.rs`
- Modify: `hydragrow-simulator/src/telemetry/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
// hydragrow-simulator/src/telemetry/mqtt_bridge.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_init() {
        let _bridge = MqttBridge::new("test-device", "mqtt://localhost:1883");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hydragrow-simulator && cargo test mqtt_bridge`
Expected: FAIL due to missing types.

- [ ] **Step 3: Write minimal implementation**

```toml
# In hydragrow-simulator/Cargo.toml add:
rumqttc = "0.22"
```

```rust
// hydragrow-simulator/src/telemetry/mqtt_bridge.rs
use rumqttc::{Client, MqttOptions, QoS};
use std::time::Duration;

pub struct MqttBridge {
    device_id: String,
    client: Client,
}

impl MqttBridge {
    pub fn new(device_id: &str, broker_uri: &str) -> Self {
        // Strip mqtt:// and parse host/port (naive for now)
        let host = broker_uri.trim_start_matches("mqtt://").split(':').next().unwrap_or("localhost");
        let mut mqttoptions = MqttOptions::new(format!("sim-{}", device_id), host, 1883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));

        let (client, mut connection) = Client::new(mqttoptions, 10);

        // Spawn background thread to poll connection
        std::thread::spawn(move || {
            for _ in connection.iter() {}
        });

        Self {
            device_id: device_id.to_string(),
            client,
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hydragrow-simulator && cargo test mqtt_bridge`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-simulator/Cargo.toml hydragrow-simulator/src/telemetry/
git commit -m "feat(simulator): add rumqttc dependency and MqttBridge skeleton"
```

### Task 2: Implement Publish methods for Sensors and Events

**Files:**
- Modify: `hydragrow-simulator/src/telemetry/mqtt_bridge.rs`
- Modify: `hydragrow-simulator/src/dispatcher.rs`

- [ ] **Step 1: Write the failing test**

```rust
// hydragrow-simulator/src/telemetry/mqtt_bridge.rs
#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_shared::SensorData;

    #[test]
    fn test_topic_generation() {
        assert_eq!(
            hydragrow_shared::topics::topic_sensors("sim-01"),
            "AGITECH/sim-01/sensors"
        );
    }
}
```

- [ ] **Step 2: Write implementation for publish_sensors and publish_event**

```rust
// In hydragrow-simulator/src/telemetry/mqtt_bridge.rs
use hydragrow_shared::SensorData;
use hydragrow_controller_core::events::OrchestratorEvent;
use hydragrow_shared::topics;

impl MqttBridge {
    pub fn publish_sensors(&mut self, data: &SensorData) {
        let topic = topics::topic_sensors(&self.device_id);
        let payload = serde_json::to_string(data).unwrap();
        let _ = self.client.publish(topic, QoS::AtLeastOnce, false, payload);
    }

    pub fn publish_event(&mut self, event: &OrchestratorEvent) {
        match event {
            OrchestratorEvent::PublishFsmState(state) => {
                let topic = topics::topic_fsm_state(&self.device_id);
                let payload = serde_json::to_string(state).unwrap();
                let _ = self.client.publish(topic, QoS::AtLeastOnce, false, payload);
            }
            // Add PublishControllerStatus, PublishDosingReport, etc mapping similarly
            _ => {}
        }
    }
}
```

- [ ] **Step 3: Update SimDispatcher**

Update `SimDispatcher` in `dispatcher.rs` to optionally take a mutable reference to an `MqttBridge` and pass the `Publish*` events to it.

- [ ] **Step 4: Commit**

```bash
git add hydragrow-simulator/src/telemetry/mqtt_bridge.rs hydragrow-simulator/src/dispatcher.rs
git commit -m "feat(simulator): implement payload serialization and MQTT publishing"
```

### Task 3: Add Mosquitto to CI for Integration Testing

**Files:**
- Modify: `.github/workflows/simulator-ci.yml`
- Create: `hydragrow-simulator/tests/mqtt_integration.rs`

- [ ] **Step 1: Update CI workflow to start mosquitto service**

```yaml
# Add to .github/workflows/simulator-ci.yml under `jobs.test.steps` before `cargo test`:
      - name: Start Mosquitto
        run: docker run -d -p 1883:1883 eclipse-mosquitto:2
```

- [ ] **Step 2: Write integration test**

```rust
// hydragrow-simulator/tests/mqtt_integration.rs
use rumqttc::{Client, MqttOptions, QoS, Event, Packet};
use std::time::Duration;
use hydragrow_simulator::telemetry::mqtt_bridge::MqttBridge;
use hydragrow_shared::SensorData;

#[test]
fn test_mqtt_publish_and_receive() {
    // 1. Subscribe to broker
    let mut mqttoptions = MqttOptions::new("test-subscriber", "localhost", 1883);
    let (mut client, mut connection) = Client::new(mqttoptions, 10);
    client.subscribe("AGITECH/sim-test/sensors", QoS::AtMostOnce).unwrap();

    // 2. Publish using bridge
    let mut bridge = MqttBridge::new("sim-test", "mqtt://localhost:1883");
    let data = SensorData::default();
    bridge.publish_sensors(&data);

    // 3. Wait for message and verify
    let mut received = false;
    for notification in connection.iter() {
        if let Ok(Event::Incoming(Packet::Publish(p))) = notification {
            assert_eq!(p.topic, "AGITECH/sim-test/sensors");
            received = true;
            break;
        }
    }
    assert!(received);
}
```

- [ ] **Step 3: Run integration test**

Run: `docker run -d -p 1883:1883 eclipse-mosquitto:2`
Run: `cd hydragrow-simulator && cargo test --test mqtt_integration`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/simulator-ci.yml hydragrow-simulator/tests/mqtt_integration.rs
git commit -m "test(simulator): add MQTT integration test running against real mosquitto in CI"
```
