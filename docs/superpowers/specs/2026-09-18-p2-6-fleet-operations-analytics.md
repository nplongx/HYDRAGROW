# P2.6 Fleet Operations / Comparative Analytics

**Status:** PROPOSED
**Phase:** P2.6
**Depends on:** Fleet View foundation, P1.4 operational state, P1.7 API contract, P1.10 observability

## Goal

Extend the fleet UI from station selection into bounded multi-station operations and comparison without turning fleet summaries into a second telemetry authority.

## Features

### 1. Spatial fleet view

Provide a Grid/Greenhouse Map switch. Map placement is presentation metadata; it MUST NOT become device identity or telemetry authority.

### 2. Station comparison

Allow comparison of 2–4 authorized stations using authoritative telemetry/history queries. The UI MUST show missing/stale/error data explicitly.

### 3. Same-crop comparison

Compare normalized metrics only when the compared stations have compatible crop/stage/recipe context. Incompatible comparisons MUST be labeled rather than silently normalized.

### 4. Export/report workflows

Support bounded CSV/PDF export for monthly dosing and Journal/history views. Export MUST honor the caller's existing authorization scope and explicit date/device limits.

### 5. Threshold workflow

Allow threshold proposals from analytics charts, but applying a threshold remains a normal authorized configuration mutation and follows ConfigurationSync.

## Acceptance criteria

- **AC-1:** Fleet map never bypasses device ownership checks.
- **AC-2:** Fleet endpoints enforce explicit device limits/pagination and bounded historical ranges.
- **AC-3:** Comparison uses authoritative telemetry/history, not duplicated fleet cache values.
- **AC-4:** Stale/missing/invalid measurements remain explicit.
- **AC-5:** Export is deterministic for a fixed device/date/filter set and excludes unauthorized records.
- **AC-6:** Threshold changes create durable configuration state and use ConfigurationSync.
- **AC-7:** Frontend tests cover map/grid switching, comparison selection, empty/error states, and export boundaries.
