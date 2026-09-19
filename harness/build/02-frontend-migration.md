# P3.2 — Page-by-page frontend migration

Status: IMPLEMENTING

## Goal

Migrate the frozen page sequence onto the established semantic design system, one page at a time, while preserving operational state meaning and existing architecture.

## Scope

- Dashboard
- Fleet
- Control
- Cultivation / Recipes
- Automation
- Logs
- Settings / Admin

Shared components, safety/fault surfaces, and automation/configuration clusters may be migrated when required by the current page, but they do not define the execution order.

P3-MIGRATION-MAP.md remains the audit/classification source. It must not be interpreted as the page execution sequence.

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

1. Select the next page from the fixed P3.2 order.
2. Audit the whole page before editing, including its page-level components.
3. Identify the semantic role of each visual state.
4. Reuse existing tokens/primitives; add a token only when a repeated semantic role is missing.
5. Preserve loading, unavailable, stale/degraded, fault, pending, and confirmed distinctions.
6. Verify the page contract: hierarchy, spacing, states, responsive behavior, accessibility, typography, color, and interaction states.
7. Run targeted tests and design-lint checks for the page.
8. Perform a scope/regression review and record page-level evidence.
9. Do not begin the next page until the current page has explicit PASS evidence against every page-contract criterion.
10. Run the required frontend verification before phase completion.

## Acceptance criteria

- Each page in the fixed sequence has explicit PASS, FAIL, or BLOCKED evidence for the shared page contract.
- Remaining page/component palette usage is classified as semantic/domain-safe or migrated to an existing semantic role within the approved page scope.
- No second visual language is introduced.
- Canonical component contracts remain satisfied.
- Safety/control authority boundaries remain unchanged.
- Responsive and accessibility requirements remain intact.
- Required frontend verification passes with fresh evidence.
- Every criterion has `PASS`, `FAIL`, or `BLOCKED` evidence in the build log.

## Approval gate

Implementation requires explicit approval after this phase plan has been reviewed. Completion of P3.2 does not authorize P3.3.
