# HYDRAGROW Agent Engineering Plan

> This plan governs the current frontend-overhaul workflow. Application implementation remains separately scoped and must follow the phase approval gates below.

## Phase sequence

1. **P3.1 — Design-system surface** — completed before this harness was installed; evidence is recorded in `harness/build-log.md`.
2. **P3.2 — Frontend migration** — migrate remaining page/component surfaces to the established semantic design system while preserving domain/state semantics.
3. **P3.3 — Hardening** — final sweep for visual drift, architecture violations, accessibility/regression gaps, and verification evidence.

## Authority

- Product/design contracts: `docs/design-system/`
- Delivery governance: `docs/DELIVERY-GOVERNANCE.md`
- Verification commands: `.agent/verify.yml`
- Durable agent rules: `AGENTS.md`
- Phase scope and acceptance criteria: `harness/build/`
- Material discoveries/decisions: `harness/context/`
- Observed progress/evidence: `harness/build-log.md`

## Execution rule

Only the selected phase may be implemented. Completion requires fresh verification and a delivery review. Starting the next phase requires a new explicit approval.
