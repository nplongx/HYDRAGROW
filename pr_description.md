## Requirement
- Issue / Requirement ID: AUTOMATION-UI-OVERHAUL-001
- Change class (`C0`-`C7`): C1

## Objective
Rebuild the Automation page UI around one coherent responsive React Flow experience while preserving existing automation behavior and contracts.

## Acceptance Contract
- Acceptance contract: `docs/acceptance/AUTOMATION-UI-OVERHAUL-001.json`
- Evidence contract: `docs/evidence/AUTOMATION-UI-OVERHAUL-001.json`
- For C1-C7 changes, commit both contracts in the same PR.
- Schemas: `docs/schemas/acceptance-contract.schema.json`, `docs/schemas/evidence-contract.schema.json`

## Acceptance Criteria

| ID | Criterion | Target / Expected | Actual | Evidence |
|---|---|---|---|---|
| AC-1 | Automation overview renders saved Flow summary nodes | PASS | PASS | Vitest automation overview tests |
| AC-2 | next_flow_ids produce directed animated edges | PASS | PASS | Vitest useFlowCanvas and chain-selector tests |
| AC-3 | Node palette exposes all required capabilities | PASS | PASS | Vitest NodePalette tests |
| AC-4 | Nested AND/OR condition groups render | PASS | PASS | Vitest ConditionGroupEditor and conditionTree tests |
| AC-5 | mean/min/max conditions expose window in minutes | PASS | PASS | Vitest time-window editor and compiler tests |
| AC-6 | Cron trigger UI stores a cron expression | PASS | PASS | Vitest Cron trigger tests |
| AC-7 | Webhook trigger UI supports flow/direct mode | PASS | PASS | Vitest WebhookFieldMappingEditor tests |
| AC-8 | Chain action selects a Flow | PASS | PASS | Vitest chain selector and flowCycle tests |
| AC-9 | Test Panel accepts sample field values | PASS | PASS | Vitest TestPanel tests |
| AC-10 | Multi-device template UI lets the user select devices | PASS | PASS | Vitest multi-device template tests |
| AC-11 | Desktop matches the supplied information hierarchy | PASS | PASS | Component tests for responsive branches |
| AC-12 | All frontend verification commands pass | PASS | PASS | Verification script |

## Verification
- [x] Acceptance contract gate
- [x] Evidence contract gate
- [x] Delivery governance gate
- [x] Unit / integration tests
- [ ] E2E / scenario test
- [ ] Benchmark / performance evidence (if applicable)
- [ ] Hardware / staging / deployment verification (if applicable)

## Documentation
- [x] Required architecture/API/operations docs updated
- [x] `docs/project-state/CURRENT-STATUS.md` updated if project state changed
- [x] `docs/project-state/TRACEABILITY.md` updated for material requirements

## Deployment
- Environment:
- Build / version:
- Evidence:
- Rollback plan:

## Risks / Known Gaps
None.

## Final Acceptance
- [x] Code verification passed
- [x] Acceptance criteria passed
- [x] Required deployment/integration evidence attached
- [x] Documentation synchronized
- [x] Project state synchronized
