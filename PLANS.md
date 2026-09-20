# HYDRAGROW Agent Engineering Plan

> This plan governs the current frontend-overhaul workflow. Application implementation remains separately scoped and must follow the phase approval gates below.

## Phase sequence

1. **P3.1 — Design-system surface** — completed before this harness was installed; evidence is recorded in `harness/build-log.md`.
2. **P3.2 — Page-by-page frontend migration** — migrate pages in the frozen product order, applying the same page contract to each page while preserving domain/state semantics.
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

## P3.2 page order

The migration order is fixed:

1. Dashboard
2. Fleet
3. Control
4. Cultivation / Recipes
5. Automation
6. Logs
7. Settings / Admin

P3-MIGRATION-MAP.md is an audit/classification map, not an implementation-order list. Component clusters discovered by the audit are migrated only as part of the page currently being processed.

Each page must satisfy the same contract before moving to the next page:

- hierarchy
- spacing
- semantic typography and color
- loading / empty / error / success / disabled / destructive states
- responsive behavior
- accessibility and interaction states
- reuse of shared design-system primitives

Page acceptance evidence must be recorded before advancing to the next page.
