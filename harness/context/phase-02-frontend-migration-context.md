# P3.2 Frontend Migration — Context

## 2026-09-19 — First harness execution (superseded execution order)

- The first harness execution incorrectly selected an automation/configuration component cluster as the implementation unit.
- This cluster was a valid semantic migration batch, but it did not follow the agreed P3.2 page-by-page execution order.
- This first implementation batch touched only:
  - `hydragrow-frontend/src/components/automation/ConfigExplorerWidget.tsx`
  - `hydragrow-frontend/src/components/automation/FlowOverviewCard.tsx`
  - `hydragrow-frontend/src/components/automation/WebhookAndChainPanel.tsx`
- `ConfigExplorerWidget` now uses existing config, text, line, and shared button semantic roles.
- `FlowOverviewCard` now maps alert/action/cron/webhook presentation to existing warning/info/config semantic roles.
- `WebhookAndChainPanel` now uses existing primary/config/warning/text roles rather than direct emerald/indigo/amber palette utilities.
- No domain theme registry, state authority, route, API, or controller logic was changed.

## Verification observed

- Automation targeted suite: 22 files / 107 tests passed.
- Design-lint guards: 3 files / 6 tests passed.
- ESLint on the three touched files: 0 errors, 2 existing `no-explicit-any` warnings in `FlowOverviewCard.tsx`.
- Production build output reached `✓ built in 6.12s` and produced `dist/index.html`; the wrapper did not return a final exit-code line, so this is recorded as build evidence, not a phase-level completion claim.
- `git diff --check`: passed.
- A full-suite run was started but later terminated while still running to avoid duplicate test processes; its exit code was 143 and therefore is **not** evidence of a passing full suite.

## Remaining phase work

P3.2 remains `IMPLEMENTING`. The harness plan is now corrected to enforce the page sequence:

1. Dashboard
2. Fleet
3. Control
4. Cultivation / Recipes
5. Automation
6. Logs
7. Settings / Admin

The previously migrated automation cluster remains part of the repository state; it is not treated as evidence that the Automation page has passed its page contract. P3.3 must not start.

## 2026-09-19 — Dashboard audit and migration

### Audit findings

- Dashboard already used the established `app-page`, `PageHeader`, `Banner`, `StateView`, `Button`, `DeviceStatePill`, and semantic status tokens.
- The page had a heading hierarchy defect: the page header supplied the H1, but the greeting was another H1 and the telemetry section used H3 before later H2 sections.
- Dashboard-only presentation surfaces still contained direct white-surface/button styling and the sensor card used a legacy hardcoded surface/elevation treatment.
- Sensor theme names (`blue`, `fuchsia`, `orange`, `cyan`, `rose`) are semantic presentation inputs to `SensorBentoCard`, not raw palette utilities; their mapping remains centralized.

### Migration

- Greeting changed from H1 to H2 so the page has one canonical H1 from `PageHeader`.
- Real-time telemetry section changed from H3 to H2.
- Dashboard status link and sensor cards now use semantic surface/elevation roles.
- Dashboard quick actions now reuse the canonical `ui-btn-outline` / `ui-btn-primary` button treatments.
- Existing operational/data/state logic was not changed.

### Verification

- Dashboard tests: 1 file / 3 tests passed.
- Design-lint guards plus Dashboard tests: 4 files / 9 tests passed.
- Touched-file ESLint: 0 errors, 13 warnings; all warnings are existing `no-explicit-any` warnings in Dashboard.
- Production build: passed; `vite build` completed in 6.17s.
- Harness structural checks: passed.
- `git diff --check`: passed.

### Browser review

- Local Vite server was started on `http://localhost:1421`.
- Browser review reached the real application login gate at `/dashboard`, not an authenticated Dashboard session.
- No authenticated Dashboard browser state or test credentials were available in the browser session, so visual review of the Dashboard itself is **BLOCKED**.
- The authenticated Dashboard must be visually reviewed at the required responsive widths before this page can be marked complete or handed off.

## 2026-09-19 — Design-token spacing collision found and fixed

- Browser inspection exposed a cross-system regression: `@theme` defined semantic tokens named `--spacing-sm/md/lg/xl/2xl`.
- Tailwind v4 reserves the `--spacing-*` namespace for spacing-derived utilities. As a result, `max-w-md` resolved to `1rem` instead of `28rem`, and `max-w-sm` resolved to `0.5rem` instead of `24rem`.
- This explains both the collapsed RouteRecovery card and the collapsed authentication card visible in browser review.
- Semantic spacing tokens were renamed to `--hg-space-*` outside `@theme`, leaving Tailwind's utility namespace intact.
- Browser verification after the fix: RouteRecovery `max-w-md` computed to `448px`; a representative authentication `max-w-sm` card computed to `384px`.
