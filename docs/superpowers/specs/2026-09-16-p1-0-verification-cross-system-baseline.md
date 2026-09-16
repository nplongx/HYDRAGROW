# P1.0 Verification / Cross-System Baseline

## Status

Proposed implementation contract.

P1.0 establishes a reproducible verification baseline for the whole HYDRAGROW system before P1.1 authorization work begins. It is a verification/environment hardening phase, not a second P0 architecture phase.

## Requirement ID

`P1.0-VERIFICATION-BASELINE`

## Change class

`C4` Cross-subsystem + `C5` CI/CD/infrastructure verification.

## Purpose

Establish one explicit, reproducible answer to:

> Which parts of HYDRAGROW currently build, test, lint, format, and integrate successfully, in which environment, with which known blockers?

P1.0 converts the current audit evidence into a repeatable baseline so later P1 work cannot silently inherit an unverified subsystem or mistake an environment failure for a product regression.

P1.0 covers:

- backend Rust verification;
- shared crate verification;
- controller-core verification;
- simulator verification;
- frontend TypeScript/Vite/Vitest/ESLint verification;
- cross-subsystem compilation and event-contract compatibility;
- PostgreSQL-backed integration-test environment readiness;
- verification evidence and traceability.

## 1. Scope boundary

P1.0 is complete only when verification status is explicit for every required subsystem.

P1.0 does **not** implement:

- canonical authorization/ownership;
- durable command lifecycle;
- scheduler fail-closed safety redesign;
- backup/restore redesign;
- Flux query redesign;
- new telemetry/state architecture;
- frontend data-architecture redesign;
- new product behavior solely to make a test green.

Known P1 design gaps remain assigned to their dedicated P1 phases.

## 2. Verification principles

1. A test/build/lint/format result is valid only when the declared command was actually executed.
2. `PASS` means the command completed successfully and its output is retained as evidence.
3. `FAIL` means the command executed and exposed a reproducible failure.
4. `BLOCKED` means the required environment/toolchain/dependency is unavailable or invalid; static inspection is not substituted for runtime success.
5. `DIAGNOSED` may be used for a failure whose root cause is understood but whose environment or implementation has not yet been repaired.
6. No assertion, test, fixture, timeout, ownership check, or error path may be weakened solely to obtain a green baseline.
7. Stop broad test reruns after four unresolved failures in one verification task; narrow the failure first.
8. Existing dirty worktree changes are preserved. Verification must not reset, clean, or overwrite unrelated work.
9. Historical verification evidence is labeled historical and must not be presented as current evidence.
10. Cross-system verification must use the repository's declared commands from `.agent/verify.yml`; commands must not be invented ad hoc as the primary gate.

## 3. Verification matrix

### 3.1 Shared

Required commands from `.agent/verify.yml`:

```text
cargo build --manifest-path hydragrow-shared/Cargo.toml
cargo test --manifest-path hydragrow-shared/Cargo.toml
cargo clippy --manifest-path hydragrow-shared/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path hydragrow-shared/Cargo.toml -- --check
```

Required evidence:

- build result;
- test count and failures;
- clippy result;
- formatting result.

### 3.2 Controller core

Required commands:

```text
cargo build --manifest-path hydragrow-controller-core/Cargo.toml
cargo test --manifest-path hydragrow-controller-core/Cargo.toml
cargo clippy --manifest-path hydragrow-controller-core/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path hydragrow-controller-core/Cargo.toml -- --check
```

The baseline must retain existing safety/FSM regression coverage. A passing unit subset is not sufficient if the declared subsystem test command remains failing.

### 3.3 Simulator

Required commands:

```text
cargo build --manifest-path hydragrow-simulator/Cargo.toml
cargo test --manifest-path hydragrow-simulator/Cargo.toml
cargo clippy --manifest-path hydragrow-simulator/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path hydragrow-simulator/Cargo.toml -- --check
```

The simulator is a cross-system contract consumer. Any new `OrchestratorEvent`, shared event, or controller event must have exhaustive simulator handling before its baseline is considered green.

The `PublishCommandLifecycle` exhaustive-match regression discovered during the P0 residual gate is specifically a P1.0 prerequisite and must remain covered by compilation.

### 3.4 Backend

Required commands:

```text
cargo build --manifest-path hydragrow-backend/Cargo.toml
cargo test --manifest-path hydragrow-backend/Cargo.toml
cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path hydragrow-backend/Cargo.toml -- --check
```

Backend verification must distinguish:

- compilation failure;
- unit-test failure;
- PostgreSQL test-environment failure;
- InfluxDB/integration dependency failure;
- lint failure;
- formatting failure.

A DB-backed failure such as SQLx test-database lifecycle contention is not evidence that the underlying domain behavior is wrong unless the test reaches and fails the relevant assertion.

Conversely, an environment diagnosis does not constitute a passing test.

### 3.5 Frontend

Gate commands:

```text
cd hydragrow-frontend && npm run build
cd hydragrow-frontend && npx vitest run
cd hydragrow-frontend && npx eslint . && npx tsc --noEmit
```

Formatter diagnostic:

```text
cd hydragrow-frontend && npx prettier --check .
```

The formatter command is diagnostic only until the repository has a checked-in Prettier dependency/configuration baseline. P1.0 must not mass-format unrelated dirty worktree files merely to manufacture a formatting pass. Existing formatter drift is recorded as `DIAGNOSED` and deferred to a dedicated formatting task.

The frontend baseline must explicitly record the Node/npm/npx availability and versions used.

The current known environment limitation is that the required system-level Node runtime is unavailable, even though repository-local `node_modules/.bin` tools exist. Local binaries without a working Node runtime do not constitute a frontend verification pass.

If the environment is repaired, execute the declared commands rather than relying on prior static inspection.

## 4. PostgreSQL test environment contract

Backend tests using `#[sqlx::test]` require a valid PostgreSQL test lifecycle.

The verification environment must provide:

1. PostgreSQL reachable by the configured SQLx test mechanism.
2. A role with permission to create/drop the temporary SQLx test databases required by `#[sqlx::test]`.
3. A clean migration source matching `hydragrow-backend/migrations`.
4. No unrelated process retaining connections to SQLx temporary databases when SQLx attempts cleanup.
5. Deterministic test execution when the test suite is run with the declared command.
6. Enough isolation that one test run cannot destroy or invalidate another test database.

The repository's Docker development database is not automatically equivalent to a valid SQLx test lifecycle. The verification record must identify the actual PostgreSQL endpoint and test-database mechanism used.

The current observed failures include:

```text
failed to connect to setup test database
code: 55006
message: database "_sqlx_test_..." is being accessed by other users
```

and secondary failures such as:

```text
code: 3D000
message: database "_sqlx_test_..." does not exist
```

and, in an invalid/non-migrated test database:

```text
code: 42P01
message: relation "users" does not exist
```

These must be resolved at the verification-environment level or traced to a concrete implementation/migration defect before the backend DB baseline can be marked `PASS`.

### 4.1 DB diagnosis rule

When a DB-backed test fails:

1. identify whether setup, migration, connection lifecycle, fixture, or application assertion failed;
2. reproduce the smallest affected test set;
3. inspect active PostgreSQL sessions when lifecycle contention is suspected;
4. verify migrations independently;
5. rerun serially only after the environment issue is understood;
6. do not alter ownership/configuration semantics to mask the failure.

## 5. Cross-system contract verification

P1.0 must verify that shared event/schema changes compile through every known consumer.

At minimum:

```text
hydragrow-shared
        |
        +--> hydragrow-controller-core
        |
        +--> hydragrow-simulator
        |
        +--> hydragrow-backend
```

Required checks:

- shared event enum changes are exhaustive across controller/simulator/backend consumers;
- command lifecycle events compile end-to-end through the event type boundary;
- serialized event shapes used across subsystem boundaries remain compatible;
- simulator dispatch does not silently discard a physical actuator event;
- backend telemetry/event consumers compile against the same shared definitions;
- no cross-system verification relies only on a frontend TypeScript model that diverges from the Rust contract.

P1.0 does not require a new generated-schema system. Schema alignment is a later P1 concern unless a current mismatch blocks verification.

## 6. Frontend runtime verification contract

When Node tooling is available, verify:

1. TypeScript compilation succeeds.
2. Vite production build succeeds.
3. Vitest suite succeeds.
4. ESLint succeeds.
5. Prettier status is recorded; a missing/unconfigured repository formatter is `DIAGNOSED`, not a reason to mass-format unrelated worktree changes.
6. P0.5 boundary tests execute, not merely compile.
7. Station switching/data isolation tests execute.
8. realtime/unknown/stale/error semantics execute where covered by existing tests.

When Node tooling is unavailable:

- mark frontend runtime verification `BLOCKED`;
- record the exact executable/toolchain limitation;
- retain static/source-audit evidence separately;
- do not call the frontend baseline `PASS`.

## 7. Verification evidence record

For every subsystem, record:

```text
Requirement ID:
Subsystem:
Environment:
Command:
Commit/worktree state:
Start/end time:
Result: PASS | FAIL | BLOCKED | DIAGNOSED
Observed output:
Failure class, if any:
Root cause, if known:
Follow-up issue/spec:
```

The record must identify whether evidence was generated from the current worktree. Historical evidence may be referenced but cannot replace current evidence.

## 8. Baseline status model

The whole-system baseline has these aggregate states:

```text
GREEN      all required checks for the scope passed
YELLOW     non-critical check is blocked but impact is bounded and recorded
RED        required executable verification failed
BLOCKED    required environment/toolchain is unavailable
```

For individual checks use `PASS`, `FAIL`, `BLOCKED`, or `DIAGNOSED`.

Aggregate `GREEN` is prohibited while any required subsystem remains `FAIL` or unclassified `BLOCKED`.

A security-sensitive or cross-system verification failure remains a release/P1 gate even if unrelated unit tests are green.

## 9. P1 entry gate

P1.1 may begin after P1.0 has established a trustworthy baseline, meaning:

1. simulator compilation is green;
2. shared/controller/backend declared build and lint/format status is known;
3. backend DB test-environment failures are either repaired or explicitly isolated as an external infrastructure blocker with reproducible evidence;
4. frontend runtime build/test/lint/typecheck status is explicitly PASS or BLOCKED with the exact toolchain limitation;
5. cross-system event consumers compile;
6. current P0 architecture is not reopened as part of verification cleanup;
7. every remaining failure has an owner, root-cause classification, and follow-up boundary.

P1.0 does not require every historical backend integration failure to be fixed if the failure is outside P1.0 scope and has a concrete owner. It does require that the failure be reproducible, understood, and prevented from being mistaken for a green baseline.

## 10. Acceptance criteria

### AC-1 - Declared verification commands

Every in-scope subsystem has an executed verification record using the commands declared in `.agent/verify.yml`.

### AC-2 - Simulator contract

The simulator builds and tests successfully after all current shared/controller event variants are handled exhaustively.

### AC-3 - Backend format/build baseline

Backend build and formatting checks execute successfully; any remaining backend test failure is classified by root cause rather than hidden.

### AC-4 - PostgreSQL lifecycle

A repeatable SQLx PostgreSQL test environment exists, and DB-backed test failures caused by test-database lifecycle contention are no longer presented as product-test failures.

### AC-5 - Frontend runtime status

Frontend build/test/lint/typecheck status is backed by execution evidence, or the exact unavailable runtime/toolchain is recorded as `BLOCKED`.

### AC-6 - Cross-system compatibility

Shared/controller/backend/simulator event contracts compile across all current consumers.

### AC-7 - Evidence integrity

No PASS is asserted from static inspection, historical output, or partial test subsets when the declared gate command has not passed.

### AC-8 - P1 boundary

P1.0 changes do not implement or redefine P1.1 authorization, P1.2 durable command lifecycle, or other P1 feature semantics.

### AC-9 - Traceability

The verification baseline and all unresolved blockers are recorded in the project-state traceability/status documentation required by Delivery Governance.

## 11. Non-goals

- fixing every backend domain failure discovered during the audit;
- redesigning SQLx itself;
- replacing PostgreSQL;
- introducing a new CI platform;
- implementing authorization;
- implementing durable command persistence;
- changing command lifecycle semantics;
- changing safety policy;
- changing frontend architecture;
- generating a new shared schema layer solely for this baseline.

## 12. Implementation order

```text
P1.0 Verification / Cross-System Baseline
    |
    +-- 1. Freeze current evidence and worktree state
    |
    +-- 2. Verify shared
    |
    +-- 3. Verify controller-core
    |
    +-- 4. Verify simulator
    |
    +-- 5. Diagnose/verify backend PostgreSQL test lifecycle
    |
    +-- 6. Verify backend build/test/lint/fmt
    |
    +-- 7. Verify frontend runtime/toolchain
    |
    +-- 8. Run cross-system event/build checks
    |
    +-- 9. Record evidence + traceability
    |
    v
P1.1 Authorization / Ownership Boundary
```

## 13. Known baseline entering P1.0

The following are known from the preceding P0 residual/whole-system audit and must be carried into the baseline rather than rediscovered:

- simulator previously failed compilation because `PublishCommandLifecycle` was not handled exhaustively; this regression has since been patched and the simulator test suite has executed successfully;
- backend formatting drift in `hydragrow-backend/src/mqtt/handlers/sensors.rs` has since been corrected and `cargo fmt --check` has passed;
- backend SQLx tests have shown temporary-database lifecycle contention (`55006`) and secondary missing-database/missing-relation failures under unstable runs;
- frontend runtime verification remains unavailable when the required Node runtime is absent, even when repository-local JavaScript packages are installed;
- historical backend/controller verification evidence exists but is not current baseline evidence.

## 14. Completion rule

P1.0 is complete when the repository has a reproducible, current verification matrix with binary evidence for every executable check that can run in the declared environment, explicit `BLOCKED` status for unavailable environments, and concrete classification/follow-up for every remaining failure.

Completion does not mean every subsystem is green. It means the system's verification state is trustworthy enough that subsequent P1 implementation work can distinguish product regressions from environment limitations and unrelated pre-existing failures.
