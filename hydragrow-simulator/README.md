# HydraGrow Simulator — Digital Twin

The `hydragrow-simulator` crate provides a host-native digital twin simulation environment for HydraGrow. It couples a first-order virtual hydroponic tank plant model and deterministic fault injection engine directly to the real `hydragrow-controller-core` state machine.

Note: The simulator is a host-native software simulation executed on standard host architectures (`x86_64`, `aarch64`), not an ESP32 hardware emulator.

---

## Architecture & Data Flow

Per tick, `Harness::tick(dt_ms)` executes the following deterministic data flow:

1. **Clock Advance:** Advance owned `SimClock` monotonic time by `dt_ms`.
2. **Fault Injection:** Query `ScenarioEngine::activate_between(prev_ms, current_ms)` and register new fault events in `Injector`.
3. **Actuator Faults:** Apply active hardware fault overrides to `VirtualHardwareState`.
4. **Plant Dynamics:** Advance virtual plant dynamics via `Tank::step(dt_ms, hw, config)`.
5. **Sensor Measurement & Faults:** Read `SensorData` via `read_sensor(&tank, &noise)` and apply active sensor faults (e.g., frozen readings).
6. **Controller FSM Execution:** Call pure FSM handler `orchestrator::tick(...)` using simulated clock timestamp.
7. **Context Mutation:** Apply state delta to `SystemContext`.
8. **Event Dispatch:** Mutate `VirtualHardwareState` based on `OrchestratorEvent`s emitted by the state machine.
9. **Telemetry Output:** Stream sensor readings to MQTT broker (via `MqttBridge`) and append tick records to CSV file (via `Recorder`) when configured.

---

## Usage Commands

Run commands from the `hydragrow-simulator` directory:

```bash
cd hydragrow-simulator

# 1. Execute unit and integration test suite
cargo test

# 2. List available scenario JSON files
cargo run -- scenario-list

# 3. Run default continuous simulation for 100 ticks
cargo run -- run --ticks 100 --tick-ms 1000

# 4. Run a specific fault injection scenario file
cargo run -- run --scenario src/scenario/library/ec_stagnant.json --ticks 15 --tick-ms 1000

# 5. Interactive step REPL
cargo run -- step

# 6. Record simulation telemetry to a CSV file
cargo run -- run --ticks 100 --record /tmp/hydragrow-sim.csv

# 7. Publish telemetry and events over MQTT to a broker
cargo run -- run --ticks 100 --mqtt mqtt://localhost:1883 --device-id sim-01
```
