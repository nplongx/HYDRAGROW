# P1.7 API Query / Mutation Consolidation

**Status:** IMPLEMENTATION IN PROGRESS — CORE CONSOLIDATION LANDED
**Phase:** P1.7
**Depends on:** P1.0 Verification / Cross-System Baseline, P1.1 Authorization / Ownership Boundary, P1.2 Durable Command Lifecycle, P1.3 Error / Failure Semantics, P1.4 Operations State Boundary, P1.5 Backup / Restore Lifecycle, P1.6 Journal / Event Model
**Scope:** canonical API query/mutation contracts, frontend data-access consolidation, backend response/error normalization, authorization-preserving route consolidation, cache/invalidation semantics, and compatibility migration

## 1. Purpose

Establish one predictable API data-access model across HydraGrow.

P1.7 addresses fragmentation that remains after P1.0–P1.6:

- frontend code calls generic `apiGet/apiPost/apiPut/apiPatch/apiDelete` directly from pages/components;
- React Query hooks exist, but their query keys, request functions, and invalidation behavior are not uniformly defined;
- some features own server state in hooks while other paths bypass feature hooks;
- backend routes expose similar resources with inconsistent naming, verbs, response envelopes, and error shapes;
- device scope is sometimes encoded in path and sometimes in query/body parameters;
- mutations do not consistently define which authoritative queries become stale after success;
- bulk mutations and command mutations need to remain distinct from ordinary CRUD mutations;
- compatibility routes and legacy API shapes make the frontend contract harder to reason about.

P1.7 consolidates the API **contract and access layer**. It does not turn every endpoint into one generic CRUD abstraction and does not erase meaningful domain boundaries.

The governing rule is:

> One resource operation has one canonical request shape, one canonical response shape, one authorization boundary, and one frontend query/mutation owner.

## 2. Architectural invariants

P1.7 MUST preserve:

- P1.1 authentication, capability, and ownership semantics;
- P1.2 durable command lifecycle as the authoritative command state;
- P1.4 operational state as observed state, not optimistic mutation state;
- P1.5 backup/restore lifecycle and explicit validation-before-mutation semantics;
- P1.6 Journal/event history and immutable event semantics;
- `StationContext` as the frontend station/device identity boundary;
- React Query as the owner of server-backed state;
- `useDeviceStore` as transient/PWM-preference state only;
- `useDeviceSync` as transport/orchestration only;
- backend authorization as authoritative for every protected operation.

P1.7 MUST NOT introduce a second server-state cache, second authorization system, second command aggregate, or second API client transport.

## 3. Scope

P1.7 covers:

- canonical frontend API access primitives;
- domain-specific query hooks;
- domain-specific mutation hooks;
- canonical query-key factories;
- query parameter serialization and validation;
- canonical success response shapes;
- canonical API error envelope;
- HTTP status semantics where currently inconsistent;
- device/station scope conventions;
- bulk query/mutation semantics;
- mutation invalidation/refetch rules;
- optimistic-update restrictions;
- route alias/deprecation strategy;
- backend handler/service boundary cleanup;
- removal of duplicate request/response conversion logic;
- frontend migration away from direct generic API calls;
- contract tests between backend responses and frontend consumers;
- bounded compatibility period for legacy routes/shapes.

## 4. Explicit non-scope

P1.7 does **not** implement or redesign:

- P1.1 authorization model;
- P1.2 command lifecycle/state machine;
- P1.3 new safety semantics;
- P1.4 operational-state model;
- P1.5 backup/restore data model;
- P1.6 Journal/event persistence model;
- P1.8 route/navigation redesign;
- P1.9 shared/generated schema project;
- P1.10 observability redesign;
- new authentication mechanisms;
- GraphQL or a new RPC framework;
- replacing REST merely for stylistic reasons;
- generic CRUD generation for domain operations that have materially different semantics;
- replacing React Query with another state-management library;
- physical-device protocol redesign;
- telemetry storage/query-engine redesign;
- database schema cleanup unrelated to API contract consolidation.

P1.7 may modify an existing route only when required to establish a canonical contract. Destructive removal of an externally used route requires the compatibility rules in this specification.

## 5. Current API/data-access surfaces

The implementation audit MUST start from the existing API surface, including at minimum:

### Backend

- `hydragrow-backend/src/main.rs`
- `hydragrow-backend/src/api/middleware/auth.rs`
- `hydragrow-backend/src/api/control.rs`
- `hydragrow-backend/src/api/config.rs`
- `hydragrow-backend/src/api/config_backup.rs`
- `hydragrow-backend/src/api/device_admin.rs`
- `hydragrow-backend/src/api/device_pairing.rs`
- `hydragrow-backend/src/api/sensor.rs`
- `hydragrow-backend/src/api/analytics.rs`
- `hydragrow-backend/src/api/alert.rs`
- `hydragrow-backend/src/api/health_topics.rs`
- `hydragrow-backend/src/api/recipe.rs`
- `hydragrow-backend/src/api/script.rs`
- `hydragrow-backend/src/api/crop_season.rs`
- `hydragrow-backend/src/api/crop_season_photo.rs`
- `hydragrow-backend/src/api/calibration.rs`
- `hydragrow-backend/src/api/fleet.rs`
- `hydragrow-backend/src/api/admin_users.rs`
- `hydragrow-backend/src/api/webhook.rs`
- `hydragrow-backend/src/api/webhook_tokens.rs`

### Frontend

- `hydragrow-frontend/src/lib/apiClient.ts`
- `hydragrow-frontend/src/platform/http.ts`
- `hydragrow-frontend/src/contexts/StationContext.tsx`
- all `useQuery`, `useInfiniteQuery`, `useMutation`, and direct `apiClient` consumers;
- page/component code that directly constructs device-scoped URLs;
- `useDeviceSync.ts` and other transport/orchestration paths that interact with server state.

The audit MUST produce an inventory before implementation changes are made.

## 6. Canonical operation taxonomy

Every API operation MUST be classified as one of:

```text
Query
Mutation
Command
Subscription/stream
Webhook/external ingress
Admin/internal operation
```

### 6.1 Query

A query reads server state and has no externally visible mutation side effect.

Examples:

- list devices;
- get device config;
- get current operational state;
- get sensor history;
- get Journal events;
- get crop seasons;
- get command lifecycle history.

Queries MUST be safe to retry at the transport layer unless the endpoint explicitly documents otherwise.

### 6.2 Mutation

A mutation changes durable server state or requests a durable state transition.

Examples:

- update configuration;
- create/update/delete recipe;
- create/end crop season;
- acknowledge Journal event;
- restore configuration;
- rename device;
- claim/unclaim device.

A mutation MUST have an explicit success contract and an explicit invalidation policy.

### 6.3 Command

A physical-device command is not treated as ordinary CRUD.

Examples:

- manual control;
- reboot;
- OTA;
- emergency control;
- device synchronization request.

Command initiation returns the P1.2 durable command identity/lifecycle result. A successful HTTP response MUST NOT be interpreted as physical actuation success or runtime confirmation.

### 6.4 Stream

WebSocket/MQTT-derived updates remain transport/subscription concerns. They MUST NOT become an alternative REST query cache or bypass the authoritative query/mutation ownership model.

## 7. Canonical resource scope

The backend MUST use explicit resource scope.

For device-scoped operations, the canonical REST form is:

```text
/api/devices/{device_id}/...
```

For fleet/user-scoped operations:

```text
/api/devices
/api/fleet/...
/api/admin/...
```

A device ID supplied in a request body or query parameter MUST NOT override the authoritative path/resource scope.

If an operation targets multiple devices, the canonical request MUST explicitly declare the target set and MUST run the P1.1 all-target authorization check before mutation.

A route MUST NOT silently switch between device scope and fleet scope based on whether `device_id` happens to be present.

## 8. Canonical frontend API client

`hydragrow-frontend/src/lib/apiClient.ts` remains the single low-level HTTP transport boundary during P1.7.

It MUST be reduced to transport concerns:

- resolve backend URL/settings;
- attach authentication headers;
- execute HTTP requests;
- apply timeout/abort behavior;
- parse the canonical API error envelope;
- return typed raw JSON/data;
- preserve HTTP status and request correlation information where available.

It MUST NOT contain feature-specific business rules, query-key logic, cache invalidation, optimistic state mutation, or device selection logic.

The canonical low-level surface MAY remain method-based:

```text
apiGet
apiPost
apiPut
apiPatch
apiDelete
```

but feature code MUST consume domain hooks/functions rather than repeatedly constructing raw URLs and bodies.

## 9. Domain API modules

Each domain with meaningful server state MUST expose a small typed API module and React Query hooks.

Recommended structure:

```text
src/api/
  devices.ts
  control.ts
  config.ts
  telemetry.ts
  journal.ts
  recipes.ts
  scripts.ts
  seasons.ts
  backups.ts
  calibration.ts
  analytics.ts
  fleet.ts
```

Exact filenames MAY differ, but ownership MUST remain one-domain-per-module.

A domain module owns:

- request/response types;
- endpoint construction;
- query parameter encoding;
- mutation payload construction;
- response normalization required by the public frontend contract.

A domain module MUST NOT own React component state.

## 10. Canonical query hooks

Every server-backed query used by the UI MUST have a canonical query owner.

Example:

```typescript
useDeviceConfig(deviceId)
useDeviceTelemetry(deviceId, range)
useJournalEvents(deviceId, filters)
useRecipes()
useCropSeason(deviceId)
```

The hook MUST own:

- query key;
- query function;
- enabled condition;
- stale/refetch policy;
- request parameter construction;
- response typing;
- error typing;
- pagination behavior where applicable.

Pages/components MUST NOT duplicate these concerns.

## 11. Canonical query keys

Query keys MUST be deterministic, serializable, and include every server-side scope/filter that changes the result.

Canonical examples:

```text
['devices']
['device', deviceId, 'config']
['device', deviceId, 'telemetry', range]
['device', deviceId, 'journal', filters]
['device', deviceId, 'command-lifecycles', filters]
['seasons', deviceId]
```

Rules:

1. Every device-scoped query includes the canonical `selectedDeviceId`/device ID.
2. Station scope MUST NOT be hidden outside the key when it changes the result set.
3. Filter objects MUST have deterministic serialization.
4. Unrelated resources MUST NOT share a key prefix that permits accidental invalidation.
5. Query keys MUST NOT contain secrets, tokens, credentials, or arbitrary unbounded payloads.
6. Switching Device A -> Device B MUST produce a different key for every device-dependent query.

## 12. Canonical mutation hooks

Every UI mutation MUST have a canonical mutation owner.

Examples:

```typescript
useUpdateDeviceConfig(deviceId)
useCreateRecipe()
useApplyRecipe(deviceId)
useResolveJournalEvent(deviceId)
useRestoreConfig(deviceId)
useRenameDevice(deviceId)
```

Mutation hooks MUST own:

- request function;
- payload typing;
- success/error typing;
- invalidation/refetch policy;
- concurrency behavior where relevant;
- optimistic-update policy where allowed.

A page MUST NOT manually call `apiPost` and then independently guess which queries to refresh.

## 13. Mutation invalidation contract

Every mutation MUST declare one of these outcomes:

```text
no cached query affected
invalidate specific query keys
invalidate resource subtree
set authoritative returned data
await server confirmation before cache update
```

Examples:

```text
rename device
  -> invalidate ['devices']

update config
  -> update/invalidate ['device', id, 'config']
  -> invalidate affected operational/config status only if contract requires it

resolve Journal event
  -> invalidate affected Journal query family

create/end season
  -> invalidate ['seasons', deviceId]

apply recipe
  -> command/durable lifecycle result
  -> do not optimistically mark runtime recipe as applied
  -> invalidate authoritative recipe/device state after confirmed observation where required
```

A mutation response MUST NOT be used to fabricate an operational state that belongs to P1.4.

## 14. Optimistic update policy

Optimistic updates are prohibited for safety-critical or physically observable state unless the API contract explicitly identifies the optimistic field as a local intent rather than authoritative state.

At minimum, no optimistic update may claim:

- pump/actuator is physically ON/OFF;
- device is READY/ONLINE;
- command is CONFIRMED;
- controller has accepted a physical action;
- synchronization has completed;
- configuration is physically applied.

Safe optimistic updates MAY be used for non-authoritative UI-only state such as local selection or presentation preferences, subject to existing architecture boundaries.

## 15. Canonical response envelope

Existing endpoints MAY return domain-specific data directly, but P1.7 MUST eliminate accidental variation within the same operation class.

For successful resource/query operations, the canonical logical contract is:

```json
{
  "data": <typed resource or collection>,
  "meta": {
    "request_id": "optional bounded identifier",
    "next_cursor": "optional cursor"
  }
}
```

For compatibility-sensitive existing endpoints, a direct legacy payload MAY remain temporarily. The frontend domain adapter MUST normalize it into the canonical internal type until the route is migrated.

Mutation success MUST communicate the durable result of the operation. It MUST NOT imply downstream physical confirmation unless that confirmation is explicitly part of the endpoint's contract.

For paginated queries:

```json
{
  "data": [...],
  "meta": {
    "next_cursor": "..."
  }
}
```

Cursor semantics MUST remain deterministic and follow the P1.6 Journal contract where Journal data is involved.

## 16. Canonical error envelope

All JSON API failures SHOULD converge on:

```json
{
  "error": {
    "code": "stable_machine_code",
    "message": "safe human-readable message",
    "details": {},
    "request_id": "optional bounded identifier"
  }
}
```

Rules:

- `code` is stable and machine-readable;
- `message` contains no secret/internal stack trace;
- `details` is bounded and endpoint-specific;
- authentication/authorization failures MUST preserve P1.1 semantics;
- validation failures MUST be distinguishable from dependency failures;
- timeout/dependency failures MUST not become successful empty results;
- command transport failure MUST remain distinct from command lifecycle state;
- internal database errors MUST not expose SQL or credentials.

Canonical status mapping SHOULD be:

```text
400 invalid request / validation
401 unauthenticated
403 authenticated but not authorized
404 authorized resource not found
409 state/concurrency/conflict
422 semantically invalid operation where used
429 rate limited
500 unexpected server failure
502/503 downstream or dependency unavailable
504 downstream timeout
```

The implementation MUST preserve any existing documented status semantics where changing them would break a known consumer; such exceptions require explicit compatibility notes.

## 17. Error normalization in frontend

The low-level client MUST throw one typed error representation, for example:

```typescript
ApiError {
  status: number;
  code?: string;
  message: string;
  details?: unknown;
  requestId?: string;
}
```

Hooks MUST expose this error without forcing every page to parse HTTP responses independently.

UI behavior MUST be based on stable error codes/statuses rather than string matching against backend prose.

## 18. Device/station context integration

`StationContext` remains the source of truth for selected device identity in the frontend.

Rules:

1. A device-dependent hook consumes the selected device ID explicitly.
2. The hook's query key includes that ID.
3. A missing/invalid selection disables the device-dependent query rather than substituting a first/default device.
4. A query failure remains an error/unavailable state, not an empty device list.
5. Device A data MUST NOT be rendered through Device B's query key.
6. Fleet-scoped queries MUST NOT accidentally inherit the selected device as a hidden filter.

## 19. Bulk operations

Bulk APIs MUST use explicit target lists and all-or-nothing authorization unless a route explicitly documents another behavior.

Canonical shape:

```json
{
  "device_ids": ["device-a", "device-b"],
  "operation": "...",
  "payload": {}
}
```

The backend MUST:

1. authenticate the principal;
2. validate capability;
3. validate the complete target list;
4. authorize every target under P1.1;
5. validate the operation payload;
6. execute according to the domain transaction/lifecycle contract;
7. return per-target outcomes only when partial execution is an intentional domain feature.

A bulk request MUST NOT silently discard unauthorized device IDs and continue with the remainder.

## 20. Commands versus mutations

P1.7 MUST explicitly separate these frontend APIs:

```text
mutation: change backend/domain state
command: request physical/controller action
```

For commands:

```text
UI action
  -> command mutation hook
  -> backend command endpoint
  -> durable command_id/lifecycle result
  -> React Query invalidation/refetch
  -> authoritative runtime observation
```

The command hook MUST NOT directly modify `useDeviceStore` to represent successful physical state.

`useDeviceSync` MAY transport runtime observations but MUST NOT become a hidden mutation client for ordinary CRUD operations.

## 21. Query/mutation ownership matrix

The implementation audit MUST classify every current frontend server interaction into one owner.

Minimum target mapping:

| Domain | Query owner | Mutation owner | Authoritative result |
|---|---|---|---|
| Devices | device query hooks | pairing/rename hooks | backend device records |
| Config | config hooks | config mutation hooks | backend config + P1.4 observation where applicable |
| Telemetry | telemetry hooks | none/command-specific | P1.4/telemetry source |
| Control | control query hooks | command hooks | P1.2 lifecycle + P1.4 runtime observation |
| Journal | Journal hooks | Journal resolution hook | P1.6 event/resolution model |
| Recipes | recipe hooks | recipe mutation/command hooks | backend recipe + runtime confirmation where required |
| Scripts | script hooks | script mutation hooks | backend script state |
| Seasons | season hooks | season mutation hooks | backend season records |
| Backup | backup hooks/mutations | backup/restore hooks | P1.5 lifecycle |
| Calibration | calibration hooks | calibration mutation/command hooks | calibration state/result |
| Analytics | analytics hooks | only domain-defined mutations | backend analytical query |
| Fleet | fleet query hooks | fleet-scoped mutations only | backend fleet records |

The exact mapping MUST be verified against the actual route inventory before implementation.

## 22. Backend service boundary

P1.7 MUST separate three concerns where currently mixed:

```text
HTTP handler
  -> request validation / auth extraction
  -> domain service
  -> persistence / external dependency
```

Handlers SHOULD NOT duplicate:

- ownership queries;
- response serialization;
- error-string construction;
- transaction policy;
- query filtering logic;
- domain state-transition rules.

P1.7 does not require a new service for every handler. Consolidation is required where duplicated behavior currently creates contract drift.

## 23. Authorization preservation

Every canonicalized route MUST retain the existing P1.1 authorization decision.

Consolidation MUST NOT:

- move ownership checks to the frontend;
- infer ownership from selected station context;
- authorize a request because the resource exists;
- trust `device_id` from body/query over route context;
- widen service-key permissions as a shortcut;
- turn a previously device-scoped route into a fleet route without explicit authorization review.

For each migrated route, the implementation MUST record:

```text
route
capability
resource scope
ownership rule
bulk rule
legacy aliases
```

## 24. Compatibility and route migration

P1.7 MUST prefer additive migration.

Migration sequence:

```text
existing route
   -> canonical contract introduced
   -> frontend migrated
   -> compatibility adapter/alias retained
   -> tests prove canonical + legacy behavior
   -> deprecation evidence recorded
   -> legacy removal only after consumer audit
```

A legacy endpoint MUST NOT be removed merely because a canonical equivalent exists.

Compatibility aliases MUST delegate to the same domain service/authorization path rather than duplicate business logic.

No compatibility alias may bypass P1.1, P1.3, P1.4, P1.5, or P1.6 semantics.

## 25. URL and verb normalization

Where multiple routes express the same resource operation, establish one canonical route convention.

Preferred rules:

```text
GET    /resource              list/read collection
POST   /resource              create resource or explicit action
GET    /resource/{id}         read one resource
PUT    /resource/{id}         replace/update defined resource representation
PATCH  /resource/{id}         partial update
DELETE /resource/{id}         delete resource
POST   /resource/{id}/action  explicit domain action/command
```

Domain actions MUST remain explicit when they have side effects or lifecycle semantics that ordinary CRUD would hide.

Examples:

```text
POST /devices/{id}/control
POST /devices/{id}/reboot
POST /devices/{id}/factory-reset
POST /devices/{id}/sync
POST /devices/{id}/restore
```

These MUST NOT be disguised as generic `PUT` operations merely to reduce endpoint count.

## 26. Query parameter contract

Query parameters MUST be:

- typed;
- allowlisted where they affect query shape;
- bounded by explicit limits;
- safely encoded;
- deterministic;
- scoped to the endpoint's resource.

Unsupported parameters MUST be rejected or explicitly ignored according to the documented endpoint contract. Silent query broadening is prohibited.

Pagination MUST use one documented strategy per resource family. Journal retains its P1.6 cursor semantics.

Time ranges MUST have explicit units and maximum bounds.

Search/filter parameters MUST NOT be concatenated into raw SQL or Flux expressions without validation/parameterization.

## 27. Query caching and freshness

P1.7 MUST document default freshness behavior by resource class.

Minimum categories:

```text
static/reference data       -> long stale time
configuration               -> moderate stale time + mutation invalidation
current operational state   -> short freshness / explicit refetch policy
telemetry history           -> query-range dependent
Journal                     -> cursor/query dependent
command lifecycle           -> short freshness after command initiation
fleet/device list           -> moderate freshness
```

Exact durations MUST be chosen from observed application behavior rather than arbitrarily applied globally.

No cache policy may turn stale operational state into authoritative current state.

## 28. Request cancellation and concurrency

Queries MUST support cancellation where the underlying client supports `AbortSignal`.

When Device A -> Device B occurs:

- obsolete Device A requests SHOULD be cancelled where practical;
- Device A results MUST NOT populate Device B keys;
- in-flight mutation results MUST retain their original resource identity;
- a mutation for Device A MUST NOT invalidate or overwrite Device B's resource data.

Concurrent mutations MUST declare whether last-write-wins, conflict rejection, serialized execution, or durable lifecycle semantics apply.

P1.7 MUST NOT silently introduce client-side retries for non-idempotent mutations.

## 29. Retry policy

Automatic retries MUST be limited to operations safe to retry.

Default expectation:

```text
GET/query                 -> retry bounded transient failures
idempotent PUT/PATCH      -> retry only with explicit safety review
POST create               -> no blind automatic retry
physical command          -> no blind automatic retry
DELETE                    -> explicit endpoint review
```

A retry MUST NOT duplicate a physical command or durable mutation.

## 30. Generated/shared types boundary

P1.7 may define TypeScript/Rust types needed to stabilize the API contract, but broad generated/shared schema alignment belongs to P1.9.

Until P1.9, duplicated frontend/backend types MAY exist where necessary, but their public field names and semantics MUST be documented and tested.

P1.7 MUST NOT introduce a large code-generation framework solely to avoid small amounts of type duplication.

## 31. Testing requirements

### 31.1 Backend contract tests

For every canonicalized operation class, test:

- success response shape;
- validation error shape;
- authentication failure;
- authorization failure;
- not-found behavior;
- conflict behavior where applicable;
- dependency failure;
- timeout behavior where applicable;
- bounded error details;
- request/resource scope preservation.

### 31.2 Authorization regression tests

For every migrated device-scoped route:

```text
authorized owner          -> success
same capability, wrong owner -> denied
missing capability         -> denied
missing authentication     -> denied
body/query device mismatch -> route scope wins / request rejected
bulk mixed ownership      -> whole operation denied unless explicitly partial
```

### 31.3 Frontend hook tests

- canonical query key contains device scope;
- filters produce deterministic keys;
- disabled query has no accidental request;
- query error is surfaced as `ApiError`;
- mutation uses the canonical domain endpoint;
- successful mutation invalidates the declared queries;
- mutation failure does not invalidate unrelated state;
- Device A mutation cannot update Device B cache;
- no direct page-level generic API call remains for migrated domains.

### 31.4 Integration tests

At minimum:

1. load fleet/device list;
2. select Device A;
3. query A config/telemetry/Journal;
4. switch to Device B;
5. verify B queries use B keys and no A data leaks;
6. mutate B config;
7. verify only required B queries are invalidated/refetched;
8. issue a physical command;
9. verify command lifecycle is returned rather than fabricated actuator success;
10. verify an unauthorized cross-device request is rejected;
11. verify a legacy alias and canonical route resolve to the same authorization/domain service during compatibility period.

### 31.5 Negative-path semantics

Tests MUST distinguish:

```text
empty successful result
query failure
permission denied
resource not found
stale cached result
command accepted
command confirmed
```

No layer may collapse these into one generic `null`/`[]`/`false` result.

## 32. Acceptance criteria

- **AC-1:** Every frontend server-backed operation belongs to one canonical query, mutation, command, or stream owner.
- **AC-2:** `apiClient.ts` remains a single low-level transport boundary and contains no feature-specific cache/business logic.
- **AC-3:** Device-scoped queries include device identity in their React Query key and never silently substitute another device.
- **AC-4:** Canonical domain hooks own request construction, typing, and query/mutation behavior; migrated pages/components do not duplicate raw API calls.
- **AC-5:** Every mutation has an explicit cache invalidation or authoritative-response policy.
- **AC-6:** Optimistic updates cannot fabricate physical actuator state, operational readiness, command confirmation, or synchronization completion.
- **AC-7:** Query and mutation failures use a typed canonical frontend error representation.
- **AC-8:** Backend JSON errors converge on a stable machine-readable error contract without leaking secrets or internal implementation details.
- **AC-9:** Device resource scope is explicit and authoritative; body/query device IDs cannot bypass route ownership.
- **AC-10:** Bulk operations validate and authorize the complete target set before mutation unless an explicitly documented partial-operation contract exists.
- **AC-11:** P1.2 command operations remain distinct from ordinary CRUD mutations and return durable lifecycle identity rather than physical success by implication.
- **AC-12:** P1.4 operational-state authority is preserved; mutation success never becomes authoritative runtime state by itself.
- **AC-13:** P1.6 Journal query/resolution semantics remain intact, including deterministic cursor behavior and resolution distinction.
- **AC-14:** Compatibility aliases, when retained, delegate to the same canonical authorization/domain service and do not create duplicate business logic.
- **AC-15:** Non-idempotent mutations and physical commands are not blindly retried by the client.
- **AC-16:** Device A -> Device B switching cannot leak A query results into B state or invalidate B state from an A mutation.
- **AC-17:** Backend route inventory and frontend call-site inventory show no unexplained duplicate canonical operations after migration.
- **AC-18:** Integration tests demonstrate canonical query, mutation, command, authorization, cache invalidation, and compatibility behavior.
- **AC-19:** P1.7 does not pull P1.8 navigation redesign, P1.9 schema-generation/alignment, or P1.10 observability redesign into the phase.

## 33. Evidence requirements

Each acceptance criterion requiring executable verification MUST record:

```text
Requirement ID:
Surface:
Canonical operation:
Authorization scope:
Expected request:
Expected response/error:
Expected cache effect:
Actual result:
Environment:
Test/evidence reference:
```

The final P1.7 evidence artifact MUST include:

- route inventory;
- frontend API-call inventory;
- canonical mapping;
- removed/retained aliases;
- authorization regression results;
- query-key/invalidation tests;
- backend contract tests;
- frontend hook tests;
- integration results;
- known compatibility exceptions.

## 34. Implementation order

Implementation MUST proceed in this order:

1. inventory all backend routes and frontend API call sites;
2. classify every operation as Query/Mutation/Command/Stream/Admin/Ingress;
3. map each operation to its P1.1 capability and resource ownership rule;
4. identify duplicate or semantically divergent routes;
5. freeze canonical resource paths and operation names;
6. freeze canonical success/error contracts without prematurely introducing P1.9 code generation;
7. harden `apiClient.ts` as the sole transport boundary;
8. create domain API modules and canonical query-key factories;
9. migrate high-use query hooks first;
10. migrate mutations with explicit invalidation policies;
11. migrate command APIs separately from CRUD mutations;
12. add compatibility adapters/aliases for retained legacy routes;
13. remove direct page/component API calls for migrated domains;
14. add authorization, cache-isolation, retry, and negative-path tests;
15. run backend/frontend integration verification;
16. produce route/call-site/evidence inventory and record unresolved compatibility exceptions.

## 35. Definition of done

P1.7 is complete when:

```text
backend route inventory complete
        +
canonical operation taxonomy complete
        +
frontend API call inventory complete
        +
canonical domain hooks established
        +
query keys deterministic and scoped
        +
mutation invalidation explicit
        +
error contract normalized
        +
authorization preserved
        +
command semantics preserved
        +
legacy compatibility documented
        +
cross-device cache isolation tested
        +
negative paths tested
        +
P1.7 evidence recorded
```

No route is considered consolidated merely because two URLs were renamed. Consolidation is complete only when request semantics, authorization, response/error behavior, frontend ownership, cache behavior, and lifecycle semantics are aligned.

## 36. Non-goals / phase boundary

P1.7 explicitly leaves these to later phases:

```text
P1.8 Route / Navigation Contract
P1.9 Shared Schema Alignment
P1.10 Observability + Cross-system Sync
```

P1.7 is the API contract/access-layer consolidation phase. It must make the existing API predictable without turning the phase into a new product architecture.

## 37. Implementation delta / migration policy

The implementation has now established the canonical access layer and completed the remaining low-risk frontend direct-call migration identified by the route/call-site audit. The following concrete rule is added to remove ambiguity in AC-4/AC-17:

1. Production UI code MUST NOT import or call `apiGet`, `apiPost`, `apiPut`, `apiPatch`, or `apiDelete` directly outside `src/api/*` and the transport's own tests.
2. Domain API modules MAY use the low-level client; React Query hooks remain the owner for server-backed UI state and mutation invalidation where the operation is represented as React Query state.
3. Explicit physical-device commands remain domain commands even when implemented through `apiPost`; they MUST retain command-specific confirmation/timeout semantics and MUST NOT be modeled as optimistic CRUD state.
4. External third-party uploads (for example Cloudinary) are not HydraGrow API calls and remain outside the single HydraGrow transport boundary.
5. Admin-only UI operations may use a typed admin domain API when their state is deliberately local/form-oriented; they MUST NOT bypass backend authorization or create a second transport.
6. Any remaining direct backend client call discovered after this migration is a P1.7 defect, not an accepted compatibility exception, unless it is inside `src/api/*` or explicitly documented as external ingress/transport.

This delta supersedes the earlier inventory note that treated the 29 production direct callers as an accepted incremental endpoint. The migration target is now zero direct HydraGrow generic-client calls outside canonical domain API modules.
