# P2.7 Verification / Operational SLOs

**Status:** PROPOSED
**Phase:** P2.7
**Depends on:** P1.0 verification baseline, P1.10 observability, P2.1–P2.6 as applicable

## Goal

Turn software cross-system correctness into measurable regression gates and operational synchronization SLOs without requiring physical HIL for software completion.

## Required measurements

### Synchronization

- desired-to-pending age;
- pending-to-published age;
- published-to-applied age;
- unknown outcome count/age;
- reconciliation failure count;
- stale observed configuration count.

### Commands

- REQUESTED→SENT latency;
- SENT→ACKNOWLEDGED latency;
- ACKNOWLEDGED→CONFIRMED latency;
- timeout/unknown rate;
- retry count;
- duplicate/idempotent command count.

### Realtime

- WebSocket reconnect count;
- broadcast lag events;
- recovery refetch count;
- malformed frame count.

### Fleet/query

- bounded query duration;
- export duration/row count;
- comparison query duration;
- pagination/limit rejection count where applicable.

## SLO process

No target may be invented. For each metric:

```text
capture baseline
define target
run controlled workload
record actual
attach evidence
```

P2.7 MUST distinguish:

```text
software SLO verified
physical timing verified
```

The second requires hardware evidence and remains outside the software gate.

## Acceptance criteria

- **AC-1:** Every P2 synchronization path exposes measurable success/failure/age signals.
- **AC-2:** Metrics use bounded labels and a documented cardinality budget.
- **AC-3:** Health/readiness exposes worker liveness separately from synchronization health.
- **AC-4:** Cross-system regression scenarios include restart, disconnect, reconnect, stale data, duplicate command, command timeout/retry, configuration loss, and out-of-order telemetry.
- **AC-5:** Evidence contains actual terminal output and environment details.
- **AC-6:** No software evidence is labeled as physical HIL evidence.
