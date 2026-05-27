# FSM System Log Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a custom `tracing-subscriber` Layer that uses `Visit` to turn structured FSM tracing events into existing MQTT `UnifiedSystemLog` JSON.

**Architecture:** `hydragrow-shared` owns the testable `SystemLogLayer` and `SystemLogVisitor`. Controller FSM code emits stable `hydragrow.system_log` tracing events; the Layer listens to those events and sends JSON through the existing `mpsc::Sender<String>`.

**Tech Stack:** Rust, `tracing`, `tracing-subscriber`, `tracing::field::Visit`, existing `UnifiedSystemLog` schema.

---

## Task 1: Layer Tests

**Files:**
- Modify: `hydragrow-shared/Cargo.toml`
- Modify: `hydragrow-shared/src/log.rs`

- [ ] **Step 1: Write failing tests**

Add tests that install `SystemLogLayer` on a local subscriber, emit `tracing::event!` with target `hydragrow.system_log`, and assert the received JSON payload preserves device ID, level, category, title, message, source, cycle ID, and timestamp.

- [ ] **Step 2: Verify red**

Run: `rtk cargo test --manifest-path hydragrow-shared/Cargo.toml system_log_layer`

Expected: FAIL because `SystemLogLayer` and event emit helpers do not exist.

## Task 2: Implement Layer and Visitor

**Files:**
- Modify: `hydragrow-shared/Cargo.toml`
- Modify: `hydragrow-shared/src/log.rs`

- [ ] **Step 1: Add dependency**

Add `tracing-subscriber = { version = "0.3", default-features = false, features = ["registry"] }`.

- [ ] **Step 2: Implement visitor**

Implement a `SystemLogVisitor` that records fields from `record_str`, `record_debug`, `record_i64`, and `record_u64`.

- [ ] **Step 3: Implement layer**

Implement `SystemLogLayer` with `Layer<S>::on_event`, filter by `event.metadata().target() == "hydragrow.system_log"`, build `UnifiedSystemLog`, serialize, send, and increment drop count on send failure.

- [ ] **Step 4: Implement emit helpers**

Add `emit_basic_system_log(SystemLogRecord)` and `emit_system_log_event(...)` helpers that only emit tracing events; they do not send MQTT directly.

- [ ] **Step 5: Verify green**

Run: `rtk cargo test --manifest-path hydragrow-shared/Cargo.toml system_log_layer`

Expected: PASS.

## Task 3: Wire Controller FSM to Layer

**Files:**
- Modify: `ESP32-C3-CONTROLLER-NODE/Cargo.toml`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/fsm/mod.rs`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/fsm/utils.rs`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/fsm/observers/system_log.rs`

- [ ] **Step 1: Add dependency**

Add `tracing-subscriber = { version = "0.3", default-features = false, features = ["registry"] }` to the controller crate.

- [ ] **Step 2: Install layer in FSM thread**

At FSM loop startup, create a scoped subscriber with `Registry::default().with(SystemLogLayer::new(fsm_mqtt_tx.clone(), log_drop_counter()))` and run the existing loop body inside `tracing::subscriber::with_default(...)`.

- [ ] **Step 3: Convert helpers**

Change `utils::send_system_log` and `SystemLogObserver::send_log` to call `emit_system_log_event` or `emit_basic_system_log`, not `SystemLogPublisher`.

- [ ] **Step 4: Verify**

Run:

```bash
rtk cargo test --manifest-path hydragrow-shared/Cargo.toml
rtk cargo check --manifest-path ESP32-C3-CONTROLLER-NODE/Cargo.toml --target riscv32imc-esp-espidf
```

Expected: shared tests pass; controller check may remain blocked if ESP target std is not installed.

