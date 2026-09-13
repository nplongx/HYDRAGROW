# HYDRAGROW System Logging — Complete Review & Enhancement Proposal

**Date:** 2026-09 (review session)  
**Scope:** Firmware (controller + sensor nodes) → shared types → backend MQTT handlers → Postgres → frontend display  
**Status:** Review complete. Findings confirmed with code evidence. Enhancement proposals included.

---

## Part A: Confirmed Issues & Bugs (Sections 1–3 from Prior Session)

### 1. "Dispensing solution without changing concentration" — silent lockout

**Root cause chain:**
- `FsmDiagnostics::diagnose_hardware_fault` (`hydragrow-shared/src/fsm.rs:182`) tracks `ec_pump_streak`/`ph_pump_streak`/`water_hydraulics_streak` — detects when pump runs but readings don't move.
- Streaks 1 & 2: only `log::warn!()` → UART serial only → lost forever without debug cable.
- Streak 3 (lockout): returns `Err(FaultCode)`, but **all 13 call sites** set `result.delta.phase = Some(SystemPhase::Fault(fault_code))` without emitting `PublishSystemLog` or `PublishFsmTransition`.
- Only **one place** in entire firmware calls `OrchestratorEvent::PublishFsmTransition` (`mimo_dosing.rs:49`) — and it's for a hard timeout, not a fault.
- Backend `transition_system_event_record` (`fsm.rs:636`) correctly processes `FaultDetected` into a critical system event — **but firmware never sends it**, so the path is dead code exercised only in unit tests.

### 2. Sensor node error logging — multiple broken layers

- **No LWT (Last Will and Testament):** `mqttClient.connect()` uses only 3 params (client_id, username, password). No will topic/payload. Contrast: controller node has proper LWT with `{"online": false, "status": "disconnected"}`.
- **`interpret_online_signal` bug** (`status.rs:24`): Only recognizes `online: bool` or `status == "online"`. All other status values ("error", "ok", OTA events, config applied, wifi list errors, auth failures) → returns `None` → `handle_device` returns early → **silently discarded**.
- **Hard-coded topic bug** (`status.rs:84,92`): `touch_topic()` writes `"controller/status"` regardless of whether the handler was called for the controller or sensor node. Sensor heartbeats are masqueraded as controller signals. Additionally, the call is **duplicated** (lines 84-90 and 92-98 are identical — copy-paste error).
- **Watchdog blind spot** (`staleness.rs:20-22`): `check_staleness()` only checks `controller/status`. No mention of `sensor/status`, `sensor/data`, or any sensor-related topic. A sensor node can be dead and no one knows.
- **`SensorSnapshot` missing `err_*` flags** (`models/script.rs:135-143`): Struct used by automation scripts only has `ph`, `ec`, `temp`, `water_level`, `phase`, `device_id`, `timestamp_ms`. The `err_ph`/`err_tds`/`err_temperature`/`err_water_level` flags that the sensor node publishes are available in the raw MQTT payload and even stored in the backend sensor model — but are **not propagated** to the script engine. Automation scripts cannot react to sensor errors.

### 3. Metadata schema gaps — why metadata is "meaningless"

- 9 `LogCategory` types, but `SystemLogEvent` only has dedicated structs for: `WaterEvent`, `SystemAlert`, `CalibrationUpdate`, `RecipeApplied`, `RecipeRejected`, `RecipeStageChanged`, `RecipeCompleted`, `BasicSystemLog`.
- **Missing:** No `DosingEvent` struct (forced into `BasicSystemLog` with just `source`/`message`/`cycle_id`). No `SensorEvent` variant at all.
- `transition_system_event_record` only persists `Fault` and `EmergencyStop` transitions (line 677: `_ => None`). Rich data like `DosingComplete{dose_a_ml,...}`, `StabilizingComplete{final_ec, final_ph,...}`, `WaterRefillComplete{success, duration_sec, final_level}` — all discarded.
- `TransitionReason::Manual` misused for automatic timeout (`mimo_dosing.rs:49`).
- Migration churn: `error` level deleted (`20260907120001`) then restored 2 days later (`20260909123000`).

---

## Part B: Newly Confirmed Issues (Section 4 Items)

### 4. `dosing_action_log` — exists but insufficient

**Schema** (`20260902000000_add_dosing_action_log.sql`):
```sql
CREATE TABLE dosing_action_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    device_id TEXT NOT NULL REFERENCES device_config(device_id),
    pump TEXT NOT NULL,
    dose_ml REAL NOT NULL,
    dosed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Writers:** Only one — `action_dispatch.rs:166` — called when the backend dispatches a dose command to the controller node via MQTT. It records `device_id`, `pump`, `dose_ml`. That's it.

**Readers:**
- `get_dosing_history_last_hour()` — used for hourly budget enforcement
- `get_last_dose_at()` — used for cooldown check

**What's missing:**
- No `ec_before`/`ec_after`/`ph_before`/`ph_after` → cannot answer "Did that dose work?"
- No `triggered_by` (automation script ID? user command? FSM auto?) → cannot audit *who* caused the dose
- No `cycle_id` → cannot correlate with the broader dosing cycle this action belongs to
- Separate table from `dosing_reports` (which stores full `DosingCycleEvent` with `payload` JSONB) → data is fragmented across two tables with no linking key
- `dosing_reports` *does* have `pump_a_ml`, `pump_b_ml`, `ph_up_ml`, `ph_down_ml`, and a full JSON payload, but it's written by `dosing_cycle.rs` handler (from firmware FSM reports), not from backend-initiated doses

### 5. `flow_execution_log` — minimal automation audit trail

**Schema** (`20260907091500_add_flow_execution_log.sql`):
```sql
CREATE TABLE flow_execution_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    script_id UUID NOT NULL,
    device_id TEXT NOT NULL REFERENCES device_config(device_id),
    status TEXT NOT NULL CHECK (status IN ('success', 'error')),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Writers:** `script_eval.rs` (after evaluating alert/action scripts) and `cron_scheduler.rs` (after cron-triggered scripts).

**What's missing:**
- No `trigger_source` — was this triggered by sensor data, cron, or flow chain?
- No `input_snapshot` — what sensor values triggered the script?
- No `output_actions` — what actions did the script *produce*? (alert? dose command? config override?)
- No `duration_ms` — how long did the script take to evaluate?
- `success_rate_percent()` function only looks at last 200 rows regardless of time window — misleading metric
- No retention policy — table grows indefinitely (unlike `system_events` which has 90-day retention)

### 6. `hydragrow-diagnostic-worker` — AI diagnosis with blind spots

**Architecture:** Polls backend fleet health → evaluates triggers (`HestiaState` warning/critical, or `WatchdogBreach`) → runs LLM-based diagnosis → writes alert via backend API.

**Issues found:**
- **Trigger sources are limited:** Only `HestiaState` (Hestia WARNING/CRITICAL) and `WatchdogBreach` (controller/status staleness). Cannot trigger on: sensor errors, dosing inefficiency, automation script failures, config changes, or multi-signal correlation.
- **Dedup is title-based** (`recent_alert_exists` by `device_id` + `reason_code`) — a recurring problem with identical code produces only one alert until the dedup window passes.
- **No logging of the diagnosis process itself** — if the LLM fails, it falls back to `unexplained_anomaly` with only a `tracing::warn` (server log only). The user never sees that a diagnosis attempt failed and was replaced with a fallback.
- **WatchdogBreach only covers `controller/status`** (inherits the same blind spot from the watchdog service) — sensor node outages don't trigger diagnostic analysis.

### 7. Config change audit trail — nonexistent

**Critical gap:** When a user updates device configuration via `update_unified_config()` or `update_config()` (config.rs:434-500, 613+), **no system event is recorded**. The config is silently overwritten in the database. There is no record of:
- What the previous values were
- Who made the change
- When the change happened (beyond `last_updated` on the config row itself, which is overwritten)
- Whether the change was successfully synced to the device

**Contrast:** Manual control commands (control.rs:312-327) *do* log to `system_events` with `user_action` category and audit metadata including `user`, `session`, `action`, `result`. Config changes — which can be far more impactful (changing EC targets, safety limits, etc.) — have zero audit trail.

The only config-related log that exists is in `config.rs:871-895`, which records recipe-related events within a DB transaction — but this is specifically for crop recipe application, not general config changes.

### 8. OTA lifecycle events — not persisted

`handle_ota_status()` (`status.rs:370-385`) receives OTA lifecycle events from the controller node, but only does two things:
1. `tracing::info!()` → server log only
2. `event_bus.send(AppEvent::ControllerStatus(value))` → forwarded to WebSocket for real-time display

**Not persisted to `system_events`.** If the user isn't watching the dashboard at the exact moment an OTA happens, the event is lost. For sensor node OTA, the situation is even worse — the command is sent (`trigger_sensor_ota`, device_admin.rs:597), but there's no return channel for status since the sensor node lacks proper status reporting (see Issue 2).

### 9. Automation script alerts — metadata is hollow

`alert_output_to_system_alert()` (`script_eval.rs:779-793`) converts script alert output to `AlertMessage` with:
- `category: "automation"` (hardcoded)
- `reason: Some("Rhai user script")` (generic string, not the script name/ID)
- `metadata: None` (always null)

The script's actual ID, name, trigger context (which sensor reading triggered it), and execution trace are **not carried** in the alert metadata. When a user sees an automation alert in the log, they cannot trace it back to which script produced it or what data triggered it.

### 10. Frontend log display — decent but limited by data gaps

The `SystemLog.tsx` page is well-built:
- Infinite-scroll pagination via TanStack Query
- Category filters for all 9 types (plus "unresolved" filter)
- Two view modes: "important" (merges technical noise) and "all_technical"
- Cycle grouping by `cycle_id`
- CSV export
- Event detail drawer
- Grafana link
- Health summary bar
- Acknowledge/resolve functionality

**But the frontend can only show what the backend records.** The `FILTERS` array includes `sensor` (commented: "Tín hiệu cảm biến") — but as noted in Issue 3, `Sensor` has **no corresponding variant** in `SystemLogEvent`, so this filter would show nothing or only `BasicSystemLog` entries.

The `eventGrouping.ts` logic merges `system`, `sensor`, `calibration` at `info` level as "technical noise" — which is a reasonable UX choice, but it means these events are hidden by default. Given how few sensor events are actually recorded, even switching to `all_technical` mode wouldn't reveal the missing data.

### 11. Data retention asymmetry

| Table | Retention | Notes |
|-------|-----------|-------|
| `system_events` | 90 days (active batch deletion) | Only log table with retention |
| `dosing_action_log` | **Indefinite** | Grows forever |
| `dosing_reports` | **Indefinite** | Full JSON payloads, grows fast |
| `flow_execution_log` | **Indefinite** | Grows with every script eval |
| `device_topic_last_seen` | **Indefinite** (but small) | Only latest timestamp per topic |

No archival strategy exists for any table except `system_events`.

### 12. Correlation gaps — no unified trace ID

The system has multiple independent log tables with no linking mechanism:
- `system_events` — general events/alerts
- `dosing_action_log` — individual pump commands
- `dosing_reports` — FSM cycle reports
- `flow_execution_log` — script execution results
- `device_topic_last_seen` — device heartbeats

A user wanting to understand "what happened at 14:30" must query 4+ tables with approximate timestamp matching. There is no `trace_id`, `correlation_id`, or `request_id` that links a user action → config change → MQTT command → firmware execution → sensor reading → script evaluation → alert.

The `cycle_id` field partially solves this for dosing (it appears in `system_events` metadata and `dosing_reports.payload`), but it's not used in `dosing_action_log`, `flow_execution_log`, or control commands.

---

## Part C: Enhancement Proposals

### Tier 1: Critical Fixes (bugs and silent failures)

**C1. Fix the silent fault transition path (firmware)**
- **What:** Every call site that transitions to `Fault` or `EmergencyStop` must emit `OrchestratorEvent::PublishFsmTransition` with the appropriate `TransitionReason` variant.
- **Where:** All 13 call sites in `stabilizing.rs`, `mimo_dosing.rs`, `monitoring.rs`, `water_phases.rs`.
- **Why:** The backend handler already correctly processes these — the data path just needs to be connected.

**C2. Fix the hard-coded `"controller/status"` topic in `handle_device`**
- **What:** Pass the actual topic category (`"controller/status"` or `"sensor/status"`) to `touch_topic()` based on the call source.
- **Where:** `status.rs:84-98` — also remove the duplicate `touch_topic` call.
- **Why:** Sensor node heartbeats are currently invisible to the watchdog.

**C3. Fix `interpret_online_signal` to handle all status values**
- **What:** Instead of returning `None` for unknown status strings, parse the full vocabulary of sensor node status messages. At minimum, treat any parseable JSON with a `status` field as a valid heartbeat (touching `device_topic_last_seen`), even if it's not an online/offline signal.
- **Where:** `status.rs:24-31` and `handle_device`.
- **Why:** Currently ~10 types of sensor node messages are silently discarded.

**C4. Extend watchdog to monitor sensor nodes**
- **What:** Add `sensor/data` (or `sensor/status`) as a second staleness check topic with its own threshold.
- **Where:** `staleness.rs:check_staleness()`, `trigger.rs` (new `SensorStaleness` trigger variant).
- **Why:** Sensor node outages are completely undetected.

**C5. Add LWT to sensor node MQTT**
- **What:** Configure MQTT client with Last Will and Testament (will topic: `{device_id}/sensor/status`, will payload: `{"online": false, "status": "disconnected"}`).
- **Where:** Sensor node firmware MQTT initialization.
- **Why:** Without LWT, the system has no way to know when a sensor node disconnects unexpectedly.

### Tier 2: Missing Logging (events that should be recorded but aren't)

**C6. Log all FSM transitions, not just Fault/EmergencyStop**
- **What:** `transition_system_event_record` should produce a `Some(NewSystemEventRecord)` for *every* transition, not just the two critical ones. Use appropriate levels: `info` for normal transitions, `warning` for unusual ones, `critical` for fault/emergency.
- **Where:** `fsm.rs:636-679` — remove the `_ => None` catch-all.
- **Why:** `DosingComplete`, `StabilizingComplete`, `WaterRefillComplete` carry rich operational data that's currently discarded.

**C7. Config change audit logging**
- **What:** Wrap every config update endpoint (`update_unified_config`, `update_config`, `update_safety_config`, `update_water_config`) with an audit event recording: old values, new values, user identity, timestamp, sync status.
- **Where:** `config.rs` — all update functions.
- **Category:** `user_action` with `event_type: "config_change"`.
- **Why:** Config changes are among the most impactful user actions and currently have zero audit trail.

**C8. OTA event persistence**
- **What:** `handle_ota_status()` should write to `system_events` (category: `device`, level based on OTA lifecycle stage).
- **Where:** `status.rs:370-385`.
- **Why:** OTA events are transient real-time signals with no historical record.

**C9. Dosing efficacy tracking**
- **What:** Extend `dosing_action_log` schema with: `ec_before`, `ec_after`, `ph_before`, `ph_after`, `cycle_id`, `triggered_by` (enum: 'fsm_auto', 'user_manual', 'script_auto'), `script_id` (nullable).
- **Where:** New migration + update `insert_dosing_action()` and its callers.
- **Why:** The current table cannot answer "Did that dose work?" — the core question from the original observation.

**C10. Automation script alert enrichment**
- **What:** `alert_output_to_system_alert()` should include: `script_id`, `script_name`, `trigger_snapshot` (the sensor values that triggered it), `execution_trace` (condition evaluation path).
- **Where:** `script_eval.rs:779-793` — populate `metadata` field.
- **Why:** Users cannot trace automation alerts back to their source.

**C11. Enrich `flow_execution_log` with execution context**
- **What:** Add columns: `trigger_source` (enum: 'sensor_data', 'cron', 'flow_chain'), `input_snapshot` (JSONB), `output_actions` (JSONB array of what the script produced), `duration_ms`.
- **Where:** New migration + update `log_success`/`log_error`.
- **Why:** The current "success/error" binary is insufficient for debugging automation behavior.

### Tier 3: Structural Improvements (metadata quality & correlation)

**C12. Add dedicated `SystemLogEvent` variants for Dosing and Sensor**
```rust
DosingEvent(DosingMetadata),    // dose_ml, pump, ec_before, ec_after, ph_before, ph_after, efficacy
SensorEvent(SensorMetadata),    // sensor_type, error_type, error_start, duration, reading_value
```
- **Where:** `hydragrow-shared/src/log.rs`
- **Why:** The two most operationally important categories are shoehorned into `BasicSystemLog` or missing entirely.

**C13. Propagate `err_*` flags to `SensorSnapshot` for automation scripts**
- **What:** Add `err_ph: Option<bool>`, `err_tds: Option<bool>`, `err_temperature: Option<bool>`, `err_water_level: Option<bool>` to `SensorSnapshot`.
- **Where:** `models/script.rs:135-143` + `sensors.rs:125` (where snapshot is built).
- **Why:** Automation scripts currently cannot detect or react to sensor failures.

**C14. Unified correlation ID**
- **What:** Introduce a `trace_id` (UUID) that flows through: user action → API endpoint → MQTT command → firmware execution → sensor reading → script evaluation → system event. Store it in all log tables.
- **Where:** Cross-cutting change across API, MQTT handlers, firmware, and all log tables.
- **Why:** Currently impossible to correlate events across tables without approximate timestamp matching.

**C15. Sensor error state tracking (edge-to-log)**
- **What:** Convert transient `err_*` boolean flags into durable state events. When `err_ph` transitions from `false → true`, emit a system event (category: `sensor`, level: `warning`). When it transitions back (`true → false`), emit a recovery event (level: `info`). Track duration.
- **Where:** Backend sensor handler (`sensors.rs`) — needs state tracking per device per sensor.
- **Why:** Sensor errors are currently transient booleans in the latest telemetry message with no history.

**C16. Fix `TransitionReason::Manual` misuse**
- **What:** Add `TransitionReason::HardTimeout` to the shared types and use it in `mimo_dosing.rs:49` (the TODO comment already suggests this).
- **Where:** `hydragrow-shared` + firmware.
- **Why:** Manual intervention records should mean actual human actions, not automatic timeouts.

### Tier 4: Operational Improvements (retention, display, diagnostics)

**C17. Retention policies for all log tables**
- **What:** Apply batch-deletion retention to `dosing_action_log` (90 days), `flow_execution_log` (90 days), `dosing_reports` (365 days — longer because it's used for analytics). Archive to cold storage if needed.
- **Where:** Extend `retention.rs` to cover additional tables.
- **Why:** Only `system_events` has retention; other tables grow indefinitely.

**C18. Expand diagnostic-worker trigger sources**
- **What:** Add trigger variants for: sensor node staleness, dosing inefficiency (multiple consecutive dose-without-effect streaks), automation script error rate exceeding threshold, abnormal config changes.
- **Where:** `trigger.rs` enum + `evaluate_triggers()`.
- **Why:** The AI diagnostic system can only react to two trigger types, missing many actionable situations.

**C19. Frontend log: show "what's missing"**
- **What:** When the sensor filter shows no events, display a contextual message explaining that sensor events are not currently being recorded (rather than just "No events"). Add a "system health" indicator showing which log categories have recent data and which have gone silent.
- **Where:** `SystemLog.tsx`, `HealthSummaryBar`.
- **Why:** Users need to know whether "no logs" means "nothing happened" or "events aren't being captured."

**C20. Firmware-side logging for pre-fault warnings**
- **What:** When `diagnose_hardware_fault` detects streak 1 or 2 (before lockout), emit a `PublishSystemLog` event (category: `alert`, level: `warning`) with the streak count, pump name, and sensor readings.
- **Where:** `hydragrow-shared/src/fsm.rs` + firmware call sites.
- **Why:** The first two warning streaks currently vanish into UART serial. Early warnings are the most valuable — by streak 3 it's too late.

---

## Summary Matrix

| # | Issue | Severity | Effort | Component |
|---|-------|----------|--------|-----------|
| C1 | Silent fault transitions | 🔴 Critical | Medium | Firmware |
| C2 | Hard-coded topic in handle_device | 🔴 Critical | Tiny | Backend |
| C3 | interpret_online_signal discards messages | 🔴 Critical | Small | Backend |
| C4 | Watchdog doesn't monitor sensors | 🔴 Critical | Small | Watchdog |
| C5 | Sensor node missing LWT | 🟠 High | Small | Firmware (sensor) |
| C6 | Normal FSM transitions not logged | 🟠 High | Medium | Backend |
| C7 | Config changes not audited | 🟠 High | Medium | Backend |
| C8 | OTA events not persisted | 🟡 Medium | Tiny | Backend |
| C9 | Dosing efficacy tracking | 🟠 High | Medium | Backend + schema |
| C10 | Automation alerts lack metadata | 🟡 Medium | Small | Backend |
| C11 | flow_execution_log too sparse | 🟡 Medium | Medium | Backend + schema |
| C12 | Missing Dosing/Sensor event variants | 🟠 High | Medium | Shared types |
| C13 | err_* flags missing from SensorSnapshot | 🟠 High | Small | Backend |
| C14 | No unified trace ID | 🟡 Medium | Large | Cross-cutting |
| C15 | Sensor error state tracking | 🟠 High | Medium | Backend |
| C16 | TransitionReason::Manual misuse | 🟡 Medium | Tiny | Shared + firmware |
| C17 | No retention on non-system tables | 🟡 Medium | Small | Backend |
| C18 | Diagnostic worker limited triggers | 🟡 Medium | Medium | Diagnostic worker |
| C19 | Frontend missing data awareness | 🟢 Low | Small | Frontend |
| C20 | Pre-fault warnings lost to UART | 🟠 High | Medium | Firmware |

**Legend:** 🔴 = Bug/broken path causing silent data loss. 🟠 = Important functionality that's absent. 🟡 = Quality/completeness improvement. 🟢 = Nice-to-have UX enhancement.
