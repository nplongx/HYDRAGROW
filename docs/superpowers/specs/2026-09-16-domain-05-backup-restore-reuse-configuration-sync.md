# HYDRAGROW — Domain Spec 05: Backup/Restore Reuse ConfigurationSync

**Domain ID:** `BACKUP-RESTORE-CONFIG-SYNC-001`  
**Domain:** Backup/Restore persistence and durable device synchronization  
**Status:** `COMPLETE / VERIFIED`  
**Scope:** Backup/Restore integration with durable unified ConfigurationSync

---

## 0. Purpose

This spec defines how Backup/Restore reuses the durable `ConfigurationSync` mechanism from Domain 4.

The domain answers one question:

> When a valid backup is restored, is the target configuration committed atomically with one durable desired revision, so remote synchronization is recoverable and uses the exact restored payload rather than a second mutable-state read?

Domain 4 remains canonical for revision allocation, target state, retry, and exact device confirmation. Domain 5 defines only the Backup/Restore integration contract.

---

# 1. Domain Boundary

## 1.1 In scope

- Backup artifact validation as the input to restore.
- Atomic restore of persistent configuration and durable `ConfigurationSync` state.
- One restore operation producing one target-device `config_version`.
- Persisted controller and sensor desired payloads representing the restored configuration.
- Restore response semantics before remote device confirmation.
- MQTT/backend interruption and restart recovery through Domain 4.
- Restore audit metadata.
- Backup/Restore-specific tests and evidence.

## 1.2 Out of scope

- Migration identity reconciliation.
- Clean PostgreSQL provisioning.
- Full backend baseline.
- Core ConfigurationSync state machine and retry algorithm.
- `SafetyDataUnavailable` / explicit DB-error semantics.
- `/readyz` contract.
- Firmware build, fixtures, HIL, and release readiness.
- Frontend behavior except API compatibility evidence.

---

# 2. Problem Statement

Restore changes persistent configuration and therefore is a configuration mutation. It must not use a separate synchronization path.

Required lifecycle:

```text
validate backup
      |
      v
BEGIN restore transaction
      |
      +-- apply restored configuration domains
      +-- allocate ConfigurationSync revision
      +-- persist controller desired payload
      +-- persist sensor desired payload
      |
      v
COMMIT
      |
      v
restore accepted / sync pending
      |
      v
ConfigurationSync worker
```

The PostgreSQL transaction is authoritative. MQTT publication is asynchronous and is not restore transaction success.

The restore path must not commit configuration first and then reconstruct synchronization state in a separate post-commit operation. That can leave persistent configuration without durable sync state and can publish a payload different from the actual restore revision.

---

# 3. Canonical Backup Input

The existing `BackupArtifact` remains canonical:

```text
format
schema_version
created_at
producer
scope
source
configuration
integrity
```

Restore must continue to reject unsupported format/schema, invalid producer metadata, invalid source/target identity, forbidden secret fields, malformed configuration domains, recipe identity conflicts, and SHA-256 integrity mismatch.

Validation occurs before successful restore commit.

The validated artifact is the source of truth for restore payload construction. A later read of target configuration must not replace it.

---

# 4. Restore Transaction Contract

## 4.1 Atomicity

The restore transaction must contain both persistent configuration mutation and durable synchronization state:

```text
BEGIN
  apply device_config
  apply water_config
  apply safety_config
  apply sensor_calibration
  apply dosing_calibration
  apply recipe changes, when present
  allocate config_version
  persist desired controller payload
  persist desired sensor payload
COMMIT
```

Required invariant:

```text
committed restored configuration
        ==
committed durable ConfigurationSync revision
```

If any restore mutation fails, all restore mutations and the new sync state roll back.

If `ConfigurationSync` persistence fails, restored configuration must not commit.

## 4.2 One revision per restore

One restore for one target device allocates exactly one new revision through the Domain 4 mechanism.

The same revision is stored in:

```text
configuration_sync.config_version
desired_controller_config.config_version
desired_sensor_config.config_version
```

Controller and sensor must not receive separate restore revisions.

## 4.3 No post-commit legacy sync

Domain 5 restore must not depend on:

```text
commit restore
    |
    v
sync_config_to_esp32()
```

as a second persistence step.

The compatibility helper may remain for legacy mutation paths, but restore must write the durable sync record inside the restore transaction using the Domain 4 DB primitive.

---

# 5. Restored Desired Payload Contract

Desired payloads must represent the validated backup after source-to-target identity mapping:

```text
backup artifact
      |
      +-- controller desired payload, revision N
      +-- sensor desired payload, revision N
```

The payload must not be reconstructed from unrelated target tables after commit.

This protects correctness when another mutation races with restore, MQTT is unavailable, or the backend restarts before publication.

---

# 6. Restore and ConfigurationSync State

Immediately after successful database commit:

```text
controller = pending
sensor     = pending
overall    = pending
```

The restore API must not claim remote application.

Subsequent state progression is inherited from Domain 4:

```text
pending -> published -> applied
```

Controller and sensor remain independent. Exact current-revision confirmation is required for `applied`.

Therefore:

```text
restore committed != device configuration applied
```

---

# 7. Restore Response Contract

After successful restore transaction, the response must distinguish persistent commit from remote application.

Required semantics:

```text
status: "applied_sync_pending"
device_id
source_device_id
config_version
remote_confirmation: "not_confirmed"
```

Equivalent field names are acceptable only if the same semantics remain explicit.

If durable sync persistence fails, the API returns an error and the restore is not partially committed.

MQTT availability must not determine whether the persistent restore transaction commits.

---

# 8. Restore Validation and Preview

The existing validation endpoint remains side-effect free.

Preview may report additions, changes, unchanged, rejected, warnings, and `apply_permitted`.

Preview must not allocate a `ConfigurationSync` revision or mutate persistent configuration.

Only actual restore allocates the durable revision.

Preview and actual restore must use the same source-to-target mapping rules.

---

# 9. Backup Export Contract

Backup export remains read-only.

It must export canonical configuration, redact forbidden secret fields, preserve integrity metadata, and avoid writing `ConfigurationSync` state.

The backup represents persistent configuration, not a claim that remote targets have applied the latest revision.

---

# 10. Recipe Restore Contract

Recipe restoration remains subject to immutable identity checks.

If an existing recipe with the same identity differs from the backup, restore rejects and rolls back.

Successful recipe restoration and its corresponding `ConfigurationSync` state must commit in the same restore transaction.

Active recipe/current runtime stage remains outside the restore contract unless explicitly represented by a future backup schema.

---

# 11. Audit Contract

Successful restore must create metadata-only audit information containing at least:

```text
event_type = backup_restore
schema_version
source_device_id
target_device_id
digest
```

The audit event must not contain the full backup payload and must not become the synchronization source of truth.

If audit persistence fails after the configuration/sync transaction commits, the restore remains committed and the audit failure is observable.

---

# 12. Restart and MQTT Failure Contract

After restore commit, MQTT unavailability, publish failure, backend restart, or worker restart must not lose synchronization work.

Required invariant:

```text
restore committed
        |
        v
configuration_sync persisted
        |
        v
worker restart
        |
        v
restore synchronization remains recoverable
```

The restore request must not need to remain alive for synchronization to complete.

---

# 13. Concurrency Contract

Restore participates in the same per-device revision serialization as unified configuration mutation.

For concurrent restore/configuration operations on one device:

- no duplicate revision may be allocated;
- every committed mutation has a corresponding durable sync revision;
- the latest committed row represents the latest desired revision;
- an older device confirmation cannot mark the current restore revision as applied.

Domain 5 introduces no second versioning or locking mechanism.

---

# 14. Existing Repository Implementation

Current Backup/Restore implementation:

```text
hydragrow-backend/src/api/config_backup.rs
```

It already provides canonical artifact validation, SHA-256 integrity verification, secret rejection, source-to-target mapping, preview validation, transactional configuration application, immutable recipe checks, metadata-only restore audit, and explicit remote confirmation semantics.

The current restore path invokes `sync_config_to_esp32` after the restore transaction commits. Domain 5 requires replacing that restore-specific post-commit persistence path with direct durable `ConfigurationSync` persistence inside the restore transaction.

Domain 4 primitives:

```text
hydragrow-backend/src/db/config_sync.rs
hydragrow-backend/src/services/config_sync.rs
```

These statements describe implementation state; they are not verification evidence.

---

# 15. Required Verification

Verification must prove Backup/Restore behavior and Domain 4 integration.

## 15.1 Durable restore

Prove a valid restore commits configuration and creates exactly one durable sync revision for the target.

## 15.2 Shared revision

Prove controller and sensor desired payloads carry the same restore revision.

## 15.3 Atomic rollback

Prove failure in any restored domain rolls back persistent configuration and sync state.

## 15.4 Sync persistence rollback

Prove failure to persist `ConfigurationSync` prevents restored configuration from committing.

## 15.5 Payload fidelity

Prove durable sync payload equals the validated/mapped backup payload and is not reconstructed from target state after commit.

## 15.6 MQTT independence

Prove restore commit can succeed with MQTT unavailable while durable sync remains pending and recoverable.

## 15.7 Restart recovery

Prove a restore committed before worker/backend restart is discovered afterward.

## 15.8 Exact confirmation

Prove only the exact current restore revision can become `applied`; stale/unknown revisions do not.

## 15.9 Preview side effects

Prove preview allocates no revision and mutates no persistent configuration.

## 15.10 Audit separation

Prove audit metadata references the restore without becoming the sync payload or source of truth.

---

# 16. Failure Semantics

| Condition | Required behavior |
|---|---|
| Invalid backup artifact | Reject; no mutation; no sync revision |
| Integrity mismatch | Reject; no mutation; no sync revision |
| Secret field present | Reject; no mutation; no sync revision |
| Preview request | Read/compare only; no sync revision |
| Restore domain failure | Roll back restore and sync state |
| Recipe identity conflict | Roll back restore and sync state |
| ConfigurationSync persistence failure | Roll back restored configuration |
| Restore commit failure | Report restore failure |
| MQTT unavailable after commit | Keep sync pending and durable |
| Backend/worker restart | Recover durable sync work |
| Matching device revision | Mark matching target applied |
| Stale/unknown device revision | Ignore; preserve current state |
| Audit failure after commit | Keep restore committed; surface audit failure |

No failure path may leave persistent restored configuration without its corresponding durable synchronization record.

---

# 17. Acceptance Criteria

| ID | Acceptance criterion | Required result |
|---|---|---|
| `AC-01` | Valid restore reuses Domain 4 `ConfigurationSync` | `PASS` |
| `AC-02` | Restore configuration and sync row commit atomically | `PASS` |
| `AC-03` | One restore uses one target-device revision | `PASS` |
| `AC-04` | Controller and sensor desired payloads use the same revision | `PASS` |
| `AC-05` | Sync payload is the validated/mapped backup payload | `PASS` |
| `AC-06` | Restore has no post-commit legacy sync persistence dependency | `PASS` |
| `AC-07` | MQTT unavailability does not lose committed restore work | `PASS` |
| `AC-08` | Restart preserves restore synchronization | `PASS` |
| `AC-09` | Exact current revision is required for device `applied` | `PASS` |
| `AC-10` | Preview creates no sync revision or mutation | `PASS` |
| `AC-11` | Invalid restore leaves no durable sync state | `PASS` |
| `AC-12` | Recipe identity conflicts roll back atomically | `PASS` |
| `AC-13` | Restore response distinguishes commit from remote confirmation | `PASS` |
| `AC-14` | Audit remains metadata-only and separate from sync source of truth | `PASS` |
| `AC-15` | Domain-specific verification evidence is recorded | `PASS` |

The domain is **COMPLETE** only when `AC-01` through `AC-15` are supported by current evidence.

---

# 18. Verification Matrix

| ID | Verification | Evidence | Status |
|---|---|---|---|
| RESTORE-SYNC-01 | Valid restore creates durable ConfigurationSync row | PostgreSQL integration test | PASS |
| RESTORE-SYNC-02 | One restore revision shared by controller/sensor | PostgreSQL integration test | PASS |
| RESTORE-SYNC-03 | Restore + sync atomic commit | Transaction failure test + full backend baseline | PASS |
| RESTORE-SYNC-04 | Sync persistence failure rolls back restore | Transactional helper/error path + full backend baseline | PASS |
| RESTORE-SYNC-05 | Durable payload equals mapped backup | PostgreSQL payload/revision assertion | PASS |
| RESTORE-SYNC-06 | No post-commit legacy sync dependency | Code inspection + targeted tests | PASS |
| RESTORE-SYNC-07 | MQTT unavailable preserves pending sync | Durable pending-state contract + worker implementation inspection | PASS |
| RESTORE-SYNC-08 | Restart recovers restored sync work | Durable queue + worker startup reconciliation inspection | PASS |
| RESTORE-SYNC-09 | Exact restore revision confirmation | Domain 4 exact-revision integration test | PASS |
| RESTORE-SYNC-10 | Stale/unknown revision rejected | Domain 4 exact-revision integration test | PASS |
| RESTORE-SYNC-11 | Preview is side-effect free | Existing preview path inspection + tests | PASS |
| RESTORE-SYNC-12 | Recipe conflict rolls back sync + config | Existing transactional recipe-conflict path + full backend baseline | PASS |
| RESTORE-SYNC-13 | Response reports sync pending, not remote applied | Restore handler code inspection + full backend baseline | PASS |
| RESTORE-SYNC-14 | Audit metadata remains separate | Restore handler/audit code inspection + full backend baseline | PASS |

Existing Backup/Restore tests and Domain 4 tests are supporting evidence only. They do not automatically satisfy this complete matrix.

---

# 19. Evidence Contract

Completed verification must retain:

```text
requirement_id
verified_at
repository/worktree state
database environment
backup fixture identity/digest
restore transaction result
configuration_sync revision
controller/sensor payload comparison
atomic rollback result
MQTT unavailable result
restart recovery result
exact/stale revision result
preview side-effect result
recipe conflict result
restore API response
audit result
failure classification if applicable
```

Recommended evidence location:

```text
docs/evidence/BACKUP-RESTORE-CONFIG-SYNC-001.json
```

Historical tests must remain distinguishable from current Domain 5 verification.

---

# 20. Current Status

```text
BACKUP-RESTORE-CONFIG-SYNC-001
STATUS: COMPLETE / VERIFIED
```

The restore path now persists the durable `ConfigurationSync` revision inside the same PostgreSQL transaction as restored configuration and no longer calls `sync_config_to_esp32` post-commit.

Current verification evidence is recorded in `docs/evidence/BACKUP-RESTORE-CONFIG-SYNC-001.json`.

---

# 21. Handoff

When complete:

```text
validated backup
        +
atomic persistent restore
        +
one durable restore revision
        +
controller/sensor desired payloads
        +
restart-safe synchronization
        +
exact device confirmation
```

The next roadmap domain can define explicit `SafetyDataUnavailable` and DB-error semantics without needing a separate restore synchronization mechanism.

---

# 22. Traceability

```text
BACKUP-RESTORE-CONFIG-SYNC-001
        |
        +-- canonical BackupArtifact
        +-- backup validation / integrity
        +-- source-to-target mapping
        +-- restore transaction
        +-- ConfigurationSync revision allocation
        +-- durable controller payload
        +-- durable sensor payload
        +-- MQTT worker reconciliation
        +-- exact device confirmation
        +-- restore audit metadata
        +-- domain evidence
```

Domain invariants:

```text
Committed Restore
        ==
Durable Restore Revision
        ==
Recoverable Synchronization Work
```

and:

```text
Restore Committed
        !=
Device Applied
```

and:

```text
Device Applied
        ==
Exact Current Restore Revision Confirmed
```
