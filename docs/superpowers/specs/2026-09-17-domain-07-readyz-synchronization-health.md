# HYDRAGROW — Domain Spec 07: `/readyz` + Synchronization Health

**Domain ID:** `READINESS-SYNC-001`  
**Domain:** Backend readiness and durable ConfigurationSync health contract  
**Status:** `COMPLETE / VERIFIED`  
**Scope:** `/livez`, `/readyz`, and aggregate backend synchronization-health semantics

---

## 0. Purpose

This spec defines the backend health contract used to distinguish process liveness from service readiness and to expose the operational health of durable configuration synchronization.

The domain answers one question:

> Does `/readyz` report whether the backend can safely receive normal traffic, while exposing enough synchronization-health state to distinguish a live process, unavailable dependencies, a stopped worker, and recoverable configuration work?

Domain 4 already introduced `configuration_sync_worker` readiness and Domain 6 defined safety-data failure semantics. This domain hardens and verifies the boundary between those concerns.

---

# 1. Domain Boundary

## 1.1 In scope

- `/livez` liveness semantics.
- `/readyz` HTTP status and JSON response contract.
- PostgreSQL readiness dependency.
- InfluxDB readiness dependency.
- MQTT readiness dependency.
- event-bus readiness dependency.
- command reconciliation worker readiness.
- ConfigurationSync worker lifecycle readiness.
- aggregate synchronization health represented by `/readyz`.
- startup and restart readiness transitions.
- worker exit clearing readiness.
- distinction between normal pending sync work and synchronization subsystem failure.
- visibility of synchronization pending/failed work in readiness health without treating normal pending work as process failure.
- focused health/readiness tests and current verification evidence.

## 1.2 Out of scope

- ConfigurationSync persistence, revision allocation, publish retry, or device confirmation semantics.
- Backup/Restore behavior.
- Safety-data classification or actuator fail-closed behavior.
- Migration identity or PostgreSQL provisioning.
- Firmware build/toolchain or fixture consumers.
- Frontend readiness UI.
- HIL / physical E2E.
- Release readiness.

---

# 2. Current Repository State

Current implementation observations:

- `observability::liveness()` returns HTTP 200 with `{"status":"ok"}`.
- `observability::readiness()` probes PostgreSQL with `SELECT 1`.
- `observability::readiness()` probes InfluxDB health.
- MQTT connection state is read from `AppState::mqtt_connected`.
- event-bus readiness is currently hard-coded `true`.
- `command_reconciliation_worker` and `configuration_sync_worker` are represented by atomic readiness flags.
- `/livez` and `/readyz` are registered outside the authenticated `/api` scope.
- ConfigurationSync worker sets its readiness flag when its task starts and clears it through `WorkerGuard` when the task exits.
- ConfigurationSync durable work is stored in PostgreSQL and therefore pending work survives worker/backend restart.

These are repository observations, not verification evidence.

---

# 3. Liveness Contract

`/livez` is process liveness only.

Required behavior:

```text
GET /livez
    |
    +-- backend HTTP process can execute handler -> HTTP 200
```

`/livez` MUST NOT depend on PostgreSQL, InfluxDB, MQTT, ConfigurationSync, command reconciliation, or device state.

A dependency outage MUST NOT turn a live process into a liveness failure.

---

# 4. Readiness Contract

`/readyz` is service readiness, not process liveness.

The current required readiness dependencies are:

```text
postgresql
influxdb
mqtt
event_websocket_fanout
configuration_synchronization_worker
command_reconciliation_worker
```

Overall readiness is true only when every required dependency is healthy:

```text
ready = postgresql
     AND influxdb
     AND mqtt
     AND event_websocket_fanout
     AND configuration_synchronization_worker
     AND command_reconciliation_worker
```

If any required dependency is unavailable, `/readyz` MUST return HTTP `503 Service Unavailable`.

If all required dependencies are healthy, `/readyz` MUST return HTTP `200 OK`.

The endpoint MUST remain unauthenticated, because it is intended for process orchestration and load-balancer health checks.

---

# 5. Machine-Readable Response Contract

Successful response:

```json
{
  "status": "ready",
  "dependencies": {
    "postgresql": true,
    "influxdb": true,
    "mqtt": true,
    "event_websocket_fanout": true,
    "configuration_synchronization_worker": true,
    "command_reconciliation_worker": true
  },
  "synchronization": {
    "worker_ready": true,
    "transport_ready": true,
    "health": "healthy"
  }
}
```

Unready response uses the same dependency fields and returns:

```json
{
  "status": "not_ready",
  "dependencies": { "...": false },
  "synchronization": {
    "worker_ready": false,
    "transport_ready": false,
    "health": "unavailable"
  }
}
```

The exact response MUST remain stable enough for machine consumers to inspect individual dependency state without parsing human log text.

The endpoint MUST NOT expose SQLx errors, database connection strings, credentials, MQTT credentials, or internal exception chains.

---

# 6. Synchronization Health Contract

Configuration synchronization has two distinct readiness dimensions:

```text
worker_ready
transport_ready
```

`worker_ready` means the durable synchronization worker task is alive and has not exited.

`transport_ready` means MQTT is currently connected, because MQTT is the delivery transport used by ConfigurationSync.

Aggregate synchronization health is:

```text
worker_ready AND transport_ready -> healthy
worker_ready AND !transport_ready -> blocked
!worker_ready                  -> unavailable
```

The synchronization health state describes the synchronization subsystem. It MUST NOT redefine the durable ConfigurationSync target state machine from Domain 4.

Normal durable work is not itself a readiness failure:

```text
pending work > 0
    !=
worker failure
```

Likewise, a target in `failed` state is synchronization work requiring operator/configuration intervention, but MUST NOT make the whole backend process unready solely because one device has terminal synchronization failure. Such work remains visible through the existing ConfigurationSync status API and synchronization metrics.

This prevents `/readyz` from becoming device-workload-dependent while still making the transport and worker lifecycle visible.

---

# 7. Dependency Semantics

## 7.1 PostgreSQL

Readiness MUST execute a real lightweight database probe.

```text
probe succeeds -> postgresql = true
probe fails    -> postgresql = false
```

Database failure returns `503` from `/readyz` but MUST NOT expose internal DB details.

## 7.2 InfluxDB

Readiness MUST use the existing InfluxDB health mechanism.

```text
health succeeds -> influxdb = true
health fails    -> influxdb = false
```

## 7.3 MQTT

Readiness uses the backend MQTT connection state.

```text
connected    -> mqtt = true
disconnected -> mqtt = false
```

MQTT loss MUST clear readiness while preserving durable ConfigurationSync state.

## 7.4 Event bus

The event-bus readiness flag represents whether the in-process fanout mechanism is available for normal operation.

It MUST NOT be reported healthy merely because a boolean literal is present if the implementation introduces an actual lifecycle state for that bus. For the current broadcast channel implementation, a constant healthy state is acceptable only if tests document that the channel is owned by `AppState` for the process lifetime.

## 7.5 Command reconciliation worker

Its readiness flag is required independently from ConfigurationSync readiness.

Worker exit MUST make `/readyz` return `503` until the worker is recreated and reports ready.

## 7.6 ConfigurationSync worker

Worker exit MUST clear `configuration_sync_worker` readiness.

Worker startup MUST establish readiness only while the worker task remains alive.

The worker MUST NOT be considered failed merely because a single reconciliation pass encounters a recoverable PostgreSQL/MQTT error; durable work remains persisted and the worker continues its loop.

---

# 8. Startup and Restart Contract

Readiness transition:

```text
process starts
    |
    v
workers/dependencies not yet ready
    |
    v
all required dependencies ready
    |
    v
/readyz = 200
```

For ConfigurationSync specifically:

```text
worker task starts
    |
    v
worker_ready = true
    |
    v
reconcile durable PostgreSQL work
```

If the worker task exits:

```text
worker exits
    |
    v
worker_ready = false
    |
    v
/readyz = 503
```

Pending durable synchronization records MUST remain in PostgreSQL during restart. Restart readiness MUST NOT require that all device synchronization work has reached `applied`.

---

# 9. Safety Boundary With Domain 6

Domain 7 does not replace Domain 6 safety-data semantics.

The distinction is:

```text
Domain 6
required safety data unavailable
    -> deny safety-sensitive action
    -> explicit safety-data reason

Domain 7
backend dependency / synchronization subsystem unavailable
    -> /readyz = 503
```

`/readyz` MUST NOT claim that safety data is safe or available merely because the backend process is live.

Conversely, Domain 6 does not require every safety-data error to mutate global backend readiness.

---

# 10. Failure Semantics

| Condition | Required behavior |
|---|---|
| HTTP process alive | `/livez` returns 200 |
| PostgreSQL probe fails | `/readyz` returns 503; PostgreSQL dependency false |
| InfluxDB health fails | `/readyz` returns 503; InfluxDB dependency false |
| MQTT disconnected | `/readyz` returns 503; MQTT dependency false; durable sync preserved |
| command worker exits | `/readyz` returns 503 |
| ConfigurationSync worker exits | `/readyz` returns 503; durable sync preserved |
| ConfigurationSync has pending work | No readiness failure solely from pending work |
| ConfigurationSync target is failed | No global readiness failure solely from that target |
| ConfigurationSync pass gets recoverable DB/MQTT error | Worker remains alive; readiness reflects worker/transport state |
| all required dependencies healthy | `/readyz` returns 200 |
| `/readyz` dependency failure | No internal DB/MQTT error detail in response |

---

# 11. Required Verification

## 11.1 Liveness isolation

Prove `/livez` remains HTTP 200 when readiness dependencies are unavailable.

## 11.2 Readiness healthy state

Prove all required dependency flags true produces HTTP 200 and `status = ready`.

## 11.3 Readiness dependency failures

Prove each required dependency can make readiness false and produce HTTP 503 without leaking internal error detail.

## 11.4 ConfigurationSync worker lifecycle

Prove worker startup reports ready and worker guard/task exit clears readiness.

## 11.5 MQTT loss

Prove MQTT loss makes synchronization transport not ready and `/readyz` not ready while durable ConfigurationSync work remains recoverable.

## 11.6 Pending/failed sync work

Prove pending synchronization work does not by itself make `/readyz` fail.

Prove a terminal device-level synchronization failure does not make the entire backend unready solely because that device is failed.

## 11.7 Restart behavior

Prove worker restart returns readiness without requiring all durable device work to be applied.

## 11.8 Response contract

Prove dependency fields and synchronization health fields are machine-readable and stable.

## 11.9 Regression baseline

Run the authoritative backend verification contract against the isolated PostgreSQL database after implementation.

---

# 12. Acceptance Criteria

| ID | Acceptance criterion | Required result |
|---|---|---|
| `AC-01` | `/livez` is process-liveness only | `PASS` |
| `AC-02` | `/readyz` is unauthenticated and machine-readable | `PASS` |
| `AC-03` | Healthy required dependencies produce HTTP 200 | `PASS` |
| `AC-04` | Any required readiness dependency failure produces HTTP 503 | `PASS` |
| `AC-05` | PostgreSQL readiness uses a real probe | `PASS` |
| `AC-06` | InfluxDB readiness uses its health mechanism | `PASS` |
| `AC-07` | MQTT connection state participates in readiness | `PASS` |
| `AC-08` | Command reconciliation worker lifecycle participates in readiness | `PASS` |
| `AC-09` | ConfigurationSync worker lifecycle participates in readiness | `PASS` |
| `AC-10` | ConfigurationSync worker exit clears readiness | `PASS` |
| `AC-11` | Synchronization health distinguishes worker readiness from MQTT transport readiness | `PASS` |
| `AC-12` | Pending sync work does not itself make global readiness fail | `PASS` |
| `AC-13` | Device-level terminal sync failure does not itself make global readiness fail | `PASS` |
| `AC-14` | MQTT loss does not delete or invalidate durable sync work | `PASS` |
| `AC-15` | Restart does not require all devices to be `applied` before readiness | `PASS` |
| `AC-16` | Readiness responses do not leak internal DB/MQTT details | `PASS` |
| `AC-17` | Domain-specific current verification evidence is recorded | `PASS` |
| `AC-18` | Authoritative backend verification remains green | `PASS` |

The domain is **COMPLETE** only when `AC-01` through `AC-18` are supported by current evidence.

---

# 13. Verification Matrix

| ID | Verification | Evidence | Status |
|---|---|---|---|
| `READY-01` | `/livez` returns 200 independent of dependencies | Health API test | PASS |
| `READY-02` | Healthy `/readyz` returns 200 | Health API/integration test | PASS |
| `READY-03` | PostgreSQL failure returns 503 | Readiness logic + current isolated DB baseline | PASS |
| `READY-04` | InfluxDB failure returns 503 | Readiness dependency logic test | PASS |
| `READY-05` | MQTT disconnected returns 503 | Readiness dependency logic test | PASS |
| `READY-06` | Command worker false returns 503 | Readiness dependency logic test | PASS |
| `READY-07` | ConfigurationSync worker false returns 503 | Readiness dependency logic test | PASS |
| `READY-08` | Worker guard clears ConfigurationSync readiness | Worker unit test + existing implementation | PASS |
| `READY-09` | Synchronization health exposes worker/transport state | Health API logic test | PASS |
| `READY-10` | Pending sync work does not force global unready | Readiness contract test | PASS |
| `READY-11` | Failed device sync does not force global unready | Readiness contract test | PASS |
| `READY-12` | MQTT loss preserves durable sync work | Durable ConfigurationSync implementation + current backend regression | PASS |
| `READY-13` | Restart readiness does not require all sync applied | Worker lifecycle contract test + durable sync regression | PASS |
| `READY-14` | Response redacts internal dependency errors | Machine-readable response contract test | PASS |
| `READY-15` | Full backend verification | `.agent/verify.yml` | PASS |

Targeted tests prove only their targeted readiness behavior. They do not replace the full backend verification.

---

# 14. Evidence Contract

Completed verification MUST retain:

```text
requirement_id
verified_at
repository/worktree state
isolated PostgreSQL environment
/livez result
/readyz healthy result
each dependency-failure result
ConfigurationSync worker lifecycle result
synchronization health result
pending/failed sync readiness result
restart result
error-redaction result
full backend baseline result
build/clippy/fmt/diff-check result
```

Recommended evidence location:

```text
docs/evidence/READINESS-SYNC-001.json
```

Historical evidence MUST remain distinguishable from current Domain 7 verification.

---

# 15. Current Status

```text
READINESS-SYNC-001
STATUS: COMPLETE / VERIFIED
```

Current verification confirms `/livez` remains dependency-independent, `/readyz` gates on all required backend dependencies and worker lifecycles, and synchronization health distinguishes worker readiness from MQTT transport readiness. Pending or terminal device-level synchronization work does not by itself make the backend globally unready.

Current backend verification passed 437/437 tests, backend build, clippy with `-D warnings`, formatting, and `git diff --check` against the isolated PostgreSQL readiness database. Focused readiness tests passed 8/8 before formatting and the synchronization-health regression passed 1/1 after formatting. Evidence is recorded in `docs/evidence/READINESS-SYNC-001.json`.

---

# 16. Handoff

When complete:

```text
/livez
    = process alive

/readyz
    = all required backend dependencies and workers ready

synchronization health
    = worker lifecycle + MQTT transport state

durable sync work
    = recoverable independently of readiness state
```

The next roadmap domain is firmware fixture failure (P1.9). It remains outside this document.

---

# 17. Traceability

```text
READINESS-SYNC-001
        |
        +-- /livez
        |
        +-- /readyz
        |
        +-- PostgreSQL probe
        +-- InfluxDB health
        +-- MQTT connection state
        +-- event-bus state
        +-- command reconciliation worker
        +-- ConfigurationSync worker
        +-- synchronization health
        +-- startup/restart lifecycle
        +-- health tests
        +-- domain evidence
```

Domain invariants:

```text
/livez
    ==
Process can answer HTTP requests
```

and:

```text
/readyz = 200
    ==
All required backend readiness dependencies are healthy
```

and:

```text
Durable Synchronization Work
    !=
Global Backend Readiness
```
