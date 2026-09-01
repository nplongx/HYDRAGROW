# Plan Execution Governance

An implementation plan is a delivery contract, not only a prompt. For a non-trivial implementation plan, the executing agent must create and maintain a Plan Execution Record in the same PR.

## Mandatory lifecycle

1. **Register** — record the plan name and requirement ID before implementation.
2. **Decompose** — map every numbered task/phase in the plan to a `TASK-*` entry.
3. **Execute** — move each task through `NOT_STARTED` → `IN_PROGRESS` → `DONE` or `BLOCKED`.
4. **Evidence** — every `DONE` task must point to concrete test, artifact, commit, or review evidence.
5. **Synchronize docs** — update the execution record and affected project-state documentation as work progresses, not only at the end.
6. **Close** — the record may become `VERIFIED` only when every task is `DONE` and required acceptance/evidence contracts pass.

## Non-negotiable completion rule

A coding PR implementing a non-trivial plan is **not complete** if the plan execution record is missing, stale, or contains `NOT_STARTED`, `IN_PROGRESS`, or `BLOCKED` tasks.

A task marked `DONE` without evidence is invalid.

If a planned task is intentionally skipped, the record must mark it `BLOCKED` or `NOT_STARTED` and explain why; the PR cannot claim full plan completion.

## Recommended location

```text
docs/project-state/plan-execution/<requirement-id>.json
```

The JSON structure is validated by `docs/schemas/plan-execution-record.schema.json`.
