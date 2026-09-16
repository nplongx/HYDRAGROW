# P0.3 Authoritative State / Telemetry Model

## Status

Proposed implementation contract.

## Purpose

Establish one source-backed model for current device state and telemetry across shared, backend, controller, and frontend.

P0.3 fixes semantic ambiguity between:

- observed sensor value
- sensor error/unavailable state
- controller health
- connection/contact state
- current actuator state
- FSM/system state
- command lifecycle
- historical telemetry

P0.3 does not redesign command lifecycle. P0.2 remains authoritative for command state.

## 1. Core invariants

1. A value is displayed as measured only when an authoritative observation exists.
2. Missing, stale, invalid, or sensor-error data is not converted to `0`, `false`, or another plausible default.
3. `last_seen` means transport/contact information only. It is not telemetry freshness.
4. Observation time and backend receipt time are separate concepts.
5. Current state and historical telemetry are separate concepts.
6. Command lifecycle and physical/observed state are separate concepts.
7. Backend acceptance/publication is not telemetry evidence.
8. A controller status message does not imply every sensor value is fresh.
9. One sensor becoming stale/error does not automatically invalidate unrelated sensors.
10. Device identity is always scoped by canonical StationContext / `device_id`.
11. Unknown remains unknown. The system must not manufacture certainty from absence of data.
12. Mock/demo values must never be represented as production observations.

## 2. Canonical state domains

### 2.1 Device availability

Answers: can the system currently communicate with this device sufficiently to operate or observe it?

Canonical states:

- `UNKNOWN`
- `ONLINE`
- `DEGRADED`
- `OFFLINE`
- `UNAVAILABLE`

Availability is derived from explicit connection/contact evidence and configured policy. It is not derived from a sensor value.

`last_seen` is supporting contact evidence only.

### 2.2 Telemetry quality

Each telemetry axis has its own quality state:

- `UNKNOWN`: no usable observation established.
- `VALID`: usable authoritative observation.
- `STALE`: observation exists but is outside the freshness policy.
- `ERROR`: source explicitly reports measurement error.
- `INVALID`: payload/value failed validation or cannot be trusted.

Quality is per axis, not one global boolean.

### 2.3 Observation

Each sensor observation has:

- `device_id`
- axis identity
- value, when valid
- unit
- observation timestamp, when provided
- backend receipt timestamp
- quality
- source
- optional sensor/controller error indicator

The repository already carries per-axis timing fields such as `ec_received_ms`, `ph_received_ms`, `temp_received_ms`, and `water_received_ms`.

These fields must not be silently reinterpreted as observation timestamps.

The existing `SensorData.time` is the source timestamp when trustworthy. If source timestamp is absent or invalid, receipt time may be retained as receipt metadata, but must not be relabeled as physical observation time.

### 2.4 Controller health

`DeviceHealthSnapshot` is controller health/diagnostics, not sensor telemetry.

Existing fields include:

- `free_heap`
- `uptime_sec`
- `rssi`
- `health_score_percent`
- `fsm_state_display`
- `log_drop_count`
- `firmware_version`
- `kalman_confidence`
- `matrix_update_count`
- `matrix_is_warm`
- `hestia`
- `timestamp_ms`

Health freshness is independent from sensor freshness.

A fresh health snapshot with stale sensor data must not make sensor data appear fresh.

### 2.5 Actuator state

Actuator state represents observed/current physical or controller-reported actuator state.

It is separate from command lifecycle.

Example:

- command lifecycle: `REQUESTED`
- actuator observation: `pump_a = false`

The UI must not display `pump_a = true` merely because a command was accepted or published.

When actuator state is not observed, it is `UNKNOWN`, not `false`.

### 2.6 FSM/system state

FSM/system state is controller/runtime operational state.

It is separate from:

- telemetry quality
- availability
- command lifecycle
- actuator state

Existing FSM events/snapshots remain source material. P0.3 may normalize their representation but must not invent new controller states.

## 3. Canonical telemetry envelope

Implementation should converge on a typed shared representation equivalent to:

```text
TelemetrySnapshot {
  device_id
  observed_at
  received_at
  source
  axes[]
  controller_health
  actuator_state
  fsm_state
}
```

Each axis should carry:

```text
TelemetryAxis {
  name
  value
  unit
  quality
  observed_at
  received_at
  source
  error_code/reason
}
```

Exact Rust field names may follow repository conventions. Do not create a second parallel model when an existing shared model can be extended safely.

`value` must be absent/null when quality does not permit a trusted value.

## 4. Sensor axes currently in scope

Current repository-backed axes:

- `ph`
- `ec`
- `temp`
- `water_level`

Existing sensor error fields:

- `err_ph`
- `err_ec`
- `err_temp`
- `err_water`

These flags are evidence for `ERROR`; they are not values.

No new physical sensor is introduced by P0.3.

## 5. Timestamp contract

Three timestamps remain conceptually distinct where available:

1. `observed_at`: when the sensor/controller says the observation occurred.
2. `received_at`: when backend received the message.
3. `last_seen_at`: latest communication/contact evidence for the device/topic.

Rules:

- Never set `observed_at = received_at` without explicitly documenting that the source provided no observation timestamp and the value is only receipt-time data.
- Never use `last_seen_at` as sensor freshness.
- Never overwrite a source observation timestamp with backend `Utc::now()` merely because a message arrived.
- Out-of-order observations must not overwrite newer observations.

## 6. Freshness contract

Freshness policy must be explicit per data class.

Minimum classes:

- sensor telemetry
- controller health
- actuator state
- FSM/system state
- connection/contact

P0.3 must not choose arbitrary universal freshness thresholds merely to make UI badges deterministic.

If a source-specific freshness threshold already exists, reuse and document it.

If no authoritative threshold exists, state is `UNKNOWN`/`STALE` according to available evidence rather than inventing a precise threshold that implies a backend or hardware guarantee.

Existing frontend sensor timeout behavior, including the current 65-second timeout, must be audited against this contract before being declared canonical.

## 7. Merge precedence

When multiple messages update the same device:

1. Validate device identity.
2. Validate payload shape and value.
3. Determine observation timestamp.
4. Reject stale/out-of-order observation for the affected axis/state field.
5. Update only fields supported by the message.
6. Preserve existing known values for unrelated fields.
7. Do not fill missing fields with defaults.

A partial sensor message must not erase valid unrelated sensor observations.

A health message must not refresh sensor timestamps.

A connection message must not refresh sensor timestamps.

A command lifecycle message must not refresh physical state unless it also contains authoritative physical observation.

## 8. Error and unavailable semantics

### Sensor error

If `err_ph = true`, pH quality is `ERROR` unless a more authoritative validation rule says otherwise.

The UI may show the last known value only if clearly marked stale/error. It must not present it as a current valid measurement.

### Missing value

Missing value means `UNKNOWN`, not zero.

Forbidden production telemetry fallbacks include:

- `ec ?? 0`
- `ph ?? 0`
- `temp ?? 0`
- `water_level ?? 0`

### Transport loss

Transport loss changes availability/contact state. It does not prove the physical value changed, became zero, or became unsafe.

### Parse/validation failure

Malformed telemetry must not update canonical current state.

The failure should remain observable through existing logging/diagnostics policy.

## 9. Backend responsibilities

Backend is authoritative for normalized server-visible current state and persistence of historical telemetry where existing storage supports it.

Backend must:

- validate `device_id` against message context where applicable
- preserve observation timestamps
- preserve per-axis error information
- distinguish receipt/contact from telemetry observation
- maintain current state without fabricating missing values
- persist historical sensor observations through existing InfluxDB path
- expose explicit unavailable/error semantics to frontend
- avoid converting DB/query errors into apparently valid telemetry

Backend must not:

- turn missing telemetry into zero
- mark a sensor fresh because another topic arrived
- use HTTP command success as state evidence
- claim physical confirmation from command publication

Existing `device_topic_last_seen` remains contact metadata, not telemetry history.

## 10. Frontend responsibilities

Frontend consumes canonical state. It must not reconstruct physical truth from UI actions.

Frontend must:

- use StationContext `selectedDeviceId` as identity authority
- keep command lifecycle separate from observed state
- render unknown/error/stale states explicitly
- preserve per-axis quality
- avoid fake defaults in production mode
- prevent stale device A telemetry from appearing under device B
- distinguish loading from unavailable from error
- show timestamp semantics honestly

`useDeviceSync` may remain an orchestration layer during P0.3, but telemetry normalization and authority should move toward dedicated typed boundaries rather than growing the god-hook.

## 11. Dashboard / Operations display contract

Dashboard is monitoring/triage.

For each displayed telemetry value, UI must be able to answer:

- what was measured?
- when was it observed?
- how fresh is it?
- is it valid?
- is it an error?
- which device does it belong to?

Operations may display actuator/runtime state, but must not derive physical state from command lifecycle alone.

If no trusted current value exists, display explicit unavailable/unknown state instead of a plausible number.

## 12. History contract

Current state is not history.

Historical telemetry remains in existing telemetry storage where available, including the InfluxDB sensor path.

P0.3 does not turn Journal into a telemetry database.

Journal remains event history.

Historical telemetry queries must preserve query errors. A failed history query must not become an empty successful result.

## 13. Mock/demo contract

Mock data is allowed only inside explicit mock/demo execution paths.

Production state must never silently fall back to values such as:

- `ec: 1.45`
- `ph: 6.12`
- `temp: 24.8`
- `water_level: 22.5`

A mock value must carry an explicit source/mode distinction if it can reach a shared UI model.

## 14. Command interaction

P0.2 owns:

`REQUESTED -> SENT -> ACKNOWLEDGED -> CONFIRMED`

P0.3 owns observed state.

A command transition may trigger a state refresh request, but it does not directly mutate authoritative physical state.

`CONFIRMED` remains valid only when authoritative runtime observation supports the requested outcome.

## 15. Reconnect / station switch

On station/device change:

- invalidate transient current-state data for previous device
- establish new device context first
- load/reconcile current state for new device
- ignore stale responses/events from previous device
- do not reuse previous device telemetry as new device current state

On reconnect:

- reconcile current state from authoritative backend/controller sources
- preserve observation timestamps
- do not mark all values fresh merely because transport reconnected
- unresolved command lifecycle remains governed by P0.2

## 16. Persistence

P0.3 must reuse existing persistence before introducing schema changes.

Known current stores:

- InfluxDB for sensor history
- `device_states` cache for current merged state
- `device_topic_last_seen` for topic contact timestamps
- PostgreSQL event/config stores for their existing domains

These stores have different authority. They must not be collapsed into one generic state table without evidence and migration planning.

No database migration is required by this spec unless implementation proves existing storage cannot represent the canonical model.

## 17. Verification contract

Required tests:

### Shared

- valid/invalid telemetry quality transitions
- serialization round-trip
- timestamp preservation
- absent value remains absent
- per-axis error semantics

### Backend

- source observation timestamp preserved
- receipt timestamp distinct
- out-of-order sample rejected/ignored
- partial payload does not erase unrelated fields
- malformed payload does not update state
- sensor history query errors are preserved
- `last_seen` does not update sensor freshness

### Frontend

- unknown does not render as zero
- error does not render as valid current value
- stale state is visibly distinct
- device A state cannot leak into device B
- reconnect does not make stale data fresh
- command lifecycle does not mutate physical state
- mock values are isolated from production mode

### Cross-subsystem

- controller observation -> MQTT -> backend -> frontend preserves device identity and timestamps
- sensor error survives transport and normalization
- health snapshot freshness does not refresh sensor freshness
- actuator observation remains separate from command lifecycle

## 18. Non-goals

- P0.2 command lifecycle redesign
- new sensors or hardware feedback
- new telemetry hardware protocol
- generic event sourcing
- replacing InfluxDB
- full frontend hook decomposition
- backend-wide decomposition
- redesign of cultivation/cycle schemas
- configuration consistency remediation from P0.4
- automatic retry engine

## 19. Acceptance criteria

P0.3 is complete only when:

1. One shared semantic model distinguishes availability, telemetry quality, controller health, actuator state, FSM/system state, and command lifecycle.
2. Sensor values are never manufactured from `null`/missing/error state in production paths.
3. Observation time, receipt time, and contact/`last_seen` time are distinct.
4. Per-axis freshness/error semantics survive backend normalization and frontend consumption.
5. Out-of-order and partial telemetry cannot corrupt current state.
6. Backend persistence retains historical telemetry without redefining Journal as telemetry history.
7. Frontend renders unknown/stale/error honestly.
8. StationContext remains canonical device identity.
9. Reconnect and station switching cannot leak state across devices.
10. P0.2 command lifecycle remains separate from physical/observed state.
11. No new physical capability is claimed without repository evidence.
12. Verification covers shared/backend/frontend boundaries; unavailable environment checks are recorded, not assumed passed.

## 20. Implementation order

1. Freeze shared telemetry/state contract in tests.
2. Audit existing `SensorData`, `DeviceHealthSnapshot`, FSM, actuator, availability, and timestamp models.
3. Define/extend shared typed current-state model without duplicating existing authority.
4. Normalize backend MQTT ingestion and current-state merge rules.
5. Correct timestamp and `last_seen` semantics.
6. Remove production telemetry default fallbacks.
7. Separate controller health and actuator state from sensor telemetry.
8. Expose canonical state to frontend.
9. Refactor `useDeviceSync` only enough to consume the canonical boundary; defer broad hook decomposition.
10. Update Dashboard/Operations rendering semantics.
11. Add reconnect/station-isolation reconciliation.
12. Verify cross-subsystem behavior.

## 21. Known implementation gaps entering P0.3

- `SensorData` currently uses required numeric fields for `ec`, `ph`, `temp`, and `water_level`, with separate error flags.
- `useDeviceSync` currently contains production fallback expressions such as `ec ?? 0`, `ph ?? 0`, `temp ?? 0`, and `water_level ?? 0`.
- Mock mode currently supplies fixed sensor values.
- `useDeviceSync` currently sets `last_seen` to current receipt time when applying a snapshot.
- Backend WebSocket handling currently sets `last_seen = Utc::now()` for device status events.
- `DeviceHealthSnapshot` has its own `timestamp_ms` and must not be treated as sensor freshness.
- InfluxDB already stores sensor history.
- `device_topic_last_seen` already stores contact timestamps.

These are implementation targets, not proof that a new telemetry protocol is required.

## 22. P0 residual gate before P1.0

P0.3 architecture is considered closed after its implementation contract is satisfied. Any remaining issue is classified as a **P0 residual blocker** only when it is a regression, build/format failure, or verification-environment failure that prevents a trustworthy P1.0 baseline.

The residual gate does not reopen the telemetry design and does not permit fallback semantics to be weakened to make verification pass.

Required residual checks before P1.0:

1. Shared/controller event changes compile across all consumers, including the simulator.
2. Backend tests are executable against the intended PostgreSQL test environment; database-backed failures must be diagnosed rather than hidden or weakened.
3. Backend formatting is clean.
4. Existing authoritative-state tests remain the evidence for unknown/stale/error semantics.

Known residual items from the readiness audit:

- simulator compile regression for the newly introduced `PublishCommandLifecycle` event;
- backend DB-backed test failures requiring environment/root-cause diagnosis;
- backend formatting drift in `hydragrow-backend/src/mqtt/handlers/sensors.rs`.

These items must be closed or explicitly classified before P1.0 verification. They do not authorize a redesign of P0.3.
