# HYDRAGROW — Domain Spec 04: Durable Unified ConfigurationSync

**Domain ID:** `CONFIG-SYNC-DURABLE-001`  
**Domain:** Durable unified configuration persistence and device synchronization  
**Status:** `SPECIFIED / IMPLEMENTATION EXISTS / VERIFICATION PENDING`  
**Scope:** Unified configuration-to-device synchronization only

---

## 0. Purpose

This spec defines the durable synchronization contract for unified HYDRAGROW device configuration.

The domain answers one question:

> When backend configuration changes, is the desired configuration durably recorded with one monotonic revision, independently reconciled to controller and sensor targets, and recoverable after MQTT/backend interruption without treating MQTT publish as final application success?

The domain covers the already-present `ConfigurationSync` implementation and defines the verification required before calling it complete.

It does **not** redefine backup/restore, runtime DB-error semantics, `/readyz`, firmware fixture compatibility, HIL, or release readiness.

---

# 1. Domain Boundary

## 1.1 In scope

- Durable `configuration_sync` PostgreSQL state.
- Unified configuration revision allocation.
- Per-device monotonic revision semantics.
- Atomic persistence of desired configuration and sync state.
- Independent controller/sensor delivery state.
- MQTT publish/retry/reconciliation worker.
- Restart recovery from durable pending state.
- Exact configuration revision confirmation.
- Stale/unknown revision rejection.
- Publish versus applied semantics.
- Sync status observability required by this domain.
- Domain-specific tests and verification evidence.

## 1.2 Out of scope

- Migration identity reconciliation.
- Clean PostgreSQL provisioning itself.
- Full backend baseline itself.
- Backup/Restore reuse of this mechanism.
- `SafetyDataUnavailable` or explicit DB-error semantics.
- `/readyz` contract beyond the worker state needed by this domain.
- Firmware build/toolchain verification.
- Canonical firmware fixture consumers.
- Frontend behavior except where an existing backend status API is used as evidence of this domain.
- HIL / physical E2E.
- Release readiness.

---

# 2. Problem Statement

Configuration synchronization must not depend on an in-memory queue or on successful MQTT publication alone.

The required lifecycle is:

```text
backend configuration mutation
        |
        v
durable desired state + revision
        |
        v
publish to controller / sensor
        |
        v
published state
        |
        v
device reports exact revision
        |
        v
applied state
```

MQTT delivery can be interrupted after database commit, and devices can restart after receiving a retained message. Therefore the PostgreSQL record is the source of truth for pending reconciliation, while device confirmation is required before `applied`.

---

# 3. Canonical Data Model

The durable table is:

```text
configuration_sync
```

It stores:

```text
device_id
config_version
desired_controller_config
desired_sensor_config
controller_state
sensor_state
controller_attempts
sensor_attempts
controller_last_attempt_at
sensor_last_attempt_at
controller_applied_at
sensor_applied_at
last_error
updated_at
```

The allowed target states are exactly:

```text
pending
published
applied
failed
```

The device row is keyed by `device_id`; the table references `device_config` and is deleted with its device configuration.

The pending index must cover rows where either target remains `pending` or `published`.

---

# 4. Configuration Revision Contract

## 4.1 Per-device monotonicity

Each device has a monotonically increasing `config_version`.

Version allocation must serialize concurrent writers for the same device.

The current implementation uses a PostgreSQL transaction advisory lock keyed by `device_id`, followed by:

```sql
SELECT COALESCE(MAX(config_version), 0) + 1
FROM configuration_sync
WHERE device_id = $1
```

The verification must prove that concurrent configuration writers cannot allocate the same revision for one device.

Versions for different devices need not share a global sequence.

## 4.2 Revision scope

One unified configuration mutation produces one revision used by both targets:

```text
device
  config_version = N
       |
       +-- controller desired config, revision N
       +-- sensor desired config, revision N
```

Controller and sensor must not receive independently allocated revisions for the same unified mutation.

## 4.3 Revision persistence

The revision and desired target payloads must be committed durably before the mutation is reported as accepted.

An MQTT publish failure after commit must leave durable state available for retry.

---

# 5. Atomic Unified Configuration Mutation

The canonical mutation path is:

```text
validate request
    |
    v
begin PostgreSQL transaction
    |
    +-- update device config
    +-- update safety config
    +-- update water config
    +-- update sensor calibration
    +-- update dosing calibration
    +-- allocate config_version
    +-- persist desired controller payload
    +-- persist desired sensor payload
    |
    v
commit
    |
    v
HTTP 202 / pending
```

No target may be reported as applied during this transaction.

The transaction must not commit a partial configuration mutation while failing to persist its corresponding synchronization record.

Conversely, a failed transaction must not leave a durable sync row claiming that the failed configuration is the current desired state.

---

# 6. Delivery State Machine

Each target has its own state:

```text
pending
   |
   | successful MQTT publish
   v
published
   |
   | exact device revision confirmation
   v
applied
```

Publish failure follows:

```text
pending
   |
   | failed attempt
   +----> pending   (attempts < 5)
   |
   +----> failed    (5th failed attempt)
```

`failed` is terminal for that retry cycle. A future explicit configuration mutation creates a new revision.

`published` is not equivalent to `applied`.

The overall state used by the existing status API is:

```text
controller=applied AND sensor=applied  -> applied
either target=failed                   -> failed
either target=pending                  -> pending
otherwise                              -> published
```

---

# 7. MQTT Delivery Contract

Configuration publication uses:

```text
QoS: AtLeastOnce
retain: true
```

Target topics are the canonical controller and sensor configuration topics for the device.

The worker must not treat successful MQTT publication as device application confirmation.

The worker must:

1. read durable pending/published work from PostgreSQL;
2. publish only targets that are still `pending`;
3. record successful publication as `published`;
4. increment attempt metadata;
5. record failures and `last_error`;
6. stop retrying a target after five failed attempts in that cycle;
7. continue processing the other target independently.

An unavailable MQTT connection must not delete or lose durable desired state.

---

# 8. Restart and Recovery Contract

The synchronization worker must reconstruct pending work from PostgreSQL after backend restart.

Required invariant:

```text
backend restart
    |
    v
durable configuration_sync remains
    |
    v
worker starts
    |
    v
pending/published targets become eligible for reconciliation
```

No in-memory-only queue is authoritative.

Worker readiness state may be reset during shutdown and set when the worker starts. This domain verifies persistence/recovery; the full `/readyz` contract belongs to the later readiness domain.

---

# 9. Device Confirmation Contract

## 9.1 Exact current revision

A device confirmation may mark a target `applied` only when its reported `config_version` exactly matches the currently durable row for that device.

The update must be conditional on:

```text
device_id = target device
AND config_version = current durable revision
```

## 9.2 Stale revision

A stale revision must not modify the current sync row.

Example:

```text
current revision = 7
device reports   = 6
```

Required result:

```text
no applied-state mutation
configuration-version-mismatch metric increment
```

## 9.3 Unknown future revision

An unknown future revision must also be rejected.

Example:

```text
current revision = 7
device reports   = 8
```

Required result:

```text
no applied-state mutation
configuration-version-mismatch metric increment
```

## 9.4 Target isolation

Controller confirmation changes only controller state.

Sensor confirmation changes only sensor state.

One target reaching `applied` must not implicitly mark the other target applied.

---

# 10. Desired Payload Contract

The durable row must contain the desired payload independently for:

```text
controller
sensor
```

Both payloads must carry the same `config_version` for one unified mutation.

The worker must publish the payload persisted in the durable row, not reconstruct a potentially newer configuration from unrelated mutable tables during a retry.

This prevents a retry from accidentally publishing configuration belonging to a later revision.

---

# 11. Existing Repository Implementation

The implementation already present in the repository includes:

```text
hydragrow-backend/migrations/20260916090000_configuration_sync.sql
hydragrow-backend/src/db/config_sync.rs
hydragrow-backend/src/services/config_sync.rs
```

The unified configuration API is in:

```text
hydragrow-backend/src/api/config.rs
```

The backend starts the worker from:

```text
hydragrow-backend/src/main.rs
```

Device confirmation is integrated in:

```text
hydragrow-backend/src/mqtt/handlers/status.rs
```

The current implementation records durable desired state, allocates revisions under a device-scoped advisory lock, publishes retained QoS 1 MQTT messages, tracks per-target attempts/state, and accepts only exact current-revision device confirmation.

These statements describe repository implementation state; they are not by themselves domain verification evidence.

---

# 12. Required Verification

Verification must cover behavior, not only compilation.

## 12.1 Persistence

Prove that a unified mutation creates one durable sync row containing both desired payloads and one revision.

## 12.2 Monotonic revision

Prove sequential mutations allocate increasing revisions.

Prove concurrent mutations for one device do not allocate duplicate revisions.

## 12.3 Atomicity

Prove a transaction failure does not leave a committed configuration-sync record for the failed mutation.

Prove a successful configuration mutation cannot commit without its durable sync record.

## 12.4 Target independence

Prove controller and sensor states can progress independently.

Example:

```text
controller = applied
sensor     = pending
```

must remain a valid intermediate state.

## 12.5 Publish/retry

Prove successful publication transitions `pending -> published`.

Prove failed publication increments attempts and retains durable work until the fifth failure.

Prove the fifth failure transitions the target to `failed`.

## 12.6 Exact confirmation

Prove matching revision transitions only the matching target to `applied`.

Prove stale and unknown revisions are ignored.

## 12.7 Restart recovery

Prove a durable pending row remains discoverable by a newly started worker after the original worker/process is gone.

## 12.8 Payload stability

Prove retry uses the persisted desired payload for its revision and does not silently replace it with a later configuration.

---

# 13. Failure Semantics

| Condition | Required behavior |
|---|---|
| PostgreSQL unavailable during mutation | Do not report durable acceptance |
| Configuration transaction rollback | No durable sync row for rolled-back mutation |
| Version allocation conflict | Mutation fails; no duplicate revision |
| MQTT unavailable | Keep durable target pending; do not lose desired state |
| MQTT publish failure before retry limit | Increment attempt; remain pending |
| Fifth publish failure | Mark target failed |
| Matching device revision | Mark only matching target applied |
| Stale device revision | Ignore; increment mismatch metric |
| Unknown future revision | Ignore; increment mismatch metric |
| One target applied, other pending | Overall state remains pending |
| One target failed | Overall state becomes failed |
| Both targets applied | Overall state becomes applied |
| Backend restart with pending rows | Worker reconstructs work from PostgreSQL |

No failure path may silently delete the durable desired configuration.

---

# 14. Acceptance Criteria

| ID | Acceptance criterion | Required result |
|---|---|---|
| `AC-01` | `configuration_sync` schema exists in current migration chain | `PASS` |
| `AC-02` | Unified mutation persists desired controller and sensor payloads | `PASS` |
| `AC-03` | One unified mutation uses one `config_version` for both targets | `PASS` |
| `AC-04` | Per-device versions are monotonic and concurrency-safe | `PASS` |
| `AC-05` | Config DB mutation and durable sync record commit atomically | `PASS` |
| `AC-06` | MQTT publish uses QoS 1 and retained delivery | `PASS` |
| `AC-07` | Publish success is represented as `published`, not `applied` | `PASS` |
| `AC-08` | Publish failures retry with bounded five-attempt policy | `PASS` |
| `AC-09` | Controller and sensor delivery state is independent | `PASS` |
| `AC-10` | Exact current device revision is required for `applied` | `PASS` |
| `AC-11` | Stale/unknown revisions cannot mutate current sync state | `PASS` |
| `AC-12` | Durable pending work survives worker/backend restart | `PASS` |
| `AC-13` | Retry publishes the durable payload belonging to its revision | `PASS` |
| `AC-14` | Sync status exposes per-target state and overall state | `PASS` |
| `AC-15` | Domain-specific verification evidence is recorded | `PASS` |

The domain is **COMPLETE** only when `AC-01` through `AC-15` are supported by current evidence.

---

# 15. Verification Matrix

| ID | Verification | Evidence | Status |
|---|---|---|---|
| SYNC-01 | Current migration creates `configuration_sync` | Current clean/test DB | PENDING |
| SYNC-02 | Durable revision roundtrip | PostgreSQL test | PENDING |
| SYNC-03 | Monotonic per-device revision | PostgreSQL/concurrency test | PENDING |
| SYNC-04 | Atomic unified mutation | Transaction test | PENDING |
| SYNC-05 | Per-target state independence | PostgreSQL test | PENDING |
| SYNC-06 | Publish success transition | Worker test | PENDING |
| SYNC-07 | Bounded retry | Worker/DB test | PENDING |
| SYNC-08 | Exact revision confirmation | PostgreSQL/MQTT handler test | PENDING |
| SYNC-09 | Stale/unknown revision rejection | Handler test | PENDING |
| SYNC-10 | Restart recovery | Worker integration test | PENDING |
| SYNC-11 | Durable payload stability | Revision/retry test | PENDING |
| SYNC-12 | Status aggregation | API test | PENDING |

Existing targeted ConfigurationSync tests are supporting evidence only. They do not automatically satisfy this complete domain matrix.

---

# 16. Evidence Contract

Completed verification must retain:

```text
requirement_id
verified_at
repository/worktree state
database environment
migration version used
revision tests
transaction atomicity result
publish/retry result
confirmation result
stale/unknown revision result
restart recovery result
payload stability result
status API result
failure classification if applicable
```

Recommended evidence location:

```text
docs/evidence/CONFIG-SYNC-DURABLE-001.json
```

Historical targeted tests must remain distinguishable from current domain verification.

---

# 17. Current Status

```text
CONFIG-SYNC-DURABLE-001
STATUS: SPECIFIED / IMPLEMENTATION EXISTS / VERIFICATION PENDING
```

The repository already contains a substantial implementation of this domain.

That implementation must not be called fully verified until the acceptance criteria and verification matrix above are supported by current tool evidence.

The Domain 3 backend baseline passing `425/425` proves the backend suite is green, but does not by itself prove every durable ConfigurationSync acceptance criterion.

---

# 18. Handoff

When complete, this domain provides:

```text
durable desired configuration
        +
monotonic per-device revision
        +
independent controller/sensor reconciliation
        +
bounded MQTT retry
        +
exact device confirmation
        +
restart recovery
```

The next roadmap domain may reuse this durable synchronization primitive for Backup/Restore.

Backup/Restore-specific semantics remain outside this document and must be verified separately.

---

# 19. Traceability

```text
CONFIG-SYNC-DURABLE-001
        |
        +-- configuration_sync migration
        |
        +-- durable DB access
        |
        +-- unified configuration transaction
        |
        +-- per-device revision allocation
        |
        +-- controller target
        |
        +-- sensor target
        |
        +-- MQTT publish/retry worker
        |
        +-- exact device revision confirmation
        |
        +-- restart recovery
        |
        +-- sync status API
        |
        +-- domain evidence
```

Domain invariant:

```text
Accepted Unified Configuration
        ==
Durable Desired Revision
        ==
Recoverable Synchronization Work
```

and:

```text
Device Applied
        ==
Exact Current Revision Confirmed
```

