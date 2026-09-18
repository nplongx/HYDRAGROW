# P2 Firmware Ownership

Date: 2026-09-18

## Sensor-node ownership

| Implementation | Status | Production authority |
| --- | --- | --- |
| ESP32-C3-SENSOR-NODE | Production sensor-node implementation | Yes |
| ESP32-C3-SENSOR-NODE-RUST | Experimental Rust implementation | No |

The C++ sensor node remains the production implementation for P2. The Rust
sensor node is retained as an experimental implementation and MUST NOT be
treated as a production-compatible replacement until it has an independently
verified firmware contract pipeline and an explicit promotion decision.

Both implementations use the canonical MQTT topic family and payload field
names from schema/contracts/registry.json. No firmware-local protocol or
schema is introduced.

Host/native fixture tests are software verification only. They do not claim
physical HIL.
