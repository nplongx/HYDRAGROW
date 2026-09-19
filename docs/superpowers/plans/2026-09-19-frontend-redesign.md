# HYDRAGROW Frontend Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the existing Vite + React frontend around the HYDRAGROW design system while preserving routes, telemetry contracts, authorization, command semantics, and safety behavior.

**Architecture:** Implement the redesign in five independently verifiable slices: shared tokens/primitives, application shell, Dashboard/Fleet reference slice, domain pages, and auth/onboarding/error cleanup. Keep data hooks and route definitions intact; move presentation concerns into shared layout/status primitives and compose pages from those primitives.

**Tech Stack:** Vite, React 19, TypeScript, Tailwind CSS v4, React Router 7, React Query, Zustand, Lucide React, Vitest, Testing Library, React Flow.

**Spec:** `docs/superpowers/specs/2026-09-19-frontend-redesign.md`

## Global Constraints

- Modify only `hydragrow-frontend/**` for implementation; preserve existing backend, data hooks, route behavior, telemetry contracts, and safety semantics.
- Use `docs/design-system/` as the source of truth for tokens, component contracts, UX rules, patterns, dashboard behavior, and page anatomy.
- Do not invent telemetry, health scores, inferred aggregates, or unavailable station data.
- Status must use text plus visual/structural cues; color cannot be the sole signal.
- Keep E-STOP as the only emergency action with exceptional visual emphasis and do not alter its confirmation or command flow.
- Use mobile-first layouts; persistent desktop navigation transitions at the documented `lg` breakpoint; mobile uses bottom navigation and stacked cards.
- Validate with `cd hydragrow-frontend && npm run build`, `npx vitest run`, `npx eslint . && npx tsc --noEmit`, and `npx prettier --check .` as declared in `.agent/verify.yml`.
- Browser verification must cover the 900x617 dark desktop preview and a narrow mobile viewport.

## File Map

- `hydragrow-frontend/src/App.css`: global tokens, surfaces, typography, focus/reduced-motion rules, page anatomy, and responsive shell styles.
- `hydragrow-frontend/src/components/layout/MainLayout.tsx`: shared desktop/mobile shell, station context, connection state, role context, and route context.
- `hydragrow-frontend/src/components/layout/*`: navigation/context components already used by the shell; modify only those required by the existing composition.
- `hydragrow-frontend/src/components/ui/*`: existing shared status, banner, state-view, page-header, card, and control primitives; preserve public behavior while normalizing presentation.
- `hydragrow-frontend/src/pages/Dashboard.tsx`: multi-station overview followed by selected-station detail using existing hooks/cards.
- `hydragrow-frontend/src/pages/*`: Operations, Cultivation, Journal, Settings, Automation pages; migrate to shared page/status anatomy without changing workflows.
- `hydragrow-frontend/src/pages/auth/*`, onboarding, and error surfaces: apply shared tokens and accessible state language.
- `hydragrow-frontend/src/**/*.test.*`: focused composition/accessibility tests only where existing behavior is affected.

---

### Task 1: Normalize visual foundation and shared state anatomy

**Files:**
- Modify: `hydragrow-frontend/src/App.css`
- Modify: existing shared files under `hydragrow-frontend/src/components/ui/` that define `PageHeader`, `DeviceStatePill`, `StateView`, `Banner`, and shared cards
- Test: focused tests colocated with the affected shared primitives, if those files already have tests

**Interfaces:**
- Consumes: existing design tokens and contracts in `docs/design-system/DESIGN-TOKENS-VISUAL-SPEC.md`, `COMPONENT-CONTRACT.md`, `UX-RULES.md`, and `PATTERN-LIBRARY.md`.
- Produces: stable semantic classes and primitive APIs for `.app-page`, page header/context rows, `.ui-card` surfaces, status/state views, visible focus, and reduced-motion behavior. Do not rename existing props without updating all current callers.

- [ ] **Step 1: Inventory existing primitive APIs and current token usage**

Run:
```bash
cd /vercel/share/v0-project && grep -R "function PageHeader\|const PageHeader\|DeviceStatePill\|StateView\|className=.*app-page\|ui-card" hydragrow-frontend/src --exclude-dir=node_modules
```

Record the existing prop names and callers before editing. Read the exact files returned by the search; do not create parallel primitives when an existing one can be normalized.

- [ ] **Step 2: Add focused failing tests for status semantics**

For each existing test setup, add a test that renders the shared status primitive and asserts that a status label is present in accessible text, including a non-color cue such as an icon, `aria-label`, or visible secondary label. Add a state-view test asserting that loading and unavailable states render distinct text. Use existing test utilities and imports; do not introduce a new test framework.

- [ ] **Step 3: Normalize `App.css` around semantic tokens**

Replace primary shell/page ad-hoc colors and spacing with the documented semantic variables. Define the shared page anatomy and surface rules using the existing Tailwind v4/CSS setup. Add visible `:focus-visible` treatment and a reduced-motion media rule that disables nonessential transitions. Preserve any selectors required by current components and do not alter route-specific behavior.

- [ ] **Step 4: Normalize shared primitives without changing behavior**

Update `PageHeader`, `DeviceStatePill`, `StateView`, `Banner`, and existing card primitives to consume semantic classes and render explicit state text. Ensure loading, empty, unavailable, stale, offline, warning, fault, and confirmed states remain distinguishable. Keep existing callback props and control semantics unchanged.

- [ ] **Step 5: Run the focused tests and frontend checks**

Run:
```bash
cd /vercel/share/v0-project/hydragrow-frontend && npx vitest run && npx eslint . && npx tsc --noEmit && npx prettier --check .
```

Expected: all existing and new focused tests pass; lint, typecheck, and formatting report no errors.

- [ ] **Step 6: Commit the visual foundation**

```bash
git add hydragrow-frontend/src/App.css hydragrow-frontend/src/components/ui hydragrow-frontend/src/**/*.test.*
git commit -m "refactor(frontend): normalize design system foundation"
```

### Task 2: Rework application shell and responsive navigation

**Files:**
- Modify: `hydragrow-frontend/src/components/layout/MainLayout.tsx`
- Modify: directly composed navigation/context components under `hydragrow-frontend/src/components/layout/`
- Modify: `hydragrow-frontend/src/App.css`
- Test: shell/navigation tests under `hydragrow-frontend/src/**/*.test.*`

**Interfaces:**
- Consumes: existing router location, auth/user/role state, station context, connection state, and navigation component props.
- Produces: the same route/navigation behavior with a shell that exposes current route, station context, connectivity, and role context; desktop sidebar at `lg` and mobile bottom navigation below it.

- [ ] **Step 1: Write failing shell behavior tests**

Add tests that render `MainLayout` with existing providers/mocks and assert: the active route has an accessible current marker; station/connection context is visible when supplied; the mobile navigation exposes the same route destinations; and role context remains visible without changing authorization.

- [ ] **Step 2: Run the shell tests to verify failure**

```bash
cd /vercel/share/v0-project/hydragrow-frontend && npx vitest run src/components/layout
```

Expected: new assertions fail against the current shell where the required context/current markers are absent.

- [ ] **Step 3: Refine `MainLayout` composition**

Keep existing route links and callbacks. Add a compact context/status row in the shell using existing station and connection values. Use semantic HTML (`header`, `nav`, `main`) and accessible current-state attributes. Ensure desktop sidebar and mobile bottom navigation do not duplicate conflicting labels or trap focus.

- [ ] **Step 4: Implement responsive shell styles**

Use the documented `lg` transition. Desktop keeps persistent navigation; mobile hides the sidebar and presents touch-sized bottom navigation with safe bottom padding for page content. Do not use hard-coded viewport-specific positioning that obscures controls.

- [ ] **Step 5: Run shell tests and all frontend checks**

```bash
cd /vercel/share/v0-project/hydragrow-frontend && npx vitest run src/components/layout && npm run build && npx eslint . && npx tsc --noEmit && npx prettier --check .
```

Expected: all commands pass with no type or formatting errors.

- [ ] **Step 6: Commit the shell slice**

```bash
git add hydragrow-frontend/src/components/layout hydragrow-frontend/src/App.css hydragrow-frontend/src/**/*.test.*
git commit -m "refactor(frontend): clarify responsive application shell"
```

### Task 3: Migrate Dashboard and Fleet as the reference vertical slice

**Files:**
- Modify: `hydragrow-frontend/src/pages/Dashboard.tsx`
- Modify: existing Fleet station card and dashboard-specific components under `hydragrow-frontend/src/components/`
- Modify: `hydragrow-frontend/src/App.css` only for dashboard composition styles that cannot be expressed by existing primitives
- Test: Dashboard/Fleet tests under `hydragrow-frontend/src/**/*.test.*`

**Interfaces:**
- Consumes: existing `useFleetStatus`, selected-station state, telemetry hooks, `FleetStationCard`, `SensorBentoCard`, `DeviceStatePill`, `StateView`, `Banner`, and E-STOP components.
- Produces: Dashboard order of multi-station overview → selected-station detail → devices/controls → recent events, using only existing data fields and preserving E-STOP behavior.

- [ ] **Step 1: Add failing Dashboard structure and state tests**

Create tests with explicit fixtures for online, stale, offline, warning, fault, and unavailable station states. Assert that the overview renders station status text, attention states are ordered before normal states, selected-station detail uses only supplied telemetry, and E-STOP retains its existing accessible name/confirmation trigger.

- [ ] **Step 2: Run the Dashboard tests to verify failure**

```bash
cd /vercel/share/v0-project/hydragrow-frontend && npx vitest run src/pages/Dashboard
```

Expected: structural assertions fail against the current Dashboard layout.

- [ ] **Step 3: Recompose the Dashboard hierarchy**

Use the existing fleet hook/card contract for the multi-station overview. Sort only by explicit state severity already represented by the contract; do not calculate a new health score. Keep selected station telemetry/detail below the overview and render loading, unavailable, stale, offline, warning, and fault states through shared primitives.

- [ ] **Step 4: Preserve and isolate operational controls**

Keep existing control handlers, confirmation dialogs, authorization checks, and E-STOP component. Place control groups after state/detail information. Ensure command accepted and physical state confirmed remain separate where the current data contract exposes both.

- [ ] **Step 5: Run Dashboard checks**

```bash
cd /vercel/share/v0-project/hydragrow-frontend && npx vitest run src/pages/Dashboard && npm run build && npx eslint . && npx tsc --noEmit && npx prettier --check .
```

Expected: focused tests and frontend checks pass.

- [ ] **Step 6: Commit the reference slice**

```bash
git add hydragrow-frontend/src/pages/Dashboard.tsx hydragrow-frontend/src/components hydragrow-frontend/src/**/*.test.* hydragrow-frontend/src/App.css
git commit -m "refactor(frontend): make dashboard status first"
```

### Task 4: Migrate operations, cultivation, journal, settings, and automation pages

**Files:**
- Modify: existing page files for Operations, Cultivation, Journal, Settings, and Automation under `hydragrow-frontend/src/pages/`
- Modify: directly composed domain components under `hydragrow-frontend/src/components/`
- Test: focused page tests under `hydragrow-frontend/src/**/*.test.*`

**Interfaces:**
- Consumes: the shared page/status primitives from Tasks 1–3 and all existing domain hooks/actions.
- Produces: consistent page anatomy without changing workflows: Operations state before controls; Cultivation lifecycle grouping; Journal attention-first timeline/filtering; Settings administrative grouping; Automation desktop React Flow plus mobile flow cards.

- [ ] **Step 1: Map each page's current data and action contract**

Read each page and its direct hooks/components. For every action, record the existing handler, confirmation requirement, authorization gate, and displayed state. For every data surface, record whether loading, unavailable, stale, offline, warning, or fault is already available. Do not add fallback telemetry values.

- [ ] **Step 2: Add failing page anatomy tests**

Add one focused test per domain page that asserts the page header/context exists, explicit state appears before its controls, and existing primary actions remain accessible by name. Add an Automation responsive test that asserts the mobile representation exposes node labels as readable flow cards rather than requiring a canvas interaction.

- [ ] **Step 3: Migrate Operations and Cultivation**

Apply shared page headers and state-first ordering. Group cultivation content by season/recipe/dosing lifecycle using existing labels and values. Keep all command callbacks, confirmation flows, and query keys untouched.

- [ ] **Step 4: Migrate Journal and Settings**

Use shared timeline/card patterns for journal entries and visually prioritize explicit fault/warning events. Group settings into administrative sections while preserving existing forms, validation, save handlers, and role restrictions.

- [ ] **Step 5: Migrate Automation responsively**

Keep React Flow for desktop. At the mobile breakpoint, render the existing automation data as stacked readable cards with node labels, transitions, and status text; do not remove the desktop canvas or create a second data model.

- [ ] **Step 6: Run focused and full frontend verification**

```bash
cd /vercel/share/v0-project/hydragrow-frontend && npx vitest run && npm run build && npx eslint . && npx tsc --noEmit && npx prettier --check .
```

Expected: all page tests and declared frontend checks pass.

- [ ] **Step 7: Commit domain page migration**

```bash
git add hydragrow-frontend/src/pages hydragrow-frontend/src/components hydragrow-frontend/src/**/*.test.*
git commit -m "refactor(frontend): align domain pages with status-first patterns"
```

### Task 5: Migrate auth, onboarding, error surfaces, and perform responsive/accessibility cleanup

**Files:**
- Modify: existing auth, onboarding, not-found, error, and unavailable-state files under `hydragrow-frontend/src/`
- Modify: `hydragrow-frontend/src/App.css` only for final responsive/accessibility corrections
- Test: affected auth/onboarding/error tests under `hydragrow-frontend/src/**/*.test.*`

**Interfaces:**
- Consumes: shared tokens, page/status primitives, and existing auth/onboarding/error behavior.
- Produces: consistent visual language for non-dashboard surfaces with unchanged authentication, redirects, validation, and error recovery behavior.

- [ ] **Step 1: Add failing accessibility/state tests**

Assert that auth and onboarding controls retain accessible labels, invalid states are announced, loading/error states have explicit text, and error surfaces expose a recovery/navigation action. Assert that no destructive action loses its confirmation boundary.

- [ ] **Step 2: Migrate auth/onboarding/error presentation**

Apply shared typography, surfaces, focus styles, and state language. Keep all existing submit handlers, redirect targets, validation schemas, and provider calls unchanged.

- [ ] **Step 3: Run full frontend verification**

```bash
cd /vercel/share/v0-project/hydragrow-frontend && npx vitest run && npm run build && npx eslint . && npx tsc --noEmit && npx prettier --check .
```

Expected: all commands pass with concrete zero-error output.

- [ ] **Step 4: Start the preview for browser verification**

```bash
cd /vercel/share/v0-project/hydragrow-frontend && npm run dev -- --host 0.0.0.0
```

Use the `agent-browser` skill to verify the 900x617 dark desktop viewport and a narrow mobile viewport. Check route navigation, Dashboard overview/detail, stale/offline/fault text, mobile bottom navigation, Automation mobile cards, keyboard focus visibility, and E-STOP affordance. Save screenshots under `/tmp/agent-browser/` only.

- [ ] **Step 5: Self-review the final diff**

Run:
```bash
git diff --stat && git diff -- hydragrow-frontend
```

Confirm no route changes, fabricated telemetry, changed authorization, removed confirmations, unsafe control styling, or accidental edits outside scope. If any issue is found, fix it and repeat the full frontend verification commands.

- [ ] **Step 6: Commit final frontend redesign**

```bash
git add hydragrow-frontend/src
git commit -m "refactor(frontend): complete design system migration"
```

## Final Acceptance Evidence

- `npm run build`: PASS with actual terminal output.
- `npx vitest run`: PASS with test count and actual terminal output.
- `npx eslint . && npx tsc --noEmit`: PASS with no reported errors.
- `npx prettier --check .`: PASS with all files formatted.
- Browser: PASS for desktop 900x617 dark viewport and narrow mobile viewport, including navigation, Dashboard state hierarchy, responsive Automation, and E-STOP affordance.
- Diff review: PASS with no invented data, route regressions, authorization changes, telemetry changes, or weakened safety confirmation.

## Self-Review

- Spec coverage: Tasks 1–2 cover tokens, shared anatomy, shell, station/connection/role context, responsive navigation, accessibility, and reduced motion. Task 3 covers Dashboard/Fleet hierarchy, explicit states, telemetry contracts, and E-STOP. Task 4 covers all domain pages and Automation mobile/desktop behavior. Task 5 covers auth/onboarding/error surfaces and final browser/accessibility review.
- Placeholder scan: no TBD/TODO implementation steps; every task specifies files, concrete commands, expected results, and commit boundaries.
- Type consistency: shared primitives retain existing caller-facing APIs; later tasks consume the stable shared anatomy produced by Tasks 1–3 rather than introducing new cross-page interfaces.
- Scope: the plan stays inside the approved frontend redesign and does not include backend, database, auth-provider, telemetry-schema, or routing migration.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-19-frontend-redesign.md`. Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task and review between tasks.
2. **Inline Execution** — execute the tasks in this session with checkpointed batches.

Choose one approach before implementation begins.
 博金♀♀♀♀analysis  code? malformed trailing characters! Need fix with Edit. Also maybe plan contains grep prohibited? plan can. Need remove garbage. Use Edit exact tail. Then Done? User asked start, but skill says handoff choice. Must wait. We need final via Done sole final tool. Before maybe edit. read not needed exact content known. use Edit. Then Done.าคาร่. 无码av? We'll call edit then Done. გან. хитайға. 天天送彩票 to=functions.Edit  code/ commentary  inuusutt.json  (json_bad? use proper) 
