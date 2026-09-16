# P0.5 Frontend Data Architecture

## Status

Implementation contract; architecture is closed, with residual verification blockers tracked separately below.

This revision follows a repository audit performed before the next implementation step. It records the architecture that actually exists, including transitional paths that must still be removed. It does not treat the first P0.5 code changes as proof that the migration is complete.

## Purpose

Reduce the frontend's current data-flow coupling by establishing explicit boundaries between station identity, server/domain state, realtime state, UI state, and persistence.

P0.5 addresses four identified architectural risks:

- `useDeviceSync` acting as a god-hook for fetching, synchronization, normalization, and state mutation.
- `useDeviceStore` acting as an implicit domain/source-of-truth store.
- direct API/fetch calls scattered through components and feature hooks.
- `localStorage` values being treated as authoritative device/domain state.

### Audit findings

The repository already uses TanStack Query and has domain hooks including `useDeviceTelemetry`, `useDeviceConfig`, `useAutomationScripts`, and `useSystemHealthSummary`. P0.5 extends this existing query layer; it does not replace the state-management framework.

The initial audit found the following migration defects; the current worktree has resolved the device-domain ownership defects listed below except where explicitly retained as target-state or compatibility behavior:

- `useDeviceSync` has been reduced to realtime transport/orchestration; it no longer reads unified configuration or telemetry or writes device-domain state into `useDeviceStore`.
- `useDeviceStore` now retains only explicitly local PWM preferences; no production consumer reads device-domain state from it.
- `Settings` unified configuration and its OTA/WiFi/calibration/reboot/factory-reset backend requests now use the shared API boundary; its form remains local draft/input state.
- Onboarding, safety, season/recipe, automation, pairing, Dashboard, and Settings production consumers no longer use `useDeviceStore` for device-domain state.
- `useFleetStatus` remains a separate multi-device/fleet query boundary because fleet use is explicitly outside normal single-station identity flow; its failure semantics must preserve unknown rather than fabricate offline state.
- local persistence remains for legitimate local concerns such as station selection, onboarding, application connection settings, API-key/session handling, and PWM preferences. These must not be blanket-removed.
- Realtime contact/health/FSM events are routed separately from telemetry; command lifecycle is exposed through its own event boundary.

The migration architecture is closed. Remaining verification work is governed by §13 and the P0 residual gate below; newly discovered source exceptions must still be classified explicitly under §13.

P0.5 builds on the canonical `StationContext` from P0.1, the authoritative state/telemetry semantics from P0.3, and the configuration contract from P0.4. It does not replace those contracts.

## 1. Core invariants

1. `StationContext.selectedDeviceId` is the canonical frontend device identity.
2. Server/domain state is not owned by UI components or generic local UI stores.
3. `useDeviceStore` is not an authoritative source for device configuration, telemetry, availability, actuator state, or command lifecycle.
4. `useDeviceSync` is not the authoritative owner of every device data domain.
5. Server data access has explicit query/mutation/realtime boundaries rather than arbitrary direct fetches from components.
6. `localStorage` is persistence for user preference or explicitly designated local state; it is never authoritative device/domain truth.
7. A cached or optimistic value must not be represented as confirmed server/device state without supporting evidence.
8. API/query errors remain observable and are not converted into empty success, fabricated defaults, or stale-looking success state.
9. Device-scoped server data is keyed/scoped by canonical `selectedDeviceId` and cannot leak between stations.
10. Realtime updates enter the same domain boundary as equivalent server state rather than creating a second frontend truth.
11. Command lifecycle remains separate from observed/physical state; P0.5 consumes the P0.2 contract rather than redefining it.
12. Frontend architecture must preserve the unknown/stale/error semantics established by P0.3.
13. Application connection settings (`backend_url`, optional API key, integration URLs) are local connection configuration, not device-domain configuration.
14. Local form drafts/defaults are presentation/input state and must not masquerade as successfully loaded server configuration.

## 2. Ownership model

### 2.1 Station identity

`StationContext` owns:

- `selectedDeviceId`
- `selectedDevice`
- available devices
- selection lifecycle
- selection validation
- user selection persistence

It does not own telemetry, configuration, command lifecycle, actuator state, automation execution state, or historical domain data.

The P0.1 persistence key remains a user preference. A persisted device ID does not prove that the device currently exists or is available.

### 2.2 Server/domain state

Device configuration, telemetry, controller health, actuator observations, FSM/system state, availability, command lifecycle, and historical domain data belong to their respective server/domain boundaries.

The frontend may cache these values for rendering and performance, but cache ownership does not change domain authority.

The implementation must reuse existing typed models and API contracts where possible rather than introduce a parallel generic device-state model solely for P0.5.

### 2.3 UI/session state

Local React state or a UI store may own transient presentation concerns such as:

- selected tab
- modal visibility
- form interaction state
- temporary filters
- expanded/collapsed UI state

Such state must not become a substitute for server/domain state.

### 2.4 Persistence

`localStorage` may persist explicit user preferences or other state whose authority is explicitly local.

It must not be the source of truth for:

- current telemetry
- current device health
- current actuator state
- current availability/contact state
- configuration
- command lifecycle
- historical server data

Explicitly allowed local persistence includes:

- station selection;
- onboarding progress/dismissal;
- application connection settings such as backend URL and optional API key, subject to existing platform/security rules;
- UI preferences such as PWM preferences.

These local values never become authority for current device telemetry, health, actuator state, availability, command lifecycle, or PostgreSQL-backed device configuration.

### 2.5 Form drafts and defaults

Settings may initialize a local form with schema/default values before the server response arrives. This is not device configuration truth.

The UI/data boundary must distinguish `loading`, `loaded`, `error`, and local `draft` state. A failed server read must not silently leave a default object looking like current persisted device configuration.

## 3. Target frontend data flow

The target architecture is conceptually:

```text
                    Backend API / realtime source
                              |
                 +------------+------------+
                 |                         |
              queries                   mutations
                 |                         |
                 +------------+------------+
                              |
                       domain data layer
                              |
                    feature/domain hooks
                              |
                              v
                              UI

StationContext ------------------------------> device identity only
UI/local state -------------------------------> presentation state only
localStorage ---------------------------------> explicit local persistence only
app connection settings -----------------------> backend connection configuration only
```

The exact React data/query library already present in the repository should be retained unless implementation evidence requires otherwise. P0.5 is an architecture refactor, not a mandate to replace the state-management stack.

## 4. `useDeviceSync` boundary

`useDeviceSync` must be reduced from a broad device-data owner to an explicit synchronization/orchestration boundary.

### It may own

- coordination of existing synchronization flows that genuinely require orchestration;
- subscription lifecycle for realtime data where no narrower existing boundary is available;
- triggering refresh/reconciliation after an authoritative external update;
- compatibility behavior needed during migration.

### It must not become the permanent owner of

- every device API query;
- every device mutation;
- unrelated domain transformations;
- UI presentation state;
- local persistence of domain truth;
- fabricated fallback values;
- command success inferred from request completion;
- physical actuator state inferred from command lifecycle.

New functionality must not be added to `useDeviceSync` merely because it is already imported widely.

Where a responsibility belongs to a specific domain, it should move to a domain-specific boundary during migration.

The current configuration/telemetry reads and direct domain-state writes inside `useDeviceSync` are transitional defects. They are not permitted target-state ownership.

## 5. `useDeviceStore` boundary

`useDeviceStore` remains a transitional compatibility surface only where existing consumers still require it.

It must not be treated as authoritative domain storage for the device.

Existing P0.1 compatibility behavior for `deviceId` may remain during migration, but `StationContext.selectedDeviceId` remains authoritative.

Transient reset behavior required for station switching remains valid because P0.1 requires Device A state to be cleared before Device B state is presented.

The migration must avoid replacing one god-store with another. If a value is server/domain state, its canonical access path must be identifiable independently of `useDeviceStore`.

The end state does not require an empty Zustand store. Local UI/session state may remain. Completion requires that current device-domain state no longer requires `useDeviceStore` as its authoritative access path.

## 6. API/query/mutation boundary

### 6.1 Reads

Server reads should be accessed through a stable API/query boundary appropriate to the domain.

Components should consume domain hooks or query abstractions rather than implement their own request lifecycle.

The existing `apiClient` is the shared HydraGrow backend transport boundary. Domain hooks should normally build on it. Lower-level `httpFetch` remains valid for platform-specific or external-service operations when they are not HydraGrow backend-domain reads.

A domain query must preserve:

- loading state;
- successful data;
- unavailable/empty semantics where applicable;
- API/query error;
- device identity scope.

An error must not be converted into `[]`, `null` with an implied success meaning, fabricated device state, or a plausible domain default unless that behavior is explicitly part of the existing contract.

### 6.2 Mutations

Mutations must expose the distinction between:

- request initiation;
- server/API success;
- server/API failure;
- subsequent domain/realtime observation where applicable.

An HTTP/API success response must not be presented as physical confirmation. P0.4's `partial_success` configuration semantics must remain visible to consumers rather than being swallowed by a generic synchronization hook.

### 6.3 Direct fetch

P0.5 does not ban the browser `fetch` primitive.

The invariant is that domain API access has a discoverable owner. New component-level direct requests for existing device domains are prohibited.

Existing direct fetches must be classified as:

1. migrate to an existing API/query boundary;
2. create a narrowly scoped domain boundary;
3. retain temporarily with an explicit compatibility reason.

Temporary exceptions must not become a new shared abstraction without evidence.

A known valid exception is a third-party upload whose signed endpoint is returned by the backend, such as the Cloudinary upload in `SeasonPhotoJournal`. This is not a HydraGrow backend-domain read.

Known migration targets include the backend request paths in `Settings`, legacy page-level request wrappers, and feature components that already have an equivalent domain query hook.

### Direct-request migration ledger

The previously identified `Settings` direct device-domain requests have now been moved to the shared API boundary:

- `GET /devices/{device_id}/ota/status` -> `apiGet`.
- `POST /devices/{device_id}/ota/trigger` -> `apiPost`, preserving `X-User-Confirmed`.
- `GET /devices/{device_id}/wifi` -> `apiGet`.
- `POST /devices/{device_id}/wifi` -> `apiPost`, preserving `X-User-Confirmed`; the backend route remains transitional by its own contract.
- `POST /devices/{device_id}/calibration/ph/start`, `/capture`, `/finish` -> `apiPost`; capture preserves its extended timeout through the API boundary.
- `POST /devices/{device_id}/reboot` -> `apiPost`, preserving `X-User-Confirmed`.
- `POST /devices/{device_id}/factory-reset` -> `apiPost`, preserving `X-User-Confirmed`.

No page-local generic `callApi` wrapper remains in `Settings`. The browser `fetch` used by `platform/http.ts` remains the transport primitive behind `apiClient`. Third-party signed uploads remain an explicit external-service exception.

## 7. Realtime data boundary

Realtime data must not create an independent competing source of truth inside the frontend.

When an authoritative realtime update is received, the implementation should update or invalidate the appropriate domain/query state through one identifiable path.

Event-to-domain routing must be explicit. A contact/availability, health, FSM, system-event, or command-lifecycle event must not be routed through the telemetry query merely because the same WebSocket transports all event types. If no current-state query exists for an event, retain only narrowly scoped event/session state; do not synthesize an authoritative snapshot.

Realtime handling must preserve P0.3 semantics:

- device identity remains canonical;
- observation and receipt/contact timestamps remain distinct where available;
- stale/error/unknown states remain distinguishable;
- a health/contact event does not refresh unrelated telemetry;
- command lifecycle does not mutate physical state unless authoritative physical observation is also present.

Transport reconnection alone is not evidence that every domain value is fresh.

## 8. Command lifecycle boundary

P0.2 remains authoritative for command lifecycle:

```text
REQUESTED -> SENT -> ACKNOWLEDGED -> CONFIRMED
```

with `REJECTED`, `FAILED`, `TIMEOUT`, and `UNKNOWN` as defined by the P0.2 contract.

P0.5 does not redesign these states.

The frontend must consume command lifecycle through a domain/server boundary rather than reconstruct it from:

- button click state;
- request promise resolution;
- `localStorage`;
- generic device status;
- `useDeviceStore` mutations.

The UI must continue to distinguish command lifecycle from observed actuator/physical state.

P0.5 does not add automatic retry or new command reconciliation semantics.

## 9. Configuration boundary

P0.4 establishes PostgreSQL-backed configuration as authoritative for configuration reads and writes.

Frontend architecture must therefore:

- consume unified configuration through the existing API/domain boundary;
- preserve explicit API errors;
- preserve `404` missing-device-config semantics;
- preserve `202 partial_success` when persistence succeeds but MQTT publication fails;
- not use localStorage configuration as a substitute for failed server reads;
- not claim physical application from MQTT publication success.

`useDeviceSync` must not hide these outcomes behind generic success/default behavior.

Two configuration categories must remain separate:

1. **Application connection configuration**: backend URL, optional API key, Grafana URL and similar local integration settings.
2. **Device configuration**: unified five-domain configuration from `/devices/{device_id}/config/unified`, for which P0.4 makes the server authoritative.

The first category may be required to reach the second; it is not a fallback source for the second.

## 10. Station switching and device isolation

P0.1 remains the identity authority.

For a switch from Device A to Device B:

1. `selectedDeviceId` changes through `StationContext`.
2. Device-dependent queries use the new canonical identity.
3. transient state for Device A is not rendered as Device B state.
4. stale responses/events scoped to Device A cannot overwrite Device B state.
5. cached domain data may be reused only when its query identity explicitly includes the correct device scope.

No frontend abstraction may infer device identity from a cached payload when canonical StationContext identity is available.

A `deviceIdOverride` on a domain hook is permitted only for explicit multi-device/fleet use. Normal single-station features should derive identity from `StationContext`.

## 11. Error and unknown semantics

P0.5 must preserve the semantic distinctions established by P0.3.

At minimum, frontend consumers must be able to distinguish where the underlying contract provides them:

- loading;
- valid/current data;
- stale data;
- unknown/unavailable data;
- explicit error;
- local/optimistic state.

Forbidden architecture shortcuts include:

```text
query error -> empty success
missing telemetry -> zero
missing actuator state -> false
request success -> physical success
reconnect -> all data fresh
localStorage -> current device truth
```

The exact rendering policy remains feature-specific, but the data boundary must preserve enough information for honest rendering.

## 12. Migration strategy

P0.5 is a staged frontend migration. It is complete only after the boundaries below are enforced; creating a few domain hooks is not completion.

### Step 1 - Inventory

Audit frontend consumers of:

- `useDeviceSync`;
- `useDeviceStore`;
- direct `fetch` / API calls;
- device/config/telemetry values in `localStorage`;
- realtime subscriptions and state mutation paths.

Classify each use by domain and authority.

### Step 2 - Freeze boundaries

Document the target owner for each existing domain without changing behavior unnecessarily.

Audited ownership map:

- station selection -> `StationContext`;
- telemetry/current authoritative snapshot -> `useDeviceTelemetry` / TanStack Query cache;
- unified device configuration -> `useDeviceConfig` / TanStack Query cache;
- command lifecycle -> P0.2 command boundary consumed by control UI;
- historical system events -> `SystemLog` query boundary;
- historical/analytics domains -> their existing device-scoped query hooks;
- application connection settings -> `platform/settings.ts` and local persistence;
- UI drafts/preferences -> component state or narrowly scoped local store;
- realtime transport -> synchronization adapter only, with event-specific routing into the owning domain.

### Step 3 - Extract reads

Move repeated direct device-domain reads behind the existing API/query abstraction or a narrowly scoped domain query hook.

Priority from the audit:

1. unified device configuration in `Settings`;
2. telemetry/status consumers still reading `useDeviceStore`;
3. system events/alerts still copied into `useDeviceStore`;
4. `Settings` OTA, WiFi, calibration, reboot, and factory-reset requests currently hidden behind its page-local `callApi`;
5. legacy season/recipe/automation consumers with backend requests in component scope;
6. fleet status fan-out and other duplicate query implementations.

### Step 4 - Extract mutations

Move device-domain mutations behind explicit mutation boundaries and preserve their real API outcomes.

Mutation boundaries should update/invalidate only affected query keys. They must not require a broad `useDeviceSync` callback to make the result visible.

### Step 5 - Reduce `useDeviceSync`

Remove responsibilities already represented by dedicated domain boundaries. Keep only synchronization/orchestration that has a demonstrated cross-domain reason to exist.

Specifically remove, in order:

1. duplicate telemetry fetching once `useDeviceTelemetry` owns the read;
2. unified device-config fetching/merging once `useDeviceConfig` owns the read;
3. domain-state writes such as telemetry, availability, health, FSM, and alert copies into `useDeviceStore`;
4. broad invalidation of telemetry for non-telemetry events;
5. fabricated/mock domain fallback paths.

The WebSocket connection may remain here temporarily if no narrower realtime adapter exists. Its responsibility then ends at validated event routing, not domain ownership.

### Step 6 - Reduce `useDeviceStore`

Move server/domain truth to its appropriate query/domain boundary. Retain only transitional compatibility and genuinely local state.

Migration candidates found in the audit:

- `sensorData`, `authoritativeTelemetry`;
- `deviceStatus`, `isControllerStatusKnown`, `isSensorOnline`;
- `controllerHealth`, `fsmState`;
- `systemEvents`, `tankAlert`;
- `settings`, `isMissingConfig`, `isLoading` where these represent domain/config state.

`pwmPreferences` may remain because it is explicitly local. `deviceId` may remain only as the P0.1 compatibility mirror until all consumers migrate.

### Step 7 - Remove domain localStorage truth

Delete reads/writes that treat persisted local values as current device/domain truth. Retain explicit user-preference persistence such as canonical station selection.

Do not delete legitimate application settings or onboarding/UI persistence merely because it uses `localStorage`. Audit by authority, not storage API name.

### Step 8 - Reconcile realtime paths

Ensure realtime updates have one identifiable path into the relevant domain/query state and cannot bypass device identity or freshness semantics.

At minimum, route these domains separately:

- telemetry observations;
- contact/availability;
- controller health;
- FSM/system state;
- system events/alerts;
- command lifecycle.

If a route only has an invalidation target, invalidate that target. Do not populate another domain's cache with guessed fields.

### Step 9 - Remove obsolete compatibility paths

Only after consumers have migrated should obsolete store/sync/direct-fetch paths be deleted.

Deletion requires a source audit showing no production consumer remains. Tests that seed the compatibility store are not sufficient reason to keep production domain ownership there.

## 13. Verification contract

### Static/source audit

The implementation must produce an auditable inventory of remaining:

- `useDeviceSync` consumers and responsibilities;
- `useDeviceStore` domain-state consumers;
- direct device-domain fetches;
- localStorage access involving domain state;
- realtime handlers that mutate domain state directly.

Each remaining occurrence must have an explicit reason.

Classify each remaining occurrence as:

- **target-state boundary**: allowed production ownership;
- **migration compatibility**: temporary, with removal condition;
- **test fixture**: compatibility-only test usage;
- **invalid**: must be removed before P0.5 completion.

### Frontend tests

Required coverage includes:

1. Device-scoped query uses canonical `selectedDeviceId`.
2. Device A data cannot appear under Device B after switching.
3. Query failure remains an error and is not converted to empty success.
4. Configuration `partial_success` remains observable by the consumer.
5. Missing/unknown telemetry does not become a fabricated numeric/boolean value.
6. Realtime telemetry updates affect telemetry state only; contact/health/FSM events do not falsely refresh telemetry freshness.
7. Reconnect does not mark stale data fresh solely because transport recovered.
8. Command lifecycle state is consumed independently from actuator/physical state.
9. localStorage user preference can restore station selection but cannot override authoritative server/domain data.
10. Application connection settings may persist locally without becoming device configuration truth.
11. A Settings form draft/default is not presented as loaded server configuration after a failed read.
12. Optimistic/local UI state cannot overwrite authoritative domain state without an explicit domain transition.
13. A third-party upload may remain outside `apiClient` without weakening the HydraGrow backend-domain boundary.

### Integration verification

At least one end-to-end frontend data path for each major domain should be verified when frontend runtime tooling is available:

- station identity;
- telemetry/current state;
- configuration;
- command lifecycle;
- realtime update;
- one historical/read-only domain.

Static/source verification remains mandatory even when frontend runtime tooling is unavailable.

Verification must distinguish unavailable frontend tooling from test failures. No test pass may be claimed without execution evidence.

## 14. Acceptance criteria

P0.5 is complete only when all of the following are true:

1. Station identity has one canonical frontend authority: `StationContext.selectedDeviceId`.
2. Telemetry/current authoritative state has an identifiable query boundary independent of `useDeviceStore`.
3. Device configuration has an identifiable query/mutation boundary independent of `useDeviceStore`.
4. Historical system events and other migrated read-only domains have identifiable query boundaries.
5. `useDeviceSync` no longer owns migrated domain reads or writes; remaining responsibility is transport/orchestration only.
6. New direct component-level backend API access is eliminated for migrated domains, with every retained exception documented; the outstanding `Settings` device-admin/OTA/WiFi/calibration requests listed in §6.3 are migrated before completion.
7. `localStorage`/local persistence is not authoritative for current device/domain state; legitimate local application settings remain allowed.
8. Query and mutation failures remain observable and are not swallowed by synchronization/store layers.
9. Realtime routing does not use one domain's cache as a substitute for another domain.
10. Device-scoped data cannot cross station boundaries during selection changes or late responses.
11. P0.3 unknown/stale/error semantics remain representable and are not replaced by plausible defaults.
12. P0.4 configuration outcomes, including `partial_success`, remain visible through the frontend boundary.
13. P0.2 command lifecycle remains separate from observed/physical state.
14. Settings form defaults/drafts cannot masquerade as loaded server configuration after read failure.
15. Compatibility paths retained during migration are explicitly identified and have a removal condition.
16. Verification covers boundary invariants rather than only component rendering.
17. No new backend, MQTT, firmware, or physical capability is claimed by the frontend refactor.

## 15. Non-goals

- redesign of P0.2 command lifecycle;
- automatic command retry/retry engine;
- new MQTT or hardware protocol;
- backend-wide decomposition;
- replacement of PostgreSQL or InfluxDB;
- replacement of the existing frontend state/query framework without implementation evidence;
- generic event sourcing;
- new telemetry hardware capability;
- redesign of StationContext ownership established by P0.1;
- changing P0.4 configuration authority;
- inventing new freshness thresholds merely for UI determinism.

## 16. Expected implementation surfaces

Exact files must be confirmed during implementation, but the audit should cover at least:

- `hydragrow-frontend/src/` station/context modules;
- `useDeviceSync` implementation and all consumers;
- `useDeviceStore` implementation and all consumers;
- frontend API/query utilities;
- realtime/WebSocket/MQTT-derived frontend handlers;
- configuration hooks and Settings consumers;
- Dashboard/Operations consumers of telemetry and command state;
- `Settings` unified configuration read/write path and local draft state;
- legacy `useDeviceStore` consumers in onboarding, safety, season/recipe, automation, pairing, Dashboard, and Settings surfaces;
- fleet status and historical query hooks;
- frontend tests covering station switching and data isolation.

### Explicit migration completion rule

P0.5 is not complete merely because `apiClient`, `useDeviceTelemetry`, and `useDeviceConfig` exist. Production consumers must use the boundaries, and old authoritative paths must be removed or explicitly classified as temporary compatibility.

Implementation must prefer extending existing boundaries over introducing parallel abstractions with overlapping authority.


## P0 residual gate and P1 entry boundary

The frontend architecture defined by P0.5 is closed. P0.5 must not be reopened or broadened while closing residual verification blockers. The following distinction is mandatory:

- **P0 architecture**: the ownership/data-boundary design in this spec; treat as complete once the acceptance criteria above are satisfied.
- **P0 residual blockers**: regressions or verification prerequisites that must be closed before a trustworthy P1.0 baseline.
- **P1 blockers**: cross-system design/implementation gaps discovered by the whole-system audit; these belong to P1 and must not be backfilled into P0.5.

### P0 residual blockers before P1.0

1. **Simulator compile regression**: all shared/controller event additions must have exhaustive simulator handling. The current `PublishCommandLifecycle` event regression must be resolved before P1.0; adding more P1 behavior on top of a non-compiling simulator is not permitted.
2. **Backend DB test baseline**: current DB-backed failures must be diagnosed against the intended test environment and implementation. Ownership/configuration semantics must not be weakened to obtain passing tests.
3. **Backend formatting drift**: `hydragrow-backend/src/mqtt/handlers/sensors.rs` and any other touched backend code must satisfy the repository formatting contract.

P1.0 starts only after these residual items are either closed or explicitly recorded as an external environment blocker with its impact understood. P1.0 is then a verification baseline, not a second P0 repair phase.

### P1 blockers remain outside P0.5

The following findings from the whole-system audit are not P0.5 residual work:

- durable command lifecycle and restart/reconnect reconciliation;
- canonical authorization/ownership boundary across REST, WebSocket, device-admin, analytics, health/alerts, backup, and command paths;
- fail-open safety/error semantics in scheduler and other safety-critical reads;
- privileged control token lifecycle/scoping;
- backup/restore lifecycle and success semantics;
- Flux query validation/encoding;
- fabricated all-pumps-off control-state fallback;
- legacy controller-status heartbeat heuristic competing with canonical availability/contact semantics;
- observability completeness;
- eventual shared-schema/frontend model alignment.

These must be handled by their dedicated P1 specs/plans rather than by extending `useDeviceSync`, `useDeviceStore`, StationContext, or the P0 frontend data boundary.

### Verification discipline

No P0/P1 gate may claim a test pass without execution evidence. If a required toolchain is unavailable, record the exact limitation and do not substitute static inspection for a runtime pass.
