# P0.4 Configuration Consistency

## Status

Implemented in worktree.

## Purpose

Make PostgreSQL configuration authoritative and failure-honest across the unified config read path, unified write path, MQTT publication path, and single-domain config endpoints.

## Core invariants

1. `fetch_optional` distinguishes row absence from database/query failure.
2. A database/query error is never converted into a plausible configuration value.
3. `device_config` is required for a usable unified configuration. Missing `device_config` returns unavailable/not-found rather than synthesizing targets.
4. Existing defaults for optional component rows are used only when the row is genuinely absent; they are never used to hide query errors.
5. The canonical `device_id` comes from the route/ownership context, not from client payload identity.
6. Unified multi-table configuration writes are committed as one PostgreSQL transaction.
7. Validation that can be performed before persistence is performed before the transaction commits any configuration changes.
8. Successful PostgreSQL persistence and MQTT publication are separate outcomes.
9. MQTT publication success means publication/transport succeeded; it is not physical application confirmation.
10. If persistence succeeds but publication fails, the API reports `partial_success` rather than `success`.
11. A failed configuration read prevents publication of an invented configuration.
12. Configuration timestamps are server-controlled for writes.

## Configuration domains

The unified contract currently combines:

- `DeviceConfig`
- `WaterConfig`
- `SafetyConfig`
- `SensorCalibration`
- `DosingCalibration`

Existing schema-backed defaults remain available for genuinely absent optional rows. P0.4 does not introduce a new configuration schema or hardware capability.

## Read semantics

### Unified read

`GET /config/unified`:

- `device_config` row exists -> return it.
- `device_config` row absent -> `404` with `reason = device_config_missing`.
- any query error -> `500` without synthesized configuration.
- optional row absent -> existing domain default may be materialized because this was the established repository contract.
- optional row query error -> `500`, never default.

### Controller sync read

`sync_config_to_esp32` uses the same failure-honest unified read. Sensor-config publication additionally propagates its query error instead of silently treating the row as absent.

## Write semantics

`PUT /config/unified`:

1. Validate write scope and device ownership.
2. Normalize every embedded `device_id` to the canonical route `device_id`.
3. Assign the server timestamp.
4. Validate dosing constraints before persistence.
5. Persist all five domains in one PostgreSQL transaction.
6. Commit only when all writes succeed.
7. Record the existing audit event after successful persistence.
8. Publish the resulting authoritative configuration to MQTT.
9. Return `200 success` only when both persistence and publication succeed.
10. Return `202 partial_success` when persistence succeeds but MQTT publication fails.

Rollback on any pre-commit write failure prevents a partially persisted unified configuration.

Single-domain update endpoints retain their existing persistence semantics but now normalize route identity and distinguish successful persistence from failed MQTT publication using `partial_success`.

## Timestamp semantics

`last_updated` / `last_calibrated` values written by server endpoints are server-controlled. Client-provided device identity is not authoritative. P0.4 does not use MQTT publication time as evidence of physical application.

## Non-goals

- configuration schema redesign
- new hardware configuration fields
- MQTT acknowledgement/physical-application protocol
- automatic retry queue
- frontend-wide state architecture rewrite
- replacement of existing PostgreSQL/InfluxDB storage

## Verification

Required evidence:

- backend compilation succeeds
- backend unit suite executes successfully
- `git diff --check` succeeds
- source audit confirms no `.ok().flatten()` remains on unified configuration query paths where it would hide DB errors
- frontend runtime verification is reported separately when Node/npm tooling is unavailable


## P0 residual verification gate

P0.4 configuration semantics remain closed: a query failure is an error, genuine absence may use the established optional-row default, and successful persistence is distinct from MQTT publication.

Before P1.0, configuration verification must establish that the implementation and its test environment can exercise these semantics without masking database failures. In particular:

- database-backed test failures are investigated at the environment/schema/fixture/implementation boundary;
- tests must not be weakened by converting ownership or configuration failures into success;
- `.ok()`, `unwrap_or_default()`, `unwrap_or(None)`, or equivalent fallback changes are not acceptable merely to make the baseline green;
- backend formatting and compilation are part of the P0 residual gate;
- unavailable frontend tooling is recorded as an environment limitation, not reported as a frontend pass.

P1 work may address broader authorization, durable lifecycle, backup/restore, and error-semantics gaps identified by the whole-system readiness audit. Those are P1 implementation scope, not reasons to reopen P0.4 configuration authority.
