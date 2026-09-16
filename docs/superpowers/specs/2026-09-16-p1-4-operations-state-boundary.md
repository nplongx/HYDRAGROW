# P1.4 Operations State Boundary

## Status

CLOSED; current runtime restart/reconnect evidence verified.

P1.4 follows the locked P1 sequence from the Readiness Audit. It is **not** a generic recovery/reconciliation phase and does not reopen P0 architecture or pull P1.5/P1.6 work forward.

The authoritative P1 sequence is:

```text
P1.0 Verification + cross-system baseline
P1.1 Auth / Authorization Boundary
P1.2 Durable Command Lifecycle
P1.3 Error / Result Semantics
P1.4 Operations State Boundary
P1.5 Backup / Restore Lifecycle
P1.6 Journal / Event Model
P1.7 API Query / Mutation Consolidation
P1.8 Route / Navigation Contract
P1.9 Shared Schema Alignment
P1.10 Observability + Cross-system Sync
```

P1.4 therefore assumes P1.0-P1.3 contracts exist, but must not depend on implementation that belongs to later phases.

## Requirement ID

`P1.4-OPERATIONS-STATE-BOUNDARY`

## Change class

`C4` Cross-subsystem + `C6` Safety/reliability + `C8` Operational state/consistency.

## Purpose

Establish one explicit boundary for **operational state**: what the backend knows about controller/device contact, runtime availability, actuator observation, synchronization progress, and operational readiness.

The governing rule is:

> Operational state must describe observed system condition, not transport intent, cached UI state, inferred defaults, or an optimistic assumption about what happened.

P1.4 closes the gap between P1.3's error/result semantics and the later P1.10 cross-system synchronization work. It defines the state boundary now, while leaving durable journaling, schema generation, and broad observability to their assigned phases.

## 1. Scope

P1.4 covers:

- canonical controller/device operational availability;
- distinction between contact, transport connectivity, telemetry freshness, and runtime readiness;
- authoritative actuator/runtime observation versus requested command state;
- operational-state freshness and staleness;
- startup/restart/reconnect state transitions;
- safe handling of partial or contradictory operational observations;
- backend operational state exposed through existing REST/WebSocket boundaries;
- frontend preservation of operational `UNKNOWN` / `STALE` / `ERROR` semantics;
- watchdog/backend/controller state alignment;
- operational-state tests across restart, reconnect, delayed and missing observations;
- bounded evidence and traceability for the above.

P1.4 may add or normalize internal state representations when required to enforce this boundary.

## 2. Explicit non-scope

P1.4 does **not** implement or redesign:

- P1.0 verification environment or simulator compilation;
- P1.1 authorization/ownership boundary;
- P1.2 durable command lifecycle;
- P1.3 generic error/result semantics already assigned there;
- P1.5 backup/restore lifecycle;
- P1.6 journal/event-sourcing model;
- P1.7 broad API query/mutation consolidation;
- P1.8 route/navigation contract;
- P1.9 generated/shared schema alignment;
- P1.10 final observability and cross-system synchronization platform;
- new MQTT architecture;
- new telemetry architecture;
- redesign of `StationContext`, React Query ownership, `useDeviceStore`, or `useDeviceSync`;
- exactly-once physical actuation claims.

P1.4 also does not use operational-state work as a reason to bypass P1.1 authorization. Existing authorization remains authoritative for every device-scoped read or mutation.

## 3. Authority model

### 3.1 Operational state

The backend is authoritative for the operational state it publishes to API/WebSocket consumers, but that state must be derived from authoritative device/runtime observations and explicitly tracked freshness.

The following are **not** authoritative operational truth:

- frontend local state;
- `localStorage`;
- React Query cache by itself;
- a previously observed state with no valid freshness;
- an MQTT reconnect event by itself;
- a command publish acknowledgement;
- a UI request payload;
- a controller command intent without runtime observation;
- an inferred default such as `OFF`, `ONLINE`, or `READY`.

### 3.2 Transport versus operational state

P1.4 must keep these concepts separate:

```text
MQTT connected
        !=
controller contact observed
        !=
telemetry fresh
        !=
runtime ready
        !=
actuator state known
```

MQTT reconnect is evidence that transport recovered. It is not evidence that the controller has resumed publishing valid state or that actuators reached a safe state.

### 3.3 Observation timestamps

Where available, retain distinct timestamps for:

- device observation time;
- backend receipt time;
- last valid contact time;
- last valid telemetry time;
- operational-state classification time.

One timestamp must not silently substitute for another.

## 4. Canonical operational state

P1.4 defines the minimum semantic dimensions below. Exact enum representation may follow the existing shared/backend contract until P1.9.

### 4.1 Contact

```text
UNKNOWN
CONTACTED
NOT_CONTACTED
```

`CONTACTED` means there is a valid, recent observation establishing device/backend contact under the existing telemetry/watchdog contract.

`NOT_CONTACTED` must not be emitted merely because an MQTT socket is disconnected unless the existing contract explicitly defines that transport condition as sufficient evidence.

### 4.2 Freshness

```text
UNKNOWN
FRESH
STALE
```

Freshness thresholds must have one source of truth. P1.4 must not introduce duplicated magic thresholds across scheduler, watchdog, backend handlers, and frontend.

### 4.3 Runtime readiness

```text
UNKNOWN
READY
NOT_READY
```

`READY` requires the authoritative runtime observation required by the contract. It must not mean merely “process exists”, “MQTT is connected”, or “last command succeeded”.

### 4.4 Actuator knowledge

```text
UNKNOWN
KNOWN
STALE
CONTRADICTORY
```

An aggregate actuator state is `KNOWN` only when all required components needed for that aggregate are known and valid.

For example:

```text
pump A = OFF
pump B = UNKNOWN
=> all-pumps-off = UNKNOWN
```

Never collapse partial knowledge into an affirmative safe state.

## 5. Operational state transition rules

### 5.1 Startup

After backend restart:

1. in-memory caches may be empty;
2. no operational state may be fabricated from empty cache;
3. state becomes known only after a valid authoritative observation;
4. stale persisted/cache data, if any, cannot by itself prove current runtime state;
5. safety-critical consumers must remain inhibited while required operational prerequisites are unknown/stale.

### 5.2 Controller restart

Controller restart must not imply:

- all actuators OFF;
- runtime READY;
- configuration synchronized;
- command physically applied.

Those facts require their respective observations.

### 5.3 MQTT reconnect

On MQTT reconnect:

- transport state may transition to connected;
- operational state remains at its previous semantic quality until fresh valid observations arrive;
- delayed/out-of-order observations must not overwrite newer state;
- reconnect must not manufacture a heartbeat, telemetry packet, or actuator observation.

### 5.4 Missing observation

Missing required observation produces `UNKNOWN` or `STALE` according to the freshness contract, never an inferred healthy value.

### 5.5 Contradictory observations

When observations disagree beyond the existing ordering/freshness rules:

- do not choose a convenient value;
- retain the contradiction/unknown semantic;
- do not authorize a safety-critical action from the contradiction;
- record bounded diagnostic context for later reconciliation.

## 6. Ordering and stale-observation protection

Operational observations must carry enough ordering information to prevent an older observation from replacing a newer one.

Acceptable ordering evidence may be an existing controller sequence number, observation timestamp, or equivalent monotonic contract. P1.4 must use the strongest existing source without inventing a second competing ordering system.

Rules:

1. newer valid observation wins over older valid observation;
2. older observation cannot restore `FRESH` after a newer state became stale;
3. delayed observation cannot overwrite a newer actuator state;
4. equal observations may be handled idempotently;
5. impossible ordering/clock data becomes `UNKNOWN` rather than being normalized into current state.

## 7. Watchdog boundary

The watchdog is an operational signal producer, not an independent authority that contradicts backend/device observation.

The contract must identify:

- what constitutes a contact signal;
- its freshness threshold;
- what backend state is derived from it;
- which conditions transition to unknown/stale/not-contacted;
- how recovery occurs after fresh observation.

The watchdog must not report a healthy state solely because its own process is alive.

## 8. Scheduler interaction

P1.4 consumes the P1.3 safety boundary rather than redefining it.

Operational prerequisites such as controller availability, fresh actuator observation, and runtime readiness may be required inputs to an action decision.

If a required operational prerequisite is unknown/stale/error:

```text
action eligibility = false
```

The scheduler must not create a second recovery loop or silently retry as part of P1.4.

Normal scheduler re-evaluation remains responsible for reconsidering the action after valid state returns.

## 9. Command interaction

P1.4 must preserve P1.2's distinction between:

```text
requested
sent
acknowledged
confirmed
```

Operational state may provide the observation used to establish `CONFIRMED`, but P1.4 must not redesign the durable command lifecycle.

In particular:

- MQTT publish is not physical application;
- controller ACK is not physical confirmation;
- command timeout is not equivalent to actuator OFF;
- unknown operational state remains unknown after command completion unless an authoritative observation resolves it.

## 10. Existing API/WebSocket boundary

P1.4 may normalize existing operational-state payloads so all consumers receive the same semantics.

Required behavior:

- REST reads distinguish current known state from stale/unknown/error;
- WebSocket updates preserve state quality and timestamps;
- device-scoped events remain subject to P1.1 authorization;
- a missing optional observation is not serialized as a misleading safe default;
- unknown future enum values must remain non-safe at frontend boundaries;
- API response semantics must not be broadened into P1.7 query/mutation redesign.

## 11. Frontend contract

The frontend keeps its existing architecture:

```text
StationContext
React Query ownership
useDeviceStore -> PWM preferences only
useDeviceSync -> transport/orchestration
```

P1.4 changes only operational-state interpretation where necessary.

Frontend requirements:

- display `UNKNOWN`, `STALE`, and `ERROR` distinctly where safety/operations depend on them;
- do not render unknown actuator state as OFF;
- do not render unknown controller contact as OFFLINE/ONLINE without contract evidence;
- do not infer synchronization completion from mutation success alone;
- after reconnect, wait for authoritative state before replacing unknown/stale state with current state;
- preserve backend authority rather than creating frontend reconciliation logic.

## 12. Recovery boundary

P1.4 includes **state re-entry**, not a new durable recovery engine.

For backend/controller restart or reconnect:

```text
transport/process recovery
        |
        v
operational state unknown/stale
        |
        v
fresh authoritative observation
        |
        v
validated operational state
        |
        v
normal consumers resume
```

No step may skip directly from process/transport recovery to `READY`, `SAFE`, `SYNCHRONIZED`, or `ACTUATOR_OFF` without evidence.

P1.5 owns backup/restore lifecycle recovery. P1.6 owns durable journal/event history. P1.10 owns final cross-system synchronization/observability.

## 13. Error semantics

Operational reads must distinguish:

| Condition | Meaning |
|---|---|
| Valid observation | Current operational fact is known |
| Legitimate absence | The requested optional fact genuinely does not exist |
| Unknown | No authoritative fact is currently available |
| Stale | A previous fact exists but freshness contract expired |
| Contradictory | Valid observations cannot currently be reconciled |
| Dependency error | Infrastructure prevented determining the state |

Dependency error must not be converted into legitimate absence, `OFF`, `READY`, `ONLINE`, or an empty successful result.

## 14. Security boundary

P1.4 preserves P1.1 security requirements:

- device ownership remains backend-authoritative;
- operational-state reads remain device-scoped;
- WebSocket subscriptions cannot broaden access;
- command IDs are not credentials;
- operational payloads contain no privileged secrets;
- unknown state is never used as authorization proof.

P1.4 must not introduce a parallel authorization mechanism.

## 15. Expected implementation surfaces

Initial audit/implementation candidates:

```text
hydragrow-backend/src/mqtt/handlers/status.rs
hydragrow-backend/src/mqtt/handlers/sensors.rs
hydragrow-backend/src/api/health_topics.rs
hydragrow-backend/src/api/sensor.rs
hydragrow-backend/src/services/cron_scheduler.rs
hydragrow-backend/src/services/action_dispatch.rs
hydragrow-backend/src/metrics.rs
hydragrow-watchdog/src/*
hydragrow-controller-core/src/*
hydragrow-simulator/src/*
hydragrow-shared/src/*
hydragrow-frontend/src/*
```

The implementation must begin with a source audit. Do not assume these are the only operational-state producers or consumers.

## 16. Required source audit

Before changing behavior, locate:

- all device availability/contact calculations;
- all watchdog heartbeat calculations;
- all MQTT connected/disconnected handlers;
- all actuator-state aggregation;
- all fallback expressions affecting operational state;
- all `unwrap_or_default()` / `unwrap_or(None)` in operational-state paths;
- all API/WebSocket operational-state payloads;
- all frontend mappings from backend state to UI state;
- all startup/reconnect state initialization;
- all timestamp/freshness checks;
- all code treating command ACK as physical confirmation.

The audit must distinguish safety-critical fallback from harmless presentation/defaulting code. Do not mass-change every fallback mechanically.

## 17. Acceptance criteria

### AC-1 - One operational-state boundary

Backend, watchdog, controller observations, REST, WebSocket, and frontend use one documented semantic model for contact/freshness/readiness/actuator knowledge.

### AC-2 - Transport separation

MQTT connected/reconnected is not treated as proof of fresh telemetry, runtime readiness, or actuator state.

### AC-3 - Startup safety

After backend restart, required operational state is unknown/stale until fresh authoritative observation arrives.

### AC-4 - Controller restart safety

Controller restart does not fabricate OFF, READY, synchronized, or physically-applied state.

### AC-5 - Reconnect safety

MQTT reconnect does not fabricate heartbeat/telemetry/actuator observations and does not bypass freshness validation.

### AC-6 - Stale ordering protection

Delayed or older observations cannot overwrite newer valid state or restore freshness incorrectly.

### AC-7 - Partial actuator knowledge

An aggregate actuator-safe/off state is unknown whenever a required component is unknown, stale, or contradictory.

### AC-8 - Dependency failure preservation

Operational dependency errors remain errors/unknown and are never represented as legitimate empty/absent/safe state.

### AC-9 - Scheduler integration

Safety-critical scheduling remains inhibited when required operational prerequisites are unknown/stale/error.

### AC-10 - Durable lifecycle preservation

P1.2 command lifecycle semantics remain unchanged; operational observations can resolve confirmation but do not redefine lifecycle states.

### AC-11 - API/WebSocket semantics

Operational state exposed through existing boundaries preserves state quality and freshness without weakening P1.1 authorization.

### AC-12 - Frontend semantics

Frontend does not collapse unknown/stale/error operational state into OFF/ONLINE/READY/synchronized.

### AC-13 - Watchdog consistency

Watchdog process health is not independently presented as device operational health.

### AC-14 - Negative-path coverage

Tests cover missing observation, stale observation, dependency failure, delayed observation, contradictory observation, backend restart, controller restart, and MQTT reconnect.

### AC-15 - Architecture preservation

No changes reopen P0.5 architecture. `StationContext`, React Query ownership, `useDeviceStore` PWM-preference role, and `useDeviceSync` transport/orchestration remain intact.

### AC-16 - Phase boundary

No P1.5 backup/restore lifecycle, P1.6 journal/event model, P1.7 API redesign, P1.8 navigation redesign, P1.9 schema-generation project, or P1.10 final observability platform is pulled into P1.4.

### AC-17 - Traceability

Every changed operational-state rule maps to a test/evidence record with requirement ID, source surface, failure condition, expected decision, actual result, and environment.

## 18. Required test matrix

At minimum:

```text
known fresh observation             -> known operational state
missing observation                 -> UNKNOWN
expired observation                 -> STALE
dependency/query failure            -> ERROR/UNKNOWN
MQTT reconnect                      -> transport recovery only
controller restart                  -> UNKNOWN until fresh observation
backend restart                    -> UNKNOWN until fresh observation
old observation after new one       -> old ignored
contradictory observations          -> CONTRADICTORY/UNKNOWN
partial actuator state              -> aggregate UNKNOWN
command ACK without runtime proof   -> not CONFIRMED by implication
fresh runtime observation           -> state becomes authoritative
frontend unknown                    -> no safe default
authorized device                   -> operational state readable
unauthorized device                 -> denied by P1.1 boundary
```

Tests must assert semantics, not merely HTTP status or object existence.

## 19. Evidence requirements

Each acceptance criterion requiring executable verification records:

```text
Requirement ID:
Surface:
Authoritative source:
Failure condition:
Expected decision:
Actual decision:
Reason/state-quality code:
Test command:
Environment:
Timestamp:
```

Static grep/source inspection is supporting evidence only. It cannot replace a required runtime test.

Environment failures are recorded as `BLOCKED`, not `PASS`.

## 20. Implementation order

```text
P1.4 Operations State Boundary
    |
    +-- 1. Freeze P1.0-P1.3 contracts and current worktree
    |
    +-- 2. Audit every operational-state producer/consumer
    |
    +-- 3. Define contact/freshness/readiness/actuator quality matrix
    |
    +-- 4. Identify and remove fabricated operational-safe defaults
    |
    +-- 5. Separate transport state from device operational state
    |
    +-- 6. Add ordering/stale-observation protection
    |
    +-- 7. Normalize watchdog/backend/controller boundary
    |
    +-- 8. Integrate scheduler prerequisites without adding a second retry engine
    |
    +-- 9. Normalize existing REST/WebSocket operational payloads
    |
    +-- 10. Preserve frontend architecture while fixing unknown/stale/error handling
    |
    +-- 11. Add negative/restart/reconnect/concurrency tests
    |
    +-- 12. Run declared verification commands
    |
    +-- 13. Record evidence + traceability
    |
    v
P1.5 Backup / Restore Lifecycle
```

## 21. Risks and rollback

### Risks

- existing code may have conflated MQTT connectivity with device health;
- frontend may have encoded backend absence as a safe default;
- restart may expose stale operational caches that were previously hidden;
- stricter ordering may surface controller clock/sequence defects;
- partial actuator knowledge may require UI changes;
- watchdog and backend may currently use different freshness assumptions.

### Rollback constraints

Rollback must never restore a fabricated safe operational state or fail-open safety decision.

If temporary compatibility is unavoidable, it must be:

- explicitly non-authoritative;
- observable;
- time-bounded;
- unable to authorize safety-critical actions from unknown prerequisites.

## 22. Completion rule

P1.4 is complete only when:

1. operational state has one explicit authority boundary;
2. transport/contact/freshness/readiness/actuator knowledge are not conflated;
3. restart/reconnect returns through unknown/stale state until fresh evidence exists;
4. delayed/contradictory observations cannot silently produce false current state;
5. dependency failures remain distinguishable from legitimate absence;
6. scheduler safety continues to consume fail-closed prerequisites;
7. frontend preserves operational uncertainty;
8. P1.1/P1.2/P1.3 semantics remain intact;
9. required negative-path evidence is current and reproducible;
10. no later P1 phase has been silently absorbed into P1.4.

P1.4 is an **operations-state boundary phase**, not a replacement for P1.5-P1.10.
