# HYDRAGROW Delivery Governance

## Purpose

A task is not complete because the code compiles or tests pass. A change is complete only when the requested outcome is verified and the project knowledge is synchronized.

## Definition of Done

Every non-trivial task must satisfy all applicable items:

1. Scope implemented and reviewed.
2. Automated verification passes.
3. Every acceptance criterion has a binary PASS/FAIL result.
4. Quantitative targets have baseline, target, actual value, and evidence when applicable.
5. Integration, staging, hardware, or deployment verification is attached when the requirement depends on a real environment.
6. Required documentation is updated in the same change.
7. `docs/project-state/CURRENT-STATUS.md` is updated when project state changes.
8. Risks, rollback considerations, and known gaps are recorded.
9. Evidence is attached to the PR or linked from a durable artifact.

## Status Vocabulary

- `PROPOSED`: requirement exists, implementation has not started.
- `IMPLEMENTING`: work is in progress.
- `VERIFIED`: implementation and acceptance criteria are verified in the appropriate environment.
- `DEPLOYED`: change has been deployed to the declared target environment.
- `ACCEPTED`: deployment/outcome is verified and documentation is synchronized.
- `BLOCKED`: a required criterion or environment is unavailable; do not claim completion.

## Change Classes

- `C0` Docs/comment-only: documentation consistency review.
- `C1` Isolated implementation: code + tests.
- `C2` API/contract/schema: code + contract compatibility + docs.
- `C3` Database/migration: design review + migration evidence + rollback plan.
- `C4` Cross-subsystem: integration verification + traceability update.
- `C5` CI/CD/infrastructure: operational verification + runbook/docs update.
- `C6` Behavior/performance: benchmark or scenario evidence with baseline/target.
- `C7` Security-sensitive: security review and evidence.

## Requirement Traceability

Each feature or material fix should be traceable as:

`Requirement -> Acceptance Criteria -> Implementation -> Test/Evidence -> Deployment -> Documentation`

The canonical project-level view is `docs/project-state/TRACEABILITY.md`.

## Documentation Sync Rule

Changes that affect behavior, contracts, architecture, operations, deployment, or project status MUST update the corresponding documentation in the same PR. A passing CI build does not waive this rule.

## Jules Review Rule

Jules must perform two separate assessments:

- **Code review:** correctness, architecture, security, tests, regressions.
- **Delivery review:** requirement coverage, measurable outcome, integration/deployment evidence, and documentation synchronization.

`LGTM` is not a valid delivery verdict when acceptance evidence or required documentation is missing.
