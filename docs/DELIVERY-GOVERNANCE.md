# HYDRAGROW Delivery Governance

The purpose of delivery governance is simple: a change is not finished when the code merely compiles. It is finished when the intended outcome is verified and the project documentation reflects what actually happened.

## Standard flow

**Plan → Implement → Verify → Update project state/docs → Review → Merge**

### Before implementation

- Read the complete implementation plan or requirement.
- Identify the intended outcome, tasks, affected systems, and documentation that will become stale.
- Start/update `docs/project-state/CURRENT-STATUS.md`.

### During implementation

- Follow the plan's task order unless a documented dependency requires a deviation.
- Keep tests and verification close to the change.
- Update `CURRENT-STATUS.md` as meaningful milestones are completed.
- Update affected architecture, API, operations, runbook, or user-facing documentation in the same change.

### Before completion

- Verify every task and stated acceptance criterion.
- Record actual results, not intentions.
- Record blockers or deviations explicitly.
- Ensure `CURRENT-STATUS.md` and other affected documentation describe the resulting state.
- The PR description must summarize completion, remaining work, verification, and documentation changes.

## What CI can enforce

CI can reliably enforce repository artifacts and explicit PR declarations. It cannot reliably infer the full task list or determine whether arbitrary prose documentation is semantically complete.

Therefore the repository treats `CURRENT-STATUS.md` and the PR checklist as the lightweight human-readable delivery record, while automated acceptance/evidence gates remain focused on machine-verifiable requirements.

## Completion rule

Do not claim a material implementation is complete when required project-state or affected documentation is missing. Continue the work or report it as blocked.
