# P3 Frontend Contract

Status: VERIFIED FOR P3.0/P3.1

## Scope

P3 starts as a frontend presentation and information-architecture overhaul over existing domain contracts. It does not replace frontend data authority, safety/control ownership, or backend contracts.

## P3.0 — Contract freeze

### Primary navigation

The primary navigation is fixed to five product surfaces:

1. `/dashboard` — Tổng quan
2. `/operations` — Vận hành
3. `/cultivation` — Canh tác
4. `/journal` — Nhật ký
5. `/settings` — Cài đặt

Fleet, pairing, backup, roles, and user management remain utility/admin surfaces. P3 does not promote Fleet to primary navigation without new discovery evidence.

### Authority boundaries

- `StationContext` owns selected station/device identity.
- React Query owns frontend server-state authority.
- WebSocket/realtime is best-effort invalidation/recovery, not a second server-state store.
- PostgreSQL/config/command/journal remain authoritative backend sources.
- Influx remains telemetry/time-series authority.
- Controller-core remains safety/control authority.

### State semantics

Pages must preserve the distinction between:

- loading vs unavailable
- stale/degraded vs offline
- fault vs normal state
- command requested vs command acknowledged/confirmed
- desired configuration vs observed/applied device state

Frontend refactors must not collapse these states into optimistic visual success.

### Responsive shell

The shell has two navigation presentations over the same route targets:

- desktop: fixed left sidebar, 256px wide
- mobile: top header + bottom navigation with safe-area padding

The content viewport must remain independently scrollable and must not be hidden behind fixed navigation.

Required review widths: 375px, 768px, 1024px, 1440px.

### Design-system rules

- Reuse existing semantic tokens and primitives.
- Prefer semantic primitives over repeated page-local visual composition.
- Keep Lucide/SVG icons; no emoji UI icons.
- Preserve visible focus states and keyboard access.
- Respect `prefers-reduced-motion`.
- Do not introduce a second visual language for P3.

## P3.1 — App Shell implementation contract

`MainLayout` is the data/orchestration boundary. `AppShell` owns shell composition. Shell subcomponents own presentation only:

```text
MainLayout
  -> AppShell
     -> MobileHeader
     -> DesktopSidebar
     -> <main><Outlet /></main>
     -> MobileBottomNav
```

`MainLayout` continues to obtain telemetry, station identity, alert count, route activity, and navigation callbacks. Shell components do not fetch domain data and do not create new state stores.

## Explicit non-goals

- no React Query replacement
- no new global state architecture
- no new cache
- no `/twin`
- no duplicate FSM
- no controller logic in React
- no backend API created only for presentation convenience
- no commercial-first IA commitment
