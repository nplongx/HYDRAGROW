# P1.1 Authorization / Ownership Boundary

## Status

Implemented in worktree; backend verification passed. Final backend suite: 364 passed, 0 failed.

P1.1 establishes one enforceable authorization boundary for device-scoped HYDRAGROW operations. It builds on the P1.0 verification baseline and the P0 canonical device/state contracts. It does not reopen P0 architecture and does not implement durable command lifecycle.

## Requirement ID

`P1.1-AUTHORIZATION-OWNERSHIP-BOUNDARY`

## Change class

`C4` Cross-subsystem + `C7` Security-sensitive.

## Purpose

Establish an explicit answer to:

> Given an authenticated principal, which devices and capabilities may that principal access, and is that boundary enforced consistently across REST, WebSocket, device administration, telemetry, automation, analytics, health, alerts, backup/restore, and command paths?

The authorization boundary must prevent a valid identity or valid capability from being used to cross device ownership boundaries.

## 1. Scope

P1.1 covers:

- canonical authentication principal/context representation;
- Firebase Bearer authentication;
- service API key authentication;
- legacy API-key compatibility where explicitly retained;
- capability/scope enforcement;
- device ownership enforcement;
- bulk/multi-device ownership checks;
- WebSocket authentication and device isolation;
- device-admin and privileged control authorization;
- authorization coverage for device-scoped REST routes;
- negative-path tests for unauthenticated, unauthorized, and cross-device access;
- authorization audit evidence and project-state traceability.

The security boundary is deny-by-default for device-scoped operations: authentication establishes who is calling; capability establishes what the principal may do; ownership establishes which device(s) the principal may affect.

## 2. Existing architecture and required correction

The current backend already has `AuthContext`, scope checks, Firebase verification, service API keys, and `device_ownership` helpers. P1.1 must consolidate these mechanisms rather than create a second authorization system.

Current anchors include:

- `hydragrow-backend/src/api/middleware/auth.rs`
- `hydragrow-backend/src/db/device_ownership.rs`
- `hydragrow-backend/src/api/scope_definitions.rs`
- `hydragrow-backend/src/api/ws.rs`
- `hydragrow-backend/src/api/device_admin.rs`
- `hydragrow-backend/src/api/config_backup.rs`
- device-scoped routes registered in `hydragrow-backend/src/main.rs`

Known boundary gaps that P1.1 must resolve include:

1. many device-scoped handlers validate scopes without consistently validating ownership;
2. WebSocket authentication is implemented separately from REST middleware;
3. legacy service/API-key authentication can carry broad scopes without an explicit ownership model;
4. device-admin, analytics, health, alert, backup/restore, and related handlers require a complete ownership audit;
5. `X-Elevated-Token` previously compared against a static `ELEVATED_CONTROL_TOKEN` environment value;
6. some bulk operations need an explicit all-device ownership rule;
7. authentication identity, capability, and resource ownership are currently represented by separate ad-hoc checks rather than one reusable policy boundary.

P1.1 must fix these gaps without weakening existing scope semantics or inventing ownership from request metadata.

## 3. Authorization model

### 3.1 Principal

Define one canonical authenticated principal representation containing, at minimum:

- principal kind (`user`, `service`/equivalent);
- stable user identity when present;
- authenticated session/token identity when present;
- granted scopes/capabilities;
- service-key identity/label when applicable.

The implementation may retain `AuthContext` as the concrete type if it is extended cleanly. A second competing auth-context type is not required.

### 3.2 Authentication

Authentication must answer only whether the caller is authenticated and establish the principal.

Required semantics:

- invalid/missing Firebase token -> `401`;
- inactive/nonexistent Firebase user -> `401` or the repository's documented authentication-equivalent response;
- invalid/missing service API key -> `401`;
- authentication errors must not silently produce an anonymous `AuthContext` for protected routes;
- automatic Firebase user provisioning may remain where already required by product behavior, but newly provisioned users receive only the documented default read capability.

### 3.3 Capability

Capability checks must use the canonical scope definitions in `scope_definitions.rs`.

Required semantics:

- missing capability -> `403`;
- `*` remains restricted to the explicitly documented root/internal use case;
- adding a route must require an explicit capability decision;
- no route may infer write capability from a read capability;
- capability and ownership are independent checks.

Existing scopes remain authoritative unless a concrete P1.1 audit proves that a scope is incorrectly assigned to a route. Any scope change must be documented and tested.

### 3.4 Ownership

For a user principal, a device-scoped operation is authorized only when the user owns the target device.

Required helper semantics:

```text
authorize(principal, capability, device_id)
    authentication valid
    AND capability granted
    AND ownership granted
```

For operations affecting multiple devices:

```text
authorize_all(principal, capability, device_ids)
    authentication valid
    AND capability granted
    AND principal owns every target device
```

The operation must fail as a whole if any requested device is outside the principal's ownership boundary. Partial authorization of a bulk mutation is prohibited unless a route explicitly documents a different semantics and has dedicated tests.

Ownership must be resolved from the authoritative `device_ownership` data, not from:

- `device_id` supplied by the frontend alone;
- selected station/client state;
- `X-User-Id` alone;
- MQTT topic strings alone;
- cached telemetry/state presence;
- scope membership;
- existence of a device record without an ownership relationship.

## 4. Route coverage audit

Every route below must have an explicit authorization classification and test coverage.

### 4.1 Device-scoped data

- telemetry/current state;
- sensor history/range statistics;
- configuration reads/writes;
- calibration;
- crop seasons and season photos;
- dosing history/analytics;
- health/device health summaries;
- system events and alerts;
- scripts and automation;
- recipe reads/writes/apply/clear;
- webhooks and webhook tokens where device-scoped;
- fleet/device summaries where device membership is exposed.

### 4.2 Device control

- manual pump control;
- emergency control;
- command lifecycle/history;
- reboot;
- factory reset;
- OTA;
- WiFi/network provisioning;
- sensor OTA;
- other device-admin operations discovered during the audit.

### 4.3 Backup/restore

Backup export and restore must require both the documented capability and ownership of the path `device_id`.

The authorization decision must use the route device identity as authoritative. A backup payload's `device_id` must not grant access to another device.

### 4.4 Fleet endpoints

Fleet endpoints may expose only devices within the authenticated user's ownership set unless the endpoint is explicitly internal/root-only.

Fleet aggregation must not become an indirect cross-device data leak through counts, latest telemetry, warning metadata, recipe state, or health information.

## 5. WebSocket authorization boundary

The WebSocket endpoint is a protected device-scoped resource and must use the same conceptual authorization policy as REST.

Required flow:

```text
authenticate token/key
       |
       v
resolve principal
       |
       v
check capability for requested WS resource
       |
       v
check ownership(scoped_device_id)
       |
       v
start event stream
```

Requirements:

- Firebase token authentication must verify the token and active user;
- service-key behavior must be explicitly classified as service/internal access rather than implicitly treated as a user's ownership;
- a user authenticated for device A must not subscribe to device B;
- event filtering remains a defense-in-depth measure, not the primary authorization boundary;
- device-scoped events must never cross the authorized device boundary;
- broad/non-device events require an explicit classification before they are forwarded;
- unauthenticated connections must not become active event subscribers;
- authorization failures must close/reject the connection without starting the device event stream.

## 6. Service keys and legacy API keys

Service credentials are not user ownership proofs.

The implementation must explicitly distinguish:

1. user principal + ownership;
2. service principal + explicitly granted service scope;
3. legacy shared API key compatibility.

The legacy API key must not silently acquire arbitrary user-device ownership merely because the request contains `X-User-Id`.

If a service principal needs access to a device-scoped route, the authorization policy must identify why that service is trusted and what resource boundary applies.

Broad legacy behavior may be retained only where required for compatibility and documented as a bounded migration exception. New device-scoped APIs must not copy the legacy broad-access pattern.

## 7. Privileged/elevated control

The current static `ELEVATED_CONTROL_TOKEN` / `X-Elevated-Token` mechanism is not an acceptable long-term user authorization boundary.

P1.1 must replace or constrain it with an explicit privileged-control mechanism that is:

- short-lived or otherwise replay-bounded;
- scoped to the privileged action class;
- bound to the authenticated principal;
- bound to the target device/resource where applicable;
- auditable without logging the secret;
- rejected when expired, malformed, or used for another resource/action.

`X-User-Confirmed` may remain a confirmation signal, but confirmation is not authentication, capability, or ownership.

The privileged mechanism must not bypass the normal authentication and ownership checks.

### 7.1 Implemented privileged mechanism

Dangerous manual control uses a backend-issued JWT carried in `X-Privileged-Token`:

- TTL: 60 seconds;
- signing algorithm: HS256;
- signing secret: `PRIVILEGED_CONTROL_SECRET`;
- claims: authenticated principal, target `device_id`, `action_class=dangerous_control`, expiry;
- issuance requires normal authenticated principal, dangerous-control capability, target-device ownership, and `X-User-Confirmed: true`;
- execution still requires normal authentication, capability, ownership, and confirmation;
- wrong principal/device/action class or expired/malformed token is rejected;
- token contents/secret are not logged.

## 8. Error semantics

P1.1 standardizes authorization failures:

| Condition | Required result |
| --- | --- |
| No valid authentication | `401 Unauthorized` |
| Valid principal, missing capability | `403 Forbidden` |
| Valid principal + capability, target outside ownership | `403 Forbidden` |
| Valid principal + capability + ownership, resource genuinely absent | route-specific `404` |
| Invalid request independent of authorization | route-specific `400` |

Ownership failure must not be converted into a successful empty result, fabricated resource, or default state.

Where resource enumeration itself is sensitive, a route may intentionally return `404` for an unauthorized resource, but this must be an explicit documented policy applied consistently to that resource class. Tests must verify the chosen semantics.

## 9. Security invariants

P1.1 must preserve these invariants:

1. A principal cannot read another user's device telemetry.
2. A principal cannot mutate another user's device configuration.
3. A principal cannot control another user's actuators.
4. A principal cannot trigger OTA/reboot/factory reset on another user's device.
5. A principal cannot export or restore another user's backup.
6. A principal cannot receive another user's device-scoped WebSocket events.
7. A missing ownership row is not equivalent to ownership.
8. A capability is not equivalent to ownership.
9. A frontend-selected device is not an authorization proof.
10. A confirmation header is not an authorization proof.
11. Authentication failures never fall back to a default privileged context.
12. Bulk operations cannot partially cross the ownership boundary.

## 10. Required test matrix

At minimum, backend tests must cover:

### Authentication

- missing credentials -> `401`;
- invalid Firebase token -> `401`;
- inactive user -> rejected;
- valid Firebase user -> canonical principal;
- valid service key -> service principal;
- legacy API key behavior remains explicitly bounded.

### Capability

- required scope present -> allowed to ownership stage;
- required scope absent -> `403`;
- read scope cannot perform write operation;
- `*` behavior remains explicitly restricted.

### Ownership

For users A and B and devices A/B:

```text
user A + device A -> allowed when capability exists
user A + device B -> denied
user B + device A -> denied
user B + device B -> allowed when capability exists
```

Cover both reads and mutations.

### Bulk

- all requested devices owned -> allowed;
- one requested device unowned -> entire operation denied;
- empty target list follows explicit route semantics;
- duplicate device IDs do not bypass ownership accounting.

### WebSocket

- authenticated owner receives own device events;
- authenticated owner receives no other-device events;
- authenticated non-owner cannot establish a device-scoped stream;
- invalid token cannot establish a stream;
- legacy/service authentication follows its explicit policy.

### Privileged control

- valid privileged authorization + ownership -> allowed;
- expired/replayed/wrongly scoped privileged credential -> denied;
- privileged credential for device A cannot operate on device B;
- `X-User-Confirmed` without normal authorization remains denied.

## 11. Implementation constraints

1. Reuse the existing `device_ownership` data model unless a schema defect makes it impossible to express the required policy.
2. Prefer reusable authorization helpers/middleware over repeating raw SQL ownership checks in every handler.
3. Keep capability checks and ownership checks independently visible in code and tests.
4. Do not move authorization responsibility into the frontend.
5. Do not use cached device state as an authorization source.
6. Do not silently broaden service-key privileges to make legacy tests pass.
7. Do not weaken tests, fixtures, or assertions to accommodate existing unauthorized paths.
8. Do not introduce durable command persistence as part of P1.1.
9. Do not change P0 authoritative telemetry/state semantics.
10. Do not use authorization changes to mask unrelated DB, MQTT, or frontend-toolchain failures.

## 12. Evidence requirements

Every authorization surface changed by P1.1 must have evidence containing:

```text
Requirement ID: P1.1-AUTHORIZATION-OWNERSHIP-BOUNDARY
Route/resource:
Principal type:
Capability:
Ownership target:
Expected result:
Actual result:
Test command:
Environment:
Timestamp:
```

Security-sensitive evidence must include negative-path results, not only successful authorization cases.

The evidence artifact must make cross-device isolation auditable without exposing credentials or tokens.

## 13. Acceptance criteria

### AC-1 - Canonical principal

All protected REST and WebSocket paths use a consistent authenticated-principal representation. Authentication failures cannot fall through to a privileged/default context.

### AC-2 - Device ownership enforcement

Every device-scoped route has an explicit ownership decision, and a user cannot access another user's device through direct, nested, or alternate API paths.

### AC-3 - Capability enforcement

Every protected operation has an explicit required capability. Missing capability is denied without being converted to an empty/default result.

### AC-4 - Cross-device isolation

Automated tests prove A/B device isolation for reads, writes, control, administration, analytics/health, backup/restore, automation, and other applicable device-scoped surfaces.

### AC-5 - Bulk ownership

Bulk operations require ownership of every target device and cannot partially execute across an ownership boundary.

### AC-6 - WebSocket boundary

WebSocket authentication and ownership are enforced before subscription, and device-scoped events cannot cross the authorized device boundary.

### AC-7 - Service/legacy credential boundary

Service and legacy API-key behavior is explicitly classified, bounded, tested, and does not treat caller-supplied user/device headers as ownership proof.

### AC-8 - Privileged control

Privileged/elevated control is no longer an unbounded static secret check; authorization is authenticated, scoped, resource-bound, replay/expiry constrained, and auditable.

### AC-9 - Error semantics

Authentication, capability, ownership, and genuine resource absence produce the documented HTTP/WebSocket failure semantics without fabricated success or data.

### AC-10 - Route audit completeness

All device-scoped REST, WebSocket, fleet, admin, analytics, health, alert, automation, command, and backup/restore surfaces have an explicit authorization classification recorded in the implementation/evidence matrix.

### AC-11 - Security regression verification

The declared backend verification commands and all new authorization tests execute successfully, with failures classified rather than hidden. No P0 contract is weakened.

### AC-12 - Traceability

`docs/project-state/TRACEABILITY.md` and `docs/project-state/CURRENT-STATUS.md` are updated with the P1.1 implementation status, evidence, residual risks, and P1.2 handoff boundary.

## 14. P1.2 boundary

P1.1 establishes who may create/observe/control a command. It does **not** make command state durable.

P1.2 remains responsible for:

- durable command records;
- restart/reconnect reconciliation;
- timeout/UNKNOWN policy;
- multi-instance command authority;
- recovery of in-flight commands.

P1.1 must not use an in-memory command lifecycle map as an authorization database.

## 15. Non-goals

- durable command lifecycle;
- command timeout/reconciliation redesign;
- scheduler fail-closed redesign;
- backup/restore lifecycle redesign beyond its authorization boundary;
- Flux query redesign except where authorization tests expose a concrete cross-device leak;
- new telemetry/state architecture;
- frontend architecture redesign;
- replacing Firebase;
- replacing PostgreSQL;
- introducing a new identity provider;
- broad RBAC redesign unrelated to device/resource authorization.

## 16. Implementation order

```text
P1.1 Authorization / Ownership Boundary
    |
    +-- 1. Freeze P1.0 baseline and worktree
    |
    +-- 2. Define canonical principal + reusable authorization policy
    |
    +-- 3. Audit/correct device-scoped REST routes
    |
    +-- 4. Audit/correct fleet + bulk routes
    |
    +-- 5. Unify WebSocket authentication/ownership boundary
    |
    +-- 6. Replace/constrain static elevated control mechanism
    |
    +-- 7. Add cross-device negative-path test matrix
    |
    +-- 8. Run backend/security verification
    |
    +-- 9. Record evidence + traceability
    |
    v
P1.2 Durable Command Lifecycle
```

## 17. Risks and rollback

### Risks

- legacy integrations may depend on broad API-key behavior;
- some existing endpoints may currently rely on implicit ownership through frontend flow rather than backend checks;
- tightening ownership can expose previously hidden integration/test fixture assumptions;
- WebSocket and REST may diverge if they retain separate policy implementations;
- changing privileged control credentials can require coordinated frontend/backend deployment.

### Rollback

Rollback must not restore an authorization bypass as a silent compatibility measure. If deployment compatibility fails, isolate the affected legacy route/credential as an explicitly documented migration exception with bounded scope and monitoring while preserving ownership checks for new/current paths.

## 18. P1.1 completion rule

P1.1 is complete only when every in-scope resource has a documented authentication, capability, and ownership decision; cross-device negative tests execute; WebSocket isolation is verified; privileged control is bounded; backend/security verification evidence is current; and project-state traceability is synchronized.

Completion does not mean every legacy credential has already been removed. It means every retained compatibility path has an explicit security boundary, owner, evidence, and migration classification.
