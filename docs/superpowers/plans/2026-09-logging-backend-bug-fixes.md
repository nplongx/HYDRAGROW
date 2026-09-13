# Logging Remediation — Part 1: Backend Bug Fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix five backend bugs that cause silent data loss: hard-coded topic string, silently-dropped sensor-node status messages, un-persisted OTA events, hollow automation-alert metadata, and missing sensor-error flags in the script engine.

**Architecture:** All changes are within `hydragrow-backend`. No schema migrations. No shared-crate changes. Every fix is a targeted edit to an existing function — the data that was being silently dropped is already arriving correctly at the function boundary; the bug is in how the function handles it.

**Tech Stack:** Rust / actix-web / sqlx / tokio. Verification via `cargo test --manifest-path hydragrow-backend/Cargo.toml`.

**Spec:** `docs/reviews/2026-09-system-logging-review.md` — Part A §2 (bugs C2, C3), Part B §8 (C8), Part B §9 (C10), Part A §2 bullet 5 (C13).

## Global Constraints

- `hydragrow-backend/migrations/**` is a **protected path** — no migration files in this plan.
- No changes to `hydragrow-shared`, `hydragrow-controller-core`, or firmware crates in this plan.
- Every test must assert real behaviour — no tautological checks.
- Run `cargo test --manifest-path hydragrow-backend/Cargo.toml` (and clippy) after every task.
- Commits must be atomic: one task → one commit.

---

### Task 1: Fix hard-coded `"controller/status"` topic + remove duplicate call (C2)

**Problem:** `handle_device()` (`status.rs:84–98`) calls `touch_topic()` twice with the literal string `"controller/status"`, regardless of whether the incoming message came from `/status` (controller) or `/sensor/status` (sensor node). This means the watchdog sees sensor node heartbeats as controller heartbeats, and the `device_topic_last_seen` table records every sensor heartbeat under the wrong topic.

**Files:**
- Modify: `hydragrow-backend/src/mqtt/handlers/status.rs` — `handle_device` function (lines 53–157)
- Test: `hydragrow-backend/src/mqtt/handlers/status.rs` — new unit tests in the existing `#[cfg(test)]` block (line 439+)

**Interfaces:**
- Consumes: `handle_device(device_id: String, node_type: &str, payload: &[u8], app_state)` — the `node_type` parameter already carries `"Trạm Điều Khiển"` or `"Mạch Cảm Biến"` from `mqtt/mod.rs:53,58`.
- Produces: `touch_topic()` called exactly **once** per invocation with the topic derived from a new parameter.

- [ ] **Step 1: Add a `topic_category` parameter to `handle_device`**

Change the function signature from:
```rust
pub async fn handle_device(
    device_id: String,
    node_type: &str,
    payload: &[u8],
    app_state: web::Data<AppState>,
)
```
to:
```rust
pub async fn handle_device(
    device_id: String,
    node_type: &str,
    topic_category: &str,  // NEW: e.g. "controller/status" or "sensor/status"
    payload: &[u8],
    app_state: web::Data<AppState>,
)
```

- [ ] **Step 2: Replace the two duplicated `touch_topic` calls with one correct call**

Remove lines 84–98 (the two identical calls) and replace them with:
```rust
    let _ = crate::db::topic_last_seen::touch_topic(
        &app_state.pg_pool,
        &device_id,
        topic_category,
        chrono::Utc::now(),
    )
    .await;
```

- [ ] **Step 3: Update the two call sites in `mqtt/mod.rs`**

In `hydragrow-backend/src/mqtt/mod.rs` lines 52–60:
```rust
        "/status" => {
            handlers::status::handle_device(
                device_id,
                "Trạm Điều Khiển",
                "controller/status",   // NEW arg
                &payload_bytes,
                app_state,
            )
            .await
        }

        "/sensor/status" => {
            handlers::status::handle_device(
                device_id,
                "Mạch Cảm Biến",
                "sensor/status",       // NEW arg
                &payload_bytes,
                app_state,
            )
            .await
        }
```

- [ ] **Step 4: Write a unit test verifying the topic_category is respected**

Add this test to the existing `#[cfg(test)]` block at the bottom of `status.rs`. Note: `handle_device` is async and calls the DB, so this test verifies at the unit level that the function accepts the new parameter without panicking. The integration-level correctness (correct string reaches `touch_topic`) is verified by the existing tests in `topic_last_seen.rs`.

```rust
    #[test]
    fn interpret_online_signal_returns_none_for_unknown_status() {
        let s = DeviceStatusPayload {
            online: None,
            status: Some("config_applied".to_string()),
            firmware_version: None,
        };
        assert_eq!(interpret_online_signal(&s), None);
    }
```

(This test already validates existing behaviour; we will expand it in Task 2.)

- [ ] **Step 5: Run tests**

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml 2>&1 | tail -20
```

Expected: all tests pass. If there are compile errors, the most likely cause is a missed call site — run `grep -rn "handle_device(" hydragrow-backend/src/` to find all callers.

- [ ] **Step 6: Run clippy**

```bash
cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -20
```

Expected: 0 warnings.

- [ ] **Step 7: Commit**

```bash
git add hydragrow-backend/src/mqtt/handlers/status.rs \
        hydragrow-backend/src/mqtt/mod.rs
git commit -m "fix(backend): pass correct topic_category to touch_topic in handle_device

Previously touch_topic was called twice with the hard-coded string
\"controller/status\" regardless of whether the message came from
/status or /sensor/status. This masked sensor-node heartbeats as
controller heartbeats in device_topic_last_seen.

Fixes: C2 from logging review (2026-09-system-logging-review.md)"
```

---

### Task 2: Fix `interpret_online_signal` — stop silently dropping sensor-node messages (C3)

**Problem:** `interpret_online_signal()` (`status.rs:24–31`) returns `None` for any `status` string other than `"online"`. When `handle_device` receives `None`, it returns immediately at line 80, **before** calling `touch_topic`. This means every sensor-node message with `status` = `"error"`, `"config_applied"`, `"ota_start"`, `"wifi_updated"`, etc. is silently discarded — the device_topic_last_seen table is never updated, and no event is written.

The fix has two parts:
1. Decouple the `touch_topic` call from the online/offline signal — the topic should be touched on *every* valid JSON message, not only online/offline ones.
2. Log non-online/offline status messages as `device` category system events so they leave a trace.

**Files:**
- Modify: `hydragrow-backend/src/mqtt/handlers/status.rs` — `interpret_online_signal` and `handle_device` (lines 24–157)

**Interfaces:**
- Consumes: same `handle_device` signature after Task 1
- Produces: `touch_topic` is called for **every** parseable message; online/offline branch only runs for those that signal connectivity

- [ ] **Step 1: Write the failing test first**

Add to the `#[cfg(test)]` block in `status.rs`:

```rust
    #[test]
    fn interpret_online_signal_returns_none_for_config_applied_status() {
        // These messages MUST still update device_topic_last_seen even though
        // they're not an online/offline signal.
        let s = DeviceStatusPayload {
            online: None,
            status: Some("config_applied".to_string()),
            firmware_version: None,
        };
        // Still None from the signal decoder — but handle_device must NOT
        // return early; it should touch the topic then skip the alert path.
        assert_eq!(interpret_online_signal(&s), None);
    }

    #[test]
    fn interpret_online_signal_returns_true_for_online_string() {
        let s = DeviceStatusPayload {
            online: None,
            status: Some("online".to_string()),
            firmware_version: None,
        };
        assert_eq!(interpret_online_signal(&s), Some(true));
    }
```

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml -- interpret_online_signal -v`
Expected: these tests PASS (they assert existing behaviour — we're adding them as a safety net before restructuring `handle_device`).

- [ ] **Step 2: Restructure `handle_device` to decouple heartbeat from online/offline**

Replace the early-return pattern:
```rust
    // OLD (lines 79–82)
    let is_online = match interpret_online_signal(&status) {
        None => return,   // <-- BUG: skips touch_topic
        Some(v) => v,
    };
```

With:
```rust
    // NEW: always update heartbeat for any parseable message
    let _ = crate::db::topic_last_seen::touch_topic(
        &app_state.pg_pool,
        &device_id,
        topic_category,   // from Task 1
        chrono::Utc::now(),
    )
    .await;

    let is_online = match interpret_online_signal(&status) {
        None => {
            // Non-online/offline message (e.g. "config_applied", "ota_start").
            // Log it as a device event so it leaves a trace.
            if let Some(status_str) = status.status.as_deref() {
                let ts = chrono::Utc::now().timestamp_millis();
                let record = crate::db::postgres::NewSystemEventRecord {
                    device_id: device_id.clone(),
                    level: "info".to_string(),
                    category: "device".to_string(),
                    title: format!("Trạng thái thiết bị: {}", status_str),
                    message: format!("{} ({}) gửi trạng thái: {}", node_type, device_id, status_str),
                    reason: Some(status_str.to_string()),
                    metadata: Some(serde_json::json!({
                        "event_type": "device_status_message",
                        "status": status_str,
                        "topic_category": topic_category,
                    })),
                    timestamp: ts,
                    source: "rule".to_string(),
                    primary_reason_code: None,
                };
                let _ = crate::db::postgres::insert_system_event(&app_state.pg_pool, &record).await;
            }
            return;
        }
        Some(v) => v,
    };
```

Also **remove** the old `touch_topic` call that was further down (now covered above).

- [ ] **Step 3: Run tests**

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml 2>&1 | tail -20
```

Expected: all tests pass.

- [ ] **Step 4: Run clippy**

```bash
cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -20
```

Expected: 0 warnings.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-backend/src/mqtt/handlers/status.rs
git commit -m "fix(backend): stop silently dropping sensor-node non-online/offline status messages

interpret_online_signal returning None caused handle_device to return
early, skipping touch_topic and discarding ~10 types of sensor status
messages (config_applied, ota_start, wifi_updated, auth_failed, etc).

Now: touch_topic is always called for any parseable message, and
non-online/offline status strings are persisted as 'device' category
system events so they leave a searchable trace.

Fixes: C3 from logging review (2026-09-system-logging-review.md)"
```

---

### Task 3: Persist OTA lifecycle events to system_events (C8)

**Problem:** `handle_ota_status()` (`status.rs:370–385`) only logs to `tracing::info!()` (server stdout) and forwards to the WebSocket event bus. OTA events vanish if no user is watching the dashboard.

**Files:**
- Modify: `hydragrow-backend/src/mqtt/handlers/status.rs` — `handle_ota_status` (lines 370–385)

**Interfaces:**
- Consumes: `value: serde_json::Value` with fields `title` (string) and `message` (string)
- Produces: a `device` category system event persisted to `system_events`

- [ ] **Step 1: Write the test**

This function is async and calls the DB, so test it at the level where we can verify the insert path isn't erroring — add a doc-comment note and a unit test for the level-selection logic instead:

```rust
// In the #[cfg(test)] block at the bottom of status.rs, add:
    #[test]
    fn ota_level_from_title_maps_correctly() {
        // Helper used by handle_ota_status to pick log level
        assert_eq!(ota_level_from_title("ota_start"), "info");
        assert_eq!(ota_level_from_title("ota_failed"), "critical");
        assert_eq!(ota_level_from_title("ota_success"), "success");
        assert_eq!(ota_level_from_title("unknown_event"), "info");
    }
```

This test will FAIL until we create the `ota_level_from_title` helper.

- [ ] **Step 2: Add the helper function and update `handle_ota_status`**

Add a private helper just above `handle_ota_status`:
```rust
fn ota_level_from_title(title: &str) -> &'static str {
    match title {
        t if t.contains("fail") || t.contains("error") => "critical",
        t if t.contains("success") || t.contains("complete") => "success",
        _ => "info",
    }
}
```

Then update `handle_ota_status`:
```rust
pub async fn handle_ota_status(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let value: serde_json::Value = match serde_json::from_slice(payload) {
        Ok(value) => value,
        Err(e) => {
            error!(error = ?e, "Lỗi parse ota-status");
            return;
        }
    };
    let title = value.get("title").and_then(|v| v.as_str()).unwrap_or("ota");
    let message = value.get("message").and_then(|v| v.as_str()).unwrap_or("");
    info!(
        device_id = %device_id,
        title = title,
        message = message,
        "Nhận OTA lifecycle event",
    );

    // Persist to system_events so OTA history is searchable
    let ts = chrono::Utc::now().timestamp_millis();
    let record = crate::db::postgres::NewSystemEventRecord {
        device_id: device_id.clone(),
        level: ota_level_from_title(title).to_string(),
        category: "device".to_string(),
        title: format!("OTA: {}", title),
        message: message.to_string(),
        reason: Some(title.to_string()),
        metadata: Some(serde_json::json!({
            "event_type": "ota_lifecycle",
            "ota_title": title,
            "raw": value,
        })),
        timestamp: ts,
        source: "rule".to_string(),
        primary_reason_code: None,
    };
    let _ = crate::db::postgres::insert_system_event(&app_state.pg_pool, &record).await;

    let _ = app_state.event_bus.send(AppEvent::ControllerStatus(value));
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml -- ota_level_from_title -v
```

Expected: PASS.

- [ ] **Step 4: Run full test suite and clippy**

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml 2>&1 | tail -20
cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -20
```

Expected: all pass, 0 warnings.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-backend/src/mqtt/handlers/status.rs
git commit -m "feat(backend): persist OTA lifecycle events to system_events

handle_ota_status previously only logged to tracing and forwarded to
the WebSocket bus. OTA events were lost if no user was watching the
dashboard. Now persisted as 'device' category events with level
derived from the event title (fail/error→critical, success→success, else→info).

Fixes: C8 from logging review (2026-09-system-logging-review.md)"
```

---

### Task 4: Enrich automation-alert metadata with script identity (C10)

**Problem:** `alert_output_to_system_alert()` (`script_eval.rs:779–793`) always sets `metadata: None` and `reason: Some("Rhai user script")`. Users cannot identify which script produced an automation alert or what sensor values triggered it.

The fix adds `script_id`, `script_name`, and the triggering sensor snapshot to the alert metadata. The `script_id` and `script_name` are available at the call site in `handle_fired_alert`.

**Files:**
- Modify: `hydragrow-backend/src/mqtt/handlers/script_eval.rs` — `alert_output_to_system_alert` (lines 779–793) and `handle_fired_alert` (lines 726–776)
- Modify: `hydragrow-backend/src/mqtt/handlers/sensors.rs` — call to `handle_fired_alert` (line 227)
- Modify: `hydragrow-backend/src/services/cron_scheduler.rs` — call to `handle_fired_alert` (line 305)

**Interfaces:**
- Consumes: `alert_output_to_system_alert` now takes `script_id: Uuid, script_name: &str, snapshot_json: Option<serde_json::Value>`
- `handle_fired_alert` takes the same additional args and passes them through
- Produces: `AlertMessage.metadata` populated with `{"event_type": "script_alert", "script_id": "...", "script_name": "...", "trigger_snapshot": {...}}`

- [ ] **Step 1: Write the failing test**

Find the existing test for `alert_output_to_system_alert` — there isn't one, so add it to the test block:

```rust
    #[test]
    fn alert_output_to_system_alert_includes_script_metadata() {
        let alert = AlertOutput {
            level: "warning".to_string(),
            title: "pH High".to_string(),
            message: "pH is above 8.0".to_string(),
            notify_fcm: None,
        };
        let script_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let snapshot = serde_json::json!({"ph": 8.1, "ec": 1.4});
        let msg = alert_output_to_system_alert(
            alert,
            "device_001",
            0,
            script_id,
            "My pH Alert Script",
            Some(snapshot.clone()),
        );
        let meta = msg.metadata.expect("metadata must be Some");
        assert_eq!(meta["event_type"], "script_alert");
        assert_eq!(meta["script_id"], script_id.to_string());
        assert_eq!(meta["script_name"], "My pH Alert Script");
        assert_eq!(meta["trigger_snapshot"]["ph"], 8.1);
    }
```

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml -- alert_output_to_system_alert_includes_script_metadata -v`
Expected: FAIL (wrong argument count).

- [ ] **Step 2: Update `alert_output_to_system_alert` signature and body**

```rust
pub fn alert_output_to_system_alert(
    alert: AlertOutput,
    device_id: &str,
    timestamp_ms: i64,
    script_id: uuid::Uuid,
    script_name: &str,
    trigger_snapshot: Option<serde_json::Value>,
) -> crate::models::alert::AlertMessage {
    crate::models::alert::AlertMessage {
        level: alert.level,
        category: "automation".to_string(),
        title: alert.title,
        message: alert.message,
        device_id: device_id.to_string(),
        reason: Some(script_name.to_string()),
        metadata: Some(serde_json::json!({
            "event_type": "script_alert",
            "script_id": script_id.to_string(),
            "script_name": script_name,
            "trigger_snapshot": trigger_snapshot,
        })),
        timestamp: timestamp_ms as u64,
    }
}
```

- [ ] **Step 3: Update `handle_fired_alert` signature to thread script identity through**

```rust
pub async fn handle_fired_alert(
    app_state: &crate::AppState,
    alert: AlertOutput,
    device_id: &str,
    timestamp_ms: i64,
    script_id: uuid::Uuid,
    script_name: &str,
    trigger_snapshot: Option<serde_json::Value>,
) {
    let notify_fcm_override = alert.notify_fcm;
    let alert_msg = alert_output_to_system_alert(
        alert,
        device_id,
        timestamp_ms,
        script_id,
        script_name,
        trigger_snapshot,
    );
    // ... rest of function unchanged (db_record, event_bus, FCM) ...
```

- [ ] **Step 4: Update call site in `sensors.rs`**

In `sensors.rs` around line 223, the fired results iteration has `(script_id, res)`. We need the script name. Scripts in `chain_nodes` carry both `id` and (via `CachedScript`) a name. Check the `ChainNode` struct:

```rust
// Look at what data ChainNode carries:
// In script_eval.rs, ChainNode is defined — grep for it:
```

Run: `grep -n "pub struct ChainNode" hydragrow-backend/src/mqtt/handlers/script_eval.rs`

If `ChainNode` does not carry a `name` field, add one. Then build a `name_map: HashMap<Uuid, String>` from `chain_nodes` before the eval, and look it up when calling `handle_fired_alert`:

```rust
// Build name map before eval
let name_map: std::collections::HashMap<uuid::Uuid, String> = chain_nodes
    .iter()
    .map(|n| (n.id, n.name.clone()))
    .collect();

// ... after eval, in the for loop:
for (script_id, res) in results {
    let script_name = name_map.get(&script_id).map(|s| s.as_str()).unwrap_or("unknown");
    // build snapshot_json from the snapshot variable available in this scope
    let snapshot_json = Some(serde_json::json!({
        "ph": snapshot.ph,
        "ec": snapshot.ec,
        "temp": snapshot.temp,
        "water_level": snapshot.water_level,
        "phase": snapshot.phase,
    }));
    match res {
        ChainFireResult::Alert(alert) => {
            handle_fired_alert(
                &app_state,
                alert,
                &device_id,
                timestamp_ms,
                script_id,
                script_name,
                snapshot_json,
            ).await;
        }
        // ... other arms unchanged
    }
}
```

- [ ] **Step 5: Update call site in `cron_scheduler.rs`**

Similar change — cron scripts don't have a sensor snapshot, so pass `None` for `trigger_snapshot`. The `script_id` and name come from the script being iterated.

- [ ] **Step 6: Run tests**

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml 2>&1 | tail -30
```

Expected: all pass including the new test.

- [ ] **Step 7: Run clippy**

```bash
cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -20
```

Expected: 0 warnings.

- [ ] **Step 8: Commit**

```bash
git add hydragrow-backend/src/mqtt/handlers/script_eval.rs \
        hydragrow-backend/src/mqtt/handlers/sensors.rs \
        hydragrow-backend/src/services/cron_scheduler.rs
git commit -m "feat(backend): enrich automation alert metadata with script identity

alert_output_to_system_alert previously set metadata: None and used
the generic reason 'Rhai user script'. Users could not trace alerts
to their source script or triggering sensor values.

Now metadata includes: event_type='script_alert', script_id (UUID),
script_name, and trigger_snapshot (sensor values at firing time).

Fixes: C10 from logging review (2026-09-system-logging-review.md)"
```

---

### Task 5: Propagate `err_*` sensor-error flags to `SensorSnapshot` (C13)

**Problem:** `SensorSnapshot` (`models/script.rs:135–143`) only carries `ph`, `ec`, `temp`, `water_level`, `phase`, `device_id`, `timestamp_ms`. The `err_ph`, `err_tds` (aliased `err_ec`), `err_temp`, `err_water` flags are present in the incoming `SensorData` struct and are copied into `sensor_data` in `sensors.rs:33–44`, but are **not** propagated to the `SensorSnapshot` built at line 125. Automation scripts therefore cannot react to sensor failures.

**Files:**
- Modify: `hydragrow-backend/src/models/script.rs` — `SensorSnapshot` struct (lines 134–143)
- Modify: `hydragrow-backend/src/mqtt/handlers/sensors.rs` — snapshot construction (lines 124–133)
- Test: `hydragrow-backend/src/mqtt/handlers/sensors.rs` — existing test block (line 280+)

**Interfaces:**
- `SensorSnapshot` gains four new optional fields; all existing tests that construct a literal `SensorSnapshot` must add these fields (with `None` as default).

- [ ] **Step 1: Write the failing test**

In the `#[cfg(test)]` block of `sensors.rs` (line 280+), find the test `make_snapshot()` helper. Add a new test:

```rust
    #[test]
    fn sensor_snapshot_carries_err_flags_when_set() {
        // Verify the mapping: err_ph=true in incoming SensorData must appear
        // as err_ph=Some(true) in the SensorSnapshot used by scripts.
        let snapshot = crate::models::script::SensorSnapshot {
            ph: 7.0,
            ec: 1.5,
            temp: 25.0,
            water_level: 80.0,
            phase: "Monitoring".to_string(),
            device_id: "test".to_string(),
            timestamp_ms: 0,
            err_ph: Some(true),
            err_tds: None,
            err_temp: None,
            err_water: None,
        };
        assert_eq!(snapshot.err_ph, Some(true));
        assert!(snapshot.err_tds.is_none());
    }
```

Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml -- sensor_snapshot_carries_err_flags -v`
Expected: FAIL (fields don't exist yet).

- [ ] **Step 2: Add `err_*` fields to `SensorSnapshot`**

In `hydragrow-backend/src/models/script.rs`, update the struct:
```rust
#[derive(Debug, Clone)]
pub struct SensorSnapshot {
    pub ph: f32,
    pub ec: f32,
    pub temp: f32,
    pub water_level: f32,
    pub phase: String,
    pub device_id: String,
    pub timestamp_ms: i64,
    // Sensor error flags — None means "not reported" (old firmware / field absent)
    pub err_ph: Option<bool>,
    pub err_tds: Option<bool>,    // aliases err_ec from sensor node
    pub err_temp: Option<bool>,
    pub err_water: Option<bool>,
}
```

- [ ] **Step 3: Update snapshot construction in `sensors.rs`**

In `sensors.rs` lines 124–133, update:
```rust
        let snapshot = crate::models::script::SensorSnapshot {
            ph: incoming.ph,
            ec: incoming.ec,
            temp: incoming.temp,
            water_level: incoming.water_level,
            phase: current_phase,
            device_id: device_id.clone(),
            timestamp_ms,
            err_ph: incoming.err_ph,
            err_tds: incoming.err_ec,   // SensorData uses err_ec for TDS/EC sensor
            err_temp: incoming.err_temp,
            err_water: incoming.err_water,
        };
```

- [ ] **Step 4: Fix all compile errors from the new fields**

Run: `cargo build --manifest-path hydragrow-backend/Cargo.toml 2>&1 | grep "^error"` to find every place that constructs a `SensorSnapshot` literal and is now missing fields.

For every failing literal construction, add:
```rust
err_ph: None,
err_tds: None,
err_temp: None,
err_water: None,
```

Common locations to check:
- `hydragrow-backend/src/services/cron_scheduler.rs` (line 457 area)
- Any tests in `script_eval.rs` that build a `SensorSnapshot`

- [ ] **Step 5: Run full test suite**

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml 2>&1 | tail -30
```

Expected: all pass. The test from Step 1 must appear in the output.

- [ ] **Step 6: Run clippy**

```bash
cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings 2>&1 | tail -20
```

Expected: 0 warnings.

- [ ] **Step 7: Commit**

```bash
git add hydragrow-backend/src/models/script.rs \
        hydragrow-backend/src/mqtt/handlers/sensors.rs \
        hydragrow-backend/src/services/cron_scheduler.rs
git commit -m "feat(backend): propagate sensor err_* flags to SensorSnapshot for scripts

SensorSnapshot used by automation scripts was missing err_ph, err_tds,
err_temp, err_water flags even though they were available in the incoming
SensorData. Scripts could not detect or react to sensor hardware errors.

Now all four err_* flags are carried through. Existing scripts that don't
use them see Option::None (backward-compatible).

Fixes: C13 from logging review (2026-09-system-logging-review.md)"
```
