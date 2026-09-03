# HYDRAGROW Current Status

> This file is the canonical project-state snapshot. Update it in the same PR when a material change alters implementation status, capability, deployment state, or blockers.

Last updated: 2026-03-31

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
| Automation | VERIFIED | AUTOMATION-007 Test Panel dry run implemented and tested |
| Backend | VERIFIED | AUTOMATION-006 Webhook trigger binding into flow and eval_webhook_chain verified with tests and evidence contract. |
| Shared contracts | IMPLEMENTING | Keep MQTT/schema changes traceable and synchronized. |
| Frontend | VERIFIED | AUTOMATION-008 MultiDeviceApplyDialog UI added. AUTOMATION-006 WebhookFieldMappingEditor UI verified. |
| Controller / firmware | VERIFIED | COMMAND-AUTH-001 HMAC verification applied to topic_controller_command and force_on emergency bypass removed in ESP32-C3-CONTROLLER-NODE, verified with static inspection, controller-core tests, and evidence contract. |
| Simulator | VERIFIED | SIMULATOR-002 deterministic Harness SimClock execution loop, CLI simulation runner, and closed-loop dosing cycle verified with Cargo test suite and evidence contract. |
| CI / automation | VERIFIED | Delivery governance workflow is present and PR contract validation is enabled. |

## Active blockers

- No project-level blocker recorded by this governance change.

## Update rule

Do not infer completion from green CI alone. A component becomes `VERIFIED`, `DEPLOYED`, or `ACCEPTED` only when the corresponding evidence exists in the relevant PR, artifact, or release record.
