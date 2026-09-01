# HYDRAGROW Change Management

## Change lifecycle

Every material system change follows:

`PROPOSED -> DESIGNED -> IMPLEMENTING -> VERIFIED -> DEPLOYED -> ACCEPTED`

A change may become `BLOCKED` whenever required evidence, environment, hardware, decision, or approval is unavailable.

## Before implementation

1. Identify the requirement or issue.
2. State the intended system outcome.
3. Classify the change (`C0`-`C7`) using `docs/DELIVERY-GOVERNANCE.md`.
4. Define falsifiable acceptance criteria.
5. Define baseline/target for measurable behavior or performance.
6. Identify affected subsystems and documentation.
7. Identify deployment environment, verification scenario, and rollback path when applicable.

## During implementation

- Keep the PR scoped to the approved objective.
- Update shared contracts and all affected consumers together.
- Add tests at the lowest useful level and integration/scenario tests when behavior crosses subsystem boundaries.
- Do not weaken tests or requirements to obtain green CI.

## Before merge

The PR must contain:

- Requirement and objective.
- Acceptance criteria with results.
- Verification evidence.
- Required documentation updates.
- Project-state/traceability updates for material changes.
- Deployment evidence or an explicit statement that deployment verification is not applicable.
- Risks and rollback considerations.

## After deployment

Record the deployed environment and build/version, verify the expected scenario, and promote the requirement to `ACCEPTED` only when outcome evidence and documentation are synchronized.

## Emergency changes

Emergency changes may use an abbreviated path only when delaying the change creates greater operational risk. The PR must still record the reason, verification performed, residual risk, and required follow-up documentation.
