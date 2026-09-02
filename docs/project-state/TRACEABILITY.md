# Requirement Traceability

This register connects product/system requirements to implementation, verification, deployment, and documentation evidence.

## Register

| Requirement | Acceptance criteria | Implementation | Verification / evidence | Deployment | Docs | Status |
|---|---|---|---|---|---|---|
| Governance foundation | Delivery contract and status artifacts exist | `docs/DELIVERY-GOVERNANCE.md` | Governance workflow | N/A | Project-state docs | VERIFIED |
| GOV-002 | AC-1 machine-readable contract; AC-2 schema validation; AC-3 traceability enforcement | `docs/acceptance/GOVERNANCE-001.json`, `.github/workflows/acceptance-contract.yml` | Acceptance contract gate + delivery governance gate | GitHub Actions / PR validation | Acceptance contract docs | VERIFIED |
| GOV-003 | AC-1 evidence contract required; AC-2 evidence is attributable; AC-3 quantitative target comparison is deterministic | `docs/schemas/evidence-contract.schema.json`, `.github/scripts/validate_evidence_contract.py`, `.github/workflows/evidence-contract.yml` | Evidence contract gate | GitHub Actions / PR validation | Evidence contract docs | VERIFIED |
| SCRIPT-CHAINING-001 | CachedScript next_flow_ids preservation and chained alert script evaluation with cycle detection and depth limiting | `hydragrow-backend/src/services/script_engine.rs`, `hydragrow-backend/src/mqtt/handlers/script_eval.rs` | `cargo test --manifest-path hydragrow-backend/Cargo.toml script` | Render / Staging | `docs/acceptance/SCRIPT-CHAINING-001.json`, `docs/evidence/SCRIPT-CHAINING-001.json` | VERIFIED |
| Project requirements | Add project-specific rows as requirements are defined | Pending | Pending | Pending | Pending | NOT_STARTED |
| MIX valve control | Support MIX and MIX_VALVE as accepted non-dosing pump names in control API and fallback pump status | `hydragrow-backend/src/api/control.rs` | `cargo test --bin hydragrow-backend api::control` | Render / Production | `hydragrow-backend/src/api/control.rs` | VERIFIED |
| AUTOMATION-001 | AC-1 trigger.type for action_command is sensor; AC-2 buildIrFromGraph passes nextFlowIds; AC-3 FlowDetailDrawer allows user to select next flows | `hydragrow-frontend/src/components/automation/reactflow/buildIr.ts`, `hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx` | `docs/evidence/AUTOMATION-001.json`, Vitest unit tests | Frontend / Web | Frontend automation docs | VERIFIED |
| LANE0-FOUNDATION-001 | AC-1 read FSM phase from cache in sensors.rs; AC-2 filter FlowDetailDrawer chain action list by kind; AC-3 Trigger panel skeleton in NodeEditorPanel | `hydragrow-backend/src/mqtt/handlers/sensors.rs`, `hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx`, `hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.tsx` | `docs/evidence/LANE0-FOUNDATION-001.json`, Cargo tests, Vitest tests | Render / Web | `docs/acceptance/LANE0-FOUNDATION-001.json` | VERIFIED |
| LANE1-CROSSKIND-CHAIN-001 | AC-1 validate-time cycle detection; AC-2 cross-kind eval_flow_chain; AC-3 frontend cycle prevention badge; AC-4 CI quality gates | `hydragrow-backend/src/services/flow_graph.rs`, `hydragrow-backend/src/mqtt/handlers/script_eval.rs`, `hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx` | `docs/evidence/LANE1-CROSSKIND-CHAIN-001.json`, Cargo tests, Vitest tests | Render / Web | `docs/acceptance/LANE1-CROSSKIND-CHAIN-001.json` | VERIFIED |

## Rules

1. Every material feature or system behavior change should add or update a row.
2. `VERIFIED` requires concrete evidence, not only a successful compile.
3. `DEPLOYED` requires the target environment and deployed version/build to be recorded.
4. `ACCEPTED` requires deployment/outcome evidence plus synchronized documentation.
5. Unknown or missing evidence must remain `NOT_STARTED`, `IMPLEMENTING`, or `BLOCKED`; never promote it based on assumption.
6. Governance/tooling-only changes are traceable here when they change the delivery control plane; they are not treated as product subsystem changes by the material-product-change detector.
