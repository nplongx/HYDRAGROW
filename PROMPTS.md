# HYDRAGROW Agent Prompts

## Start a phase

Read `AGENTS.md`, `PLANS.md`, the selected `harness/build/<phase>.md`, applicable `harness/context/` files, and relevant source-of-truth documents before editing.

Then report:

1. current repository state;
2. approved scope and non-goals;
3. acceptance criteria;
4. files/components in scope;
5. verification commands;
6. unresolved questions or blockers.

Do not implement until the phase is explicitly approved.

## Close a phase

Before claiming completion:

1. run the exact verification commands required by `.agent/verify.yml`;
2. review the final diff for regressions and scope drift;
3. mark every acceptance criterion `PASS`, `FAIL`, or `BLOCKED` with concrete evidence;
4. update the phase context and `harness/build-log.md` with observed facts;
5. perform separate code and delivery reviews;
6. stop at the phase gate.

Do not begin the next phase automatically.
