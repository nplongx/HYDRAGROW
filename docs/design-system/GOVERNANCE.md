# Design System Governance (`hydragrow-frontend`)

Concrete rules for who decides what, what is frozen, and when a UI/data
mismatch needs design review. Written against `AGENTS.md` and
`docs/DELIVERY-GOVERNANCE.md` as the existing authority — no new process is
invented here.

---

## (a) Token-naming conflicts between parallel agent worktrees

**Resolution maps to the existing mechanism in `AGENTS.md` §3 — no new body,
no new vote, no new lock file:**

1. **One worktree + one branch per session, with a positive file perimeter**
   (`AGENTS.md` §3, "Explicit file ownership for parallel worktrees" +
   "Positive scope, not just negative constraints"). A token rename in
   `hydragrow-frontend/src/App.css` is only legitimate inside a task whose
   declared bounds include `App.css`. A session touching `App.css` without
   that assignment is out of scope — its change loses, full stop.
2. **If two in-scope sessions collide** (both assigned overlapping token
   work), the existing "Rebase before PR" rule decides: fetch latest base,
   rebase, re-run verification. The session that rebases adapts its names to
   whatever already landed; if the rebase leaves an empty diff, the work
   already landed and no PR is opened. First-to-land on the base branch wins;
   the loser renames.
3. **No test weakening to resolve a conflict** (`AGENTS.md` §3): neither side
   may loosen `hardcodedColors` / `orphanClasses` / `contrast` assertions or
   silently extend `colorAllowlist.json` to make its branch green. The Task 1
   closing note applies — allowlist entries require the per-file
   categorical-vs-drift classification review.
4. **Protected-paths and delivery gates backstop the rest**: a token change
   that ships UI code must also touch `docs/project-state/TRACEABILITY.md`
   and `docs/project-state/CURRENT-STATUS.md` (`AGENTS.md` §4), so a silent
   rename without docs fails CI (`delivery-governance.yml`) regardless of
   which worktree wrote it.

Practically: before renaming/adding a `--color-*` token or a shared
`.ui-*`/`.farm-*`/`.page-header*` class, announce the intended name in the
task's worktree branch and grep `src/` for collisions. After landing, the
loser (if any) runs `cd hydragrow-frontend && npx vitest run
src/lib/design-lint/` to confirm its tree is still clean under the new names.

---

## (b) Locked vs free tokens and classes

Counts below are live `grep -rln` results over `hydragrow-frontend/src`
(`--include="*.tsx"`), produced during Tasks 1–2. Threshold: **10+ files =
locked** (rename requires a migration plan + codemod, not a bare edit).

### Locked — do not rename/remove without a migration plan

| Token / class | Files using it | Notes |
|---|---|---|
| `text-primary` | 53 | most-used text utility |
| `border-line` | 43 | card borders, dividers |
| `text-text-muted` | 41 | secondary copy |
| `bg-pill` | 31 | pill backgrounds |
| `text-status` | 30 | status text (`DeviceStatePill` `online`) |
| `text-faint` | 28 | labels/captions; Task 3 fixed to `#556d5e` — contrast guard pins it |
| `ui-card` | 17 | content blocks on nearly every page |
| `app-page` | 11 | mandatory page wrapper (§2–3 of the standard) |

Also locked by policy (regardless of count): `--color-page-bg` (`#dcf0dc`),
`--color-surface` (`#ffffff`), `--color-primary-deep`, `--color-error` — the
contrast test (`contrast.test.ts`) and the EmergencyStopButton guard pin
their values/roles.

### Free to change (with the normal PR checklist)

- Tokens/classes used in **fewer than 10 files**: `bg-success-bg` (4 files),
  `page-header-title` (6 files), `farm-status-pill`, `farm-section-title`,
  `farm-muted-panel`, `.ui-state-icon` (new in Task 2 — 1 definition, 1 user).
- The **categorical automation palette** (`indigo`/`purple`/`rose`/`teal`/
  `orange`/`cyan` inside `src/components/automation/**`, see
  `colorAllowlist.json`): free *within* that directory, forbidden outside it
  (CHUAN §1.1). Do not remove an allowlist entry without confirming the
  categorical scheme has been redesigned onto tokens.
- New tokens for genuinely new semantics may be added to the `@theme` block
  in `src/App.css` — but a new token must ship with a `contrast.test.ts`
  pair if it will ever render text (Task 3 precedent).

### What "locked" forbids vs allows

- Forbids: renaming, deleting, or retargeting the hex behind a locked token
  without updating all call sites in the same PR and re-running the full
  `npx vitest run` suite.
- Allows: adding *new* states/variants alongside (e.g. a new `DeviceStatePill`
  state) — additive changes don't disturb existing call sites.

---

## (c) UI/data mismatches (Discrepancy-3-style): bugfix or design review?

**Rule: standard bugfix if a guard test exists; design review only if no guard
exists yet.**

- `Roles.tsx` Discrepancy 3 (`CAPABILITIES` vs `ROLE_DEFAULT_SCOPES`
  mismatch) was a **data-consistency bugfix**, not a design decision — the
  correct matrix already existed in `ROLE_DEFAULT_SCOPES`, the UI table just
  disagreed with it. It shipped as `fix(roles): align capability matrix with
  ROLE_DEFAULT_SCOPES` with no design review, which was the right call.
- Tasks 1–4 of this audit add exactly the kind of guard that makes future
  mismatches bugfix-class: `hardcodedColors.test.ts` (token drift),
  `orphanClasses.test.ts` (undefined classes), `contrast.test.ts` (AA pairs),
  `EmergencyStopButton.guard.test.tsx` (touch target + color uniqueness).
  When a guard fails, the fix is mechanical (use the token/component the
  guard points at) — file it, fix it, cite the guard in the PR.
- **Design review is required only when no guard covers the conflict** —
  e.g. two plausible token choices for a new semantic state, or a new
  categorical palette that needs an allowlist entry. In that case the PR must
  state the options, the Figma reference (`Hi-Fi — Full App (v1)`), and the
  chosen mapping, and update `docs/design-system/PATTERN-LIBRARY.md` in the
  same PR.
- Until the Roles `CAPABILITIES`/`ROLE_DEFAULT_SCOPES` parity gets an
  automated guard, it stays a manual PR-checklist item
  (`docs/design-system/PR-QA-CHECKLIST.md` item 8) — and any drift found
  there is still a standard bugfix, not a redesign.

- **2026-09-13 Layer 2 audit**: running `hardcodedColors.test.ts`
  against the full tree (not just files touched by a given PR) found
  21 pre-existing violations, none related to the PR that had just
  landed. Three (`SensorBentoCard.tsx`, `ConfigBackup.tsx`,
  `RecipeBuilder.tsx`) are outside every in-flight Layer 2 track and
  were allowlisted with a dated, named comment rather than fixed —
  this is the correct move per this section: a guard failure outside
  a session's declared file perimeter is not that session's bugfix to
  make. The other 18, all inside the six Layer 2 tracks, were fixed as
  part of the track that already had to touch that file.

---

## (d) Layer 3 tracked decisions & Q1 extension points governance

**Rule: Q1 Extension Points are design debt until Q1 resolves.**

1. **Design Debt Classification:** All extension points authored in Layer 3 (dynamic role arrays in `PermissionMatrix`, chunked validation in `InviteForm`, warning sort and grouping in `FleetStationCard`/`FleetView`, and persona hooks in `OnboardingWizard`) represent defensive architectural hooks created to prevent premature commitment to either hobbyist or commercial branches. Until empirical signal (production SQL telemetry or 3/5 archetype interviews) formally unblocks Open Question 1, these extension points are tracked design debt.
2. **Mandatory Documentation Gate:** No feature that is dependent on Open Question 1 (household vs commercial priority) may ship to production without updating `docs/design-system/layer3/Q1-DECISION-FRAMEWORK.md` to record which branch was taken, along with the concrete empirical evidence justifying that branch.
3. **Parity Preservation:** Changes to component props or state machines within Layer 3 components must maintain backward compatibility with single-station household operations while keeping the extension hooks open for commercial tier enablement.

