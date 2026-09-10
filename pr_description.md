## Requirement
- Issue / Requirement ID: SEC-2025-0009
- Change class (`C0`-`C7`): C1

## Objective
Update `rumqttc` from `0.22.0` to `0.24.0` in `hydragrow-simulator` crate, which transitively upgrades `ring` to `0.17.14`. This resolves the `ring` panic vulnerability (RUSTSEC-2025-0009).

## Acceptance Contract
- Acceptance contract: `docs/acceptance/SEC-2025-0009.json`
- Evidence contract: `docs/evidence/SEC-2025-0009.json`
- For C1-C7 changes, commit both contracts in the same PR.
- Schemas: `docs/schemas/acceptance-contract.schema.json`, `docs/schemas/evidence-contract.schema.json`

## Acceptance Criteria

| ID | Criterion | Target / Expected | Actual | Evidence |
|---|---|---|---|---|
| AC-1 | ring is upgraded to >= 0.17.12 | PASS | PASS | docs/evidence/SEC-2025-0009.json |

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
- Environment: Local testing & CI
- Build / version: 0.1.0 (Simulator)
- Evidence: CI tests passing
- Rollback plan: Revert this PR

## Risks / Known Gaps
None. Only affects `hydragrow-simulator` locally.

## Final Acceptance
- [x] Code verification passed
- [x] Acceptance criteria passed
- [x] Required deployment/integration evidence attached
- [x] Documentation synchronized
- [x] Project state synchronized
