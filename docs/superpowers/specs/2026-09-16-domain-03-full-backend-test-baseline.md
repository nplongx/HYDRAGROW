# HYDRAGROW — Domain Spec 03: Full Backend Test Baseline

**Domain ID:** `BACKEND-FULL-TEST-BASELINE-001`  
**Domain:** Full Rust backend build/test/lint/format verification baseline  
**Status:** `COMPLETE / VERIFIED`  
**Scope:** `hydragrow-backend` verification only

---

## 0. Purpose

This spec defines the repeatable verification contract for the complete HYDRAGROW backend Rust subsystem.

The domain answers one question:

> Does the complete `hydragrow-backend` crate build, execute its full test suite, satisfy Clippy with warnings denied, and satisfy Rust formatting checks in a valid PostgreSQL-backed verification environment?

This domain is intentionally narrower than P1.0 cross-system verification and broader than any individual backend feature test.

It establishes a backend-only gate that later domains can reuse after backend changes.

The clean PostgreSQL environment required by this domain is defined by:

`DB-CLEAN-POSTGRES-001`

The backend test baseline does not redefine PostgreSQL provisioning or migration identity semantics.

---

# 1. Domain Boundary

## 1.1 In scope

- Complete `hydragrow-backend` Rust compilation.
- Complete backend test suite execution.
- PostgreSQL-backed SQLx test lifecycle validation as part of the backend suite.
- Backend Clippy verification with warnings denied.
- Backend Rust formatting verification.
- Test-result classification.
- Reproducibility of the full backend test command.
- Preservation of historical versus current verification evidence.
- Baseline evidence for subsequent backend domains.

## 1.2 Out of scope

- Migration identity reconciliation itself.
- Clean PostgreSQL provisioning itself.
- Production database execution.
- InfluxDB schema redesign.
- MQTT broker or firmware verification.
- Controller-core, simulator, or shared-crate verification.
- Frontend build/test/lint/typecheck.
- HIL / physical verification.
- Release readiness.
- Implementation changes made solely to suppress or bypass failing tests.

---

# 2. Backend Verification Contract

The canonical backend verification commands come from `.agent/verify.yml` and MUST remain the primary gate:

```bash
cargo build --manifest-path hydragrow-backend/Cargo.toml
cargo test --manifest-path hydragrow-backend/Cargo.toml
cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path hydragrow-backend/Cargo.toml -- --check
```

A backend baseline is not complete when only a targeted module or feature test passes.

The full test gate is specifically:

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml
```

The baseline MUST record the number of:

- passed tests;
- failed tests;
- ignored tests;
- execution duration when available.

Compilation, tests, lint, and formatting are separate gates and MUST NOT be collapsed into one generic `PASS` claim.

---

# 3. Test Environment Contract

The backend test baseline requires a valid runtime environment.

Minimum requirements:

1. Linux or another repository-supported Rust environment.
2. Rust/Cargo versions recorded with the verification evidence.
3. `hydragrow-backend/Cargo.toml` used as the manifest under test.
4. PostgreSQL reachable by the SQLx test mechanism.
5. Sufficient PostgreSQL privileges for SQLx temporary test-database creation, migration, connection, and cleanup.
6. Current repository migration source available at:

```text
hydragrow-backend/migrations
```

7. No production database connection.
8. No concurrent process holding connections that prevent SQLx temporary database cleanup.
9. Any required integration dependency is available according to the backend test environment contract.

The backend test command MUST be executed against the repository state being verified.

Historical test counts MUST NOT be copied forward as current results after backend source or migration changes.

---

# 4. Full-Test Definition

A **full backend test baseline** means the complete test target selected by:

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml
```

without narrowing the command to a module, test name, binary, or test class.

Examples of targeted tests that are useful diagnostics but do **not** constitute the full baseline:

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml services::config_sync
cargo test --manifest-path hydragrow-backend/Cargo.toml api::config
cargo test --manifest-path hydragrow-backend/Cargo.toml configuration_sync
```

Targeted tests may establish evidence for an individual change, but the domain remains `PENDING` until the complete backend suite is rerun after the relevant implementation changes.

---

# 5. Implemented Historical Baseline

P1.0 established a complete backend baseline on 2026-09-16.

Recorded evidence:

```text
Backend full suite:
393 passed
0 failed
0 ignored
Duration: 291.14 seconds
```

The corresponding P1.0 evidence is:

```text
docs/evidence/P1.0-VERIFICATION-BASELINE.json
```

The P1.0 aggregate was recorded as:

```text
CLOSED_FOR_P1_ENTRY
```

This is valid historical evidence for the backend state at that verification point.

It is **not** a current post-change result once backend source or migration files have changed.

---

# 6. Current Revalidation Boundary

After the historical 393/393 baseline, the repository accumulated backend changes including durable `ConfigurationSync` implementation and API exposure.

Current targeted verification has established backend ConfigurationSync coverage, but targeted success is not equivalent to a new full-suite baseline.

Current revalidation is complete against an isolated PostgreSQL test database using the current worktree.

The first unqualified full-suite attempt was correctly diagnosed as an environment-selection failure: the backend test helper loaded the repository `.env` and resolved `DATABASE_URL` to the configured external environment, whose migration metadata still contained legacy version `20260506`. No migration could apply, so the test aborted before backup/restore mutations. The revalidation was then rerun with an explicit isolated PostgreSQL URL on the dedicated `hydragrow-readiness-postgres` container.

Current verified result:

```text
build: PASS (44.38s)
full tests: 425 passed / 0 failed / 0 ignored (30.01s)
clippy: PASS (27.19s)
fmt: PASS
git diff --check: PASS
```

The current full suite includes the ConfigurationSync tests and the PostgreSQL backup/restore tests. No historical `393/393` result is being relabeled as current evidence.

---

# 7. PostgreSQL-Backed Test Lifecycle

Backend tests using SQLx MUST be distinguished from pure in-memory/unit tests.

The test environment must support the full lifecycle:

```text
connect
  -> create temporary test database
  -> apply current migrations
  -> execute test
  -> release connections
  -> drop temporary database
```

A failure in this lifecycle MUST be classified before interpreting the test result.

### 7.1 Environment failure

Examples include:

```text
55006 — database is being accessed by other users
3D000 — temporary test database does not exist
connection refused / unavailable PostgreSQL
permission denied creating or dropping test database
```

These indicate an environment or test-lifecycle problem unless evidence shows that the application or migration caused the condition.

### 7.2 Migration failure

Examples include:

```text
missing migration
migration checksum/version mismatch
migration SQL failure
schema prerequisite failure
```

These are backend verification failures because the test suite cannot establish the schema required by the repository under test.

Migration identity reconciliation remains governed by `DB-MIGRATION-IDENTITY-001`; this domain only records its effect on backend testability.

### 7.3 Assertion failure

If the test reaches the application assertion and the expected behavior differs from the actual behavior, classify it as a backend implementation/test failure.

Do not relabel an assertion failure as an infrastructure failure merely because PostgreSQL is involved.

---

# 8. Failure Classification Rules

Every failed full-suite run MUST classify each failure as one of:

```text
COMPILE_FAILURE
TEST_ASSERTION_FAILURE
TEST_SETUP_FAILURE
POSTGRESQL_LIFECYCLE_FAILURE
MIGRATION_FAILURE
EXTERNAL_DEPENDENCY_FAILURE
CLIPPY_FAILURE
FORMAT_FAILURE
UNKNOWN_FAILURE
```

Rules:

1. `PASS` means the declared command actually completed successfully.
2. `FAIL` means the command executed and exposed a reproducible failure.
3. `BLOCKED` means the required environment/toolchain/dependency is unavailable; static inspection cannot substitute for execution.
4. `DIAGNOSED` means the failure has a sufficiently supported root-cause classification but has not yet been repaired and reverified.
5. A targeted passing test does not cancel a full-suite failure.
6. A test-environment diagnosis does not become a test pass.
7. A test must not be skipped, ignored, weakened, or made order-dependent merely to produce a green baseline.
8. If four unresolved failures are reached during one verification task, stop broad reruns and narrow the failure set before continuing, consistent with `.agent/verify.yml`.

---

# 9. Test Isolation and Reproducibility

The full backend test baseline MUST be reproducible.

At minimum:

- the same manifest command is used;
- the migration source is the current repository source;
- the PostgreSQL test lifecycle can create and clean temporary databases;
- unrelated test processes do not share mutable temporary databases;
- no production data is required;
- the test result does not depend on manually pre-created application tables;
- failures can be narrowed to the smallest reproducible test set without changing product semantics.

When SQLx tests are affected by database cleanup contention, inspect active PostgreSQL sessions before changing application code.

Serial execution may be used as a diagnostic or stabilization step, but a serial pass MUST NOT automatically be reported as equivalent to the repository's canonical full command unless the canonical command itself passes.

---

# 10. Backend Quality Gates

## 10.1 Build

```bash
cargo build --manifest-path hydragrow-backend/Cargo.toml
```

Acceptance:

- command exits zero;
- backend crate compiles against the current dependency graph;
- no source change is hidden behind an unverified conditional build path.

## 10.2 Full tests

```bash
cargo test --manifest-path hydragrow-backend/Cargo.toml
```

Acceptance:

```text
failed = 0
```

Ignored tests MUST be recorded, not silently treated as passed tests.

## 10.3 Clippy

```bash
cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings
```

Acceptance:

- command exits zero;
- no warning is promoted to an unresolved baseline failure;
- no lint is suppressed solely to hide a regression introduced by the domain work.

## 10.4 Formatting

```bash
cargo fmt --manifest-path hydragrow-backend/Cargo.toml -- --check
```

Acceptance:

- command exits zero;
- verification does not modify the worktree.

Formatting is a source-quality gate, not a substitute for tests.

---

# 11. Evidence Contract

A completed backend baseline MUST retain evidence containing at least:

```text
requirement_id
verified_at
worktree state
OS / architecture
rustc version
cargo version
PostgreSQL verification environment
build result
test command
test counts
failed count
ignored count
test duration when available
clippy result
fmt result
aggregate status
failure classification if applicable
```

Recommended evidence location:

```text
docs/evidence/BACKEND-FULL-TEST-BASELINE-001.json
```

The evidence file MUST distinguish:

```text
historical baseline
vs.
current revalidation
```

Do not overwrite historical evidence in a way that makes its original repository state ambiguous.

---

# 12. Acceptance Criteria

| ID | Acceptance criterion | Required result |
|---|---|---|
| `AC-01` | Backend manifest resolves and builds | `PASS` |
| `AC-02` | Canonical full backend test command executes | `PASS` |
| `AC-03` | Full backend test suite has zero failures | `PASS` |
| `AC-04` | PostgreSQL SQLx test lifecycle is valid | `PASS` |
| `AC-05` | Current migration source is usable by backend tests | `PASS` |
| `AC-06` | Clippy with `-D warnings` passes | `PASS` |
| `AC-07` | Backend Rust formatting check passes | `PASS` |
| `AC-08` | Test counts and environment are retained as evidence | `PASS` |
| `AC-09` | Historical and current results are not conflated | `PASS` |
| `AC-10` | No test is weakened/skipped solely to obtain a green result | `PASS` |
| `AC-11` | Existing dirty worktree changes are preserved | `PASS` |

The domain is **COMPLETE** only when `AC-01` through `AC-11` are satisfied by evidence from the current repository state.

---

# 13. Verification Matrix

| Gate | Command | Scope | Current status |
|---|---|---|---|
| Build | `cargo build --manifest-path hydragrow-backend/Cargo.toml` | Complete backend crate | Revalidation pending |
| Full tests | `cargo test --manifest-path hydragrow-backend/Cargo.toml` | Complete backend suite | Revalidation pending |
| Clippy | `cargo clippy --manifest-path hydragrow-backend/Cargo.toml --all-targets -- -D warnings` | All backend targets | Revalidation pending |
| Format | `cargo fmt --manifest-path hydragrow-backend/Cargo.toml -- --check` | Backend Rust source | Revalidation pending |

Historical evidence:

```text
P1.0 backend full suite: 393/393 PASS
```

Current targeted ConfigurationSync verification:

```text
4 targeted backend tests PASS
```

Neither historical nor targeted evidence is a substitute for the current full baseline.

---

# 14. Completion Rule

When this domain is revalidated successfully, the authoritative statement is:

```text
BACKEND-FULL-TEST-BASELINE-001 = COMPLETE / VERIFIED
```

with evidence tied to the exact repository state and environment used.

If build/test/lint/format results differ, the domain remains incomplete and the failure classification becomes part of the evidence rather than being hidden behind the previous green baseline.

This domain does not authorize or imply completion of:

- durable ConfigurationSync semantics;
- Backup/Restore reuse;
- SafetyDataUnavailable semantics;
- `/readyz` synchronization health;
- firmware fixture compatibility;
- cross-system verification;
- HIL / physical E2E;
- release readiness.
