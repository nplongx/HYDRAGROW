# P1.3 Safety-Critical Failure Semantics / Operational Truth

## Status

Implementation substantially complete; cross-system runtime verification remains open.

P1.3 closes the next reliability and safety boundary after P1.2. It makes safety-critical backend decisions fail-closed, removes fabricated actuator/device state, makes availability/contact semantics canonical, hardens backup/restore and query boundaries, and makes operational outcomes observable without changing P0/P1.1/P1.2 authority.

P1.3 builds on:

- P0.3 authoritative state/telemetry semantics;
- P0.4 configuration consistency;
- P0.5 frontend data boundaries;
- P1.0 verification discipline;
- P1.1 authentication/capability/ownership;
- P1.2 durable command lifecycle.

It does not reopen those contracts.

## Requirement ID

`P1.3-SAFETY-FAILURE-SEMANTICS`

## Change class

`C4` Cross-subsystem + `C6` Safety/reliability + `C7` Security-sensitive + `C8` Operational observability.

## Purpose

HYDRAGROW must never turn missing, stale, invalid, or failed observations into an affirmative safety fact.

The system must answer, explicitly and conservatively:

- Is the controller actually known to be reachable?
- Is the current actuator state known, unknown, or stale?
- Is automation allowed to act when an input is unavailable?
- Did backup/restore persist, publish, and apply successfully, or only one of those steps?
- Is a query valid and safely scoped before execution?
- Are operational events complete enough to diagnose a failure?
- Can every frontend/backend state that claims safety or success be traced to authoritative evidence?

The governing rule is:

> Unknown is not safe, successful, offline, zero, false, or confirmed unless the underlying contract explicitly defines that value as the correct default.

---

## 1. Scope

P1.3 covers:

- fail-closed behavior in safety-critical automation/scheduler decisions;
- scheduler handling of missing, stale, malformed, or unavailable inputs;
- removal of fabricated all-pumps-off/all-safe state;
- canonical controller availability/contact semantics;
- watchdog/backend/frontend alignment on availability;
- backup/restore lifecycle and success semantics;
- Flux query validation, parameterization, and device scoping;
- operational observability completeness for safety-critical paths;
- consistency of shared/backend/frontend representations where they affect authoritative safety or failure semantics;
- negative/failure/restart tests and evidence.

P1.3 does **not** redesign:

- the canonical command lifecycle;
- durable command persistence or retry authority;
- P1.1 authorization/ownership semantics;
- the telemetry data model established by P0.3;
- the frontend state architecture established by P0.5;
- MQTT transport;
- PostgreSQL or InfluxDB as storage systems;
- the controller FSM unless required to preserve the failure semantics defined here;
- product-specific crop/recipe logic unrelated to safety/failure semantics.

---

## 2. Existing findings carried into P1.3

The whole-system audit identified several gaps outside P1.2:

1. scheduler and safety-critical reads may fail open when required state is unavailable;
2. some paths can synthesize an all-pumps-off or otherwise safe-looking control state without authoritative evidence;
3. controller heartbeat/contact heuristics can compete with canonical availability/contact semantics;
4. backup/restore needs explicit separation between persistence, transport publication, and physical application success;
5. Flux query construction requires validation/encoding and strict device scope;
6. operational observability is incomplete across safety-critical transitions and failures;
7. shared-schema/frontend model alignment remains incomplete where semantic drift can produce false state.

These findings are P1.3 work, not reasons to reopen P0 architecture.

---

## 3. Core invariants

### 3.1 Fail closed

A safety-critical decision must deny action when a required prerequisite is unknown, unavailable, stale beyond its contract, malformed, or contradictory.

Examples:

```text
missing safety state -> do not execute dependent automation
stale interlock input -> do not execute dependent action
query failure -> do not treat result as empty/false/zero
unknown actuator state -> do not display as safely off
invalid configuration -> do not execute it
```

A fail-closed decision must produce an auditable reason code.

### 3.2 Unknown remains unknown

The following substitutions are prohibited unless explicitly defined by the authoritative contract:

```text
query error       -> empty result
missing telemetry -> zero
missing pump state -> OFF
unknown contact    -> OFFLINE
stale state        -> current state
backup failure     -> success
MQTT publish      -> physical application
```

P1.2 command lifecycle remains the authority for command outcome. P1.3 must not create a competing success state.

### 3.3 Authoritative state only

Safety decisions may use only state from an explicitly authoritative source.

Cached state, UI state, last-known state, localStorage, request payloads, MQTT topic identity, or an inferred default are not authoritative merely because they are available.

A last-known value may be retained for diagnostics, but it must carry freshness/validity metadata and must not silently become current truth.

### 3.4 One availability/contact semantic

Controller availability/contact must have one canonical meaning and source.

Heartbeat heuristics may provide evidence to the canonical mechanism, but must not create a competing `online/offline` truth with different thresholds or timestamps.

The distinction between:

- last contact;
- current availability classification;
- telemetry freshness;
- controller health;
- actuator state

must remain explicit.

### 3.5 Safety decision traceability

Every safety-critical allow/deny decision must be explainable from:

```text
input observations
+ freshness/validity
+ policy/configuration
+ decision
+ reason code
+ timestamp
```

Do not log secrets or unrestricted payloads.

---

## 4. Scheduler fail-closed semantics

The scheduler is allowed to execute an automation action only when every required prerequisite is known-valid under the applicable policy.

### 4.1 Required input states

Each scheduler condition must distinguish at minimum:

```text
KNOWN_VALID
UNKNOWN
STALE
INVALID
ERROR
```

Exact implementation types may follow existing shared/controller contracts.

`UNKNOWN`, `STALE`, `INVALID`, and `ERROR` are not equivalent to a normal false sensor value.

### 4.2 Decision rules

For safety-critical predicates:

```text
all required inputs KNOWN_VALID
    + predicate true
    -> action eligible

any required input UNKNOWN/STALE/INVALID/ERROR
    -> action denied/deferred

all required inputs KNOWN_VALID
    + predicate false
    -> action not eligible
```

The scheduler must not execute because an unavailable input happens to evaluate as false, zero, empty, or missing.

### 4.3 Fail-closed reason

A denied/deferred action caused by unavailable state must record a bounded reason such as:

- `INPUT_UNKNOWN`;
- `INPUT_STALE`;
- `INPUT_INVALID`;
- `INPUT_ERROR`;
- `SAFETY_INTERLOCK_UNKNOWN`;
- `CONTROLLER_UNAVAILABLE`.

Reason metadata must be safe to persist/log.

### 4.4 Retry/re-evaluation

P1.3 does not create a second command retry engine.

Scheduler re-evaluation may occur on the next normal tick or authoritative input update.

A previously denied action must not be reconstructed as an implicit command merely because its prerequisites later become valid. Existing automation semantics determine whether a fresh trigger is required.

### 4.5 Startup behavior

After backend/controller restart:

- safety-critical scheduler actions remain inhibited until required authoritative state is known-valid;
- stale cached values do not authorize immediate execution;
- reconciliation may restore observation state but must not manufacture a successful safety predicate;
- startup itself is not evidence that devices are safe or available.

---

## 5. Actuator-state truth

The system must not fabricate a safe actuator snapshot.

### 5.1 All-pumps-off prohibition

A missing or failed actuator-state query must never be converted into:

```text
pump_1 = OFF
pump_2 = OFF
...
all_pumps_off = true
```

unless authoritative observation explicitly establishes those states.

### 5.2 Emergency stop semantics

Emergency stop remains safety-priority and must preserve P0/P1.2 command semantics.

P1.3 must distinguish:

1. stop command requested;
2. command published;
3. controller acknowledged;
4. actuator/runtime state observed safe;
5. unresolved/unknown safety state.

An emergency-stop request is not itself proof that every actuator is off.

If authoritative confirmation is unavailable, the system must expose the unresolved state rather than claiming `all_off`.

### 5.3 Partial actuator knowledge

If only some actuators are known:

```text
known OFF + unknown OFF? -> overall safety state remains UNKNOWN
```

Do not collapse partial knowledge into a boolean aggregate that implies complete observation.

Where the product requires a fleet/aggregate safety indicator, its semantics must explicitly distinguish:

- all observed safe;
- at least one observed unsafe;
- incomplete/unknown observation.

### 5.4 Control UI

Frontend controls may show command intent and observed actuator state separately.

A control button's disabled/enabled state must not be used as proof of physical actuator state.

---

## 6. Controller availability and contact

P0.3 establishes authoritative contact/availability semantics. P1.3 consolidates consumers around that contract.

### 6.1 Canonical source

Use the established device contact/availability evidence rather than a parallel legacy heartbeat heuristic.

If multiple signals exist, define their relationship explicitly:

```text
raw contact evidence
        |
        v
canonical availability/contact classification
        |
        +--> watchdog
        +--> backend API
        +--> WebSocket
        +--> frontend
        +--> scheduler safety gate
```

### 6.2 Timestamps

Keep distinct:

- device-observed time where available;
- backend receipt time;
- last-contact time;
- derived availability classification time.

Do not use one timestamp for multiple meanings.

### 6.3 Thresholds

Availability thresholds must be defined once per contract and not duplicated as unrelated magic numbers across backend, watchdog, firmware, and frontend.

A consumer may derive a presentation label from canonical availability, but must not silently create a different operational threshold.

### 6.4 Reconnect

MQTT reconnect is evidence of transport recovery, not proof of complete device state recovery.

After reconnect:

- contact may become known;
- telemetry may remain stale;
- actuator state may remain unknown;
- scheduler safety prerequisites remain blocked until their own evidence is valid.

---

## 7. Backup / restore lifecycle

Backup and restore must distinguish storage success from transport and physical application success.

### 7.1 Backup

A successful backup means the documented backup artifact was durably created according to its contract.

It does not imply:

- MQTT publication;
- controller receipt;
- physical configuration application.

### 7.2 Restore

Restore must expose separate stages where applicable:

```text
validate artifact
    |
authorize target device
    |
persist requested configuration
    |
publish synchronization command
    |
controller acknowledgement
    |
authoritative runtime/config observation
```

The API must not return a single `success` value that hides a later failed stage.

If persistence succeeds but MQTT publication fails, return the repository's documented partial-success semantics and preserve the durable state.

If physical application is not confirmed, do not report physical success.

### 7.3 Device identity

The route `device_id` remains the authorization/resource identity under P1.1.

A backup payload's embedded device ID must not grant access to another device.

Restore must validate that the caller owns/has capability for the target device before reading or mutating it.

### 7.4 Secret safety

Backup artifacts and logs must not expose:

- API keys;
- bearer tokens;
- privileged-control tokens;
- WiFi passwords;
- webhook secrets;
- private signing material.

If a backup format necessarily contains credential material, it must use the repository's established secret-protection mechanism rather than plaintext persistence.

### 7.5 Idempotent restore

Repeated restore requests with the same explicit idempotency key must follow P1.2-style idempotency semantics where the operation is command-backed.

A restore retry must not accidentally target another device or silently create a second logical command.

---

## 8. Flux query safety

InfluxDB/Flux queries must be treated as a data-access boundary, not as arbitrary string assembly.

### 8.1 Query construction

User/device-provided values must be encoded/parameterized using the repository's supported Flux-safe mechanism.

Never concatenate unvalidated user input into executable Flux syntax where a safe parameter/encoding path exists.

### 8.2 Device scope

Every device-scoped Flux query must carry an explicit authoritative device predicate.

The query target must come from the backend-authorized route/device context, not directly from an untrusted frontend selection.

P1.1 ownership remains the authorization boundary.

### 8.3 Allowlist

Where query shape is controlled by the API, prefer an allowlist of supported measurements, fields, aggregation functions, and time ranges.

Reject unsupported query dimensions instead of silently broadening the query.

### 8.4 Bounds

Enforce bounded:

- time ranges;
- result sizes;
- query complexity where practical;
- pagination/limit behavior.

A failed query must return an explicit error/unknown result, not an empty successful dataset unless the query actually returned zero rows.

### 8.5 Tests

Security tests must include:

- quote/escape-breaking input;
- attempted cross-device selector;
- unsupported measurement/field;
- excessive time range;
- query timeout/failure;
- empty legitimate result vs query failure distinction.

---

## 9. Observability

P1.3 closes observability gaps for safety-critical decisions.

### 9.1 Required event fields

Every safety-critical decision/event should expose, as appropriate:

- event ID;
- device ID;
- command ID when command-related;
- authenticated principal for user-originated actions;
- source subsystem;
- decision/outcome;
- reason code;
- relevant state quality/freshness;
- timestamp;
- correlation/request ID where available.

Do not include credentials or unrestricted sensitive payloads.

### 9.2 Metrics

At minimum expose counters/gauges for:

- scheduler safety denials;
- unknown/stale safety inputs;
- controller unavailable decisions;
- actuator-state unknown observations;
- emergency-stop unresolved confirmations;
- backup failures;
- restore partial failures;
- restore physical-confirmation failures;
- Flux query validation rejections;
- Flux query failures/timeouts;
- availability transitions;
- safety interlock trips;
- contradictory/late observations.

Metric labels must be bounded. Do not use raw user input, command payloads, or high-cardinality secrets as labels.

### 9.3 Logs

Operational logs must permit reconstruction of the decision path without requiring secret-bearing payloads.

Use stable reason codes instead of arbitrary error strings as the primary aggregation key.

### 9.4 Audit vs metrics

Metrics are operational signals.

Audit/history is the durable record where the underlying event requires historical accountability.

Neither metric nor log state may become authoritative device state.

---

## 10. Shared schema / frontend semantic alignment

P1.3 addresses semantic drift only where it can cause false safety or failure claims.

### 10.1 Canonical enums

Shared Rust contracts remain authoritative for cross-system lifecycle/state enums where they already exist.

Frontend types must not invent incompatible values for:

- availability;
- state quality;
- command lifecycle;
- safety/interlock outcome.

### 10.2 Unknown preservation

If the backend sends `UNKNOWN`, `STALE`, `ERROR`, or an equivalent explicit quality state, frontend parsing must preserve it.

Unknown enum values from future backend versions must not silently become a safe-looking default.

### 10.3 API success semantics

Frontend success indicators must reflect the actual API contract.

For multi-stage operations, a partial-success response remains partial success.

Command request completion remains distinct from durable lifecycle and physical confirmation.

### 10.4 No frontend safety authority

Frontend state can render safety status and request safety actions.

It cannot authorize a backend scheduler action, claim ownership, confirm physical state, or turn an unknown observation into safe state.

---

## 11. API/error semantics

P1.3 must preserve explicit failure classes through API boundaries.

At minimum distinguish where applicable:

```text
400 invalid request/query
401 unauthenticated
403 unauthorized
404 genuinely absent resource
409 idempotency/conflict
422 semantically invalid operation
500 backend/internal failure
503 required dependency unavailable
```

Exact repository conventions may differ, but the implementation must not convert dependency failure into a valid empty/safe response.

For safety-critical state reads:

```text
success + valid state -> state
success + empty legitimate result -> empty/unknown per contract
query/dependency error -> explicit error/unavailable
stale data -> stale
```

---

## 12. Testing requirements

### 12.1 Scheduler

Database/unit/integration coverage must include:

1. valid fresh inputs permit eligible action;
2. missing required input blocks action;
3. stale required input blocks action;
4. malformed input blocks action;
5. dependency/query failure blocks action;
6. controller unavailable blocks dependent action;
7. fail-closed reason is recorded;
8. startup does not execute based on stale cached safety state;
9. later recovery of inputs does not fabricate a historical execution;
10. normal false predicate remains distinguishable from unknown/error.

### 12.2 Actuator state

Test:

1. all actuators observed off -> aggregate safe/off where contract permits;
2. one actuator observed on -> aggregate unsafe/not-off;
3. one actuator unknown -> aggregate unknown/incomplete;
4. query failure -> not fabricated as all-off;
5. emergency-stop request without physical confirmation -> unresolved, not confirmed safe;
6. authoritative confirmation -> safe state only when predicate is satisfied.

### 12.3 Availability/contact

Test:

1. valid contact updates canonical availability;
2. stale contact becomes stale/unavailable according to contract;
3. telemetry freshness and contact freshness remain distinct;
4. reconnect does not fabricate fresh telemetry;
5. watchdog/backend/frontend consume the same canonical classification;
6. legacy heartbeat heuristic cannot override canonical state.

### 12.4 Backup/restore

Test:

1. unauthorized backup denied;
2. unauthorized restore denied;
3. payload device ID cannot bypass target ownership;
4. backup persistence failure is failure;
5. restore persistence success + MQTT failure is partial success;
6. restore MQTT success without physical confirmation is not physical success;
7. secret fields are not exposed in artifact/log/API history;
8. repeated idempotent restore submission does not create duplicate logical commands.

### 12.5 Flux

Test:

1. safe device query returns only authorized device data;
2. injection-like device/field input is rejected or safely encoded;
3. unsupported query shape is rejected;
4. excessive time range is rejected/clamped according to contract;
5. query failure is not empty success;
6. legitimate zero-row result remains distinguishable from query failure.

### 12.6 Observability

Test/evidence must prove:

- safety-denial metrics increment;
- reason codes are bounded and present;
- command correlation is preserved where applicable;
- logs contain no protected credentials;
- metrics do not create unbounded label cardinality.

### 12.7 Cross-system

When toolchains are available, verify:

```text
shared
  -> controller-core
  -> firmware
  -> watchdog
  -> backend
  -> simulator
  -> frontend
```

Any semantic enum/schema change must compile and execute through all current consumers.

---

## 13. Security requirements

P1.3 must preserve P1.1 and P1.2 security boundaries:

- authentication occurs before protected operation;
- capability is checked explicitly;
- ownership comes from authoritative `device_ownership`;
- command IDs are not authorization credentials;
- lifecycle events cannot cross device boundaries;
- backup payloads cannot grant ownership;
- Flux selectors cannot bypass ownership;
- privileged tokens are never persisted/logged as secrets;
- failure does not fall back to a privileged/default context;
- unknown state does not become an authorization or safety proof.

No safety bypass may be introduced to preserve legacy behavior.

---

## 14. Migration strategy

Recommended order:

```text
1. Inventory safety-critical decisions and current fallback values
2. Freeze P0.3/P0.4/P1.1/P1.2 authority boundaries
3. Define canonical safety-input quality + availability semantics
4. Remove fabricated all-safe/all-pumps-off fallbacks
5. Make scheduler prerequisites fail closed
6. Consolidate controller contact/availability consumers
7. Harden backup/restore outcome stages
8. Harden Flux query construction + scope + bounds
9. Add safety-critical observability
10. Align shared/backend/frontend failure enums where required
11. Add negative/restart/integration/concurrency tests
12. Run cross-system verification
13. Record evidence + traceability
```

Migration must be incremental and behavior-preserving for valid known-good inputs.

For previously fabricated values, the intended behavior change is explicit: expose unknown/error and inhibit safety-critical action.

---

## 15. Acceptance criteria

### AC-1 - Fail-closed scheduler

Safety-critical scheduler decisions do not execute when required inputs are unknown, stale, invalid, or unavailable.

### AC-2 - No fabricated actuator safety

Missing/error/partial actuator state cannot produce an all-off/all-safe result without authoritative evidence.

### AC-3 - Emergency-stop truth

Emergency-stop request, command acknowledgement, and physical safe-state confirmation remain distinct; unresolved physical state is not reported as confirmed safe.

### AC-4 - Canonical availability

Backend, watchdog, controller integration, WebSocket, and frontend consume one documented availability/contact semantic without competing heartbeat heuristics.

### AC-5 - Freshness separation

Contact freshness, telemetry freshness, controller health, and actuator state remain distinct and are not substituted for one another.

### AC-6 - Backup/restore semantics

Backup and restore expose persistence, transport, and physical application outcomes separately where applicable; partial success is not reported as complete physical success.

### AC-7 - Backup authorization/security

Backup/restore remains protected by P1.1 capability + ownership, and payload device identity cannot bypass authorization. Secrets are not exposed.

### AC-8 - Flux query safety

Device-scoped Flux queries are safely constructed, bounded, authorized, and distinguish legitimate empty results from query failures.

### AC-9 - Unknown/error preservation

Backend and frontend preserve explicit unknown/stale/error states rather than substituting safe-looking defaults.

### AC-10 - Safety observability

Safety-critical denials, failures, availability transitions, and unresolved outcomes have bounded reason codes and operational metrics/audit coverage without secrets or unbounded metric labels.

### AC-11 - Restart/reconnect safety

Backend/controller/MQTT restart or reconnect cannot cause a safety-critical action to execute from stale/unknown state merely because transport/process recovery completed.

### AC-12 - Cross-system semantic alignment

Required availability, quality, safety, and failure semantics compile and remain compatible across shared, controller, watchdog, simulator, backend, and frontend consumers.

### AC-13 - Negative-path verification

Tests cover unknown, stale, dependency failure, fabricated-state prevention, backup/restore partial failure, Flux rejection/failure, and restart/reconnect cases.

### AC-14 - P1.1/P1.2 preservation

No authorization/ownership invariant or durable command lifecycle invariant is weakened or duplicated by P1.3.

### AC-15 - Traceability

`docs/project-state/TRACEABILITY.md` and `docs/project-state/CURRENT-STATUS.md` record P1.3 status, evidence, residual risks, and the boundary to the next phase.

---

## 16. Non-goals

- redesigning command lifecycle;
- replacing PostgreSQL, InfluxDB, MQTT, Firebase, or existing storage infrastructure;
- replacing P1.1 authorization;
- implementing a second scheduler;
- inventing new actuator capabilities;
- claiming exactly-once physical actuation;
- replacing the P0.3 telemetry architecture;
- frontend architecture redesign;
- generic event sourcing;
- broad RBAC redesign;
- changing product behavior for known-valid inputs without a safety reason.

---

## 17. Expected implementation surfaces

Exact files must be confirmed by source audit. Likely surfaces include:

```text
hydragrow-backend/src/services/cron_scheduler.rs
hydragrow-backend/src/services/action_dispatch.rs
hydragrow-backend/src/mqtt/handlers/sensors.rs
hydragrow-backend/src/mqtt/handlers/status.rs
hydragrow-backend/src/api/config_backup.rs
hydragrow-backend/src/api/health_topics.rs
hydragrow-backend/src/api/analytics.rs
hydragrow-backend/src/api/sensor.rs
hydragrow-backend/src/db/postgres.rs
hydragrow-backend/src/metrics.rs
hydragrow-controller-core/src/*
ESP32-C3-CONTROLLER-NODE/src/*
hydragrow-watchdog/src/*
hydragrow-simulator/src/*
hydragrow-shared/src/*
hydragrow-frontend/src/*
```

Source audit must locate every safety-critical fallback before implementation begins.

---

## 18. Evidence requirements

Evidence must identify, for each acceptance criterion:

```text
Requirement ID: P1.3-SAFETY-FAILURE-SEMANTICS
Surface:
Authoritative source:
Failure condition:
Expected decision:
Actual decision:
Reason code:
Test command:
Environment:
Timestamp:
```

For security-sensitive or safety-critical cases, evidence must include negative-path execution.

A static grep proving that a fallback exists or was removed is supporting evidence only; it does not replace runtime verification where executable tooling is available.

Environment blockers must use the P1.0 status model and must not be reported as passing tests.

---

## 19. Risks and rollback

### Risks

- changing fail-open behavior may expose previously hidden automation/configuration failures;
- availability threshold consolidation may reveal disagreement between watchdog and backend behavior;
- backup/restore stage separation may change existing API success handling;
- Flux validation may reject previously accepted but unsafe query shapes;
- replacing fabricated state with unknown may require frontend rendering changes;
- cross-system semantic alignment may expose stale generated/manual models.

### Rollback

Rollback must not restore fabricated safety state or fail-open scheduler behavior.

If a deployment incompatibility requires temporary compatibility behavior, it must be:

- explicitly scoped;
- non-authoritative;
- observable;
- time-bounded;
- prohibited from authorizing safety-critical actions when required state is unknown.

Do not roll back to an implementation that reports unknown actuators as safely off merely to preserve UI appearance.

---

## 20. Implementation order

```text
P1.3 Safety-Critical Failure Semantics / Operational Truth
    |
    +-- 1. Inventory safety-critical reads, decisions, and fabricated fallbacks
    |
    +-- 2. Freeze P0.3/P0.4/P1.1/P1.2 authority contracts
    |
    +-- 3. Define canonical quality + availability decision matrix
    |
    +-- 4. Remove fabricated actuator/all-safe state
    |
    +-- 5. Enforce scheduler fail-closed behavior
    |
    +-- 6. Consolidate controller availability/contact semantics
    |
    +-- 7. Harden backup/restore lifecycle and outcome reporting
    |
    +-- 8. Harden Flux query construction, scope, and bounds
    |
    +-- 9. Add safety observability and bounded reason codes
    |
    +-- 10. Align shared/backend/frontend semantic contracts
    |
    +-- 11. Add negative/restart/cross-system tests
    |
    +-- 12. Run declared verification commands
    |
    +-- 13. Record evidence + traceability
    |
    v
Next platform/reliability phase
```

---

## 21. Completion rule

P1.3 is complete only when safety-critical paths fail closed, unknown state is never fabricated into a safe state, availability/contact semantics have one authoritative meaning, backup/restore outcomes are explicit, Flux queries are safely bounded/scoped, and safety-critical failures are observable.

The final evidence must demonstrate at minimum:

```text
known-good input
unknown input
stale input
dependency failure
fabricated actuator-state prevention
emergency-stop unresolved state
controller unavailable
backend restart
MQTT reconnect
backup/restore partial failure
Flux query rejection/failure
frontend unknown/error preservation
cross-system verification
```

A happy-path scheduler test or a successful backup alone is insufficient.
