# HYDRAGROW P2 — System Evolution Specification

**Status:** PROPOSED
**Phase:** P2
**Date:** 2026-09-18
**Depends on:** P0 foundation, P1 software cross-system verification, Digital Twin software gate
**Hardware gate:** Physical HIL remains separate and deferred; P2 software work MUST NOT claim physical verification.

## 1. Purpose

P2 moves HYDRAGROW from a verified modular monolith into a more maintainable, observable, multi-station software system without changing its established authority model.

The governing rule is:

> **Decompose boundaries, not truth.**

P2 MUST preserve PostgreSQL/configuration/command/Journaling authority, InfluxDB telemetry authority, controller-core safety authority, canonical MQTT/schema contracts, StationContext/React Query frontend authority, and Digital Twin behavior. P2 is incremental evolution, not a rewrite.

## 2. P2 entry gate

P2 may start when the following are true:

- Software cross-system Backend + PostgreSQL + MQTT + Digital Twin E2E is green.
- Durable command lifecycle is implemented and verified.
- Durable configuration synchronization is implemented and verified.
- `/readyz` distinguishes dependency readiness from synchronization health.
- Canonical schema registry remains the only production wire-contract source.
- Physical HIL is explicitly recorded as deferred, not silently treated as PASS.

Current P2 entry state: **software gate PASS; physical HIL deferred**.

## 3. Architectural invariants

P2 MUST NOT introduce:

- a second command lifecycle or FSM;
- a second frontend server-state cache;
- a second MQTT production namespace;
- a second schema registry;
- a new database solely for P2 decomposition;
- an authentication bypass;
- a cloud Digital Twin or AI control loop;
- exactly-once physical actuation claims;
- a microservice split without an independently justified operational boundary.

P2 MUST preserve:

```text
Frontend
  StationContext
  React Query
  API adapters
       |
       v
Backend application/domain services
       |
       +--> PostgreSQL durable authority
       +--> InfluxDB telemetry authority
       +--> MQTT transport
       +--> WebSocket notification/recovery path
       |
       v
Controller-core / physical controller
```

## 4. P2 workstreams

P2 is split into independently testable workstreams:

1. **P2.1 Backend Domain/Application Decomposition** — extract focused application services and persistence boundaries from the modular monolith.
2. **P2.2 Configuration Synchronization Platform** — make configuration delivery one reusable reconciliation subsystem for settings, WiFi, and backup/restore.
3. **P2.3 Realtime Recovery Boundary** — split WebSocket transport, event routing, and authoritative recovery without making realtime a second cache.
4. **P2.4 Frontend Performance and Data Boundaries** — route-level code splitting, query ownership cleanup, and coarse invalidation reduction.
5. **P2.5 Canonical Firmware Contract Pipeline** — make C++ sensor, Rust sensor, and controller firmware consume canonical fixtures in CI and define implementation ownership.
6. **P2.6 Fleet Operations and Comparative Analytics** — fleet map/spatial layout, station comparison, threshold/report workflows, and bounded query costs.
7. **P2.7 Verification and Operational SLOs** — convert cross-system correctness into measurable regression gates, synchronization SLOs, and durable evidence.

Each workstream MUST have its own acceptance/evidence record before being marked `VERIFIED`.

## 5. Cross-workstream state model

P2 uses the following state distinctions everywhere:

```text
desired       persisted intended state
observed      producer-reported state
applied       producer accepted/applied state
cached        local acceleration only
transport     communication condition
sync          convergence condition
```

No P2 feature may infer physical success from:

```text
HTTP 2xx
MQTT publish success
WebSocket send success
cache write
```

## 6. Performance contract

Every P2 behavior/performance change MUST record:

```text
baseline
target
actual
environment
measurement command
```

Default software targets:

- no new unbounded metric label cardinality;
- no unbounded retry loop;
- no periodic polling introduced where event/reconciliation state already exists;
- frontend initial route payload MUST not increase when a feature is moved to a lazy route chunk;
- synchronization worker retry cadence MUST remain bounded and observable;
- fleet endpoints MUST use bounded queries and explicit pagination/limits for historical data.

Exact numerical latency/SLO targets belong to P2.7 after a reproducible baseline is captured; no invented baseline is permitted.

## 7. Security contract

Every extracted service MUST receive an already-authorized principal/context from the API boundary. Domain decomposition MUST NOT move ownership checks into an optional caller convention.

Logs, metrics, traces, fixtures, exports, and comparison views MUST NOT expose API keys, JWTs, MQTT credentials, calibration secrets, or private user data beyond the existing authorized response contract.

## 8. Migration and compatibility

P2 prefers additive migration:

```text
existing behavior
    -> new boundary
    -> dual-read/adapter only when necessary
    -> verify
    -> remove obsolete path
```

Database migrations MUST preserve historical migration identity and provide an explicit upgrade/rollback strategy before deployment to an existing database.

Legacy route/API compatibility MUST be removed only after the documented compatibility window and evidence show no supported consumer depends on it.

## 9. Verification pyramid

Every workstream uses:

```text
Unit
  -> pure domain/policy/serialization

Integration
  -> service + PostgreSQL/Influx/MQTT as applicable

Cross-system
  -> Backend + PostgreSQL + MQTT + Digital Twin

Browser/UI
  -> frontend route/query behavior where applicable

Physical HIL
  -> separate gate; P2 software completion MUST NOT imply it
```

## 10. Acceptance gate for P2 phase completion

P2 phase may be marked `VERIFIED` only when:

- all selected P2 workstreams have binary PASS/FAIL/BLOCKED acceptance results;
- every changed domain has unit/integration tests appropriate to its risk;
- cross-system regression remains green;
- schema registry checks remain green;
- frontend build/typecheck/tests remain green;
- backend check/clippy/fmt/tests remain green;
- no protected-path or migration-governance violation exists;
- evidence JSON is valid and references concrete commands/results;
- `CURRENT-STATUS.md` and `TRACEABILITY.md` match the implementation state;
- physical HIL remains explicitly `BLOCKED`/`DEFERRED` if no hardware evidence exists.

## 11. Non-goals

P2 does not:

- rewrite the backend into microservices;
- replace Actix-web, SQLx, PostgreSQL, InfluxDB, MQTT, WebSocket, React Query, or controller-core;
- redesign the canonical MQTT protocol;
- turn Digital Twin into a production control authority;
- replace physical HIL with simulation;
- add a community/social control plane;
- optimize bundle size by removing required functionality.
