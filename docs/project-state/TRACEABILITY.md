# Requirement Traceability

This register connects product/system requirements to implementation, verification, deployment, and documentation evidence.

## Register

| Requirement | Acceptance criteria | Implementation | Verification / evidence | Deployment | Docs | Status |
|---|---|---|---|---|---|---|
| Governance foundation | Delivery contract and status artifacts exist | `docs/DELIVERY-GOVERNANCE.md` | Governance workflow | N/A | Project-state docs | VERIFIED |
| GOV-002 | AC-1 machine-readable contract; AC-2 schema validation; AC-3 traceability enforcement | `docs/acceptance/GOVERNANCE-001.json`, `.github/workflows/acceptance-contract.yml` | Acceptance contract gate + delivery governance gate | GitHub Actions / PR validation | Acceptance contract docs | VERIFIED |
| GOV-003 | AC-1 evidence contract required; AC-2 evidence is attributable; AC-3 quantitative target comparison is deterministic | `docs/schemas/evidence-contract.schema.json`, `.github/scripts/validate_evidence_contract.py`, `.github/workflows/evidence-contract.yml` | Evidence contract gate | GitHub Actions / PR validation | Evidence contract docs | VERIFIED |
| Project requirements | Add project-specific rows as requirements are defined | Pending | Pending | Pending | Pending | NOT_STARTED |

## Rules

1. Every material feature or system behavior change should add or update a row.
2. `VERIFIED` requires concrete evidence, not only a successful compile.
3. `DEPLOYED` requires the target environment and deployed version/build to be recorded.
4. `ACCEPTED` requires deployment/outcome evidence plus synchronized documentation.
5. Unknown or missing evidence must remain `NOT_STARTED`, `IMPLEMENTING`, or `BLOCKED`; never promote it based on assumption.
6. Governance/tooling-only changes are traceable here when they change the delivery control plane; they are not treated as product subsystem changes by the material-product-change detector.
