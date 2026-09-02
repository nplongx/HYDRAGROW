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
| Backend | VERIFIED | Script chaining next_flow_ids preservation and eval_alert_scripts_chained verified with SCRIPT-CHAINING-001 contract. |
| Shared contracts | IMPLEMENTING | Keep MQTT/schema changes traceable and synchronized. |
| Frontend | VERIFIED | AUTOMATION-001 action_command trigger type and next flow selection verified with unit tests and evidence contract. |
| Controller / firmware | IMPLEMENTING | Use staging or hardware evidence where behavior depends on real devices. |
| Simulator | IMPLEMENTING | Keep scenario coverage aligned with production contracts. |
| CI / automation | IMPLEMENTING | Strict code-quality gate added for Rust and frontend; verify the new workflow on the PR before marking it VERIFIED. |

## Active blockers

- No project-level blocker recorded by this governance change.

## Update rule

Do not infer completion from green CI alone. A component becomes `VERIFIED`, `DEPLOYED`, or `ACCEPTED` only when the corresponding evidence exists in the relevant PR, artifact, or release record.
