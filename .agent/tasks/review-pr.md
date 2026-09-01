# [Jules Review] PR #{{PR_NUMBER}} — {{PR_TITLE}}

> **Thay thế `{{PR_NUMBER}}` và `{{PR_TITLE}}` trước khi dispatch.**
> Dùng: `./scripts/jules review --pr <number>` để tự điền.

## Task type: Code + Delivery Review

Jules must assess both implementation correctness and whether the requested system outcome has actually been demonstrated. Green CI is not, by itself, evidence that the requirement is fulfilled.

---

## Phase 1 — Discovery (viết KHÔNG code)

- [ ] Checkout PR branch: `gh pr checkout {{PR_NUMBER}}`
- [ ] Read `AGENTS.md`, `.agent/rules/jules-protocol.md`, and `docs/DELIVERY-GOVERNANCE.md`.
- [ ] Read module-rule for every touched subsystem.
- [ ] List changed files: `git diff main...HEAD --name-only`
- [ ] Identify subsystem(s) and Change Class (`C0`-`C7`).
- [ ] Identify Requirement / Issue ID and acceptance criteria from the PR body.
- [ ] Identify required documentation and project-state artifacts.

## Phase 2 — Static / Automated Verification

Run the applicable existing project verification commands and paste relevant output. Do not weaken tests or requirements to obtain a pass.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace

# Frontend when hydragrow-frontend/ is touched
cd hydragrow-frontend
npx tsc --noEmit
npx eslint .
npx vitest run
```

## Phase 3 — Delivery Acceptance

### 3.1 Requirement traceability

Build this table from the actual PR, issue, and repository state:

| Acceptance ID | Requirement / expected outcome | Target | Actual | Evidence | Result |
|---|---|---|---|---|---|
| AC-1 | ... | ... | ... | PR artifact / test / deployment record | PASS/FAIL |

Rules:

- Every stated acceptance criterion must have a result.
- A test passing proves the tested behavior, not the entire product requirement.
- Quantitative requirements require baseline, target, actual value, and reproducible evidence.
- If evidence is unavailable, mark the criterion `FAIL` or `BLOCKED`; do not infer success.

### 3.2 Deployment / integration outcome

When the requirement depends on multiple services, staging, hardware, firmware, network behavior, or real deployment, verify:

| Item | Expected | Actual | Evidence | Result |
|---|---|---|---|---|
| Environment | ... | ... | ... | PASS/FAIL |
| Build/version | ... | ... | ... | PASS/FAIL |
| Scenario | ... | ... | ... | PASS/FAIL |
| Rollback | ... | ... | ... | PASS/FAIL |

If no real-environment verification is required, explicitly state why.

### 3.3 Documentation synchronization

Check:

- [ ] Architecture/API/contract docs updated when behavior or interfaces changed.
- [ ] Operations/runbook/deployment docs updated when operational behavior changed.
- [ ] `docs/project-state/CURRENT-STATUS.md` updated when project state changed.
- [ ] `docs/project-state/TRACEABILITY.md` updated for material requirements.
- [ ] No contradictory documentation remains.

Missing required documentation is a delivery finding, even when code and tests are green.

---

## Phase 4 — Code Review Findings

**Module-rules compliance**
- [ ] SQL/DB logic only in `db/` module.
- [ ] MQTT topic constants come from `hydragrow-shared`.
- [ ] Shared type changes update all affected consumers in the same PR.
- [ ] No unsafe `unwrap()`/`.expect()` on production paths.

**Code quality**
- [ ] No dead code or unused imports.
- [ ] Correct error handling.
- [ ] Naming and architecture follow repository conventions.

**Security**
- [ ] No hardcoded secrets/credentials.
- [ ] Protected routes use the appropriate auth middleware.
- [ ] Inputs are validated before persistence.

**Test quality**
- [ ] New logic has meaningful assertions.
- [ ] Tests describe behavior and relevant edge cases.

---

## Output format

Jules post comment với cấu trúc:

```markdown
## Review Summary — PR #{{PR_NUMBER}}

### Delivery Acceptance
| Criterion | Target | Actual | Evidence | Result |
|---|---|---|---|---|
| ... | ... | ... | ... | PASS/FAIL/BLOCKED |

### Documentation / Project State
- Required docs: PASS/FAIL
- CURRENT-STATUS: PASS/FAIL/N/A
- TRACEABILITY: PASS/FAIL/N/A

### Critical (block merge)
| File | Line | Issue |
|---|---|---|
| ... | ... | ... |

### Warning (should fix)
| File | Line | Issue |
|---|---|---|
| ... | ... | ... |

### Verification
| Check | Command | Result | Notes |
|---|---|---|---|
| ... | ... | PASS/FAIL | ... |

### Verdict
- [ ] ACCEPTED — code, outcome, evidence and docs all pass
- [ ] NEEDS CHANGES — one or more required items fail
- [ ] BLOCKED — required external environment/evidence is unavailable
```

**Important:** `LGTM` is reserved for code review only. The delivery verdict must be `ACCEPTED`, `NEEDS CHANGES`, or `BLOCKED`.

For every Critical/Warning, Jules creates a finding issue using `jules-finding`.

---

*Template: `.agent/tasks/review-pr.md` | Dispatch: `./scripts/jules review --pr {{PR_NUMBER}}`*
