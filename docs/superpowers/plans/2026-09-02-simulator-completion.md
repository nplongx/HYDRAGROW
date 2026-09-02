# HydraGrow Simulator Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hoàn thiện `hydragrow-simulator` thành digital twin/controller-in-the-loop chạy được bằng CLI, có simulated clock deterministic, virtual hardware phản hồi đúng `OrchestratorEvent`, plant + sensor feedback, scenario/fault scheduling, MQTT/CSV telemetry và test end-to-end đủ để debug FSM thật.

**Architecture:** Giữ `hydragrow-simulator` là crate host-native độc lập, gọi `hydragrow-controller-core::core::fsm::orchestrator::tick()` làm decision engine và `hydragrow-shared` làm nguồn sự thật cho config, sensor types và MQTT topics. Tách các trách nhiệm thành virtual hardware/event mapping, scenario/fault runtime, plant/sensor model, telemetry sinks, harness và CLI; `Harness` sở hữu simulated clock và là nơi nối tất cả các layer theo thứ tự `faults -> plant -> sensor -> FSM -> delta -> events -> sinks`.

**Tech Stack:** Rust 2024, Cargo crate độc lập; `hydragrow-controller-core`, `hydragrow-shared`, `serde`/`serde_json`, `clap`, `rumqttc`, `insta`; host-native only.

---

## Correction trước khi bắt đầu

1. Các roadmap Phase 0–4 trong repo đã được triển khai một phần hoặc toàn bộ, nên plan này **không dựng lại scaffolding**. Tại `main` hiện tại đã có `Cargo.toml`, `harness.rs`, `Tank`, sensor model, scenario format, injector, recorder và MQTT bridge. `main.rs` vẫn chưa wired. 
2. `docs/superpowers/specs/module-rules/simulator.md` hiện nói crate chỉ được phụ thuộc vào `hydragrow-controller-core` và `hydragrow-shared`, trong khi `Cargo.toml` hiện thực tế đang dùng `serde`, `serde_json`, `anyhow`, `clap`, `tracing`, `rumqttc`, và `insta`. Không được phá tính năng hiện có bằng cách xoá các dependency này chỉ để làm tài liệu “khớp”. Foundation task phải sửa rule thành: simulator chỉ được phụ thuộc các crate production host-native cần thiết cho serialization/CLI/MQTT, test tooling; tuyệt đối không phụ thuộc backend hay ESP32 firmware. `Cargo.toml` hiện tại xác nhận các dependency đó. 
3. Roadmap cũ yêu cầu dispatcher match đầy đủ `OrchestratorEvent`; source `hydragrow-controller-core/src/core/fsm/events.rs` hiện có 25+ variant và không `non_exhaustive`, vì vậy event mapping phải dùng exhaustive `match` và không có `_ => {}`.
4. `SensorFrozen` hiện chưa làm gì và `PumpStuckOn/Off` chỉ xử lý `PUMP_A`; scenario engine phải biến fault thành hành vi deterministic, không chỉ parse JSON.
5. `ec_stagnant.json` hiện chỉ chứa `PumpStuckOn(PUMP_A)`; tên scenario không được coi là bằng chứng rằng lỗi “EC stagnant” đã được mô phỏng đầy đủ. Acceptance phải kiểm chứng causal chain từ scenario -> injector -> hardware/measurement -> FSM fault/phase nếu core có logic tương ứng.

## Current State / Evidence

- `main.rs` định nghĩa `run`, `step`, `scenario-list` nhưng `step` chỉ in `Starting step repl...`, `run` chỉ in `Continuous run not fully wired yet`. 
- `Harness::tick()` hiện dùng `SystemTime::now()` cho `now_ms`, tăng `uptime_ms`, gọi `orchestrator::tick()`, dispatch events, nhưng chưa `ctx.apply_delta(...)` trước khi chạy tick kế tiếp; `SimDispatcher` hiện chỉ gửi event tới MQTT bridge. 
- `Tank::step()` đã có EC/pH thay đổi theo PWM nhưng chưa cập nhật volume từ refill/drain và water-level đang thay đổi theo `dt_sec` trực tiếp. 
- `read_sensor()` nhận `NoiseConfig` nhưng cố định `ec_noise = 0.0` và `ph_noise = 0.0`, tức noise đang là API giả. 
- `Recorder` ghi CSV nhưng dùng `unwrap()` cho production path và chưa được Harness/CLI dùng. 
- `MqttBridge` có publish sensors và publish FSM event, nhưng Harness chưa publish sensor data mỗi tick và `PublishFsmState` đang phát payload `{}` thay vì snapshot/status contract thực tế. 
- Chỉ có scenario library `ec_stagnant.json`; CLI đang hard-code thêm `sensor_timeout.json` dù file không tồn tại. 

## File map

**Create**
- `hydragrow-simulator/src/event_dispatcher.rs` — exhaustive translation từ `OrchestratorEvent` sang virtual hardware/telemetry operations.
- `hydragrow-simulator/src/scenario/engine.rs` — scenario loading, deterministic event scheduling, fault activation.
- `hydragrow-simulator/src/simulation.rs` — cấu trúc runtime config/clock/output contract dùng chung cho CLI và integration tests, giữ `main.rs` mỏng.
- `hydragrow-simulator/tests/harness_e2e.rs` — controller-core-compatible full loop tests.
- `hydragrow-simulator/tests/scenario_runtime.rs` — timing/activation tests của scenario engine.
- `hydragrow-simulator/tests/plant_feedback.rs` — feedback tests không phụ thuộc CLI.

**Modify**
- `hydragrow-simulator/Cargo.toml` — chỉ điều chỉnh dependency/features khi test/implementation thực sự cần.
- `hydragrow-simulator/src/lib.rs`
- `hydragrow-simulator/src/dispatcher.rs`
- `hydragrow-simulator/src/actuators/virtual_hw.rs`
- `hydragrow-simulator/src/faults/injector.rs`
- `hydragrow-simulator/src/harness.rs`
- `hydragrow-simulator/src/plant/tank.rs`
- `hydragrow-simulator/src/sensors/sensor_model.rs`
- `hydragrow-simulator/src/scenario/format.rs`
- `hydragrow-simulator/src/telemetry/recorder.rs`
- `hydragrow-simulator/src/telemetry/mqtt_bridge.rs`
- `hydragrow-simulator/src/main.rs`
- `docs/superpowers/specs/module-rules/simulator.md`
- `docs/superpowers/specs/module-rules/README.md` only if the simulator verification command changes.

**Tests/snapshots**
- `hydragrow-simulator/tests/harness_e2e.rs`
- `hydragrow-simulator/tests/scenario_runtime.rs`
- `hydragrow-simulator/tests/plant_feedback.rs`
- `hydragrow-simulator/tests/mqtt_integration.rs`
- `hydragrow-simulator/tests/snapshot_dosing.rs`
- `hydragrow-simulator/tests/snapshots/*` when explicitly reviewed.

---

### Task 1: Reconcile simulator rules and freeze the baseline

**Files:**
- Modify: `docs/superpowers/specs/module-rules/simulator.md`
- Modify: `docs/superpowers/specs/module-rules/README.md` only if the command text needs correction
- Test: `hydragrow-simulator/Cargo.toml` via existing crate checks; no source code change in this task.

- [ ] **Step 1: Record the real dependency contract**

Replace the dependency rule with the following exact policy:

```markdown
2. **Dependencies:** Production code may depend on host-native libraries required by the simulator boundary (serialization, CLI, tracing, MQTT, and deterministic test support) in addition to `hydragrow-controller-core` and `hydragrow-shared`. It MUST NOT depend on `hydragrow-backend`, `ESP32-C3-CONTROLLER-NODE`, `esp-idf-*`, or embedded-only hardware crates.
```

- [ ] **Step 2: Run the current simulator gate before implementation**

Run:

```bash
cd hydragrow-simulator
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Expected: capture the current result; this is the baseline, not a new acceptance criterion. Any pre-existing failure must be listed in the implementation PR before being fixed.

- [ ] **Step 3: Commit only the rule correction**

```bash
git add docs/superpowers/specs/module-rules/simulator.md docs/superpowers/specs/module-rules/README.md
git commit -m "docs(simulator): reconcile host-native dependency rules"
```

---

## Parallel Execution Lanes

After Task 1 is merged into `main`, split the independent implementation tasks by actual file overlap:

| Lane | Branch | Tasks | Owns |
|---|---|---|---|
| `lane/virtual-hardware` | `feat/simulator-virtual-hardware` | Task 2 | `virtual_hw.rs`, `event_dispatcher.rs`, `dispatcher.rs`, their unit tests |
| `lane/plant-sensor` | `feat/simulator-plant-sensor` | Task 3 | `tank.rs`, `sensor_model.rs`, plant/sensor tests |
| `lane/scenario-faults` | `feat/simulator-scenario-faults` | Task 4 | `scenario/format.rs`, `scenario/engine.rs`, `faults/injector.rs`, scenario tests |
| `lane/telemetry` | `feat/simulator-telemetry` | Task 5 | `recorder.rs`, `mqtt_bridge.rs`, telemetry integration tests |

Do **not** start the Harness/CLI lane until these four lanes have been merged serially into `main`, because `Harness` is the integration point that touches all four boundaries. Create each lane with a separate worktree, and never run two sessions in the same checkout:

```bash
git worktree add ../HYDRAGROW-lane-virtual-hardware -b feat/simulator-virtual-hardware main
git worktree add ../HYDRAGROW-lane-plant-sensor -b feat/simulator-plant-sensor main
git worktree add ../HYDRAGROW-lane-scenario-faults -b feat/simulator-scenario-faults main
git worktree add ../HYDRAGROW-lane-telemetry -b feat/simulator-telemetry main
```

Merge serially from the integration checkout:

```bash
git merge feat/simulator-virtual-hardware
cd hydragrow-simulator && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test

git merge feat/simulator-plant-sensor
cd hydragrow-simulator && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test

git merge feat/simulator-scenario-faults
cd hydragrow-simulator && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test

git merge feat/simulator-telemetry
cd hydragrow-simulator && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

Remove each merged worktree immediately after a green merge:

```bash
git worktree remove ../HYDRAGROW-lane-virtual-hardware
git branch -d feat/simulator-virtual-hardware
```

Repeat for the other three lanes.

---

### Task 2: Exhaustive virtual hardware event dispatcher

**Files:**
- Create: `hydragrow-simulator/src/event_dispatcher.rs`
- Modify: `hydragrow-simulator/src/actuators/virtual_hw.rs`
- Modify: `hydragrow-simulator/src/dispatcher.rs`
- Modify: `hydragrow-simulator/src/lib.rs`
- Test: inline unit tests in `event_dispatcher.rs` and `virtual_hw.rs`

- [ ] **Step 1: Write failing tests for the hardware contract**

```rust
#[test]
fn set_dosing_pump_updates_target_and_pwm() {
    let mut hw = VirtualHardwareState::default();
    apply_event(
        &mut hw,
        &OrchestratorEvent::SetDosingPump {
            pump: DosingPumpTarget::NutrientA,
            on: true,
            pwm_percent: 65,
        },
    );
    assert!(hw.pump_a.on);
    assert_eq!(hw.pump_a.pwm_percent, 65);
}

#[test]
fn set_water_pump_updates_only_selected_direction() {
    let mut hw = VirtualHardwareState::default();
    apply_event(&mut hw, &OrchestratorEvent::SetWaterPump { direction: WaterDirection::In });
    assert!(hw.water_pump_in.on);
    assert!(!hw.water_pump_out.on);
}
```

Add equivalent tests for `NutrientB`, `PhUp`, `PhDown`, `SetMistValve`, `SetMixValve`, `SetOsakaPump`, and `StartOsakaSoft`.

- [ ] **Step 2: Run the focused tests and verify they fail**

Run:

```bash
cd hydragrow-simulator
cargo test event_dispatcher::tests -- --nocapture
```

Expected: FAIL because `apply_event` does not yet exist and `VirtualHardwareState` does not expose the final field semantics.

- [ ] **Step 3: Implement exhaustive event mapping**

Use an explicit `match` over every current `OrchestratorEvent` variant:

```rust
pub fn apply_event(hw: &mut VirtualHardwareState, event: &OrchestratorEvent) {
    match event {
        OrchestratorEvent::SetDosingPump { pump, on, pwm_percent } => {
            let target = match pump {
                DosingPumpTarget::NutrientA => &mut hw.pump_a,
                DosingPumpTarget::NutrientB => &mut hw.pump_b,
                DosingPumpTarget::PhUp => &mut hw.pump_ph_up,
                DosingPumpTarget::PhDown => &mut hw.pump_ph_down,
            };
            target.on = *on;
            target.pwm_percent = (*pwm_percent).min(100);
        }
        OrchestratorEvent::SetWaterPump { direction } => {
            hw.water_pump_in.on = matches!(direction, WaterDirection::In);
            hw.water_pump_out.on = matches!(direction, WaterDirection::Out);
        }
        OrchestratorEvent::SetMistValve { on } => hw.mist_valve = *on,
        OrchestratorEvent::SetMixValve { on } => hw.mix_valve = *on,
        OrchestratorEvent::SetOsakaPump { pwm_percent } => {
            hw.osaka_pwm_percent = (*pwm_percent).min(100);
        }
        OrchestratorEvent::StartOsakaSoft { target_pwm_percent } => {
            hw.osaka_pwm_percent = (*target_pwm_percent).min(100);
        }
        OrchestratorEvent::SaveNvsSnapshot
        | OrchestratorEvent::SaveLastWaterChange { .. }
        | OrchestratorEvent::SaveCurrentStageIndex { .. }
        | OrchestratorEvent::PublishFsmState
        | OrchestratorEvent::PublishCalibrationUpdate
        | OrchestratorEvent::PublishDosingReport { .. }
        | OrchestratorEvent::PublishSystemLog { .. }
        | OrchestratorEvent::PublishRecipeStageChanged { .. }
        | OrchestratorEvent::PublishCommandRejected { .. }
        | OrchestratorEvent::RequestSensorForcePublish
        | OrchestratorEvent::SetSensorContinuousMode { .. }
        | OrchestratorEvent::PublishFsmTransition { .. }
        | OrchestratorEvent::PublishDosingCycle { .. }
        | OrchestratorEvent::TriggerOtaUpdate
        | OrchestratorEvent::UpdateWifiList { .. }
        | OrchestratorEvent::RebootDevice
        | OrchestratorEvent::FactoryReset => {
            tracing::debug!(?event, "simulator event has no direct virtual-hardware mutation");
        }
    }
}
```

Do not use `_ => {}`. Update `dispatcher.rs` so `SimDispatcher` calls `apply_event` before forwarding messaging events to the configured telemetry sinks.

- [ ] **Step 4: Run focused tests and clippy**

```bash
cargo test event_dispatcher -- --nocapture
cargo test actuators::virtual_hw -- --nocapture
cargo clippy --all-targets -- -D warnings
```

Expected: PASS and no warnings.

- [ ] **Step 5: Commit the lane**

```bash
git add hydragrow-simulator/src/actuators/virtual_hw.rs hydragrow-simulator/src/event_dispatcher.rs hydragrow-simulator/src/dispatcher.rs hydragrow-simulator/src/lib.rs
git commit -m "feat(simulator): dispatch FSM events to virtual hardware"
```

---

### Task 3: Close the plant -> sensor feedback loop

**Files:**
- Modify: `hydragrow-simulator/src/plant/tank.rs`
- Modify: `hydragrow-simulator/src/sensors/sensor_model.rs`
- Create: `hydragrow-simulator/tests/plant_feedback.rs`

- [ ] **Step 1: Write failing plant tests**

```rust
#[test]
fn nutrient_a_flow_uses_config_gain_and_pwm() {
    let mut tank = Tank {
        volume_l: 10.0,
        ec: 1.0,
        ph: 6.0,
        temp: 25.0,
        water_level: 50.0,
    };
    let config = ControllerConfig {
        ec_gain_per_ml: 0.5,
        pump_a_capacity_ml_per_sec: 2.0,
        ..Default::default()
    };
    let hw = VirtualHardwareState {
        pump_a: VirtualPump { on: true, pwm_percent: 50 },
        ..Default::default()
    };
    tank.step(1000, &hw, &config);
    assert!((tank.ec - 1.05).abs() < 1e-6);
}

#[test]
fn refill_and_drain_change_volume_and_level() {
    let mut tank = Tank {
        volume_l: 10.0,
        ec: 1.0,
        ph: 6.0,
        temp: 25.0,
        water_level: 50.0,
    };
    // Use config.water_level_* / tank_height from the real ControllerConfig fields.
    // Assert that the in/out flows move both volume and level consistently and clamp at physical bounds.
}
```

- [ ] **Step 2: Write failing sensor-noise test**

```rust
#[test]
fn sensor_noise_is_deterministic_for_a_seeded_config() {
    let tank = Tank { volume_l: 10.0, ec: 1.5, ph: 6.2, temp: 24.5, water_level: 40.0 };
    let cfg = NoiseConfig { ec_noise_std_dev: 0.05, ph_noise_std_dev: 0.1, seed: 42 };
    let first = read_sensor(&tank, &cfg);
    let second = read_sensor(&tank, &cfg);
    assert_eq!(first.ec, second.ec);
    assert_eq!(first.ph, second.ph);
}
```

- [ ] **Step 3: Run the focused tests and verify failure**

```bash
cargo test --test plant_feedback -- --nocapture
```

Expected: FAIL because current `Tank` ignores volume/physical bounds and `read_sensor()` hard-codes zero noise.

- [ ] **Step 4: Implement the host-native deterministic model**

Keep the model linear and configuration-driven. The core flow formulas must use only `ControllerConfig` fields:

```rust
let dt_sec = dt_ms as f32 / 1000.0;
let nutrient_ml = config.pump_a_capacity_ml_per_sec
    * dt_sec
    * (actuators.pump_a.pwm_percent as f32 / 100.0);
let ec_delta = nutrient_ml * config.ec_gain_per_ml / self.volume_l.max(f32::EPSILON);
self.ec += ec_delta;
```

Implement pH using `ph_shift_up_per_ml` / `ph_shift_down_per_ml`. Implement refill/drain from the existing `water_level_*` and tank-height-related configuration fields; keep `volume_l`, `water_level`, and bounds internally consistent. Document that this is a first-order linear model, not a fluid-dynamics solver.

For sensor noise, use a small deterministic PRNG local to the simulator instead of adding a general-purpose random dependency. Store a seed/state in `NoiseConfig` and transform the generated unit values into a bounded Gaussian-like perturbation sufficient for deterministic tests. `NoiseConfig::none()` must remain exactly zero-noise.

- [ ] **Step 5: Run focused tests and snapshots**

```bash
cargo test --test plant_feedback -- --nocapture
cargo test snapshot_dosing -- --nocapture
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Expected: PASS. Any changed `insta` snapshot must be reviewed manually before accepting it.

- [ ] **Step 6: Commit the lane**

```bash
git add hydragrow-simulator/src/plant/tank.rs hydragrow-simulator/src/sensors/sensor_model.rs hydragrow-simulator/tests/plant_feedback.rs
git commit -m "feat(simulator): close deterministic plant sensor feedback loop"
```

---

### Task 4: Scenario engine and real fault scheduling

**Files:**
- Modify: `hydragrow-simulator/src/scenario/format.rs`
- Create: `hydragrow-simulator/src/scenario/engine.rs`
- Modify: `hydragrow-simulator/src/scenario/mod.rs`
- Modify: `hydragrow-simulator/src/faults/injector.rs`
- Modify: `hydragrow-simulator/src/lib.rs`
- Create: `hydragrow-simulator/tests/scenario_runtime.rs`

- [ ] **Step 1: Write failing scenario timing tests**

```rust
#[test]
fn a_fault_activates_once_when_simulated_time_crosses_at_ms() {
    let scenario = Scenario {
        initial_tank: sample_tank(),
        faults: vec![FaultEvent {
            at_ms: 5000,
            kind: FaultEventKind::PumpStuckOn { pump: "PUMP_A".into() },
        }],
    };
    let mut engine = ScenarioEngine::new(scenario);
    assert!(engine.activate_between(0, 1000).is_empty());
    assert_eq!(engine.activate_between(4000, 5000).len(), 1);
    assert!(engine.activate_between(5000, 6000).is_empty());
}
```

- [ ] **Step 2: Write failing sensor-freeze test**

```rust
#[test]
fn sensor_frozen_fault_reuses_the_activation_sample() {
    let mut injector = Injector::new();
    injector.add_active_fault(FaultEventKind::SensorFrozen { sensor: "EC".into() });
    let mut first = sample_sensor(1.2);
    injector.apply_sensor_faults(&mut first);
    let frozen = first.ec;
    let mut later = sample_sensor(1.8);
    injector.apply_sensor_faults(&mut later);
    assert_eq!(later.ec, frozen);
}
```

- [ ] **Step 3: Run focused tests and verify failure**

```bash
cargo test --test scenario_runtime -- --nocapture
```

Expected: FAIL because `Injector::apply_sensor_faults()` is currently a no-op and fault activation is currently implemented ad hoc inside one integration test.

- [ ] **Step 4: Implement a cursor-based scenario engine**

```rust
pub struct ScenarioEngine {
    scenario: Scenario,
    next_fault: usize,
}

impl ScenarioEngine {
    pub fn new(mut scenario: Scenario) -> Self {
        scenario.faults.sort_by_key(|fault| fault.at_ms);
        Self { scenario, next_fault: 0 }
    }

    pub fn activate_between(&mut self, previous_ms: u64, current_ms: u64) -> Vec<FaultEventKind> {
        let mut out = Vec::new();
        while let Some(fault) = self.scenario.faults.get(self.next_fault) {
            if fault.at_ms <= previous_ms {
                self.next_fault += 1;
                continue;
            }
            if fault.at_ms > current_ms {
                break;
            }
            out.push(fault.kind.clone());
            self.next_fault += 1;
        }
        out
    }
}
```

Refactor `Injector` so hardware faults cover all supported pump target names, and sensor-freeze faults retain the first frozen sample per sensor. Unknown fault target names must return a structured error from the scenario-loading/validation path, not silently do nothing.

- [ ] **Step 5: Add scenario validation and deterministic file loading**

Add a loader that reports the exact path and JSON parse error:

```rust
pub fn load_scenario(path: &Path) -> anyhow::Result<Scenario> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario {}", path.display()))?;
    let scenario = serde_json::from_str::<Scenario>(&content)
        .with_context(|| format!("invalid scenario JSON in {}", path.display()))?;
    validate_scenario(&scenario)?;
    Ok(scenario)
}
```

- [ ] **Step 6: Run tests and commit**

```bash
cargo test --test scenario_runtime -- --nocapture
cargo test scenario -- --nocapture
cargo fmt --check
cargo clippy --all-targets -- -D warnings

git add hydragrow-simulator/src/scenario hydragrow-simulator/src/faults/injector.rs hydragrow-simulator/src/lib.rs hydragrow-simulator/tests/scenario_runtime.rs
git commit -m "feat(simulator): add deterministic scenario engine and faults"
```

---

### Task 5: Make telemetry sinks usable and safe

**Files:**
- Modify: `hydragrow-simulator/src/telemetry/recorder.rs`
- Modify: `hydragrow-simulator/src/telemetry/mqtt_bridge.rs`
- Modify: `hydragrow-simulator/src/telemetry/mod.rs`
- Modify: `hydragrow-simulator/tests/mqtt_integration.rs`

- [ ] **Step 1: Write failing recorder error-propagation test**

```rust
#[test]
fn recorder_returns_io_errors_instead_of_panicking() {
    let result = Recorder::new("/definitely/missing/dir/out.csv");
    assert!(result.is_err());
}
```

Change `Recorder::new()` and `record()` to return `anyhow::Result<()>` or a typed `io::Result`, while keeping the CSV schema stable.

- [ ] **Step 2: Write failing MQTT payload tests**

```rust
#[test]
fn sensor_payload_uses_shared_topic_and_roundtrips_json() {
    let data = sample_sensor();
    let payload = serde_json::to_string(&data).unwrap();
    let decoded: SensorData = serde_json::from_str(&payload).unwrap();
    assert_eq!(decoded.device_id, data.device_id);
    assert_eq!(decoded.ec, data.ec);
}
```

Also add a test that a `PublishFsmState` event produces a non-empty structured payload, not `{}`.

- [ ] **Step 3: Run focused tests and verify failure**

```bash
cargo test --test mqtt_integration -- --nocapture
cargo test telemetry -- --nocapture
```

Expected: the new error-propagation test fails until `Recorder::new` returns a result; the FSM state payload assertion fails until the bridge uses the shared snapshot/status representation.

- [ ] **Step 4: Refactor MQTT bridge around explicit sinks**

Keep all topics from `hydragrow_shared::topics`; never hard-code an `AGITECH/...` literal in simulator code.

Use explicit methods:

```rust
pub fn publish_sensors(&mut self, data: &SensorData) -> anyhow::Result<()>;
pub fn publish_fsm_state(&mut self, snapshot: &FsmSnapshot) -> anyhow::Result<()>;
pub fn publish_event(&mut self, event: &OrchestratorEvent) -> anyhow::Result<()>;
```

`publish_event` must serialize the event payloads that the backend actually understands, or explicitly log/no-op for persistence/device-control events that have no simulator-side network representation. Do not swallow publish errors silently; return/log them at the sink boundary.

- [ ] **Step 5: Keep MQTT tests deterministic**

Retain the in-process mock broker strategy already present in `tests/mqtt_integration.rs`; do not reintroduce a dependency on a machine-local Mosquitto daemon.

Run:

```bash
cargo test --test mqtt_integration -- --nocapture
cargo test telemetry -- --nocapture
cargo clippy --all-targets -- -D warnings
```

Expected: PASS without requiring an external broker.

- [ ] **Step 6: Commit**

```bash
git add hydragrow-simulator/src/telemetry hydragrow-simulator/tests/mqtt_integration.rs
git commit -m "feat(simulator): wire safe CSV and MQTT telemetry sinks"
```

---

### Task 6: Integrate the simulator Harness around a simulated clock

**Files:**
- Modify: `hydragrow-simulator/src/harness.rs`
- Modify: `hydragrow-simulator/src/lib.rs`
- Create: `hydragrow-simulator/tests/harness_e2e.rs`

- [ ] **Step 1: Write the failing full-loop test**

```rust
#[test]
fn harness_runs_controller_against_virtual_plant() {
    let config = test_controller_config();
    let tank = test_tank();
    let noise = NoiseConfig::none();
    let mut harness = Harness::new(config, tank, noise);

    for _ in 0..20 {
        harness.tick(1000).unwrap();
    }

    assert_eq!(harness.uptime_ms(), 20_000);
    assert!(harness.tank.ec.is_finite());
    assert!(harness.ctx.phase != SystemPhase::Unknown);
}
```

Create a second test based on `hydragrow-controller-core/tests/e2e/full_dosing_cycle.rs` that asserts the same expected phase sequence while replacing direct sensor mutation with the simulator loop.

- [ ] **Step 2: Run the focused integration test and verify failure**

```bash
cargo test --test harness_e2e -- --nocapture
```

Expected: FAIL because current `Harness::tick()` returns `TickResult` directly, uses wall-clock time, never applies `delta`, never schedules scenario faults internally, and the dispatcher does not mutate hardware from FSM events.

- [ ] **Step 3: Replace wall clock with explicit simulated time**

Use a monotonic simulator clock owned by the harness:

```rust
pub struct SimClock {
    now_ms: u64,
    uptime_ms: u64,
}

impl SimClock {
    pub fn advance(&mut self, dt_ms: u64) {
        self.now_ms = self.now_ms.saturating_add(dt_ms);
        self.uptime_ms = self.uptime_ms.saturating_add(dt_ms);
    }
}
```

The harness must pass `clock.now_ms` and `clock.uptime_ms` to `orchestrator::tick`; tests must not depend on `SystemTime::now()`.

- [ ] **Step 4: Implement the exact tick order**

```rust
pub fn tick(&mut self, dt_ms: u64) -> anyhow::Result<TickResult> {
    let previous_ms = self.clock.uptime_ms;
    self.clock.advance(dt_ms);

    for fault in self.scenario_engine.as_mut()
        .map(|engine| engine.activate_between(previous_ms, self.clock.uptime_ms))
        .into_iter()
        .flatten()
    {
        self.injector.add_active_fault(fault);
    }

    self.injector.apply_hardware_faults(&mut self.hw);
    self.tank.step(dt_ms, &self.hw, &self.config);
    let mut sensor = read_sensor(&self.tank, &mut self.noise)?;
    self.injector.apply_sensor_faults(&mut sensor);

    let mut result = orchestrator::tick(
        self.clock.now_ms,
        self.clock.uptime_ms,
        &self.config,
        &sensor,
        self.sensor_last_update_ms,
        &mut self.ctx,
    );
    self.ctx.apply_delta(&mut result.delta);
    self.dispatcher.dispatch(&mut self.hw, &self.ctx, &sensor, &result.events)?;
    self.last_sensor = sensor;
    Ok(result)
}
```

The exact helper names may vary, but the dataflow and ordering must remain explicit. Do not reintroduce `SystemTime::now()` inside the simulation loop.

- [ ] **Step 5: Publish/record per-tick telemetry**

The Harness owns optional outputs:

```rust
pub struct HarnessOutputs {
    pub recorder: Option<Recorder>,
    pub mqtt: Option<MqttBridge>,
}
```

Every tick publishes the freshly generated `SensorData` and records the current phase + virtual pump state after event dispatch. This makes CLI `--record` and `--mqtt` real consumers rather than dead flags.

- [ ] **Step 6: Run all simulator tests and inspect snapshot diffs**

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Expected: PASS. Review all changed `.snap` files manually; do not use blanket snapshot acceptance.

- [ ] **Step 7: Commit**

```bash
git add hydragrow-simulator/src/harness.rs hydragrow-simulator/src/lib.rs hydragrow-simulator/tests/harness_e2e.rs
git commit -m "feat(simulator): integrate deterministic controller-in-the-loop harness"
```

---

### Task 7: Wire the CLI into the real simulation runtime

**Files:**
- Create: `hydragrow-simulator/src/simulation.rs`
- Modify: `hydragrow-simulator/src/main.rs`
- Modify: `hydragrow-simulator/src/lib.rs`
- Test: add CLI parser tests inline in `main.rs` or `simulation.rs`

- [ ] **Step 1: Write parser tests**

```rust
#[test]
fn parses_run_options() {
    let cli = Cli::try_parse_from([
        "hydragrow-sim",
        "run",
        "--scenario", "src/scenario/library/ec_stagnant.json",
        "--ticks", "100",
        "--tick-ms", "1000",
        "--device-id", "sim-01",
        "--record", "out.csv",
    ]).unwrap();

    match cli.command {
        Commands::Run { ticks, tick_ms, device_id, .. } => {
            assert_eq!(ticks, 100);
            assert_eq!(tick_ms, 1000);
            assert_eq!(device_id, "sim-01");
        }
        _ => panic!("wrong command"),
    }
}
```

- [ ] **Step 2: Run parser tests and verify the existing CLI is incomplete**

```bash
cargo test main -- --nocapture
```

Expected: FAIL because the current command model does not expose `ticks`/`tick-ms` and `main()` only prints placeholders.

- [ ] **Step 3: Define explicit CLI configuration**

```rust
#[derive(Subcommand)]
enum Commands {
    Run {
        #[arg(short, long)]
        scenario: Option<PathBuf>,
        #[arg(long, default_value_t = 100)]
        ticks: u64,
        #[arg(long, default_value_t = 1000)]
        tick_ms: u64,
        #[arg(long, default_value = "sim-dev")]
        device_id: String,
        #[arg(short, long)]
        mqtt: Option<String>,
        #[arg(short, long)]
        record: Option<PathBuf>,
    },
    Step { #[arg(short, long)] scenario: Option<PathBuf> },
    ScenarioList,
}
```

Keep default behavior deterministic and non-interactive for `run`; use `step` only for a REPL.

- [ ] **Step 4: Implement `simulation.rs` as the reusable CLI-to-Harness adapter**

```rust
pub fn build_harness(config: ControllerConfig, options: &RunOptions) -> anyhow::Result<Harness> {
    let tank = options
        .scenario
        .as_ref()
        .map(|path| load_scenario(path))
        .transpose()?
        .map(|scenario| Tank::from_initial(&scenario.initial_tank))
        .unwrap_or_default();

    Ok(Harness::builder(config, tank)
        .device_id(options.device_id.clone())
        .mqtt(options.mqtt.clone())
        .record(options.record.clone())
        .build()?)
}
```

The concrete builder API may be implemented as small constructor helpers, but `main.rs` must only parse arguments, load configuration, and call the runtime.

- [ ] **Step 5: Implement `ScenarioList` from actual files**

Do not hard-code `sensor_timeout.json` or any nonexistent filename. Walk `src/scenario/library/` and print `.json` files that are actually present; if the directory cannot be read, return an error with the path.

- [ ] **Step 6: Implement step REPL without `unwrap()` on I/O**

```rust
fn run_interactive_step(mut harness: Harness) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout())?;
        line.clear();
        stdin.read_line(&mut line)?;
        match line.trim() {
            "q" | "quit" => break,
            value => {
                let dt_ms = value.parse::<u64>()
                    .map_err(|_| anyhow::anyhow!("enter milliseconds, or q to quit"))?;
                let result = harness.tick(dt_ms)?;
                println!("uptime={} phase={:?} events={}", harness.uptime_ms(), harness.ctx.phase, result.events.len());
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 7: Verify the real CLI**

Run:

```bash
cargo run -- run --ticks 5 --tick-ms 1000
cargo run -- run --scenario src/scenario/library/ec_stagnant.json --ticks 15 --tick-ms 1000
cargo run -- scenario-list
printf '100\nq\n' | cargo run -- step
```

Expected: `run` prints a deterministic final state and per-tick summary, `scenario-list` prints only existing scenario files, and `step` advances uptime by exactly the entered durations.

- [ ] **Step 8: Commit**

```bash
git add hydragrow-simulator/src/simulation.rs hydragrow-simulator/src/main.rs hydragrow-simulator/src/lib.rs
git commit -m "feat(simulator): wire deterministic run step and scenario CLI"
```

---

### Task 8: End-to-end acceptance, documentation, and cleanup

**Files:**
- Modify: `hydragrow-simulator/tests/scenario_ec_stagnant.rs`
- Modify: `hydragrow-simulator/tests/snapshot_dosing.rs`
- Modify: `hydragrow-simulator/tests/mqtt_integration.rs` only for contract updates
- Modify: `README.md` if simulator usage/CI text is stale
- Create: `hydragrow-simulator/README.md`

- [ ] **Step 1: Replace the ad-hoc EC stagnant test with the real scenario engine**

Remove the manual fault scheduling loop from `scenario_ec_stagnant.rs` and use the same `Harness` path used by the CLI:

```rust
let mut harness = Harness::from_scenario(test_controller_config(), "src/scenario/library/ec_stagnant.json")?;
for _ in 0..15 {
    harness.tick(1000)?;
}
assert!(harness.uptime_ms() == 15_000);
assert!(harness.hw.pump_a.on);
```

Add the observable FSM assertion that proves the intended fault/phase occurred; do not assert only that the stuck pump bit is true.

- [ ] **Step 2: Add a full controller-in-the-loop acceptance scenario**

The test must verify all four layers in one run:

```text
scenario fault activation
    -> Injector
    -> VirtualHardwareState
    -> Tank.step()
    -> read_sensor()
    -> orchestrator::tick()
    -> ctx.apply_delta()
    -> OrchestratorEvent dispatch
    -> telemetry sink
```

Assert representative outputs from each link, including simulated time, EC movement, pump state, FSM phase, and at least one recorded/published telemetry payload.

- [ ] **Step 3: Add `hydragrow-simulator/README.md`**

Document exact host-native commands:

```bash
cd hydragrow-simulator
cargo test
cargo run -- scenario-list
cargo run -- run --ticks 100 --tick-ms 1000
cargo run -- run --scenario src/scenario/library/ec_stagnant.json --ticks 15 --tick-ms 1000
cargo run -- step
cargo run -- run --ticks 100 --record /tmp/hydragrow-sim.csv
cargo run -- run --ticks 100 --mqtt mqtt://localhost:1883 --device-id sim-01
```

Explain that the simulator calls the real controller-core FSM and that the plant is intentionally a first-order host-native model, not an ESP32 emulator.

- [ ] **Step 4: Run the complete simulator quality gate**

From repository root:

```bash
(cd hydragrow-simulator && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)
```

Then verify the executable smoke tests:

```bash
(cd hydragrow-simulator && cargo run -- run --ticks 1 --tick-ms 1000)
(cd hydragrow-simulator && cargo run -- scenario-list)
```

Expected: all checks PASS, the binary exits 0, and no command prints the old placeholder strings `Starting step repl...` or `Continuous run not fully wired yet`.

- [ ] **Step 5: Review generated snapshots and outputs**

Inspect every changed `tests/snapshots/*.snap` file and any sample CSV produced during tests. Accept only changes explained by an intentional simulator behavior change.

- [ ] **Step 6: Clean up merged worktrees**

```bash
git worktree list
git worktree remove ../HYDRAGROW-lane-virtual-hardware
git worktree remove ../HYDRAGROW-lane-plant-sensor
git worktree remove ../HYDRAGROW-lane-scenario-faults
git worktree remove ../HYDRAGROW-lane-telemetry
```

Then delete already-merged lane branches with `git branch -d ...`.

- [ ] **Step 7: Commit documentation/acceptance updates**

```bash
git add hydragrow-simulator README.md

git commit -m "docs(simulator): document completed digital twin workflow"
```

---

## Self-Review Checklist

### Spec coverage
- Controller core remains the real FSM entry point: Task 6.
- Virtual hardware fully represents `OrchestratorEvent`: Task 2.
- Plant model reads all chemistry/flow coefficients from `ControllerConfig`: Task 3.
- Sensor noise can be disabled and deterministic: Task 3.
- Scenario scheduling is simulated-time driven: Task 4.
- Fault injection has observable behavior, including sensor freeze: Task 4.
- CSV recorder is wired and cannot panic on ordinary I/O failure: Task 5.
- MQTT publishing uses shared topics and deterministic in-process tests: Task 5.
- CLI run/step/scenario-list are real, not placeholders: Task 7.
- End-to-end controller-in-the-loop evidence covers the full feedback chain: Task 8.

### Placeholder scan
- No task says “TBD”, “TODO”, “implement later”, “add appropriate error handling”, or “write tests for the above”.
- Every changed code area has an explicit test and command.

### Type/API consistency
- `DosingPumpTarget` is the enum from controller-core.
- `WaterDirection` is the enum from controller-core.
- `SensorData`/`FsmSnapshot` are imported from `hydragrow-shared`; no local duplicate types are introduced.
- `Harness::tick` returns a `Result<TickResult>` so CLI and tests can handle telemetry/sink I/O failures without panic.

### Parallel-worktree review
- Task 1 is serialized as the shared governance/foundation correction.
- Tasks 2–5 have disjoint ownership by actual file path and can run in separate worktrees.
- Task 6 is deliberately serialized after those lanes because `harness.rs` integrates every subsystem.
- Task 7 is serialized after Task 6 because CLI wiring depends on the final Harness API.
- Merges happen one at a time from one integration checkout, with the full simulator gate after every merge.

## Definition of Done

`hydragrow-simulator` is complete when all of the following are true:

1. `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` pass in the simulator crate.
2. `cargo run -- run --ticks N` advances simulated time deterministically and exits normally.
3. `step` changes only simulated time/state and supports clean quit/error reporting.
4. The FSM's hardware events change virtual actuator state through an exhaustive match.
5. The next simulation tick sees those actuator changes in the plant model.
6. Plant state becomes sensor data, sensor data feeds the real FSM, and `ContextDelta` is applied every tick.
7. Scenario faults activate exactly once at their scheduled simulated time.
8. At least one real scenario demonstrates a causal FSM response, not just a parser-level assertion.
9. CSV and MQTT sinks are optional, wired, deterministic in tests, and do not silently panic.
10. No ESP32-specific dependency enters the simulator crate.
11. The repository contains no active CLI placeholder text and no stale scenario names are advertised.
12. All merged worktrees have been removed.
