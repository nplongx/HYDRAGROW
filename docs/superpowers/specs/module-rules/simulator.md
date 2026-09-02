# Simulator Rules

1. **Host-native only:** This crate runs on the host (Linux/macOS/Windows). Do NOT add dependencies on `esp-idf-sys` or any ESP32-specific hardware crates.
2. **Dependencies:** Production code may depend on host-native libraries required by the simulator boundary (serialization, CLI, logging, MQTT client) and internal core/shared workspace crates (`hydragrow-controller-core`, `hydragrow-shared`). Do NOT depend on `hydragrow-backend` or `ESP32-C3-CONTROLLER-NODE`.
3. **Testing:** All behavior (Plant model, Fault injection, Scenarios) MUST be fully tested via standard `cargo test` unit tests or snapshot tests (`insta`).
