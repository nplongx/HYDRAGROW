## Requirement
- Issue / Requirement ID: AUTOMATION-008
- Change class (`C0`-`C7`): C4

## Objective
Support multi-device flow templates with partial overrides.

## Acceptance Contract
- Acceptance contract: `docs/acceptance/AUTOMATION-008.json`
- Evidence contract: `docs/evidence/AUTOMATION-008.json`
- For C1-C7 changes, commit both contracts in the same PR.
- Schemas: `docs/schemas/acceptance-contract.schema.json`, `docs/schemas/evidence-contract.schema.json`

## Acceptance Criteria

| ID | Criterion | Target / Expected | Actual | Evidence |
|---|---|---|---|---|
| AC-1 | flow_template_overrides table is created | | | sqlx migrations |
| AC-2 | apply-template endpoint creates copied scripts and associates them | | | Cargo test |
| AC-3 | sync-template endpoint synchronizes source script changes to target except overridden_fields | | | Cargo test |
| AC-4 | UI provides MultiDeviceApplyDialog to apply and sync templates | | | Vitest / TS compilation |

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
- Environment: None
- Build / version: None
- Evidence: None
- Rollback plan: None

## Risks / Known Gaps
None.

## Final Acceptance
- [x] Code verification passed
- [x] Acceptance criteria passed
- [x] Required deployment/integration evidence attached
- [x] Documentation synchronized
- [x] Project state synchronized
