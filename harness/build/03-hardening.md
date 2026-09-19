# P3.3 — Hardening

Status: PROPOSED

## Goal

Convert the frontend overhaul into an auditable, regression-resistant state before delivery.

## Scope

- Final semantic-token and palette-drift sweep.
- Architectural boundary checks relevant to the P3 contract.
- Responsive/accessibility regression checks.
- Full frontend verification and delivery evidence.

## Non-goals

- No feature expansion.
- No redesign of the frozen information architecture.
- No domain/controller/state architecture changes.

## Source of truth

- `docs/design-system/P3-FRONTEND-CONTRACT.md`
- `docs/design-system/COMPONENT-CONTRACT.md`
- `docs/design-system/PAGE-CONTRACT.md`
- `docs/DELIVERY-GOVERNANCE.md`
- `.agent/verify.yml`

## Acceptance criteria

- No unclassified visual drift remains in the approved P3 migration scope.
- Architecture checks confirm the authority boundaries remain intact.
- Required tests, build, lint, and formatting checks have fresh evidence.
- Delivery documentation is synchronized.
- Known gaps and rollback considerations are recorded.

## Approval gate

Do not begin until P3.2 is accepted and a separate approval is given for hardening.
