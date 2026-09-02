# HYDRAGROW Current Status

> This file is the canonical project-state snapshot. Update it in the same PR when a material change alters implementation status, capability, deployment state, or blockers.

Last updated: 2026-09-02

## Status vocabulary

- `NOT_STARTED` — planned but no implementation evidence.
- `IMPLEMENTING` — work exists but acceptance is incomplete.
- `VERIFIED` — acceptance criteria are verified with evidence.
- `DEPLOYED` — verified build is deployed to the declared environment.
- `ACCEPTED` — deployed behavior is accepted and documentation is synchronized.
- `BLOCKED` — progress depends on a missing decision, environment, hardware, or evidence.

## Current snapshot

| Area | Status | Evidence / next action |
|---|---|---|
| Backend | VERIFIED | LANE1-CROSSKIND-CHAIN-001 validate-time cycle detection and cross-kind eval_flow_chain verified with tests and evidence contract. |
| Shared contracts | IMPLEMENTING | Keep MQTT/schema changes traceable and synchronized. |
| Frontend | VERIFIED | LOG-NEUTRAL-LEGEND-001 Neutral token and EventLegend component verified with Vitest tests and evidence contract. |
| Controller / firmware | IMPLEMENTING | Use staging or hardware evidence where behavior depends on real devices. |
| Simulator | IMPLEMENTING | Keep scenario coverage aligned with production contracts. |
| CI / automation | VERIFIED | Delivery governance workflow is present and PR contract validation is enabled. |

## Active blockers

- No project-level blocker recorded by this governance change.

## Update rule

Do not infer completion from green CI alone. A component becomes `VERIFIED`, `DEPLOYED`, or `ACCEPTED` only when the corresponding evidence exists in the relevant PR, artifact, or release record.
