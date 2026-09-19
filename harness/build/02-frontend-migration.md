# P3.2 — Frontend migration

Status: IMPLEMENTING

## Goal

Migrate remaining frontend presentation surfaces onto the established semantic design system while preserving operational state meaning and existing architecture.

## Scope

- Shared UI components and page-level surfaces identified by the existing palette audit.
- Remaining automation/configuration visual drift.
- Safety/fault surfaces where palette usage can be converted without changing behavior.
- Page-level presentation migration after shared primitives are stable.

## Non-goals

- Do not blindly drive every palette utility to zero.
- Do not alter domain theme registries whose colors encode domain semantics without an explicit semantic mapping.
- No React Query replacement, new global state, duplicate FSM, controller logic, or backend presentation APIs.
- No route/IA changes beyond the frozen P3 contract.

## Source of truth

- `docs/design-system/P3-FRONTEND-CONTRACT.md`
- `docs/design-system/COMPONENT-CONTRACT.md`
- `docs/design-system/PAGE-CONTRACT.md`
- `docs/design-system/DESIGN-TOKENS-VISUAL-SPEC.md`
- `docs/design-system/PR-QA-CHECKLIST.md`
- `.agent/verify.yml`

## Required workflow

1. Audit the target surface before editing.
2. Identify the semantic role of each visual state.
3. Reuse existing tokens/primitives; add a token only when a repeated semantic role is missing.
4. Preserve loading, unavailable, stale/degraded, fault, pending, and confirmed distinctions.
5. Run targeted tests after each logical cluster.
6. Run the required frontend verification before phase completion.
7. Perform a scope/regression review and record evidence.

## Acceptance criteria

- Remaining page/component palette usage is classified as semantic/domain-safe or migrated to an existing semantic role.
- No second visual language is introduced.
- Canonical component contracts remain satisfied.
- Safety/control authority boundaries remain unchanged.
- Responsive and accessibility requirements remain intact.
- Required frontend verification passes with fresh evidence.
- Every criterion has `PASS`, `FAIL`, or `BLOCKED` evidence in the build log.

## Approval gate

Implementation requires explicit approval after this phase plan has been reviewed. Completion of P3.2 does not authorize P3.3.
