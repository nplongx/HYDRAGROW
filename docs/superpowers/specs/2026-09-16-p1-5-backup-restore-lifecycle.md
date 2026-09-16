# P1.5 Backup / Restore Lifecycle

**Status:** SPECIFIED — NOT IMPLEMENTED
**Phase:** P1.5
**Depends on:** P1.0 Verification / Cross-System Baseline, P1.1 Authorization / Ownership Boundary, P1.2 Durable Command Lifecycle, P1.3 Error / Failure Semantics, P1.4 Operations State Boundary
**Scope:** configuration backup and restore lifecycle only

## 1. Purpose

Define one trustworthy lifecycle for exporting, validating, previewing, applying, and recovering configuration backups across HydraGrow. A backup is a versioned configuration artifact, not a live operational-state snapshot.

P1.5 MUST preserve the existing StationContext, React Query ownership, `useDeviceStore` PWM-preference boundary, and `useDeviceSync` transport/orchestration boundary.

P1.5 MUST NOT implement P1.6 Journal/Event Model, P1.7 API consolidation, P1.8 navigation redesign, P1.9 schema unification beyond the backup envelope, or P1.10 observability expansion.

## 2. Problem statement

Current backup/import UI and configuration APIs do not yet define a complete lifecycle for:

- what configuration is included;
- how artifact version and provenance are represented;
- how malformed, incompatible, partial, or stale artifacts are handled;
- how restore is authorized and scoped;
- how a multi-domain restore remains atomic;
- how a failed restore is prevented from leaving mixed configuration;
- how dry-run/preview differs from apply;
- how restore results are surfaced without pretending that live device state changed successfully.

P1.5 closes those semantic gaps without changing unrelated command, telemetry, or authorization architecture.

## 3. Definitions

### 3.1 Backup artifact

A self-contained, versioned JSON document containing configuration data and metadata sufficient for validation and restore.

A backup artifact MUST NOT contain:

- passwords, API keys, Firebase tokens, private keys, access tokens, or secrets;
- transient telemetry;
- controller health snapshots;
- current actuator state;
- durable command lifecycle records;
- journal/event history unless explicitly added by a later phase.

### 3.2 Scope

P1.5 supports explicit scope values:

- `station`: station-owned configuration and its selected devices;
- `device`: configuration for one owned device;
- `controller`: controller-specific persistent configuration associated with one device/controller identity.

`all` MAY be represented by a collection of explicit station/device/controller scopes, but MUST NOT mean “everything in the database”.

### 3.3 Restore

Restore means applying validated configuration from an artifact into an authorized target. Restore does not imply that the controller has accepted, persisted, or activated the resulting configuration remotely.

## 4. Canonical artifact envelope

The canonical artifact MUST contain:

```json
{
  "format": "hydragrow-backup",
  "schema_version": 1,
  "created_at": "RFC-3339 timestamp",
  "producer": {
    "application": "HydraGrow",
    "version": "application version"
  },
  "scope": "station | device | controller",
  "source": {
    "station_id": "optional id",
    "device_ids": ["..."],
    "controller_ids": ["..."]
  },
  "configuration": {},
  "integrity": {
    "algorithm": "SHA-256",
    "digest": "hex digest"
  }
}
```

Exact nested configuration types MUST reuse existing canonical shared/backend schemas. P1.5 MUST NOT create parallel copies of existing domain configuration models merely for backup.

Unknown top-level fields MUST be ignored only when forward-compatible semantics are explicitly supported. Unknown required fields or unsupported `schema_version` MUST fail validation.

## 5. Export contract

Export MUST:

1. authenticate the caller through the existing P1.1 boundary;
2. resolve target scope from explicit station/device/controller identity;
3. verify ownership/access for every exported resource;
4. read all required configuration inside one consistent database snapshot/transaction;
5. exclude secrets and runtime state;
6. construct the canonical envelope;
7. calculate integrity digest over the canonical payload excluding the digest field itself;
8. return/download the artifact only after successful construction.

An export MUST fail rather than silently omit an authorized configuration domain required by the selected scope.

Export MUST NOT mutate operational configuration.

## 6. Import validation lifecycle

Import MUST be staged. Uploading/parsing an artifact MUST NOT modify persistent configuration.

Validation order:

1. parse JSON;
2. validate envelope and `format`;
3. validate `schema_version`;
4. validate scope/source identifiers;
5. verify integrity digest;
6. validate every nested configuration domain against current schema and domain constraints;
7. resolve target resources and authorization;
8. detect immutable-identity conflicts;
9. produce a deterministic preview/diff.

Validation failure MUST identify the stage/category without leaking secrets or internal database details.

## 7. Restore preview

Before apply, API/UI MUST support a dry-run/preview result containing at minimum:

- target resource;
- domain;
- additions;
- changes;
- unchanged values;
- rejected values;
- immutable/conflicting identities;
- warnings;
- whether apply is permitted.

Preview MUST be side-effect free.

Preview MUST NOT report “restored” or “applied”.

## 8. Restore authorization

Restore authorization MUST reuse P1.1 ownership/scope enforcement.

The server MUST derive target ownership from authenticated identity and server-side resource lookup. Artifact metadata MUST NOT grant permission.

A user authorized to read a backup is not automatically authorized to restore it.

Cross-owner restore MUST be denied unless the existing authorization model explicitly permits that operation.

Authorization checks MUST occur before mutation and MUST cover every target resource in a multi-resource restore.

## 9. Atomic apply

A restore affecting multiple configuration domains MUST apply in one database transaction.

Required behavior:

- all validated domains commit together;
- any validation/application failure rolls back the whole restore;
- no partially restored domain may be committed as success;
- transaction commit is the point at which the server reports persistent restore success.

Remote controller synchronization is a separate post-commit phase. A database commit MUST NOT be rolled back merely because MQTT/controller synchronization fails.

If post-commit synchronization fails, result MUST explicitly distinguish:

- persistent configuration: applied;
- remote synchronization: pending/failed.

P1.5 MUST NOT invent a new durable command lifecycle; P1.2 remains authoritative for command lifecycle semantics.

## 10. Identity and conflict rules

Restore MUST distinguish configuration identity from target identity.

By default:

- station/device/controller IDs in an artifact identify source resources;
- target IDs are supplied explicitly by the authorized restore request;
- immutable identity fields cannot be overwritten by configuration payload;
- duplicate/conflicting resources require explicit mapping or fail closed.

Restore MUST NOT silently create ownership relationships, users, API credentials, or authorization grants.

## 11. Schema compatibility

`schema_version` is mandatory.

Supported compatibility classes:

- same version: validate and apply;
- older supported version: migrate in memory to current representation, then preview/apply;
- newer version: reject as unsupported;
- malformed/unknown version: reject.

Migration MUST be deterministic and side-effect free before transaction apply.

A failed migration MUST leave persistent configuration unchanged.

## 12. Secrets and sensitive data

Backups MUST be safe to download/store outside the server.

Never export:

- API keys;
- Firebase credentials/tokens;
- MQTT credentials/passwords;
- Wi-Fi passwords;
- private cryptographic material;
- session/cookie data.

If configuration contains a secret-backed field, the artifact MUST contain a non-secret marker or omit the secret according to that domain's existing contract. Restore MUST preserve the existing secret unless an explicit, separately authorized secret-management operation exists.

Logs MUST NOT print the artifact contents or integrity payload when secrets could be present.

## 13. Restore result semantics

The API MUST distinguish at least:

- `validated`: artifact valid, no mutation;
- `rejected`: validation/authorization failure, no mutation;
- `applied`: persistent configuration committed;
- `applied_sync_pending`: persistent configuration committed, controller synchronization not yet confirmed;
- `applied_sync_failed`: persistent configuration committed, synchronization attempt failed.

HTTP status codes MUST follow existing backend conventions. Do not encode successful persistence as HTTP failure solely because asynchronous controller synchronization failed.

## 14. Idempotency and repeated restore

Applying the same artifact to the same target with no intervening changes SHOULD produce no-op changes and MUST NOT corrupt configuration.

Repeated restore MUST NOT duplicate collection records that are logically unique.

Where an operation can be retried after ambiguous network failure, server-side behavior MUST avoid accidental double-application. P1.5 may use an idempotency key at the restore API boundary, but MUST NOT redesign P1.2 command lifecycle.

## 15. Failure and recovery

Failure classes MUST remain distinguishable:

- malformed artifact;
- unsupported schema;
- integrity mismatch;
- domain validation failure;
- authorization failure;
- target conflict;
- database transaction failure;
- post-commit controller/MQTT synchronization failure.

Database transaction failure MUST leave pre-restore persistent configuration intact.

A post-commit synchronization failure MUST NOT cause the UI to claim database rollback.

If recovery requires another restore, the system MUST expose enough result information to identify the affected target and configuration version without exposing secrets.

## 16. Backup history

P1.5 MAY persist backup metadata/history, but if implemented it MUST store metadata only unless encrypted artifact storage is explicitly specified.

Minimum metadata:

- backup ID;
- created timestamp;
- creator identity reference;
- scope;
- source/target resource IDs;
- schema version;
- integrity digest;
- result/status.

Backup history MUST respect P1.1 ownership filtering.

Deleting a backup metadata record MUST NOT delete active device configuration.

## 17. Frontend contract

The Config Backup page MUST use the canonical StationContext/device context.

Required states:

- scope selection;
- export loading/success/failure;
- import parsing;
- validation failure;
- preview/diff;
- authorization denied;
- apply confirmation;
- apply success;
- persistent success + controller sync pending/failed.

The UI MUST NOT infer successful restore from HTTP request completion alone.

React Query remains owner of server backup/restore data. `useDeviceStore` MUST NOT become backup lifecycle state storage.

`useDeviceSync` remains transport/orchestration only; backup lifecycle state belongs to the backup feature/query/mutation boundary.

## 18. API shape

Exact route names may follow existing project conventions, but the contract MUST provide separate operations for:

- export;
- validate/preview import;
- apply validated restore;
- optional backup history.

A single endpoint MUST NOT ambiguously combine upload, validation, mutation, and remote synchronization into one unobservable operation.

## 19. Audit boundary

P1.5 MUST expose enough structured result information for operational diagnosis.

Do not introduce a new journal/event model. If audit records are required before P1.6, use the existing event/log infrastructure without redefining its lifecycle.

## 20. Testing requirements

### Shared

- artifact envelope serialization roundtrip;
- schema-version handling;
- digest determinism;
- malformed artifact rejection;
- secret exclusion;
- backward-compatible migration tests where versions exist.

### Backend

- export scope/ownership;
- import validation without mutation;
- preview is side-effect free;
- unauthorized restore rejected;
- cross-owner restore rejected;
- multi-domain atomic rollback;
- successful commit;
- post-commit MQTT/controller failure classification;
- repeated restore/idempotency;
- integrity mismatch;
- unsupported schema;
- immutable identity conflict.

### Frontend

- preview states;
- rejected restore does not invalidate valid cached configuration;
- applied state refreshes through React Query;
- sync-pending/failure is distinct from persistence failure;
- context changes cannot apply a preview to a different target silently.

### Runtime

At minimum, verify against real PostgreSQL and MQTT/controller simulation:

1. export valid artifact;
2. validate/preview without mutation;
3. apply multi-domain restore atomically;
4. inject validation failure and prove zero partial writes;
5. restart backend during/around lifecycle and verify no false success;
6. make controller/MQTT unavailable after database commit and verify `applied_sync_pending`/failure semantics;
7. restore connectivity and verify synchronization through existing command/config transport without changing P1.2 lifecycle semantics.

## 21. Acceptance criteria

- **AC-1:** Backup artifact has explicit format/schema/integrity metadata.
- **AC-2:** Export contains only authorized configuration and no secrets/runtime state.
- **AC-3:** Export is consistent across all domains in selected scope.
- **AC-4:** Import validation is side-effect free.
- **AC-5:** Unsupported/malformed/tampered artifacts fail closed.
- **AC-6:** Restore requires authorization for every target resource.
- **AC-7:** Multi-domain restore is atomic.
- **AC-8:** Database persistence and remote controller synchronization have distinct result semantics.
- **AC-9:** Restore failure cannot leave a partially committed configuration transaction.
- **AC-10:** Immutable identities and cross-owner conflicts are never silently overwritten.
- **AC-11:** Repeated restore is safe and deterministic.
- **AC-12:** Frontend uses StationContext + React Query and does not move backup lifecycle ownership into `useDeviceStore`.
- **AC-13:** Runtime tests verify PostgreSQL transaction behavior and MQTT/controller synchronization failure separately.
- **AC-14:** P1.5 does not implement P1.6-P1.10 semantics or redesign P0.5/P1.1/P1.2 boundaries.

## 22. Non-goals

P1.5 does NOT include:

- full database disaster recovery;
- PostgreSQL physical backups/PITR;
- arbitrary filesystem/server snapshot restore;
- firmware image backup;
- telemetry/history export;
- event journal export;
- user/account migration;
- credential/secret export;
- P1.2 command lifecycle redesign;
- P1.6 event/journal redesign;
- P1.7 API-wide consolidation;
- P1.8 route/navigation redesign;
- P1.9 cross-language schema cleanup beyond the backup envelope;
- P1.10 observability redesign.

## 23. Completion rule

P1.5 is complete only when:

1. the artifact and lifecycle contracts above are implemented;
2. all applicable shared/backend/frontend tests pass;
3. atomic rollback is proven with an executable database test;
4. authorization and secret-exclusion behavior are executable tests;
5. post-commit synchronization failure is executable-tested separately from database failure;
6. runtime PostgreSQL + MQTT/controller verification is recorded;
7. evidence distinguishes current executable results from historical/static inspection;
8. every unavailable environment is explicitly marked `BLOCKED` rather than treated as `PASS`;
9. P1.5 documentation and traceability are updated;
10. no later P1 feature semantics are pulled into the implementation.

## 24. Evidence contract

Create `docs/evidence/P1.5-BACKUP-RESTORE-LIFECYCLE.json` only after implementation verification.

Evidence MUST record:

- timestamp/environment;
- artifact schema version tested;
- shared test result;
- backend test result;
- frontend test/build/typecheck result;
- authorization tests;
- atomic rollback test;
- runtime PostgreSQL result;
- runtime MQTT/controller synchronization result;
- explicit `PASS`, `FAIL`, `BLOCKED`, or `DIAGNOSED` status for each executable check;
- acceptance criteria AC-1 through AC-14;
- remaining gaps with concrete classification/follow-up.
