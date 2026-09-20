# P3.1 — Design-system surface

Status: VERIFIED

## Goal

Establish the semantic frontend surface and shell baseline without changing application data authority or control architecture.

## Scope

- Semantic design tokens and reusable UI primitives.
- App shell geometry and shared visual primitives.
- Migration of affected shared UI components to semantic tokens.

## Non-goals

- No backend/API changes.
- No new global state, cache, FSM, or controller logic.
- No page-wide migration beyond the approved surface work.

## Source of truth

- `docs/design-system/P3-FRONTEND-CONTRACT.md`
- `docs/design-system/COMPONENT-CONTRACT.md`
- `docs/design-system/DESIGN-TOKENS-VISUAL-SPEC.md`
- `.agent/verify.yml`

## Acceptance criteria

- Semantic tokens are used by the shared surface instead of repeated palette literals.
- Desktop shell width is 256px and mobile navigation remains safe-area aware.
- Focus, disabled, status, and interactive states remain distinguishable without changing domain semantics.
- Frontend build, tests, lint, and diff checks provide evidence.

## Evidence

Commit `6a33de724` (`refactor(frontend): overhaul design-system surface`) records the implementation and verification performed before this harness was introduced.

## Approval gate

Do not reopen P3.1 unless a later regression or review finding explicitly requires it.
