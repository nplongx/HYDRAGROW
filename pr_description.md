## Requirement
- Issue / Requirement ID: AUTOMATION-007
- Change class (`C0`-`C7`): C1

## Objective
Implement a Test Panel to dry-run automation scripts with sample values and provide a trace of condition evaluations.

## Acceptance Contract
- Acceptance contract: `docs/acceptance/AUTOMATION-007.json`
- Evidence contract: `docs/evidence/AUTOMATION-007.json`
- For C1-C7 changes, commit both contracts in the same PR.
- Schemas: `docs/schemas/acceptance-contract.schema.json`, `docs/schemas/evidence-contract.schema.json`

## Acceptance Criteria

| ID | Criterion | Target / Expected | Actual | Evidence |
|---|---|---|---|---|
| AC-1 | Endpoint /devices/{device_id}/scripts/test returns true and trace when condition met | true | true | cargo test api::script::tests |
| AC-2 | Endpoint returns false with failing leaf marked | true | true | cargo test api::script::tests |
| AC-3 | eval_condition_tree agrees with compiled Rhai on random cases | true | true | cargo test api::script::tests |
| AC-4 | UI Test Panel is implemented and can be rendered | true | true | npm run test |

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
- Environment: Local testing
- Build / version: Head
- Evidence: Unit test output
- Rollback plan: Revert commit

## Risks / Known Gaps
None.

## Final Acceptance
- [x] Code verification passed
- [x] Acceptance criteria passed
- [x] Required deployment/integration evidence attached
- [x] Documentation synchronized
- [x] Project state synchronized
