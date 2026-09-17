# HYDRAGROW — Domain Spec 06: SafetyDataUnavailable / Explicit DB-Error Semantics

**Domain ID:** `SAFETY-DATA-UNAVAILABLE-001`  
**Domain:** Safety-critical database input availability and explicit DB-error semantics  
**Status:** `COMPLETE / VERIFIED`  
**Scope:** Backend safety decision paths that depend on PostgreSQL-backed safety data

---

## 0. Purpose

This spec defines the contract for PostgreSQL-backed safety inputs when the data is missing, unreadable, stale by schema/contract, or the database is unavailable.

The domain answers one question:

> Can the backend distinguish “no safety configuration exists” from “the database failed”, and can it guarantee that missing or unreadable safety data never becomes an implicit safe default for an action that could actuate hardware?

The central rule is fail-closed for safety decisions, while remaining explicit and machine-readable about the failure cause.

Domain 4/5 durable ConfigurationSync and Domain 3 backend verification remain prerequisites but are not redefined here.

---

# 1. Domain Boundary

## 1.1 In scope

- PostgreSQL `safety_config` availability for safety-critical decisions.
- PostgreSQL `dosing_calibration` availability where required to calculate a safe dose.
- PostgreSQL dosing history and last-dose timestamp availability where required by the safety gate.
- Distinction between missing safety data and DB query/connection failure.
- Fail-closed behavior for manual and automated dosing decisions.
- Explicit API/error semantics for DB failures.
- Stable machine-readable error codes/reasons.
- Preservation of internal DB details in logs while preventing leakage through API responses.
- Safety decision metrics and execution-log classification.
- Tests proving DB-error paths cannot silently become defaults.

## 1.2 Out of scope

- ConfigurationSync revision allocation or MQTT retry.
- Backup/Restore transaction semantics.
- PostgreSQL provisioning or migration identity.
- `/readyz` overall synchronization health, except where this domain defines a safety dependency that a later readiness domain may consume.
- InfluxDB telemetry availability semantics except where explicitly compared with PostgreSQL safety inputs.
- Firmware safety implementation.
- Frontend UX beyond API error-contract compatibility.
- HIL / physical E2E.
- Release readiness.

---

# 2. Safety Data Model

Safety-critical decisions MUST classify each required input as one of:

```text
AVAILABLE
MISSING
DB_ERROR
INVALID
```

These states are not interchangeable.

For PostgreSQL-backed safety data:

```text
row exists + query succeeds + values valid = AVAILABLE
query succeeds + required row absent       = MISSING
query fails / connection unavailable       = DB_ERROR
row exists + required values invalid        = INVALID
```

`MISSING`, `DB_ERROR`, and `INVALID` MUST NOT be converted into a usable safety configuration by applying `Default` values.

---

# 3. Fail-Closed Safety Contract

Any action whose safety decision requires unavailable safety data MUST be denied.

Required invariant:

```text
Safety data unavailable
        ==
No safety authorization
        ==
No safety-sensitive actuation
```

This applies to at least:

- manual dosing commands;
- script-generated `dose` commands;
- cron-triggered `dose` commands;
- any future actuator path that explicitly declares dependency on `safety_config` or dosing calibration.

The backend MUST NOT infer:

```text
DB error -> default SafetyConfig
DB error -> zero history
DB error -> no previous dose
DB error -> allow
```

For non-safety informational reads, normal API-specific missing/error semantics may remain separate.

---

# 4. Required PostgreSQL Inputs

## 4.1 Safety configuration

The following values are safety inputs and require successful retrieval:

```text
max_dose_per_cycle
max_dose_per_hour
cooldown_sec
```

Other `SafetyConfig` fields may be consumed by future safety checks, but if a decision declares them required, the same availability contract applies.

If the `safety_config` row is absent, the safety decision is `MISSING` and MUST be denied.

If the query fails, the safety decision is `DB_ERROR` and MUST be denied.

## 4.2 Dosing calibration

For dose actions whose duration or delivered quantity depends on pump calibration, the required `dosing_calibration` row MUST exist.

Missing calibration is not equivalent to a zero-capacity calibration and MUST NOT result in an estimated dose of zero.

Invalid or non-positive pump capacity MUST produce `INVALID` and deny the dose.

## 4.3 Dosing history

When the safety algorithm requires hourly history or last-dose timing, successful empty query results are valid data:

```text
history query succeeds + zero rows = AVAILABLE, empty history
last-dose query succeeds + no row = AVAILABLE, no previous dose
```

But query failure is `DB_ERROR`, not an empty result.

Therefore:

```text
DB error != empty history
DB error != no previous dose
```

---

# 5. Error Classification Contract

Safety decision code MUST preserve the difference between data state and transport/storage failure.

Recommended canonical reason codes:

```text
SAFETY_DATA_MISSING
SAFETY_DATA_DB_ERROR
SAFETY_DATA_INVALID
DOSING_CALIBRATION_MISSING
DOSING_CALIBRATION_DB_ERROR
DOSING_CALIBRATION_INVALID
DOSING_HISTORY_DB_ERROR
LAST_DOSE_DB_ERROR
```

Equivalent names are acceptable only if they remain stable, machine-readable, and distinguish the same failure classes.

Internal SQLx error text MUST be logged with sufficient context for diagnosis but MUST NOT be returned directly to an external API caller.

---

# 6. API Error Semantics

The canonical JSON error envelope from `api/error.rs` remains authoritative for REST normalization.

Safety-critical API failures MUST provide a stable machine code inside that envelope.

For a DB failure affecting a safety decision, the semantic response is:

```text
HTTP 503 Service Unavailable
code = safety_data_unavailable
```

For required safety data that is absent but the database itself is healthy:

```text
HTTP 503 Service Unavailable
code = safety_data_unavailable
details.reason = SAFETY_DATA_MISSING
```

For invalid persisted safety data:

```text
HTTP 503 Service Unavailable
code = safety_data_unavailable
details.reason = SAFETY_DATA_INVALID
```

The exact public HTTP status may be represented by an equivalent dependency/data-unavailable status only if the machine code and fail-closed behavior remain unchanged.

Server responses MUST NOT expose:

- SQL statements;
- PostgreSQL connection strings;
- credentials;
- internal hostnames;
- raw SQLx error chains.

Request correlation IDs remain available through the existing error middleware.

---

# 7. Manual Control Contract

`validate_manual_dose_safety()` currently reads dosing calibration and safety limits before permitting a dosing command.

Domain 6 requires:

1. Missing calibration denies the dose explicitly.
2. Calibration DB error denies the dose explicitly.
3. Missing safety configuration denies the dose explicitly.
4. Safety configuration DB error denies the dose explicitly.
5. A DB error MUST NOT be returned as a successful or partial-successful actuation result.
6. No MQTT publish may occur when required safety data is unavailable.

The API may expose a stable safety-data-unavailable response while retaining detailed server logs.

---

# 8. Automated Script / Cron Contract

Automated action paths have the same safety authority as manual commands.

For sensor-triggered scripts:

```text
script fires
    |
    v
safety data load
    |
    +-- AVAILABLE -> evaluate safety -> possibly publish
    +-- MISSING   -> deny
    +-- DB_ERROR  -> deny
    +-- INVALID   -> deny
```

For cron-triggered scripts, the current implementation already classifies several input failures through `SAFETY_DECISIONS_TOTAL` and execution logs. Domain 6 requires PostgreSQL safety-data failures to use explicit safety reason codes rather than being collapsed into a generic `Mqtt` error.

The following are specifically prohibited:

```text
get_safety_config() fails
        |
        v
SafetyConfig::default()
        |
        v
allow dose
```

and:

```text
history query fails
        |
        v
empty history
        |
        v
allow dose
```

---

# 9. Existing Risk Points

Current repository inspection identifies these semantics requiring hardening:

### 9.1 Safety defaults in manual control

`load_max_dose_per_cycle()` currently substitutes `SafetyConfig::default()` when no row exists.

This is acceptable only for a non-safety informational default. It is **not** acceptable for a safety authorization decision.

### 9.2 Sensor-triggered action path

`handle()` in `mqtt/handlers/sensors.rs` currently uses:

```text
get_safety_config -> Ok(safety context)
                   -> Err => None
```

and later skips dispatch when the context is absent.

That is directionally fail-closed, but the failure class is not retained as an explicit safety-data error contract.

### 9.3 Sensor-triggered history defaults

The sensor path currently uses `unwrap_or_default()` for dosing history and last-dose lookup. Domain 6 requires these failures to be distinguishable from successful empty results when those values are required by the safety decision.

### 9.4 Cron safety path

`cron_scheduler.rs` currently wraps safety-data errors in `ActionDispatchError::Mqtt` and later records `SAFETY_INPUT_ERROR`.

Domain 6 requires a dedicated safety-data error classification rather than using MQTT as the semantic category for a PostgreSQL safety failure.

These are implementation observations, not verification evidence.

---

# 10. Error Propagation Contract

Safety data loaders SHOULD expose typed errors rather than string-only errors.

Recommended shape:

```text
SafetyDataError
  Missing
  Database(sqlx::Error)
  Invalid(String)
```

Higher layers MUST preserve the classification.

For example:

```text
DB loader
    |
    v
SafetyDataError::Database
    |
    +-- REST -> safety_data_unavailable / 503
    +-- automation -> deny + SAFETY_DATA_DB_ERROR
    +-- metric -> denied
    +-- log -> internal diagnostic detail
```

The exact Rust type may differ, but collapsing all cases into `String` or `anyhow::Error` without preserving the semantic category is insufficient for this domain.

---

# 11. No-Fallback Rules

The following fallbacks are prohibited on safety-critical decision paths:

| Failed input | Prohibited fallback |
|---|---|
| `safety_config` missing | `SafetyConfig::default()` |
| `safety_config` DB error | `SafetyConfig::default()` |
| `dosing_calibration` missing | zero/nominal capacity |
| `dosing_calibration` DB error | default calibration |
| dosing history DB error | empty history |
| last-dose DB error | `None` previous dose |
| invalid safety value | clamp silently into a permissive value |
| safety query timeout | allow action because MQTT is healthy |

Defaults remain permitted only where the caller explicitly declares the result informational and non-actuating.

---

# 12. Metrics and Observability

Safety data failures MUST be observable without leaking sensitive DB details.

`SAFETY_DECISIONS_TOTAL` SHOULD classify at least:

```text
SAFETY_DATA_MISSING / denied
SAFETY_DATA_DB_ERROR / denied
SAFETY_DATA_INVALID / denied
DOSING_CALIBRATION_MISSING / denied
DOSING_CALIBRATION_DB_ERROR / denied
DOSING_CALIBRATION_INVALID / denied
DOSING_HISTORY_DB_ERROR / denied
LAST_DOSE_DB_ERROR / denied
```

Existing `SAFETY_DECISIONS_TOTAL` labels MUST NOT be changed in a way that destroys established historical interpretation unless the migration is explicitly documented.

Logs SHOULD include:

```text
device_id
decision = denied
reason_code
dependency = postgres
```

Raw SQL error details stay server-side.

Execution logs for automated flows SHOULD retain the stable reason code so operators can distinguish a safety-data outage from a rejected dose limit.

---

# 13. Transaction and Actuation Ordering

For every safety-sensitive action:

```text
load required safety data
        |
        v
classify availability
        |
        v
evaluate safety
        |
        v
persist durable command intent, where applicable
        |
        v
publish actuator command
```

No actuator publish may occur before required safety data has been successfully loaded and validated.

If safety-data retrieval fails after command intent persistence, the command MUST still not transition to a published actuator state. Existing durable-command semantics must remain consistent with this rule.

---

# 14. Informational Read Semantics

Domain 6 does not require every configuration GET to return 503 when a row is absent.

The distinction is:

```text
informational read
    -> endpoint-specific missing/error semantics

safety authorization input
    -> fail closed + explicit safety-data classification
```

For example, `GET /config/safety` may retain a normal `404` for a genuinely missing configuration row if that is its documented resource semantics.

The same missing row used to authorize a dose MUST be `SAFETY_DATA_MISSING` and denied.

---

# 15. Required Verification

## 15.1 Missing safety row

Remove or isolate the `safety_config` row for a test device and prove a safety-sensitive action is denied.

## 15.2 Safety DB failure

Use an isolated PostgreSQL test setup that can force the relevant query to fail and prove the action is denied with `SAFETY_DATA_DB_ERROR` rather than defaulting.

## 15.3 Missing calibration

Prove a dose requiring calibration is denied when the calibration row is absent.

## 15.4 Calibration DB failure

Prove calibration query failure is classified separately from missing calibration.

## 15.5 History DB failure

Prove a failed history query is not treated as an empty history.

## 15.6 Last-dose DB failure

Prove a failed last-dose query is not treated as `None`.

## 15.7 No MQTT on unavailable safety data

Prove the safety-data failure occurs before actuator publish.

## 15.8 Manual API semantics

Prove manual dosing returns an explicit safety-data-unavailable error and does not claim success/partial success.

## 15.9 Automated semantics

Prove sensor/cron action paths deny the action and record the stable safety reason.

## 15.10 Error redaction

Prove API responses contain no SQLx error text, credentials, connection strings, or internal DB details.

## 15.11 Empty successful history

Prove successful zero-row history remains a valid empty input and is not confused with DB failure.

## 15.12 Regression baseline

Run the canonical backend verification contract after implementation changes.

---

# 16. Failure Semantics

| Condition | Required behavior |
|---|---|
| Safety row exists and valid | Safety evaluation may proceed |
| Safety row missing | Deny; `SAFETY_DATA_MISSING` |
| Safety query DB error | Deny; `SAFETY_DATA_DB_ERROR` |
| Safety values invalid | Deny; `SAFETY_DATA_INVALID` |
| Calibration exists and valid | Duration/dose calculation may proceed |
| Calibration missing | Deny; `DOSING_CALIBRATION_MISSING` |
| Calibration DB error | Deny; `DOSING_CALIBRATION_DB_ERROR` |
| Calibration invalid | Deny; `DOSING_CALIBRATION_INVALID` |
| History query succeeds with zero rows | Continue with empty history |
| History query fails | Deny; `DOSING_HISTORY_DB_ERROR` |
| Last-dose query succeeds with no row | Continue with no previous dose |
| Last-dose query fails | Deny; `LAST_DOSE_DB_ERROR` |
| Safety-data failure before publish | No MQTT actuator publish |
| REST safety-data failure | `503` + stable machine code |
| Internal DB error detail | Log only; never expose externally |

---

# 17. Acceptance Criteria

| ID | Acceptance criterion | Required result |
|---|---|---|
| `AC-01` | Safety-critical DB inputs have explicit availability states | `PASS` |
| `AC-02` | Missing `safety_config` fails closed | `PASS` |
| `AC-03` | Safety DB failure fails closed | `PASS` |
| `AC-04` | Missing calibration fails closed | `PASS` |
| `AC-05` | Calibration DB failure fails closed | `PASS` |
| `AC-06` | History DB failure is not converted to empty history | `PASS` |
| `AC-07` | Last-dose DB failure is not converted to no previous dose | `PASS` |
| `AC-08` | Invalid safety data fails closed | `PASS` |
| `AC-09` | No actuator MQTT publish occurs when required safety data is unavailable | `PASS` |
| `AC-10` | Manual API exposes stable safety-data-unavailable semantics | `PASS` |
| `AC-11` | Automated paths preserve explicit safety failure classification | `PASS` |
| `AC-12` | API does not expose raw DB error details | `PASS` |
| `AC-13` | Successful empty history remains distinguishable from DB failure | `PASS` |
| `AC-14` | Metrics/logs expose stable safety failure reason | `PASS` |
| `AC-15` | Canonical backend verification remains green after implementation | `PASS` |
| `AC-16` | Domain evidence records every safety failure class tested | `PASS` |

The domain is **COMPLETE** only when `AC-01` through `AC-16` are supported by current evidence.

---

# 18. Verification Matrix

| ID | Verification | Evidence | Status |
|---|---|---|---|
| `SAFETY-DATA-01` | Missing safety row denies action | Backend integration test | PASS |
| `SAFETY-DATA-02` | Safety DB error denies action | Backend integration/test fixture | PASS |
| `SAFETY-DATA-03` | Invalid safety values deny action | Backend test | PASS |
| `SAFETY-DATA-04` | Missing dosing calibration denies dose | Backend test | PASS |
| `SAFETY-DATA-05` | Calibration DB error denies dose | Backend integration/test fixture | PASS |
| `SAFETY-DATA-06` | History DB error is not empty history | Backend test | PASS |
| `SAFETY-DATA-07` | Last-dose DB error is not `None` | Backend test | PASS |
| `SAFETY-DATA-08` | No MQTT publish on safety-data failure | Mock/fixture test | PASS |
| `SAFETY-DATA-09` | Manual REST response has stable code | API test | PASS |
| `SAFETY-DATA-10` | Sensor automation records explicit reason | Handler test | PASS |
| `SAFETY-DATA-11` | Cron automation records explicit reason | Scheduler test | PASS |
| `SAFETY-DATA-12` | Raw DB details are redacted | API/error middleware test | PASS |
| `SAFETY-DATA-13` | Empty successful history remains valid | Backend test | PASS |
| `SAFETY-DATA-14` | Safety metrics classify DB/missing failures | Metrics test | PASS |
| `SAFETY-DATA-15` | Full backend verification after implementation | `.agent/verify.yml` | PASS |

Current targeted tests plus the full 432-test backend run provide the verification baseline. Unit classification tests exercise missing/DB-error/invalid safety and calibration states, successful empty-history/no-previous-dose states, stable reason codes, ActionDispatch safety classification, and API error redaction. Full backend verification confirms these changes compile and coexist with the current backend integration suite.

---

# 19. Evidence Contract

Completed verification MUST retain:

```text
requirement_id
verified_at
repository/worktree state
PostgreSQL verification environment
safety row present/missing result
safety DB-error result
safety invalid-data result
calibration missing/DB-error/invalid results
history DB-error result
last-dose DB-error result
MQTT publish suppression result
manual API response/code
sensor automation reason
cron automation reason
metrics classification
error redaction result
full backend baseline result
failure classification if applicable
```

Recommended evidence location:

```text
docs/evidence/SAFETY-DATA-UNAVAILABLE-001.json
```

Historical backend evidence MUST remain distinguishable from current Domain 6 verification.

---

# 20. Current Status

```text
SAFETY-DATA-UNAVAILABLE-001
STATUS: COMPLETE / VERIFIED
```

Current verification confirms that safety-data failures are represented by an explicit typed semantic contract across manual, webhook, sensor-triggered, and cron-triggered paths. Required safety inputs fail closed on missing, DB-error, and invalid states; successful empty history/no-previous-dose remain valid data states.

Current backend verification passed 432/432 tests, clippy with `-D warnings`, formatting, and `git diff --check` against the isolated PostgreSQL readiness database. Evidence is recorded in `docs/evidence/SAFETY-DATA-UNAVAILABLE-001.json`.

---

# 21. Handoff

When complete:

```text
Required Safety Data
        |
        v
Explicit Availability Classification
        |
        +-- AVAILABLE -> Safety Evaluation
        +-- MISSING   -> Deny
        +-- DB_ERROR  -> Deny
        +-- INVALID   -> Deny
```

The next roadmap domain can then define `/readyz` and synchronization health without ambiguity about whether a backend safety dependency failure is a missing resource, database outage, or unsafe data state.

---

# 22. Traceability

```text
SAFETY-DATA-UNAVAILABLE-001
        |
        +-- safety_config
        +-- dosing_calibration
        +-- dosing history
        +-- last-dose timestamp
        +-- typed safety-data classification
        +-- fail-closed authorization
        +-- manual control
        +-- sensor automation
        +-- cron automation
        +-- stable API error code
        +-- metrics / execution logs
        +-- error redaction
        +-- backend verification evidence
```

Domain invariant:

```text
Safety Authorization
        ==
Required Safety Data AVAILABLE and VALID
```

and:

```text
DB Error / Missing / Invalid Safety Data
        !=
Safe Default
```

