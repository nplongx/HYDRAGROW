# P1.9 Shared Schema Alignment

**Status:** Specification
**Date:** 2026-09-16
**Repository:** `/home/long/HYDRAGROW`
**Depends on:** P1.6 Journal Event Model, P1.7 API Query/Mutation Consolidation, P1.8 Route Navigation Contract

## 1. Problem

HYDRAGROW currently has several independently maintained representations of the same concepts:

- `hydragrow-shared` Rust types define important MQTT/domain payloads and have strong Rust serde tests.
- Backend models mix persistence (`sqlx::FromRow`) and serialization concerns.
- Backend API request/response DTOs duplicate or reshape shared types.
- Frontend `src/types/models.ts` manually recreates many backend/shared schemas.
- Sensor-node Rust/C++ firmware manually constructs JSON and does not consume `hydragrow-shared`.
- MQTT topic tests cover topic construction/parsing, but not a complete cross-language topic+payload matrix.
- No OpenAPI/JSON Schema/`utoipa`/Swagger/code-generation pipeline exists today.

Concrete drift includes:

- `SensorData` emits canonical `ec`, while accepting legacy `tds`/`err_tds`; Influx rows use `f64` and convert to shared `f32`.
- Frontend telemetry types omit shared `controller_received_ms`/per-axis receipt fields and omit `runtime_ready`/`actuator_contradictory` from authoritative telemetry.
- `CropSeason.start_time` and `status` are free-form strings; recipe persistence and wire representations differ (`duration_days`/`current_stage_id` versus `duration_sec`/`current_stage_index`).
- `PumpCommandReq`/MQTT command input accepts nested `params` plus duplicated top-level fields and a `pump` alias.
- Journal persistence, backend API, and frontend event types differ in timestamp and field shape.
- Backend config GET is sectioned while frontend normalizes to a flatter model.
- Backend config/calibration models use `serde_json::Value` where extension semantics are not always explicit.
- Frontend `TelemetryAxis.name` and `AppSettings` permit effectively arbitrary values.
- API error handling can diverge from P1.7's canonical `{error:{code,message,details,request_id}}` envelope.
- WiFi secret actions, FSM, health, watchdog, and recipe status have representation differences.

P1.9 establishes explicit ownership and canonical wire contracts. It does **not** force database models, UI view models, firmware internals, and API contracts into one struct.

## 2. Goals

1. Establish one authoritative definition for every cross-system wire contract.
2. Make frontend/backend consumers derive from or validate against those contracts instead of independently guessing shapes.
3. Separate wire/API DTOs, shared domain types, persistence models, and UI view models.
4. Define exact serialization semantics: names, requiredness, nullability, defaults, unknown fields, enums, timestamps, IDs, numbers, and units.
5. Align HTTP API, MQTT/controller, journal, telemetry, commands, configuration, recipe, health, and error contracts.
6. Preserve compatibility explicitly rather than through accidental serde defaults/aliases.
7. Add cross-language fixture and round-trip tests that detect Rust/backend/frontend/firmware drift.
8. Make schema changes reviewable and CI-verifiable.

## 3. Non-goals

- Replacing all internal Rust structs with generated types.
- Exposing SQL rows directly as API responses.
- Rewriting the database merely to make names identical.
- Rewriting firmware architecture solely to share Rust source.
- Redesigning P1.6 journal semantics, P1.7 API ownership/error semantics, or P1.8 route/context semantics.
- Assuming a complete OpenAPI product or code-generation stack.

## 4. Ownership model

Use four explicit layers:

1. **Canonical wire contract** — exact JSON/MQTT representation exchanged across process/language boundaries.
2. **Shared domain contract** — semantic Rust types used by backend/controller/shared logic where useful.
3. **Persistence model** — SQL/Influx representation and adapters.
4. **UI/view model** — presentation state, route state, derived fields, and form state.

Rule: **one canonical wire schema per cross-system contract; multiple internal representations are allowed.**

`hydragrow-shared` is the strongest existing ownership point, but not every shared type is an HTTP DTO. API envelopes, query parameters, pagination, authorization context, and persistence adapters remain separate where semantics differ.

The audit found no existing OpenAPI/JSON Schema/`utoipa`/Swagger/codegen pipeline. Implementation must therefore not assume one. The first implementation increment must introduce one reviewable canonical contract artifact, mechanically checked against the chosen source of truth. If a generator is introduced, it must replace manual duplication rather than create another independently editable source.

## 5. Canonical wire rules

### 5.1 JSON names

- `snake_case` is canonical unless an external protocol explicitly requires another spelling.
- Canonical serializers emit only canonical names.
- Legacy aliases are input-only during a documented migration window.
- `tds`, `err_tds`, `pump`, and equivalent old spellings are compatibility aliases, not parallel fields.

### 5.2 Requiredness/nullability

Every field is classified as required non-null, required nullable, optional/omittable, server-defaulted input, or response-only.

`null` and omission are not interchangeable unless explicitly specified. For PATCH semantics, omission means unchanged; `null` means clear only for fields declared nullable/clearable.

Rust `Option<T>` and TypeScript `?:` are not sufficient evidence of the wire contract.

### 5.3 Unknown fields and extensions

Readers may ignore unknown fields for forward-compatible, non-security-sensitive payloads where safe. Writers emit only canonical fields. Commands, authentication/signature envelopes, and security-sensitive discriminators must reject unsafe unknown values.

`[key: string]: unknown` in `AppSettings` is not a general escape hatch. Remove it from canonical transport types or replace it with an explicitly named, bounded extension object.

### 5.4 Enums

Closed enums use exact canonical strings. `KnownValue | string` is not an enum contract.

- `CommandLifecycle`: closed, `SCREAMING_SNAKE_CASE`.
- `TelemetryQuality`/`TelemetryAvailability`: closed uppercase values.
- `TelemetrySource`: closed `snake_case` values.
- Crop season status/stage and FSM state: use typed enums for known supported values; extensibility must be explicit.
- `WifiSecretAction`: one canonical wire casing; old casing input aliases only during migration.

### 5.5 IDs

Transport IDs are opaque strings unless a stronger existing format is required. Define resource kind, maximum/allowed format where validation exists, case sensitivity, stability, and client-generation rules.

P1.8 route/device IDs use the same resource identity semantics. URL/query/body IDs are selectors, never authorization grants. Authorization remains server-side and within `StationContext`/capability boundaries.

### 5.6 Timestamps

Canonical cross-system timestamps use RFC 3339 UTC strings unless a protocol explicitly requires epoch integers.

Distinguish:

- `observed_at`: source observation time;
- `received_at`: backend/application receipt time;
- `classified_at`: derived-state classification time;
- lifecycle/event times.

Existing fields such as `timestamp_ms`, `requested_at_ms`, and `updated_at_ms` remain only where protocol/storage semantics require them and must state `epoch milliseconds` explicitly. Never substitute receipt time for a missing observation time.

### 5.7 Numbers

Each numeric field must specify precision guarantee, range, unit, and whether non-finite values are forbidden. JSON does not encode `f32` versus `f64`.

Resolve the current `Influx f64 -> shared f32 -> frontend number` boundary. Preserve source precision at the wire/backend boundary unless a documented business reason requires reduction. Any reduction must be tested. `NaN`, `Infinity`, and `-Infinity` are invalid wire values.

### 5.8 Units

At minimum document:

- EC: `mS/cm`;
- pH: dimensionless pH;
- temperature: `°C`;
- water level: `cm`;
- volume: `mL`;
- flow: `mL/s`;
- duration: seconds for `*_sec` fields;
- PWM: explicit integer range;
- percentage: `%`;
- epoch fields: `ms` only when explicitly named.

Unit suffixes in field names do not replace contract-level unit metadata.

## 6. Canonical contract inventory

### 6.1 Device identity/scope

Define one device/station identity contract reused by HTTP DTOs, MQTT payloads, route state, journal, telemetry, commands, and configuration. Scope metadata does not prove authorization.

### 6.2 Telemetry

Align `SensorData` and `AuthoritativeTelemetrySnapshot` with all semantically authoritative shared fields:

- canonical `ec` rather than emitted `tds`;
- axis value, quality, unit, `observed_at`, `received_at`, source, and error code;
- controller health, actuator state, FSM state;
- `runtime_ready` and `actuator_contradictory`;
- `operational_state`;
- controller/per-axis receipt timing where part of the sensor contract.

Frontend must not silently omit canonical telemetry fields. `TelemetryAxis.name` must be closed for current axes (`ph`, `ec`, `temp`, `water_level`) unless a bounded extension mechanism is documented.

### 6.3 Actuator/pump state

`PumpStatus` is the canonical observed actuator shape. Optional PWM/pulse fields retain explicit omitted/null semantics. Command intent is not observed actuator state.

### 6.4 Commands/lifecycle

Canonical command input/output has one primary structure: `action`, optional `target`, optional typed `params`, and command metadata where applicable.

The duplicated top-level `pump_id`, `duration_sec`, and `pwm` fields in `PumpCommandReq`/`MqttCommandIn` are legacy input compatibility only. Canonical output uses one location. `pump` is a legacy alias for `pump_id`.

P1.2/P1.7 lifecycle semantics remain authoritative. `CommandLifecycle` values and epoch-millisecond fields must match the shared contract.

### 6.5 Configuration/safety/calibration

Separate persistence rows, API DTOs, shared semantic contracts, and frontend form models.

Backend sections (`DeviceConfig`, `SensorCalibration`, `PumpCalibration`, `DosingCalibration`, `SafetyConfig`) map explicitly to canonical API sections or an explicitly documented flattened contract. Frontend normalization is an adapter, not an alternate undocumented schema.

Every numeric config/calibration field gets unit/range/precision metadata. `serde_json::Value` is allowed only for explicitly extensible, bounded objects with documented semantics.

### 6.6 Crop seasons/recipes

`CropSeason` uses typed timestamp/status contracts and canonical IDs. Persistence and wire recipe models remain separate. Conversion from `duration_days`/`current_stage_id` to `duration_sec`/`current_stage_index` must be lossless; otherwise preserve required precision/identity through schema or persistence changes.

`DeviceRecipeStatus` must have one documented response nesting; frontend callers must not infer whether `active_recipe` is flat or nested.

### 6.7 Journal/events

P1.6 event semantics remain authoritative. Align backend/frontend event DTOs on actor, category, severity, occurrence, resolution, device, metadata, and timestamps. Ensure all canonical emitted log levels are represented.

Journal filtering, pagination, and export use the same event semantics; export-specific rows are adapters.

### 6.8 Pagination/filtering

Every paginated API declares cursor/offset strategy, size bounds/default, stable sort/tie-breaker, next-cursor representation, and filter types. Cursors are opaque to frontend callers. Existing P1.6 compound cursor semantics remain authoritative.

### 6.9 API errors

P1.7's envelope is canonical:

```json
{
  "error": {
    "code": "...",
    "message": "...",
    "details": {},
    "request_id": "..."
  }
}
```

The P1.7 status-to-code mapping remains canonical. Frontend `ApiError` normalizes this shape. Legacy string/object error bodies may be handled only by a compatibility adapter.

### 6.10 Auth/capability/scope

Reuse canonical resource IDs across API modules, route state, `StationContext`, journal, commands, and device APIs. Client-provided IDs never grant access.

### 6.11 WiFi/secrets

Separate public WiFi status from secret-bearing mutations. Canonicalize `WifiSecretAction` casing. Secrets must never enter debug logs, telemetry snapshots, journal exports, error details, or generic extension objects.

### 6.12 Health/watchdog/FSM

Classify health/watchdog types as shared canonical contracts or local implementation types. Remove duplicate `TopicStatus` definitions where semantics are identical. Use explicit adapters when internal FSM enum encoding differs from wire encoding.

## 7. Schema artifact/source of truth

Introduce a canonical, reviewable contract artifact under the repository schema/contract area. It must capture at least field names/types, requiredness/nullability, enums, timestamp formats/units, numeric constraints/units, extensibility, and compatibility metadata.

Given the current repository evidence, the minimum viable mechanism is:

1. `hydragrow-shared` remains semantic Rust ownership where appropriate.
2. Canonical wire fixtures/schema metadata become the cross-language reference.
3. Rust serde tests prove emitted JSON against fixtures.
4. TypeScript tests consume the same fixtures.
5. Backend endpoint tests consume the same fixtures.
6. Firmware tests consume representative fixtures where practical.

Do not introduce a second editable schema that can drift from this artifact.

## 8. Compatibility/versioning

Version only contracts requiring independent evolution. When compatibility cannot be safely inferred, version is explicit in payload/protocol metadata.

Default migration policy: readers accept N-1 where practical; writers emit N. N-1 removal requires evidence that known producers are migrated.

For every legacy alias (`tds`, `err_tds`, `pump`, duplicated command fields, old WiFi casing, old enum casing): document owner, reason, accepted input, canonical output, tests, and removal condition. Never emit aliases canonically.

A change is breaking if it changes requiredness, meaning, units, precision guarantees, enum interpretation, timestamp semantics, ID semantics, nesting, or canonical field names in a way that breaks an existing producer/consumer. Breaking changes require versioning or staged migration.

## 9. Migration sequence

1. Inventory each cross-system contract, owner, producers/consumers, adapters, and aliases.
2. Create canonical metadata/fixtures for errors, telemetry, commands, journal, config, and recipes.
3. Align shared Rust serde output; retain compatibility only in input adapters.
4. Separate backend DTOs from DB/Influx models and add explicit conversions.
5. Align frontend transport types with canonical/generated/validated contracts; keep UI-only models separate.
6. Align Rust/C++ firmware JSON producers/consumers.
7. Align MQTT topic + payload as one tested contract.
8. Migrate aliases: accept N-1, emit N, instrument remaining legacy inputs, then remove only with evidence.
9. Enforce drift checks in CI.

Keep schema migration separate from unrelated business logic changes.

## 10. Test strategy

### 10.1 Fixture corpus

For each major wire contract provide minimum-valid, full, optional/null, enum, unknown-field, malformed, N-1 compatibility, numeric-boundary, and redaction-sensitive fixtures as applicable.

### 10.2 Rust

Extend existing shared schema tests to prove exact names/casing, alias input-only behavior, omission/null semantics, enum encoding, timestamps, numeric validity, compatibility, and fixture round-trips.

### 10.3 TypeScript

Add runtime validation at untrusted transport boundaries where appropriate plus compile-time assertions. Tests consume canonical fixtures and detect missing fields, type/enum/timestamp/number drift, telemetry omissions, and error-envelope divergence.

Do not runtime-validate purely internal UI objects that never cross a trust/process boundary.

### 10.4 Backend

Endpoint tests assert canonical request/response/error bodies. Persistence adapter tests remain separate from transport serialization tests.

### 10.5 Firmware/controller

Add fixture-based producer/consumer checks for Rust and C++ sensor nodes for applicable payloads.

### 10.6 MQTT

Test exact topic, direction, device isolation, canonical payload, malformed payload, unknown action/enum, legacy input, command metadata/lifecycle, and secret redaction.

### 10.7 CI

Extend `.github/workflows/shared-schema-check.yml` or add a narrowly scoped companion workflow. Schema checks must fail on unintentional canonical fixture/serialization drift.

## 11. Security constraints

- Validate schemas before executing commands.
- Unknown unsafe command actions/enums fail closed.
- IDs never grant authorization.
- Errors do not leak secrets/raw payloads.
- Backup/journal/export schemas preserve existing redaction rules.
- Extension objects are bounded in size and key/value count.
- Signature validation uses canonical serialized fields; aliases must not create alternate signed meanings.
- Numeric bounds and units are validated before control actions.

## 12. Evidence

Create:

`docs/evidence/P1.9-SHARED-SCHEMA-ALIGNMENT.json`

Record canonical artifact/version, source-of-truth mechanism, contract inventory, aliases/status, fixture coverage, Rust/TypeScript/backend/firmware/MQTT tests, CI result, and known unverified areas.

Evidence must distinguish focused tests from full-suite results. Never report a full repository suite as green unless actually run and passed.

## 13. Acceptance criteria

- [ ] Every cross-system contract has one documented owner and layer classification.
- [ ] Canonical wire contracts cover device identity, telemetry, actuator state, commands/lifecycle, configuration/safety/calibration, crop seasons/recipes, journal/events, pagination/filtering, errors, auth/scope, WiFi, health/FSM, and applicable MQTT payloads.
- [ ] Backend persistence models are not accidentally used as API schemas.
- [ ] Frontend transport types no longer independently redefine canonical fields without an explicit adapter/extension reason.
- [ ] Names, enum casing, requiredness, nullability, timestamp semantics, precision, and units are documented and tested.
- [ ] The `Influx f64 -> shared f32 -> frontend number` boundary is explicit; no silent precision loss remains undocumented.
- [ ] Legacy aliases are classified as compatibility behavior and are input-only.
- [ ] Authoritative telemetry includes canonical shared fields, including `runtime_ready` and `actuator_contradictory`.
- [ ] Journal shapes align with P1.6 semantics.
- [ ] P1.7 canonical error envelope is consistent frontend/backend.
- [ ] Recipe conversion preserves required precision and identity.
- [ ] Firmware/controller and MQTT payloads have representative cross-language coverage.
- [ ] Canonical fixtures prove serialization and compatibility behavior.
- [ ] CI rejects unintentional schema drift.
- [ ] Secrets are excluded/redacted appropriately.
- [ ] Existing P1.6/P1.7/P1.8 worktree changes are not reset, cleaned, reverted, stashed, or committed by P1.9.

## 14. Definition of done

P1.9 is complete when the canonical wire-contract mechanism exists, priority cross-system schemas are mapped and aligned, persistence/UI adapters are explicit, compatibility behavior is bounded and tested, cross-language fixtures cover the agreed surface, CI detects drift, and the evidence artifact records actual validation results and limitations.

A Rust-local schema suite alone is insufficient evidence of completion.

## 15. Implementation constraints

- Start from existing `hydragrow-shared`; do not assume OpenAPI/codegen architecture.
- Preserve P1.6 journal, P1.7 API/error, and P1.8 route/context semantics.
- Prefer explicit adapters over `FromRow` serialization.
- Prefer canonical emitted JSON plus compatibility input parsing over dual emission.
- Do not alter or clean unrelated dirty worktree changes.
- This specification does not authorize P1.9 implementation or commit activity by itself.
