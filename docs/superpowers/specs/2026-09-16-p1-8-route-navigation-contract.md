# P1.8 Route / Navigation Contract

**Status:** SPEC DRAFT — NOT IMPLEMENTED
**Phase:** P1.8
**Depends on:** P1.0 Verification / Cross-System Baseline, P1.1 Authorization / Ownership Boundary, P1.2 Durable Command Lifecycle, P1.3 Safety / Failure Semantics, P1.4 Operations State Boundary, P1.5 Backup / Restore Lifecycle, P1.6 Journal / Event Model, P1.7 API Query / Mutation Consolidation
**Scope:** frontend URL namespace, route taxonomy, navigation ownership, auth/capability gates, station/device context, deep-link semantics, redirects, URL state, browser history, and compatibility of existing routes

## 1. Purpose

Establish one deterministic, testable Route / Navigation contract for HydraGrow.

P1.8 addresses fragmentation that remains after P1.0–P1.7:

- route definitions live centrally in `App.tsx`, while navigation targets are repeated as string literals throughout pages/components;
- primary navigation, utility routes, legacy aliases, and page-level redirects are not represented by one explicit route contract;
- authentication exists globally, but route-level capability/role behavior is not expressed as a reusable contract;
- station/device identity is owned by `StationContext`, while URLs currently do not have a documented relationship to that context;
- legacy deep links redirect to merged pages, but alias, query, hash, and browser-history behavior is not formally defined;
- some pages are mounted through routes even when their old standalone URLs are now aliases;
- invalid, unauthorized, or missing station/device context needs deterministic recovery rather than ad-hoc page behavior;
- route entry, in-app navigation, refresh, back/forward, and notification/bookmark deep links must resolve to the same authorization and context rules.

The governing rule is:

> Every navigable destination has one canonical route identity, one declared access contract, one resource scope, one URL-state contract, and one deterministic entry/recovery behavior.

P1.8 is a **frontend navigation contract**. It does not create a second authorization system and does not replace backend authorization.

## 2. Architectural invariants

P1.8 MUST preserve:

- P1.1 authentication, capability, and ownership semantics;
- backend authorization as the authoritative security boundary;
- P1.2 command lifecycle as authoritative for command state;
- P1.3/P1.4 operational truth, including unknown/error/stale states;
- P1.5 backup/restore staged validation and mutation semantics;
- P1.6 Journal identity, filtering, cursor, and station/device scope semantics;
- P1.7 canonical API/resource paths and query/mutation/command taxonomy;
- `StationContext` as the frontend station/device identity boundary;
- React Query as the owner of server-backed state;
- browser history semantics provided by React Router;
- direct deep links behaving consistently with in-app navigation.

P1.8 MUST NOT introduce:

- a client-only authorization system that can grant access;
- a second station/device identity store;
- route-specific copies of server state;
- route-driven mutation of operational truth;
- a second router or competing route namespace;
- hidden URL aliases that cannot be inventoried and tested.

## 3. Scope

P1.8 covers:

- canonical route namespace and route taxonomy;
- route manifest/registry as the single navigation metadata owner;
- public/authenticated/role-gated/utility/legacy/not-found route classes;
- authentication and capability entry behavior;
- station/device resource scope declarations;
- URL path parameters and search parameters;
- deterministic URL parse/serialize/canonicalization rules;
- station/device context preservation and transition rules;
- deep links from bookmarks, browser refresh, notification links, and external entry;
- legacy path compatibility and redirect semantics;
- 404 and invalid URL-state behavior;
- missing/invalid/unavailable station recovery;
- navigation history, replace vs push semantics;
- active navigation state and canonical link targets;
- route-level tests and navigation evidence.

## 4. Explicit non-scope

P1.8 does **not** implement or redesign:

- P1.1 authorization policy or backend permission enforcement;
- P1.2 command lifecycle/state machine;
- P1.3 safety decision semantics;
- P1.4 operational-state model;
- P1.5 backup/restore artifact/data model;
- P1.6 Journal persistence/query model;
- P1.7 API transport, query/mutation architecture, or API resource semantics;
- P1.9 shared/generated schema project;
- P1.10 observability redesign;
- authentication provider replacement;
- backend route redesign unless a separate compatibility defect is discovered and explicitly handed to the owning phase;
- visual redesign of pages/navigation unrelated to route semantics;
- browser history implementation outside React Router's normal contract;
- native mobile deep-link infrastructure unless required to preserve the web route contract.

P1.8 MAY introduce frontend route metadata and route helpers. It MUST NOT silently change an API endpoint merely to make a URL convenient.

## 5. Current route baseline

The current frontend uses React Router with `BrowserRouter` and a `MainLayout` route tree.

Current primary destinations are:

```text
/dashboard
/operations
/cultivation
/journal
/settings
```

Current utility/secondary destinations include:

```text
/pairing
/fleet
/config-backup
/user-management
/roles
```

Current legacy aliases redirect into merged destinations:

```text
/control          -> /operations
/automation      -> /operations
/seasons         -> /cultivation
/crop-seasons    -> /cultivation
/recipes         -> /cultivation
/dosing-history  -> /cultivation
/logs            -> /journal
/analytics       -> /journal
```

`/` currently redirects to `/dashboard`.

This baseline is an input to implementation, not an implicit permanent contract. P1.8 freezes the intended canonical namespace through an explicit route manifest and compatibility inventory.

The audit also records the following current gaps to prevent accidental omission during implementation:

- `App.tsx` has no explicit catch-all route, so an unknown authenticated path does not currently have a deterministic 404 contract;
- the current Vercel configuration rewrites API paths but does not document an SPA fallback for browser deep links;
- `MainLayout` uses exact pathname comparison for active state, so future nested canonical routes require a router-derived active-state policy;
- merged page tabs currently use local state, so legacy aliases such as `/automation`, `/recipes`, and `/analytics` cannot preserve a requested sub-view;
- `/user-management` currently mounts `Roles`, while a separate `UserManagement` page component exists, so route ownership must be made explicit rather than inferred from filenames;
- `FleetView` uses `navigate(-1)`, which has different behavior for an in-app entry versus a direct deep link and therefore requires an explicit fallback policy;
- the current `mock_auth=true` query/local-storage bypass is a test/development concern and MUST NOT become part of the production route contract.

## 6. Canonical route taxonomy

Every frontend route MUST belong to exactly one route class.

```text
Public
Authenticated
Role/capability-gated
Utility
Legacy alias
Not-found / recovery
```

A route MAY have both `Authenticated` and `Role/capability-gated` properties, but its manifest entry MUST make the effective access requirement explicit.

### 6.1 Public

Public routes are reachable without an authenticated Firebase user.

Examples include future dedicated authentication URLs if introduced.

Current login/register/forgot-password UI is rendered by `AuthGate` rather than being represented as normal application routes. P1.8 MUST either:

1. keep this behavior and explicitly declare authentication views as non-route application states; or
2. migrate them to canonical public routes.

The implementation MUST choose one model and test it. Both models MUST NOT coexist ambiguously.

### 6.2 Authenticated

Application routes require an authenticated principal.

Unauthenticated entry MUST resolve deterministically to the authentication entry state. After authentication, the implementation SHOULD preserve a safe intended destination when one exists.

The intended destination MUST:

- be same-origin/internal;
- be a canonical application route or an approved legacy alias;
- not bypass authorization or context checks;
- not contain sensitive credentials/tokens.

### 6.3 Role/capability-gated

A route MAY declare a required capability or role for UX gating.

Client-side gating is only presentation/navigation behavior. The backend remains authoritative and MUST continue to return the appropriate authorization error when access is denied.

The frontend MUST distinguish:

```text
unauthenticated -> authentication entry
authenticated but unauthorized -> explicit denied state
authenticated and authorized -> route content
```

A `403` MUST NOT be silently converted into a redirect to an unrelated page solely to hide authorization failure.

### 6.4 Utility

Utility routes are authenticated destinations that are not part of the five primary navigation tabs.

Examples:

```text
/pairing
/fleet
/config-backup
/roles
```

Utility routes MUST remain reachable by direct URL and MUST declare their access/context requirements in the route manifest.

### 6.5 Legacy alias

A legacy alias exists only for compatibility.

It MUST:

- redirect to exactly one canonical destination;
- use `replace` unless preserving the old URL in browser history is explicitly required;
- have a documented deprecation reason;
- define query/hash preservation behavior;
- not duplicate page/business logic;
- be included in the compatibility inventory and tests.

### 6.6 Not-found / recovery

Unknown paths MUST resolve to a deterministic not-found/recovery experience.

Malformed or unsupported URL state MUST NOT silently load an unrelated resource.

## 7. Canonical route manifest

P1.8 SHOULD introduce a typed route manifest/registry as the source of truth for navigation metadata.

A conceptual entry is:

```ts
type RouteDefinition = {
  id: string;
  path: string;
  class: 'public' | 'authenticated' | 'utility' | 'legacy' | 'recovery';
  capability?: string;
  scope: 'global' | 'station' | 'device' | 'admin';
  canonicalPath?: string;
  preserveSearch?: boolean;
  preserveHash?: boolean;
  entryPolicy: 'push' | 'replace' | 'redirect' | 'render';
};
```

The exact TypeScript shape MAY differ, but every route contract MUST express the same information.

Navigation components MUST consume route identities/helpers rather than scattering raw route strings where practical.

A route ID MUST remain stable even if its UI label changes.

## 8. Canonical route namespace

Canonical application routes MUST use lowercase, stable, human-readable path segments.

Path segments MUST NOT encode transient UI labels.

The canonical namespace MUST avoid creating two canonical paths for the same destination.

The implementation MUST explicitly decide whether resource identity is encoded in the path or remains in `StationContext`.

For the current HydraGrow architecture, device/station identity is primarily owned by `StationContext`. Therefore P1.8 MUST NOT introduce a URL device ID as a second authoritative identity without a concrete route requirement and an authorization-safe synchronization contract.

If a future route requires a device path parameter, that parameter MUST be validated against authorized devices before the page consumes device-scoped server state.

## 9. Authentication and authorization entry contract

Route entry MUST be evaluated in this order:

```text
1. Parse route + URL state
2. Determine authentication state
3. Determine required capability/scope
4. Resolve station/device context if required
5. Render route or deterministic recovery/denied state
```

No page component may assume that being mounted implies authorization.

The backend remains the source of truth for authorization. A frontend route guard MUST never be treated as a security boundary.

### 9.1 Unauthenticated entry

For an authenticated route entered directly:

- the user sees the authentication entry state;
- the original safe destination MAY be retained;
- authentication completion MUST return to the canonical destination only after normal route/context checks.

### 9.2 Authenticated but unauthorized entry

If the user is authenticated but lacks the required capability/scope:

- render a deterministic access-denied state or equivalent approved UX;
- preserve the requested URL for diagnostics where safe;
- do not loop between routes;
- do not claim the resource is missing when the actual condition is authorization denial.

### 9.3 Backend denial after route entry

A route that loads server state and receives `401/403` MUST use the P1.7 canonical error semantics.

The route MUST NOT infer that `403` means the device is offline, missing, deleted, or invalid unless the API contract explicitly says so.

## 10. Station / device context contract

`StationContext` is the authoritative frontend identity boundary for the currently selected station/device.

P1.8 MUST define for every route whether it is:

```text
global
station-scoped
device-scoped
admin-scoped
```

### 10.1 Global routes

Global routes do not require a selected device to render their route shell.

Example:

```text
/fleet
/pairing
```

Whether their child operations require a device is defined separately by the page contract.

### 10.2 Station/device-scoped routes

A device-scoped route MUST NOT render device-specific server state using an arbitrary URL/body/query device ID that disagrees with `StationContext`.

If no valid selected device exists, the route MUST enter one deterministic recovery state:

```text
choose/select station/device
pair/configure station/device
or approved global fallback
```

The implementation MUST choose the appropriate recovery target per route and document it.

### 10.3 Station switch

Changing `selectedDeviceId` MUST:

- update the authoritative context;
- invalidate/refetch device-scoped server state according to P1.7;
- leave global route identity unchanged unless the route cannot exist without the previous device;
- never expose data from the previous device as if it belonged to the new device.

### 10.4 URL and station context precedence

If a route eventually supports a device URL parameter, precedence MUST be explicit:

1. route parameter is parsed and validated;
2. authorization against available/owned devices is established;
3. context is synchronized through `StationContext` using one defined policy;
4. server queries use the resulting authoritative device identity.

An arbitrary URL MUST NOT override ownership boundaries.

## 11. URL state contract

Only state that materially affects navigation/deep-link semantics SHOULD be encoded in the URL.

Examples:

- Journal filters that users expect to bookmark/share;
- selected sub-view/tab when the tab is a meaningful destination;
- bounded pagination/cursor state when explicitly supported;
- route resource identity when the destination itself depends on that identity.

Transient UI state MUST remain local unless the route contract explicitly requires persistence.

### 11.1 Search parameter rules

Search parameters MUST have:

- canonical names;
- documented type/domain;
- deterministic defaults;
- explicit invalid-value behavior;
- deterministic serialization order where string equality matters;
- bounded values for pagination/filter fields;
- no secrets, auth tokens, or raw credentials.

Unknown search parameters SHOULD be ignored for forward compatibility unless they could alter security/resource scope. Security-sensitive unknown parameters MUST NOT override scope.

### 11.2 Canonicalization

Equivalent URL states MUST canonicalize to one representation where practical.

Examples:

```text
missing default filter == explicit default filter
empty optional value == omitted value
legacy path == canonical path after redirect
```

Canonicalization MUST NOT discard meaningful user state.

### 11.3 Hash fragments

Hash fragments MUST have an explicit policy per route:

- supported and preserved;
- supported and normalized;
- or unsupported and removed during canonicalization.

A route MUST NOT accidentally depend on hash state without declaring it.

### 11.4 Tab/sub-view state

If a page contains meaningful tabs or sub-views, P1.8 MUST explicitly classify that state as either:

- URL-addressable route/search state; or
- intentionally local UI state.

If a legacy route represented a specific sub-view, its redirect MUST either preserve that semantic through the canonical URL state or explicitly document that the old deep link is intentionally collapsed. Silent loss of a meaningful destination is not acceptable for a supported compatibility alias.

For the current merged pages, implementation MUST make an explicit decision for at least:

```text
/operations       (control / automation sub-views)
/cultivation      (seasons / recipes / dosing-history sub-views)
/journal          (logs / analytics-related views where still applicable)
```

## 12. Deep-link contract

The same route URL MUST produce the same logical destination whether entered by:

- clicking a navigation control;
- typing/pasting the URL;
- browser refresh;
- browser back/forward;
- notification/email/bookmark link;
- legacy alias followed by redirect.

The implementation MUST test direct entry for all canonical routes.

A deep link MUST pass the same authentication, capability, and station/device checks as in-app navigation.

## 13. Navigation transition contract

Every navigation transition MUST have an intentional history policy.

### 13.1 Push

Use normal navigation (`push`) for user-selected destinations that should be recoverable through Back.

Examples:

```text
/dashboard -> /operations
/settings -> /pairing
```

### 13.2 Replace

Use `replace` for canonicalization and compatibility redirects where the old URL is not a meaningful history destination.

Examples:

```text
/ -> /dashboard
/automation -> /operations
/logs -> /journal
```

### 13.3 In-page URL state

Changes to search/hash state MUST define whether they push or replace history.

High-frequency filter typing SHOULD avoid creating an unusable Back-stack unless product behavior explicitly requires it.

### 13.4 Back/forward

Back/forward MUST restore the route and supported URL state without bypassing current authorization/context checks.

If the selected device is no longer authorized/available, the route MUST enter its deterministic recovery state rather than displaying stale data.

### 13.5 Back navigation from utility pages

`navigate(-1)` MUST NOT be the only recovery behavior for a utility page that can be entered directly. If browser history has no safe in-application destination, the page MUST use a deterministic fallback such as its canonical parent/global destination.

## 14. Primary navigation contract

`MainLayout` MUST expose exactly the canonical primary destinations defined by the route manifest.

Primary navigation MUST NOT point to legacy aliases.

Active state MUST be derived from the canonical route identity rather than independent string comparisons that can drift from the router.

Desktop and mobile navigation MUST use the same route definitions.

A route may be reachable without appearing in primary navigation. Such routes remain directly navigable through their canonical URL and approved contextual links.

## 15. Cross-page navigation contract

Pages/components SHOULD use route helpers/route IDs rather than duplicating raw destination strings.

Cross-page links MUST target canonical routes.

A link MUST NOT:

- bypass a required context selection;
- encode an unauthorized device identity;
- imply successful physical actuation merely because navigation occurred;
- turn a command lifecycle ID into a route identity unless the destination explicitly represents command lifecycle state.

Command completion, device health, and Journal resolution remain separate from route navigation.

## 16. Legacy compatibility contract

The compatibility inventory MUST include:

```text
legacy path
canonical target
redirect mode
query preservation
hash preservation
deprecation status
reason
owner/test
```

For the current baseline:

| Legacy path | Canonical target | Required behavior |
|---|---|---|
| `/` | `/dashboard` | replace redirect |
| `/control` | `/operations` | replace redirect |
| `/automation` | `/operations` | replace redirect |
| `/seasons` | `/cultivation` | replace redirect |
| `/crop-seasons` | `/cultivation` | replace redirect |
| `/recipes` | `/cultivation` | replace redirect |
| `/dosing-history` | `/cultivation` | replace redirect |
| `/logs` | `/journal` | replace redirect |
| `/analytics` | `/journal` | replace redirect |

The implementation MUST explicitly test whether search/hash state is preserved for each alias. It MUST NOT assume preservation merely because React Router performs a navigation.

Legacy aliases MUST NOT become a second supported navigation surface after P1.8.

### 16.1 Alias semantic preservation

An alias that historically represented a meaningful sub-view MUST be classified as one of:

```text
preserve-subview
collapse-to-parent
retired-with-explicit-break
```

`collapse-to-parent` is allowed only when the product intentionally merged the old destination and the compatibility inventory documents the semantic change. Tests MUST assert the chosen behavior.

## 17. Missing configuration and unavailable context

The current `MainLayout` contains a global missing-config recovery path that redirects users toward `/settings` behaviorally.

P1.8 MUST make this a route-aware policy:

- routes that genuinely require backend/device configuration may use `/settings` recovery;
- global routes that can operate without device configuration MUST remain reachable;
- recovery MUST NOT trap users in redirect loops;
- the route that caused recovery MUST remain diagnosable;
- a missing configuration condition MUST NOT be represented as an authorization denial.

Similarly, `StationContext` states such as `NoSelection`, `InvalidSelection`, `Unavailable`, and `PermissionDenied` MUST map to explicit route entry behavior where relevant.

## 18. 404 and invalid URL-state contract

Unknown route paths MUST produce a deterministic 404/recovery page.

Malformed URL state MUST produce one of:

```text
canonicalized valid state
safe default state
explicit invalid-state message
```

The implementation MUST NOT:

- throw an uncaught rendering exception;
- silently select another user's device/resource;
- silently convert malformed state into a privileged/default resource;
- issue an infinite redirect loop.

### 18.1 SPA hosting contract

For browser-based deployment, the hosting layer MUST serve the application entry document for canonical client-side routes so that refresh/direct deep links reach React Router.

The implementation MUST verify this for the production deployment configuration (currently Vercel configuration is part of the route audit).

An API rewrite MUST NOT be treated as an SPA fallback.

## 19. Route error semantics

Route-level errors MUST distinguish at least:

```text
Authentication required
Access denied
Missing/invalid context
Invalid URL state
Resource unavailable
Not found
Unexpected application error
```

These states MAY share UI primitives, but their semantic meaning MUST remain distinguishable.

A backend `404` for a resource and a router `404` for an unknown path MUST NOT be conflated.

## 20. Notifications and external links

Any future notification/deep-link producer that targets the frontend MUST reference a canonical route identity.

Notification links MUST NOT target deprecated aliases unless they are intentionally compatibility links.

Links carrying resource identifiers MUST undergo the same authorization and context validation as ordinary route entry.

External URLs such as Grafana/Cloudinary are not HydraGrow application routes and remain governed by their owning feature/API contracts.

## 21. Testing requirements

### 21.1 Route manifest tests

Test:

- every canonical route has a unique ID/path;
- no two canonical routes claim the same path;
- every alias resolves to exactly one canonical target;
- every route declares scope/access class;
- primary navigation references only canonical routes.

### 21.2 Authentication tests

Test:

- unauthenticated direct entry;
- authentication completion return destination;
- authenticated entry;
- authenticated but unauthorized route;
- backend `401`/`403` after route mount;
- no redirect loops.

### 21.3 Station/device tests

Test:

- route with valid selected device;
- no selected device;
- invalid selection;
- unavailable device;
- permission-denied device;
- station switch while remaining on a scoped route;
- cross-device isolation;
- URL/device identity mismatch if device URLs are introduced.

### 21.4 URL-state tests

Test:

- canonical parse/serialize;
- defaults;
- invalid values;
- unknown parameters;
- search preservation policy;
- hash preservation policy;
- canonicalization idempotence;
- bounded pagination/cursor state where applicable.

### 21.5 Deep-link/history tests

Test:

- direct URL entry for every canonical route;
- refresh;
- back/forward;
- push vs replace behavior;
- legacy alias redirect;
- query/hash behavior through redirect;
- recovery from invalid context.

### 21.6 404/recovery tests

Test:

- unknown path;
- malformed path parameters;
- malformed search state;
- missing configuration;
- missing station/device context;
- explicit denied state.

### 21.7 Hosting/deployment route tests

Test or otherwise verify that production hosting serves the SPA entry for each canonical browser route and does not confuse application routes with `/api/*` rewrites.

### 21.7 Cross-system tests

Where feasible, verify:

- route entry -> P1.7 API query uses the expected canonical resource scope;
- route context changes -> P1.7 invalidation/refetch behavior;
- Journal route state remains consistent with P1.6 filters/cursor semantics;
- command navigation does not imply P1.2 physical success;
- P1.1 backend denial remains distinguishable from router 404.

## 22. Security requirements

P1.8 MUST treat all URL state as untrusted input.

Specifically:

- URL parameters MUST NOT grant capabilities;
- URL device IDs MUST NOT bypass ownership checks;
- route guards MUST NOT be treated as security controls;
- sensitive tokens MUST NOT be placed in paths, search params, or hashes;
- redirect destinations MUST be constrained to safe internal canonical routes;
- legacy aliases MUST preserve the same authorization behavior as canonical destinations;
- stale browser history MUST not reveal data from a device the user no longer has access to.

## 23. Migration strategy

Implementation SHOULD proceed in this order:

1. inventory all current canonical, utility, legacy, and orphaned page destinations;
2. define typed route manifest and route IDs;
3. freeze canonical paths and alias targets;
4. define access/scope metadata for every route;
5. define URL-state parsers/serializers for routes that need them;
6. centralize navigation helpers and primary nav targets;
7. implement deterministic auth/capability/context entry behavior;
8. implement 404 and invalid-state recovery;
9. migrate page/component navigation literals to route helpers;
10. add deep-link/history/compatibility tests;
11. verify P1.1/P1.7 semantics remain unchanged;
12. publish route inventory/evidence.

Migration MUST be additive where possible. Existing bookmarks and notification links MUST continue to resolve through the compatibility inventory until their aliases are intentionally retired in a later phase.

## 24. Acceptance criteria

### AC-1 — Canonical route namespace

Every supported application destination has one canonical path and stable route ID.

### AC-2 — Complete route inventory

All current route definitions, utility pages, legacy aliases, orphaned page candidates, and external-link boundaries are inventoried.

### AC-3 — Explicit route taxonomy

Every route declares its public/authenticated/role-gated/utility/legacy/recovery classification and resource scope.

### AC-4 — Authentication entry consistency

Unauthenticated direct entry and in-app entry produce the same authentication behavior, with safe intended-destination preservation where implemented.

### AC-5 — Authorization consistency

Authenticated users lacking required capability receive an explicit denied state; frontend guards never replace backend authorization.

### AC-6 — Station/device boundary

Every scoped route declares how it consumes `StationContext`, handles missing/invalid/unavailable selection, and prevents cross-device leakage.

### AC-7 — URL-state contract

All supported path/search/hash state has documented grammar, defaults, bounds, parse/serialize behavior, and canonicalization rules.

### AC-8 — Deep-link equivalence

Direct URL, refresh, in-app navigation, notification/bookmark entry, and browser history resolve through the same route/access/context rules.

### AC-9 — Navigation history semantics

Push vs replace behavior is explicit and tested for canonical navigation, redirects, and URL-state changes.

### AC-10 — Primary navigation consistency

Desktop/mobile primary navigation references only canonical route identities and derives active state from the router contract.

### AC-11 — Legacy compatibility

Every legacy path redirects deterministically to one canonical target, with tested query/hash preservation behavior and no duplicate page logic.

### AC-12 — 404/recovery behavior

Unknown paths and invalid URL state produce deterministic recovery without crashes, privilege escalation, or redirect loops.

### AC-12a — SPA deep-link hosting

Production browser hosting serves the SPA entry for canonical client-side routes, including direct refresh/deep-link entry; API rewrite behavior remains separate.

### AC-13 — Missing-context behavior

Routes that require configuration or station/device context have deterministic recovery targets and do not incorrectly classify missing context as authorization failure.

### AC-14 — Backend error distinction

Router 404, resource 404, 401, 403, invalid URL state, and unavailable resource remain semantically distinguishable.

### AC-15 — Command/navigation separation

Navigating to a command-related destination never implies that a physical command succeeded or that operational state is healthy.

### AC-16 — P1.6 Journal compatibility

Journal route state remains compatible with P1.6 scope/filter/cursor semantics and P1.6 authorization/identity boundaries.

### AC-17 — P1.7 API compatibility

Route migration does not introduce alternate API transports, duplicate resource scope semantics, or bypass P1.7 query/mutation ownership.

### AC-18 — Cross-device isolation

Changing station/device context cannot expose cached or rendered data belonging to the previous unauthorized/unavailable device.

### AC-19 — Accessibility/navigation integrity

Primary navigation and route transitions expose correct current-page semantics and remain keyboard/direct-link navigable without relying solely on visual state.

### AC-20 — Regression verification

Applicable frontend route/component tests pass, and blocked broader suites are documented with exact scope and cause.

### AC-21 — Traceability

Route manifest, compatibility inventory, tests, evidence, and project traceability identify P1.8 coverage and any explicit deferred items.

### AC-22 — Merged-tab compatibility

Legacy aliases that formerly represented a page sub-view either preserve that sub-view through canonical URL state or have an explicit, tested `collapse-to-parent` compatibility decision.

## 25. Evidence requirements

P1.8 evidence MUST include:

```json
{
  "phase": "P1.8",
  "requirement": "Route / Navigation Contract",
  "status": "...",
  "canonical_routes": [],
  "legacy_aliases": [],
  "route_scope_matrix": [],
  "access_matrix": [],
  "url_state_contracts": [],
  "verified": [],
  "blocked": [],
  "acceptance": {},
  "known_exceptions": [],
  "timestamp": "..."
}
```

Evidence MUST identify:

- exact route inventory;
- exact tests/commands executed;
- direct-entry/deep-link coverage;
- auth/403/404 coverage;
- station/device isolation coverage;
- legacy alias coverage;
- URL-state coverage;
- known unrelated failures or environment blockers.

## 26. Implementation order

Recommended implementation batches:

### Batch A — Route contract

- route manifest/IDs;
- canonical paths;
- access/scope metadata;
- legacy alias registry;
- 404 route.

### Batch B — Navigation ownership

- primary nav migration;
- route helpers;
- active-state derivation;
- page/component navigation migration.

### Batch C — Entry/context guards

- authentication behavior;
- capability/role UX gates;
- StationContext integration;
- missing-context recovery;
- resource/access error distinction.

### Batch D — URL state/deep links

- parsers/serializers;
- canonicalization;
- search/hash policy;
- refresh/back/forward behavior;
- notification/bookmark compatibility.

### Batch E — Verification

- route manifest tests;
- auth/403/404 tests;
- context isolation tests;
- legacy redirect tests;
- deep-link/history tests;
- evidence and traceability update.

## 27. Definition of done

P1.8 is complete only when all of the following are true:

- canonical route manifest exists and is the single navigation metadata owner;
- every supported destination has a stable route ID and documented scope/access behavior;
- primary navigation uses canonical route identities only;
- legacy aliases are inventoried, deterministic, and tested;
- URL state is explicitly parsed, bounded, canonicalized, and tested where supported;
- direct deep links behave consistently with in-app navigation;
- production SPA hosting supports direct canonical-route entry;
- authentication, authorization denial, missing context, invalid URL state, resource 404, and router 404 are distinguishable;
- StationContext remains the authoritative station/device boundary;
- cross-device navigation/context switching is isolated and tested;
- browser refresh/back/forward semantics are verified;
- command navigation does not imply physical success;
- P1.6 Journal and P1.7 API contracts remain intact;
- no second router, authorization system, or device identity store is introduced;
- required evidence and traceability are updated;
- applicable verification commands are green or explicitly documented as blocked.

## 28. Risks and rollback

### Risks

- changing canonical paths can break bookmarks, notifications, and external integrations;
- over-eager URL canonicalization can discard meaningful user state;
- route guards can accidentally hide backend authorization errors;
- synchronizing URL device identity with `StationContext` can create race conditions or cross-device leakage;
- replacing redirects with render-time navigation can create history pollution or loops;
- consolidating navigation literals can expose previously orphaned pages that lack complete route contracts.

### Rollback constraints

Rollback MUST preserve:

- existing working canonical destinations;
- authorization semantics;
- StationContext ownership boundary;
- P1.6 Journal scope;
- P1.7 API contracts;
- compatibility aliases that remain externally referenced.

Rollback MUST NOT reintroduce duplicate route definitions merely to restore one page.

## 29. Phase boundary / handoff

P1.8 establishes the frontend route/navigation contract.

Future phases MAY consume the route manifest for:

- generated navigation metadata;
- notification/deep-link generation;
- analytics/observability route dimensions;
- shared/generated schema alignment;
- native mobile route integration.

Those concerns MUST consume the P1.8 contract rather than creating independent route semantics.
