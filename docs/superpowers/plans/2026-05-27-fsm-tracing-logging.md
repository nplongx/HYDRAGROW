# FSM Tracing Logging Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace manual FSM system-log publishing helpers with a tracing-oriented bridge while keeping the MQTT `UnifiedSystemLog` payload contract unchanged.

**Architecture:** Put the testable bridge in `hydragrow-shared` because it depends only on `UnifiedSystemLog`, `std::sync::mpsc::Sender`, and `tracing`. The ESP32 controller calls that bridge from FSM helpers and observers, then emits regular runtime diagnostics with `tracing::{debug, info, warn, error}`.

**Tech Stack:** Rust 2021/2024, `tracing`, `tracing-log`, `serde_json`, existing `hydragrow-shared::log` schema.

---

## File Structure

- Modify `hydragrow-shared/Cargo.toml`: add `tracing = "0.1"`.
- Modify `hydragrow-shared/src/log.rs`: add `SystemLogPublisher`, `SystemLogRecord`, drop-count support, and unit tests.
- Modify `ESP32-C3-CONTROLLER-NODE/Cargo.toml`: add `tracing = "0.1"` and `tracing-log = "0.2"`.
- Modify `ESP32-C3-CONTROLLER-NODE/src/main.rs`: initialize `LogTracer` before `EspLogger`.
- Modify `ESP32-C3-CONTROLLER-NODE/src/fsm/utils.rs`: remove local manual log packaging and delegate to shared `SystemLogPublisher`.
- Modify `ESP32-C3-CONTROLLER-NODE/src/fsm/observers/system_log.rs`: use the shared publisher and `tracing::warn!`.
- Modify `ESP32-C3-CONTROLLER-NODE/src/fsm/mod.rs`, `orchestrator.rs`, `dispatcher.rs`: replace touched `log::` macros with `tracing::` macros.

## Task 1: Shared Tracing Bridge

**Files:**
- Modify: `hydragrow-shared/Cargo.toml`
- Modify: `hydragrow-shared/src/log.rs`

- [ ] **Step 1: Write failing tests**

Add tests in `hydragrow-shared/src/log.rs` under `#[cfg(test)]`:

```rust
#[test]
fn publisher_builds_basic_system_log_payload() {
    let (tx, rx) = std::sync::mpsc::channel();
    let drop_count = std::sync::atomic::AtomicU32::new(0);
    let publisher = SystemLogPublisher::new(&tx, &drop_count);

    publisher.publish_basic(SystemLogRecord {
        device_id: "device-1",
        level: LogLevel::Warning,
        category: LogCategory::UserAction,
        title: "Safety Timeout",
        source: "fsm_command",
        message: "Pump stopped",
        cycle_id: Some("cycle-7"),
        timestamp_ms: 1234,
    });

    let payload = rx.recv().expect("system log payload");
    let decoded: UnifiedSystemLog = serde_json::from_str(&payload).expect("valid json");
    assert_eq!(decoded.device_id, "device-1");
    assert_eq!(decoded.level, LogLevel::Warning);
    assert_eq!(decoded.category, LogCategory::UserAction);
    assert_eq!(decoded.title, "Safety Timeout");
    assert_eq!(decoded.timestamp_ms, 1234);
    match decoded.event {
        SystemLogEvent::BasicSystemLog(metadata) => {
            assert_eq!(metadata.source, "fsm_command");
            assert_eq!(metadata.message, "Pump stopped");
            assert_eq!(metadata.cycle_id.as_deref(), Some("cycle-7"));
        }
        _ => panic!("expected basic system log"),
    }
    assert_eq!(drop_count.load(std::sync::atomic::Ordering::Relaxed), 0);
}

#[test]
fn publisher_increments_drop_count_when_channel_is_closed() {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    drop(rx);
    let drop_count = std::sync::atomic::AtomicU32::new(0);
    let publisher = SystemLogPublisher::new(&tx, &drop_count);

    publisher.publish_basic(SystemLogRecord {
        device_id: "device-1",
        level: LogLevel::Info,
        category: LogCategory::System,
        title: "Dropped",
        source: "test",
        message: "closed",
        cycle_id: None,
        timestamp_ms: 1,
    });

    assert_eq!(drop_count.load(std::sync::atomic::Ordering::Relaxed), 1);
}
```

- [ ] **Step 2: Run tests and verify failure**

Run: `rtk cargo test --manifest-path hydragrow-shared/Cargo.toml publisher_`

Expected: FAIL because `SystemLogPublisher` and `SystemLogRecord` do not exist.

- [ ] **Step 3: Implement minimal bridge**

Add `SystemLogRecord<'a>` and `SystemLogPublisher<'a>` to `hydragrow-shared/src/log.rs`. `publish_basic` should emit a `tracing` event with the same fields, build `UnifiedSystemLog`, serialize it, send it to the provided channel, and increment the provided `AtomicU32` on send failure.

- [ ] **Step 4: Verify shared tests pass**

Run: `rtk cargo test --manifest-path hydragrow-shared/Cargo.toml publisher_`

Expected: PASS.

## Task 2: Controller Wiring

**Files:**
- Modify: `ESP32-C3-CONTROLLER-NODE/Cargo.toml`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/main.rs`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/fsm/utils.rs`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/fsm/observers/system_log.rs`

- [ ] **Step 1: Add dependencies**

Add:

```toml
tracing = "0.1"
tracing-log = "0.2"
```

- [ ] **Step 2: Initialize log compatibility**

In `main`, call:

```rust
let _ = tracing_log::LogTracer::init();
```

before `EspLogger::initialize_default();`.

- [ ] **Step 3: Delegate `send_system_log`**

Update `fsm::utils::send_system_log` to create `SystemLogPublisher::new(tx, &LOG_DROP_COUNT)` and call `publish_event` or `publish_basic` with the current timestamp.

- [ ] **Step 4: Update observer**

Replace direct JSON construction in `SystemLogObserver::send_log` with `SystemLogPublisher`. Replace `log::warn!` imports/usages in that file with `tracing::warn!`.

- [ ] **Step 5: Check controller**

Run: `rtk cargo check --manifest-path ESP32-C3-CONTROLLER-NODE/Cargo.toml`

Expected: PASS, unless ESP-IDF network/toolchain setup blocks dependency resolution.

## Task 3: FSM Macro Migration

**Files:**
- Modify: `ESP32-C3-CONTROLLER-NODE/src/fsm/mod.rs`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/fsm/orchestrator.rs`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/fsm/dispatcher.rs`

- [ ] **Step 1: Replace touched `log` imports**

Use `tracing::{debug, error, info, warn}` where needed.

- [ ] **Step 2: Replace FSM status macros**

Change touched `log::debug!`, `log::error!`, `info!`, and `warn!` calls in FSM modules to `tracing` equivalents while keeping message text and fields stable.

- [ ] **Step 3: Verify**

Run:

```bash
rtk cargo test --manifest-path hydragrow-shared/Cargo.toml publisher_
rtk cargo check --manifest-path ESP32-C3-CONTROLLER-NODE/Cargo.toml
```

Expected: shared tests PASS and controller check PASS or report only environment/toolchain limitations.

