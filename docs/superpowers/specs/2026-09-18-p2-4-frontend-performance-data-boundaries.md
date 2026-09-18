# P2.4 Frontend Performance / Data Boundaries

**Status:** PROPOSED
**Phase:** P2.4
**Depends on:** P0.1, P0.3, P0.5, P1.7, P1.8

## Goal

Reduce initial frontend payload and unnecessary refetching while preserving StationContext, React Query authority, and realtime recovery semantics.

## Scope

- route-level lazy loading;
- automation editor chunk isolation;
- query-key normalization;
- mutation invalidation precision;
- coarse device-health refetch reduction;
- removal of duplicate server-state ownership;
- performance measurement.

## Rules

- `StationContext` remains selected-device identity authority.
- React Query remains server-state authority.
- `useDeviceStore` remains UI/transient preference state only.
- `useDeviceSync` remains transport/orchestration, not a cache.
- Lazy loading MUST NOT delay safety-critical controls after the user enters the control route.
- No behavior may be removed solely to reduce bundle size.

## Acceptance criteria

- **AC-1:** Dashboard/Operations/Automation/Journal/Settings/Fleet route modules are independently lazy-loadable.
- **AC-2:** Automation editor dependencies are not loaded on the initial dashboard route.
- **AC-3:** Initial bundle has a measured baseline and target; actual is recorded from a reproducible production build.
- **AC-4:** Mutation invalidation targets only affected query families where authoritative data permits it.
- **AC-5:** Realtime events do not cause duplicate REST refetches for the same update within one recovery cycle.
- **AC-6:** Full Vitest, TypeScript, ESLint, and production build remain green.
- **AC-7:** Accessibility and deep-link route behavior remain unchanged.
