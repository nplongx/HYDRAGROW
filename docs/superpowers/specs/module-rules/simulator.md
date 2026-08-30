# Simulator Rules

1. **Host-native only:** This crate runs on the host (Linux/macOS/Windows). Do NOT add dependencies on `esp-idf-sys` or any ESP32-specific hardware crates.
2. **Dependencies:** Can only depend on `hydragrow-controller-core` and `hydragrow-shared`. Do NOT depend on `hydragrow-backend` or `ESP32-C3-CONTROLLER-NODE`.
3. **Testing:** All behavior (Plant model, Fault injection, Scenarios) MUST be fully tested via standard `cargo test` unit tests or snapshot tests (`insta`).
