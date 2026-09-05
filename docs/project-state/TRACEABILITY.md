# Traceability

| Requirement ID | Acceptance Criteria | Implementation Files | Tests / Evidence | Verification |
| --- | --- | --- | --- | --- |
| AUTOMATION-UI-OVERHAUL-001 | AC-1..AC-12 | hydragrow-frontend/src/components/automation/* | docs/evidence/AUTOMATION-UI-OVERHAUL-001.json | Vitest |
| AUTOMATION-CONTEXT-001 | AC-1..AC-8 | hydragrow-backend/src/{services,mqtt,api}/*, hydragrow-frontend/src/lib/automation/* | docs/evidence/AUTOMATION-CONTEXT-001.json | Vitest & Cargo test |
| CONTROLLER-FSM-001 | AC-1..AC-13 | hydragrow-controller-core/src/*, hydragrow-shared/src/*, hydragrow-simulator/src/* | docs/evidence/CONTROLLER-FSM-001.json | Cargo test (158 core, 28 sim, 27 shared) |
