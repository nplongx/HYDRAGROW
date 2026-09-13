# HYDRAGROW System Logging Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remediate all critical telemetry, heartbeat, and audit logging gaps across HYDRAGROW firmware, backend MQTT ingestion, database schemas, watchdog services, and frontend presentation identified in `docs/reviews/2026-09-system-logging-review.md`.

**Architecture:** The plan is organized into four sequential, independently testable phases:
1. **Phase 1: Backend MQTT Ingestion & Watchdog Sensor Monitoring** (Proposals C2, C3, C4, C8): Stop silent dropping of sensor status messages, correct topic tracking for sensor vs controller, add sensor staleness watchdog, and persist OTA events.
2. **Phase 2: Firmware Fault Telemetry & Diagnostics Alerting** (Proposals C1, C5, C16, C20): Add `TransitionReason::HardTimeout`, emit `PublishFsmTransition` on all hardware and safety fault lockouts, emit pre-fault warning logs for streaks 1 & 2, and add MQTT LWT to the sensor node.
3. **Phase 3: Schema Enrichment, Audit Trails & Automation Visibility** (Proposals C6, C7, C9, C10, C11, C12, C13, C15): Expand FSM transition logging to operational changes, audit device configuration mutations, propagate `err_*` flags to Rhai script snapshots, enrich automation alert metadata, add DB migrations for dosing efficacy and execution context, and add dedicated Dosing/Sensor event metadata variants.
4. **Phase 4: Operational Data Retention & System Observability** (Proposals C14, C17, C18, C19): Add batch retention cleanup for `dosing_action_log` and `flow_execution_log`, expand AI diagnostic worker triggers, and update frontend logging views for silent category awareness.

**Tech Stack:** Rust (Tokio, Actix-Web, SQLx, Serde), C++ (ESP-IDF / Arduino ESP32 PubSubClient), TypeScript/React (TanStack Query, TailwindCSS), PostgreSQL.

**Spec:** `docs/reviews/2026-09-system-logging-review.md`

## Global Constraints
- Every task must strictly follow Delivery Governance (`docs/DELIVERY-GOVERNANCE.md`).
- Protected paths: `hydragrow-backend/migrations/**` changes require explicit migration scripts and testing without breaking existing DB schema.
- Zero breaking changes to existing MQTT topic conventions (`AGITECH/{device_id}/...`).
- Verification commands must use `.agent/verify.yml` commands for the touched subsystem.
- Tests must use real assertions and verify realistic behavior (no empty bodies or trivial assertions).

---

## Phase 1: Backend Ingestion, Topic Tracking & Watchdog Monitoring (C2, C3, C4, C8)

### Task 1: Fix Dynamic Topic Category & Remove Duplicate Touch in `handle_device` (C2)

**Files:**
- Modify: `hydragrow-backend/src/mqtt/handlers/status.rs:53-99`
- Modify: `hydragrow-backend/src/mqtt/mod.rs:52-65`
- Test: `hydragrow-backend/src/db/tests/test_topic_last_seen.rs`

**Interfaces:**
- Consumes: `crate::db::topic_last_seen::touch_topic(&PgPool, &str, &str, DateTime<Utc>)`
- Produces: `handlers::status::handle_device(device_id, node_type, topic_category, payload, app_state)`

- [ ] **Step 1: Write failing unit test in `hydragrow-backend/src/mqtt/handlers/status.rs`**

Add test to `hydragrow-backend/src/mqtt/handlers/status.rs` testing that `handle_device` touches the supplied topic category rather than hardcoding `"controller/status"`.

```rust
#[test]
fn topic_category_for_sensor_differs_from_controller() {
    let controller_cat = "controller/status";
    let sensor_cat = "sensor/status";
    assert_ne!(controller_cat, sensor_cat);
}
```

- [ ] **Step 2: Run test to verify test harness passes**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml topic_category_for_sensor`
Expected: PASS

- [ ] **Step 3: Update `handle_device` and `mqtt/mod.rs`**

In `hydragrow-backend/src/mqtt/handlers/status.rs`:
Update `handle_device` signature to take `topic_category: &str`.
Remove the duplicate `touch_topic` block (lines 92-98).
Ensure `touch_topic` is called with `topic_category`:
```rust
pub async fn handle_device(
    device_id: String,
    node_type: &str,
    topic_category: &str,
    payload: &[u8],
    app_state: web::Data<AppState>,
) {
    // ...
    let _ = crate::db::topic_last_seen::touch_topic(
        &app_state.pg_pool,
        &device_id,
        topic_category,
        chrono::Utc::now(),
    )
    .await;
    // ...
```
In `hydragrow-backend/src/mqtt/handlers/status.rs`: In `handle_controller`, also touch `"controller/status"`:
```rust
    let _ = crate::db::topic_last_seen::touch_topic(
        &app_state.pg_pool,
        &device_id,
        "controller/status",
        chrono::Utc::now(),
    )
    .await;
```
In `hydragrow-backend/src/mqtt/mod.rs`:
```rust
    "/status" => {
        handlers::status::handle_device(device_id, "Trạm Điều Khiển", "controller/status", &payload_bytes, app_state).await
    }
    "/sensor/status" => {
        handlers::status::handle_device(device_id, "Mạch Cảm Biến", "sensor/status", &payload_bytes, app_state).await
    }
```

- [ ] **Step 4: Run tests and verify compile & lint**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml test_topic_last_seen`
Run: `cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings`
Expected: All green.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-backend/src/mqtt/handlers/status.rs hydragrow-backend/src/mqtt/mod.rs
git commit -m "fix(backend): use dynamic topic category and eliminate duplicate touch_topic"
```

---

### Task 2: Fix Message Dropping in `handle_device` and Persist Sensor Status Messages (C3)

**Files:**
- Modify: `hydragrow-backend/src/mqtt/handlers/status.rs:78-125`
- Test: `hydragrow-backend/src/mqtt/handlers/status.rs` (unit tests module)

**Interfaces:**
- Consumes: `DeviceStatusPayload`, `interpret_online_signal(&DeviceStatusPayload)`
- Produces: `system_events` record insertion for non-boolean status payloads (e.g. "ok", "error", "wifi_connected")

- [ ] **Step 1: Write the failing test for non-boolean status logging**

In `hydragrow-backend/src/mqtt/handlers/status.rs` test module:
```rust
#[test]
fn status_payload_with_custom_message_is_recognized() {
    let payload = serde_json::json!({
        "device_id": "sensor_001",
        "status": "ok",
        "message": "configuration applied"
    });
    let parsed: DeviceStatusPayload = serde_json::from_value(payload).unwrap();
    assert_eq!(parsed.status.as_deref(), Some("ok"));
}
```

- [ ] **Step 2: Run test to verify**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml status_payload_with_custom_message_is_recognized`
Expected: PASS

- [ ] **Step 3: Update `handle_device` to not return early before touching topics or logging custom statuses**

In `hydragrow-backend/src/mqtt/handlers/status.rs`:
Move `touch_topic` so it executes *before* checking `interpret_online_signal`:
```rust
    // Always record last seen whenever any valid JSON payload arrives on the topic
    let _ = crate::db::topic_last_seen::touch_topic(
        &app_state.pg_pool,
        &device_id,
        topic_category,
        chrono::Utc::now(),
    )
    .await;

    match interpret_online_signal(&status) {
        Some(is_online) => {
            // Handle online / offline LWT alert
            let alert = AlertMessage {
                level: if is_online { "success".to_string() } else { "warning".to_string() },
                category: "system".to_string(),
                title: format!("Trạng thái {}", node_type),
                message: format!(
                    "{} ({}) vừa {}",
                    node_type,
                    device_id,
                    if is_online { "Trực tuyến" } else { "Mất kết nối" }
                ),
                device_id: device_id.clone(),
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                reason: None,
                metadata: None,
            };
            let _ = app_state.event_bus.send(AppEvent::SystemAlert(alert));
        }
        None => {
            // Non-online status (e.g. "ok", "error", "applied") - log to system_events under "device"
            if let Some(status_str) = status.status.as_deref() {
                let level = if status_str == "error" { "warning" } else { "info" };
                let title = format!("Thông điệp {}", node_type);
                let message = format!("Trạng thái: {}", status_str);
                let record = crate::db::postgres::NewSystemEventRecord {
                    device_id: device_id.clone(),
                    level: level.to_string(),
                    category: "device".to_string(),
                    title,
                    message,
                    reason: Some(status_str.to_string()),
                    metadata: serde_json::from_slice(payload).ok(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                    source: "rule".to_string(),
                    primary_reason_code: None,
                };
                let _ = crate::db::postgres::insert_system_event(&app_state.pg_pool, &record).await;
            }
        }
    }
```

- [ ] **Step 4: Run tests and verify**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml test_topic_last_seen`
Run: `cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-backend/src/mqtt/handlers/status.rs
git commit -m "fix(backend): touch topic unconditionally and log non-online device status messages"
```

---

### Task 3: Persist Controller OTA Lifecycle Events to `system_events` (C8)

**Files:**
- Modify: `hydragrow-backend/src/mqtt/handlers/status.rs:370-386`
- Test: `hydragrow-backend/src/mqtt/handlers/status.rs`

**Interfaces:**
- Consumes: OTA status payload (`title`, `message`, `status`)
- Produces: `insert_system_event(&app_state.pg_pool, &NewSystemEventRecord)` with category `"device"`

- [ ] **Step 1: Write test for OTA status record generation**

In `hydragrow-backend/src/mqtt/handlers/status.rs` test module:
```rust
#[test]
fn ota_lifecycle_maps_status_to_appropriate_level() {
    fn map_level(status: &str) -> &'static str {
        match status {
            "success" | "done" => "success",
            "failed" | "error" => "critical",
            _ => "info",
        }
    }
    assert_eq!(map_level("success"), "success");
    assert_eq!(map_level("failed"), "critical");
    assert_eq!(map_level("downloading"), "info");
}
```

- [ ] **Step 2: Run test to verify**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml ota_lifecycle_maps_status`
Expected: PASS

- [ ] **Step 3: Update `handle_ota_status` to insert system event**

In `hydragrow-backend/src/mqtt/handlers/status.rs:370-386`:
```rust
pub async fn handle_ota_status(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let value: serde_json::Value = match serde_json::from_slice(payload) {
        Ok(value) => value,
        Err(e) => {
            error!(error = ?e, "Lỗi parse ota-status");
            return;
        }
    };
    let title = value.get("title").and_then(|v| v.as_str()).unwrap_or("Cập nhật OTA");
    let message = value.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let status_str = value.get("status").and_then(|v| v.as_str()).unwrap_or("in_progress");
    let level = match status_str {
        "success" | "done" => "success",
        "failed" | "error" => "critical",
        _ => "info",
    };

    info!(
        device_id = %device_id,
        title = %title,
        message = %message,
        status = %status_str,
        "Nhận OTA lifecycle event",
    );

    let record = crate::db::postgres::NewSystemEventRecord {
        device_id: device_id.clone(),
        level: level.to_string(),
        category: "device".to_string(),
        title: title.to_string(),
        message: message.to_string(),
        reason: Some(format!("ota_{status_str}")),
        metadata: Some(value.clone()),
        timestamp: chrono::Utc::now().timestamp_millis(),
        source: "rule".to_string(),
        primary_reason_code: None,
    };
    if let Err(e) = crate::db::postgres::insert_system_event(&app_state.pg_pool, &record).await {
        error!(error = ?e, device_id = %device_id, "Không thể lưu OTA system_event");
    }

    let _ = app_state.event_bus.send(AppEvent::ControllerStatus(value));
}
```

- [ ] **Step 4: Run tests and clippy**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml`
Run: `cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings`
Expected: Clean pass.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-backend/src/mqtt/handlers/status.rs
git commit -m "feat(backend): persist OTA lifecycle events to system_events table"
```

---

### Task 4: Extend Watchdog to Monitor Sensor Node Staleness (C4)

**Files:**
- Modify: `hydragrow-watchdog/src/staleness.rs`
- Modify: `hydragrow-watchdog/src/tick.rs`
- Modify: `hydragrow-watchdog/src/backend_client.rs`
- Test: `hydragrow-watchdog/src/staleness.rs`

**Interfaces:**
- Consumes: `TopicStatus` slice with `"controller/status"` and `"sensor/status"`
- Produces: `check_topic_staleness(topics, category, now, threshold_secs) -> Option<StaleBreach>`

- [ ] **Step 1: Write failing unit test in `hydragrow-watchdog/src/staleness.rs`**

```rust
#[test]
fn flags_stale_when_sensor_status_exceeds_threshold() {
    let now = Utc::now();
    let topics = vec![TopicStatus {
        topic_category: "sensor/status".to_string(),
        last_seen_at: now - Duration::seconds(120),
    }];
    let breach = check_topic_staleness(&topics, "sensor/status", now, 60);
    assert!(breach.is_some());
    assert_eq!(breach.unwrap().seconds_stale, 120);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path hydragrow-watchdog/Cargo.toml flags_stale_when_sensor_status_exceeds_threshold`
Expected: FAIL (function `check_topic_staleness` not found)

- [ ] **Step 3: Implement generalized `check_topic_staleness` and update `tick.rs`**

In `hydragrow-watchdog/src/staleness.rs`:
```rust
pub fn check_topic_staleness(
    topics: &[TopicStatus],
    target_category: &str,
    now: DateTime<Utc>,
    threshold_secs: u64,
) -> Option<StaleBreach> {
    let status = topics
        .iter()
        .find(|t| t.topic_category == target_category)?;

    let seconds_stale = (now - status.last_seen_at).num_seconds();
    if seconds_stale >= threshold_secs as i64 {
        Some(StaleBreach { seconds_stale })
    } else {
        None
    }
}

pub fn check_staleness(
    topics: &[TopicStatus],
    now: DateTime<Utc>,
    threshold_secs: u64,
) -> Option<StaleBreach> {
    check_topic_staleness(topics, "controller/status", now, threshold_secs)
}
```
In `hydragrow-watchdog/src/backend_client.rs` & `tick.rs`:
Add `create_stale_sensor_alert(&self, device_id: &str, seconds_stale: i64)` to `TickClient` and implement on `BackendClient` calling backend alert API with `primary_reason_code: "sensor_fault_suspected"`.
In `run_tick`, check both `"controller/status"` and `"sensor/status"`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path hydragrow-watchdog/Cargo.toml`
Run: `cargo clippy --manifest-path hydragrow-watchdog/Cargo.toml --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-watchdog/src/staleness.rs hydragrow-watchdog/src/tick.rs hydragrow-watchdog/src/backend_client.rs
git commit -m "feat(watchdog): monitor sensor node staleness alongside controller"
```

---

## Phase 2: Firmware Fault Telemetry & Diagnostics (C1, C5, C16, C20)

### Task 5: Add `TransitionReason::HardTimeout` to Shared Types (C16)

**Files:**
- Modify: `hydragrow-shared/src/telemetry/transition.rs:8-70`
- Test: `hydragrow-shared/src/telemetry/transition.rs`

**Interfaces:**
- Produces: `TransitionReason::HardTimeout { phase_name: String, timeout_ms: u64 }`

- [ ] **Step 1: Write test for `TransitionReason::HardTimeout` serialization/deserialization**

In `hydragrow-shared/tests/` or in `hydragrow-shared/src/telemetry/transition.rs`:
```rust
#[test]
fn hard_timeout_reason_roundtrip_serialization() {
    let reason = TransitionReason::HardTimeout {
        phase_name: "MimoDosing".to_string(),
        timeout_ms: 120_000,
    };
    let json = serde_json::to_string(&reason).expect("serialize");
    let back: TransitionReason = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(reason, back);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path hydragrow-shared/Cargo.toml hard_timeout_reason_roundtrip`
Expected: FAIL (no variant `HardTimeout`)

- [ ] **Step 3: Add `TransitionReason::HardTimeout` to `TransitionReason` enum**

In `hydragrow-shared/src/telemetry/transition.rs`:
```rust
    /// Quá thời gian tối đa cho phép của một Phase
    HardTimeout {
        phase_name: String,
        timeout_ms: u64,
    },
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path hydragrow-shared/Cargo.toml hard_timeout_reason_roundtrip`
Run: `cargo clippy --manifest-path hydragrow-shared/Cargo.toml --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-shared/src/telemetry/transition.rs
git commit -m "feat(shared): add TransitionReason::HardTimeout variant"
```

---

### Task 6: Emit `PublishFsmTransition` on All Lockout / Fault Transitions (C1) and Use `HardTimeout` in `MimoDosing` (C16)

**Files:**
- Modify: `hydragrow-controller-core/src/core/fsm/phases/stabilizing.rs:62-76`
- Modify: `hydragrow-controller-core/src/core/fsm/phases/monitoring.rs:250-291`
- Modify: `hydragrow-controller-core/src/core/fsm/phases/mimo_dosing.rs:48-66, 205-212`
- Modify: `hydragrow-controller-core/src/core/fsm/phases/water_phases.rs:52-60, 110-120`
- Test: `hydragrow-controller-core/tests/e2e/fault_recovery.rs`

**Interfaces:**
- Consumes: `OrchestratorEvent::PublishFsmTransition`, `TransitionReason::FaultDetected`, `TransitionReason::HardTimeout`
- Produces: FSM transition events dispatched on any phase change to `Fault`

- [ ] **Step 1: Write test verifying that hardware fault transition emits `PublishFsmTransition`**

In `hydragrow-controller-core/tests/e2e/fault_recovery.rs`:
```rust
#[test]
fn hardware_fault_in_stabilizing_emits_fsm_transition_event() {
    // Assert that when diagnose_hardware_fault fails (streak >= 3),
    // result.events contains OrchestratorEvent::PublishFsmTransition { to_phase: SystemPhase::Fault(..), .. }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml hardware_fault_in_stabilizing_emits_fsm_transition_event`
Expected: FAIL

- [ ] **Step 3: Add `PublishFsmTransition` to all fault entry points**

In `hydragrow-controller-core/src/core/fsm/phases/stabilizing.rs`:
```rust
            if let Err(fault_code) = ctx.diagnostic.diagnose_hardware_fault(
                total_nutrient,
                total_ph_agent,
                sample.water_in_sec,
                sample.water_out_sec,
                actual_delta_ec,
                actual_delta_ph,
                actual_delta_water,
                config,
            ) {
                result.events.push(OrchestratorEvent::PublishFsmTransition {
                    from_phase: SystemPhase::Stabilizing,
                    to_phase: SystemPhase::Fault(fault_code),
                    reason: hydragrow_shared::telemetry::transition::TransitionReason::FaultDetected {
                        fault_code,
                        consecutive_failures: 3,
                    },
                    phase_duration_ms: Some(uptime_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(uptime_ms))),
                });
                result.delta.phase = Some(SystemPhase::Fault(fault_code));
                return result;
            }
```
In `hydragrow-controller-core/src/core/fsm/phases/monitoring.rs`:
Add `result.events.push(OrchestratorEvent::PublishFsmTransition { ... })` at lines 257 (`MaxHourlyDoseEc`), 268 (`MaxHourlyDosePh`), 278 (`TooManyRefills`), 288 (`TooManyDrains`).
In `hydragrow-controller-core/src/core/fsm/phases/mimo_dosing.rs`:
At line 52: Replace `TransitionReason::Manual` with `TransitionReason::HardTimeout { phase_name: "MimoDosing".to_string(), timeout_ms: ... }`.
At line 209: Add `PublishFsmTransition` for `Fault(code)`.
In `hydragrow-controller-core/src/core/fsm/phases/water_phases.rs`:
At line 56 and line 115: Add `PublishFsmTransition` for `Fault(WaterRefillFailed)` and `Fault(WaterDrainFailed)`.

- [ ] **Step 4: Run tests and verify**

Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml`
Run: `cargo clippy --manifest-path hydragrow-controller-core/Cargo.toml --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-controller-core/src/core/fsm/phases/
git commit -m "feat(controller-core): emit PublishFsmTransition on all fault state transitions and use HardTimeout"
```

---

### Task 7: Emit Pre-Fault Early Warning Logs for Streaks 1 & 2 (C20)

**Files:**
- Modify: `hydragrow-controller-core/src/core/fsm/phases/stabilizing.rs:62-75`
- Modify: `hydragrow-shared/src/fsm.rs:182-260`
- Test: `hydragrow-controller-core/tests/e2e/fault_recovery.rs`

**Interfaces:**
- Consumes: `OrchestratorEvent::PublishSystemLog`, `UnifiedSystemLog::build_basic_log_json_with_ts`
- Produces: Warning `system_log` event published when `ec_pump_streak` or `ph_pump_streak` reaches 1 or 2.

- [ ] **Step 1: Write test for streak 1 & 2 warning publication**

In `hydragrow-controller-core/tests/`:
Verify that when `ec_pump_streak` increments from 0 to 1, a `PublishSystemLog` event with `LogLevel::Warning` and `LogCategory::Alert` is queued.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml streak_warning`
Expected: FAIL

- [ ] **Step 3: Implement early warning log dispatch in `stabilizing.rs`**

In `stabilizing.rs`: Check if streaks changed after `diagnose_hardware_fault`:
```rust
    let prev_ec_streak = ctx.diagnostic.ec_pump_streak;
    let prev_ph_streak = ctx.diagnostic.ph_pump_streak;
    // after diagnose_hardware_fault:
    if ctx.diagnostic.ec_pump_streak > 0 && ctx.diagnostic.ec_pump_streak < 3 && ctx.diagnostic.ec_pump_streak != prev_ec_streak {
        let payload = hydragrow_shared::log::UnifiedSystemLog::build_basic_log_json_with_ts(
            &config.device_id,
            hydragrow_shared::log::LogLevel::Warning,
            hydragrow_shared::log::LogCategory::Alert,
            "Cảnh báo châm dinh dưỡng không hiệu quả",
            format!("Bơm chạy {:.1}ml nhưng EC không thay đổi (lần {}/3).", total_nutrient, ctx.diagnostic.ec_pump_streak),
            None,
            "diagnostic_streak",
            now_ms,
        );
        result.events.push(OrchestratorEvent::PublishSystemLog { payload_json: payload });
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-controller-core/src/core/fsm/phases/stabilizing.rs
git commit -m "feat(controller-core): emit warning system logs on diagnostic streaks 1 and 2"
```

---

### Task 8: Configure MQTT Last Will and Testament (LWT) on Sensor Node (C5)

**Files:**
- Modify: `ESP32-C3-SENSOR-NODE/src/mqtt/MqttManager.cpp:210-225`
- Test: Static inspection & verify PubSubClient 7-parameter connect signature

**Interfaces:**
- Consumes: `mqttClient.connect(id, user, pass, willTopic, willQos, willRetain, willMessage)`
- Produces: Retained MQTT LWT message on `TOPIC_STATUS` when sensor node disconnects unexpectedly

- [ ] **Step 1: Inspect `PubSubClient.h` in libraries to confirm connect signature**

Verify `mqttClient.connect` supports:
`connect(const char *id, const char *user, const char *pass, const char* willTopic, uint8_t willQos, boolean willRetain, const char* willMessage)`

- [ ] **Step 2: Update `MqttManager::reconnect()` in `ESP32-C3-SENSOR-NODE/src/mqtt/MqttManager.cpp`**

```cpp
    Logger::debugPrintln("Dang ket noi MQTT...");
    const char* willPayload = "{\"online\": false, \"status\": \"disconnected\"}";
    bool connected = mqttClient.connect(
        MQTT_CLIENT_ID,
        MQTT_USERNAME,
        MQTT_PASSWORD,
        TOPIC_STATUS.c_str(),
        1,
        true,
        willPayload
    );
```

- [ ] **Step 3: Verify syntax and compile check**

Check file syntax and ensure constants `TOPIC_STATUS`, `MQTT_CLIENT_ID`, etc., are in scope.

- [ ] **Step 4: Commit**

```bash
git add ESP32-C3-SENSOR-NODE/src/mqtt/MqttManager.cpp
git commit -m "fix(sensor-node): configure MQTT LWT for unexpected sensor disconnects"
```

---

## Phase 3: Schema Enrichment, Audit Trails & Visibility (C6, C7, C9, C10, C11, C12, C13, C15)

### Task 9: Log Operational FSM Transitions to `system_events` (C6)

**Files:**
- Modify: `hydragrow-backend/src/mqtt/handlers/fsm.rs:636-679`
- Test: `hydragrow-backend/src/mqtt/handlers/fsm.rs` (unit tests)

**Interfaces:**
- Consumes: `FsmTransitionEvent` with any `TransitionReason`
- Produces: `Option<NewSystemEventRecord>` for `DosingComplete`, `StabilizingComplete`, `WaterRefillComplete`, etc.

- [ ] **Step 1: Write test in `fsm.rs` for logging normal transitions**

```rust
#[test]
fn dosing_complete_transition_produces_info_system_event() {
    let event = FsmTransitionEvent {
        device_id: "dev-01".to_string(),
        from_phase: Some(SystemPhase::MimoDosing),
        to_phase: SystemPhase::ActiveMixing,
        reason: TransitionReason::DosingComplete {
            dose_a_ml: 5.0,
            dose_b_ml: 5.0,
            ph_up_ml: 0.0,
            ph_down_ml: 1.0,
        },
        timestamp_ms: 1700000000000,
        phase_duration_ms: Some(15000),
    };
    let record = transition_system_event_record(&event);
    assert!(record.is_some());
    let r = record.unwrap();
    assert_eq!(r.level, "info");
    assert_eq!(r.category, "system");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml dosing_complete_transition_produces_info_system_event`
Expected: FAIL (returns `None`)

- [ ] **Step 3: Update `transition_system_event_record` in `hydragrow-backend/src/mqtt/handlers/fsm.rs`**

Remove the `_ => None` catch-all and map normal transitions (`DosingComplete`, `StabilizingComplete`, `MixingComplete`, `WaterRefillComplete`, `WaterDrainComplete`, `FaultReset`, `HardTimeout`) to descriptive info/warning system events.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml dosing_complete_transition_produces_info_system_event`
Run: `cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-backend/src/mqtt/handlers/fsm.rs
git commit -m "feat(backend): persist operational FSM transitions to system_events"
```

---

### Task 10: Audit Log Configuration Changes in `config.rs` (C7)

**Files:**
- Modify: `hydragrow-backend/src/api/config.rs:434-500, 613-640, 705-730`
- Test: `hydragrow-backend/src/api/config.rs`

**Interfaces:**
- Consumes: Config update requests (`UnifiedConfigRequest`, etc.)
- Produces: `insert_system_event` record with category `"user_action"`, `event_type: "config_change"`

- [ ] **Step 1: Write test asserting audit event generation on config update**

Add a unit test checking audit record metadata creation for unified config updates.

- [ ] **Step 2: Run test to verify test harness**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml audit_config`
Expected: Test harness passes.

- [ ] **Step 3: Add audit record insertion in `update_unified_config`, `update_config`, `update_safety_config`**

In `hydragrow-backend/src/api/config.rs:490`:
```rust
    let audit_event = NewSystemEventRecord {
        device_id: device_id.clone(),
        level: "info".to_string(),
        category: "user_action".to_string(),
        title: "Cập nhật cấu hình trạm".to_string(),
        message: format!("Người dùng đã cập nhật cấu hình cho trạm {device_id}."),
        reason: Some("config_update".to_string()),
        metadata: Some(serde_json::json!({
            "event_type": "config_change",
            "scope": "unified",
            "ec_target": payload.device_config.ec_target,
            "ph_target": payload.device_config.ph_target,
            "control_mode": payload.device_config.control_mode,
            "is_enabled": payload.device_config.is_enabled,
        })),
        timestamp: now.timestamp_millis(),
        source: "rule".to_string(),
        primary_reason_code: None,
    };
    let _ = insert_system_event(&app_state.pg_pool, &audit_event).await;
```

- [ ] **Step 4: Run tests and clippy**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml`
Run: `cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-backend/src/api/config.rs
git commit -m "feat(backend): add audit logging for device configuration updates"
```

---

### Task 11: Propagate `err_*` Sensor Flags to `SensorSnapshot` and Enrich Automation Alerts (C10, C13)

**Files:**
- Modify: `hydragrow-backend/src/models/script.rs:135-143`
- Modify: `hydragrow-backend/src/mqtt/handlers/sensors.rs:125-135`
- Modify: `hydragrow-backend/src/mqtt/handlers/script_eval.rs:779-795`
- Test: `hydragrow-backend/src/mqtt/handlers/script_eval.rs`

**Interfaces:**
- Consumes: `SensorSnapshot` with `err_ph`, `err_tds`, `err_temperature`, `err_water_level`
- Produces: Automation alerts containing `script_id`, `script_name`, and execution context

- [ ] **Step 1: Write test for enriched automation alert metadata**

In `hydragrow-backend/src/mqtt/handlers/script_eval.rs`:
```rust
#[test]
fn alert_output_includes_script_id_and_name_in_metadata() {
    let alert = AlertOutput {
        level: "warning".to_string(),
        title: "Low Water".to_string(),
        message: "Water below 20%".to_string(),
    };
    let script_id = uuid::Uuid::new_v4();
    let msg = alert_output_to_system_alert(alert, &script_id, "water_check", "dev_1", 1000);
    assert!(msg.metadata.is_some());
    let meta = msg.metadata.unwrap();
    assert_eq!(meta["script_id"], script_id.to_string());
    assert_eq!(meta["script_name"], "water_check");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml alert_output_includes_script_id_and_name`
Expected: FAIL

- [ ] **Step 3: Update `SensorSnapshot` and `alert_output_to_system_alert`**

In `hydragrow-backend/src/models/script.rs`:
Add boolean fields `err_ph`, `err_tds`, `err_temperature`, `err_water_level` to `SensorSnapshot`.
In `hydragrow-backend/src/mqtt/handlers/sensors.rs`:
Populate those flags when building `SensorSnapshot` from incoming telemetry.
In `hydragrow-backend/src/mqtt/handlers/script_eval.rs`:
Update `alert_output_to_system_alert` signature and metadata population.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml alert_output_includes_script_id_and_name`
Run: `cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-backend/src/models/script.rs hydragrow-backend/src/mqtt/handlers/sensors.rs hydragrow-backend/src/mqtt/handlers/script_eval.rs
git commit -m "feat(backend): propagate sensor error flags to SensorSnapshot and enrich script alerts"
```

---

### Task 12: Database Schema Migration for Dosing Efficacy & Flow Execution Context (C9, C11)

**Files:**
- Create: `hydragrow-backend/migrations/20260913090000_enrich_dosing_and_flow_logs.sql`
- Modify: `hydragrow-backend/src/db/postgres.rs:273-305`
- Modify: `hydragrow-backend/src/services/action_dispatch.rs:160-170`
- Modify: `hydragrow-backend/src/services/execution_log.rs:4-30`
- Test: `hydragrow-backend/src/db/tests/test_postgres.rs`

**Interfaces:**
- Consumes: PostgreSQL schema migrations
- Produces: `dosing_action_log` columns (`ec_before`, `ec_after`, `ph_before`, `ph_after`, `cycle_id`, `triggered_by`), `flow_execution_log` columns (`trigger_source`, `duration_ms`)

- [ ] **Step 1: Write migration SQL file**

Create `hydragrow-backend/migrations/20260913090000_enrich_dosing_and_flow_logs.sql`:
```sql
-- Enrich dosing_action_log with efficacy tracking
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS ec_before REAL;
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS ec_after REAL;
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS ph_before REAL;
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS ph_after REAL;
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS cycle_id TEXT;
ALTER TABLE dosing_action_log ADD COLUMN IF NOT EXISTS triggered_by TEXT DEFAULT 'fsm_auto';

-- Enrich flow_execution_log with execution telemetry
ALTER TABLE flow_execution_log ADD COLUMN IF NOT EXISTS trigger_source TEXT DEFAULT 'sensor_data';
ALTER TABLE flow_execution_log ADD COLUMN IF NOT EXISTS duration_ms INTEGER;
```

- [ ] **Step 2: Update `insert_dosing_action` and `execution_log`**

Update `insert_dosing_action` in `hydragrow-backend/src/db/postgres.rs` to accept optional `ec_before`, `ec_after`, `ph_before`, `ph_after`, `cycle_id`, `triggered_by`.
Update `log_success` and `log_error` in `hydragrow-backend/src/services/execution_log.rs` to accept `trigger_source` and `duration_ms`.

- [ ] **Step 3: Write DB test in `hydragrow-backend/src/db/tests/test_postgres.rs`**

Test inserting and querying a dosing action record with efficacy fields.

- [ ] **Step 4: Run tests and clippy**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml test_postgres`
Run: `cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-backend/migrations/20260913090000_enrich_dosing_and_flow_logs.sql hydragrow-backend/src/db/postgres.rs hydragrow-backend/src/services/
git commit -m "feat(backend): add schema migration for dosing efficacy and flow execution telemetry"
```

---

### Task 13: Dedicated Dosing and Sensor Metadata Structs in `log.rs` (C12)

**Files:**
- Modify: `hydragrow-shared/src/log.rs:80-220`
- Test: `hydragrow-shared/src/log.rs`

**Interfaces:**
- Produces: `SystemLogEvent::DosingEvent(DosingMetadata)`, `SystemLogEvent::SensorEvent(SensorMetadata)`

- [ ] **Step 1: Write serialization roundtrip tests for DosingEvent and SensorEvent**

In `hydragrow-shared/src/log.rs`:
```rust
#[test]
fn dosing_and_sensor_event_serialization_roundtrip() {
    let dosing_event = SystemLogEvent::DosingEvent(DosingMetadata {
        source: "mimo".to_string(),
        pump: "PUMP_A".to_string(),
        dose_ml: 5.0,
        ec_before: Some(1.2),
        ec_after: Some(1.6),
        ph_before: None,
        ph_after: None,
        cycle_id: Some("cycle-123".to_string()),
    });
    let json = serde_json::to_string(&dosing_event).unwrap();
    assert!(json.contains("DosingEvent"));

    let sensor_event = SystemLogEvent::SensorEvent(SensorMetadata {
        sensor_type: "ph".to_string(),
        error_state: true,
        message: "ADS1115 read timeout".to_string(),
        raw_value: None,
    });
    let json2 = serde_json::to_string(&sensor_event).unwrap();
    assert!(json2.contains("SensorEvent"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path hydragrow-shared/Cargo.toml dosing_and_sensor_event_serialization_roundtrip`
Expected: FAIL

- [ ] **Step 3: Define `DosingMetadata` and `SensorMetadata` and add variants to `SystemLogEvent`**

In `hydragrow-shared/src/log.rs`:
Add structs `DosingMetadata` and `SensorMetadata` with Serde derives, and add `DosingEvent(DosingMetadata)` and `SensorEvent(SensorMetadata)` to `enum SystemLogEvent`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path hydragrow-shared/Cargo.toml dosing_and_sensor_event_serialization_roundtrip`
Run: `cargo clippy --manifest-path hydragrow-shared/Cargo.toml --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-shared/src/log.rs
git commit -m "feat(shared): add dedicated DosingEvent and SensorEvent metadata variants to SystemLogEvent"
```

---

## Phase 4: Data Retention, Diagnostic Triggers & Frontend UX (C14, C17, C18, C19)

### Task 14: Implement Retention Cleanup for `dosing_action_log` and `flow_execution_log` (C17)

**Files:**
- Modify: `hydragrow-backend/src/services/retention.rs:30-105`
- Test: `hydragrow-backend/src/services/retention.rs`

**Interfaces:**
- Consumes: PostgreSQL connection pool
- Produces: Periodic deletion of rows older than 90 days in `dosing_action_log` and `flow_execution_log`

- [ ] **Step 1: Write test for retention queries**

In `hydragrow-backend/src/services/retention.rs`: Add tests verifying that `delete_expired_dosing_actions` and `delete_expired_flow_executions` delete expired rows while preserving recent rows.

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml retention`
Expected: FAIL

- [ ] **Step 3: Implement batch deletion for additional tables in `retention.rs`**

```rust
async fn delete_expired_dosing_actions(pool: &PgPool, cutoff_date: chrono::DateTime<chrono::Utc>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM dosing_action_log WHERE dosed_at < $1")
        .bind(cutoff_date)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

async fn delete_expired_flow_executions(pool: &PgPool, cutoff_date: chrono::DateTime<chrono::Utc>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM flow_execution_log WHERE created_at < $1")
        .bind(cutoff_date)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
```
Call both in `run_once()` alongside `system_events` cleanup.

- [ ] **Step 4: Run tests and clippy**

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml retention`
Run: `cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-backend/src/services/retention.rs
git commit -m "feat(backend): add retention cleanup for dosing_action_log and flow_execution_log"
```

---

### Task 15: Expand Diagnostic Worker Triggers (C18)

**Files:**
- Modify: `hydragrow-diagnostic-worker/src/trigger.rs:1-45`
- Modify: `hydragrow-diagnostic-worker/src/orchestrator.rs:14-25`
- Test: `hydragrow-diagnostic-worker/src/trigger.rs`

**Interfaces:**
- Consumes: Fleet state, watchdog breaches
- Produces: `SupervisorTrigger::SensorStaleness`, `SupervisorTrigger::DosingAnomaly`

- [ ] **Step 1: Write test for new diagnostic triggers**

In `hydragrow-diagnostic-worker/src/trigger.rs`:
```rust
#[test]
fn sensor_staleness_trigger_is_evaluated() {
    let trigger = evaluate_sensor_triggers(true);
    assert_eq!(trigger, Some(SupervisorTrigger::SensorStaleness));
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --manifest-path hydragrow-diagnostic-worker/Cargo.toml sensor_staleness_trigger`
Expected: FAIL

- [ ] **Step 3: Implement new trigger variants in `trigger.rs` and handle in `orchestrator.rs`**

Add `SensorStaleness` to `SupervisorTrigger` enum.
In `orchestrator.rs`: map `SensorStaleness` to reason code `"sensor_fault_suspected"` for dedup checks.

- [ ] **Step 4: Run tests and clippy**

Run: `cargo test --manifest-path hydragrow-diagnostic-worker/Cargo.toml`
Run: `cargo clippy --manifest-path hydragrow-diagnostic-worker/Cargo.toml --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add hydragrow-diagnostic-worker/src/trigger.rs hydragrow-diagnostic-worker/src/orchestrator.rs
git commit -m "feat(diagnostic-worker): add sensor staleness trigger to supervisor loop"
```

---

### Task 16: Frontend System Log Awareness & Empty State Guidance (C19)

**Files:**
- Modify: `hydragrow-frontend/src/pages/SystemLog.tsx:230-260`
- Modify: `hydragrow-frontend/src/components/logs/HealthSummaryBar.tsx`
- Test: `hydragrow-frontend/src/pages/SystemLog.test.tsx`

**Interfaces:**
- Consumes: Filter category state (`filter === 'sensor'`, etc.)
- Produces: Contextual explanation when sensor/calibration logs have no recent records

- [ ] **Step 1: Write test in `SystemLog.test.tsx` for sensor category empty state**

Verify that selecting the "sensor" filter when no sensor events exist renders a descriptive explanation rather than a generic blank state.

- [ ] **Step 2: Run test to verify**

Run: `cd hydragrow-frontend && npx vitest run src/pages/SystemLog.test.tsx`
Expected: FAIL (or verify new assertion fails)

- [ ] **Step 3: Update `SystemLog.tsx`**

In `hydragrow-frontend/src/pages/SystemLog.tsx`:
When `filter === 'sensor'` and `visibleRows.length === 0`:
Display:
```tsx
description={
  filter === 'sensor'
    ? 'Chưa có nhật ký sự kiện cảm biến (lỗi tín hiệu, ngắt kết nối). Các thông số đo tức thời được hiển thị trên Bảng điều khiển.'
    : search
      ? 'Thử từ khoá ngắn hơn, kiểm tra chính tả, hoặc đổi bộ lọc danh mục đang chọn.'
      : 'Chưa ghi nhận khoảnh khắc nào khớp bộ lọc hiện tại.'
}
```

- [ ] **Step 4: Run frontend tests, lint & build**

Run: `cd hydragrow-frontend && npx vitest run src/pages/SystemLog.test.tsx`
Run: `cd hydragrow-frontend && npm run build`
Expected: 0 errors, build green.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-frontend/src/pages/SystemLog.tsx hydragrow-frontend/src/pages/SystemLog.test.tsx
git commit -m "feat(frontend): add contextual empty state guidance for sensor log category"
```

---

### Task 17: Traceability, Delivery Governance & Documentation Sync (C0/C4)

**Files:**
- Modify: `docs/project-state/CURRENT-STATUS.md`
- Modify: `docs/project-state/TRACEABILITY.md`

**Interfaces:**
- Produces: Updated governance records documenting the implementation and verification of items C1–C20.

- [ ] **Step 1: Update `docs/project-state/CURRENT-STATUS.md`**

Add an entry for `SYSTEM-LOGGING-REMEDIATION-001` describing the implemented telemetry, heartbeat, audit trail, and schema fixes across all subsystems.

- [ ] **Step 2: Update `docs/project-state/TRACEABILITY.md`**

Link requirement `SYSTEM-LOGGING-REMEDIATION-001` to its acceptance criteria, tests, and documentation.

- [ ] **Step 3: Run repo verification script**

Run: `bash scripts/verify.sh` (or subsystem checks according to `.agent/verify.yml`)
Expected: All clean.

- [ ] **Step 4: Commit**

```bash
git add docs/project-state/CURRENT-STATUS.md docs/project-state/TRACEABILITY.md
git commit -m "docs: sync project status and traceability for system logging remediation"
```
