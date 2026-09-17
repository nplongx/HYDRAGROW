# HYDRAGROW — Domain Spec 10: Full Cross-System Verification

**Domain ID:** `DOMAIN-10-FULL-CROSS-SYSTEM-VERIFICATION`  
**Domain:** Full cross-system verification  
**Status:** `SPECIFIED / IMPLEMENTATION IN PROGRESS`

## 1. Objective

Verify the integrated HYDRAGROW stack after Domains 1–9 without redesigning contracts or introducing new feature scope.

Verification must cover the repository-owned boundaries that were changed by the roadmap:

- shared canonical schema/fixtures;
- backend configuration synchronization, backup/restore, safety-data error semantics, readiness;
- frontend configuration synchronization/status behavior;
- firmware C++ sensor and Rust controller canonical fixture consumers;
- CI coupling for schema and firmware consumers.

This domain is verification-first. Production behavior changes are out of scope unless an executable cross-system failure proves a defect introduced by the roadmap and the minimal fix stays inside this domain.

## 2. Preconditions

- Preserve dirty worktree. No reset, clean, revert, or stash.
- Never connect to production PostgreSQL.
- Use isolated PostgreSQL only where backend integration tests require it.
- Before ESP-IDF firmware build/test commands, explicitly run:
  `source ~/export-esp.sh`
- Treat `Unsupported target x86_64-unknown-linux-gnu` as a firmware toolchain limitation, not backend failure.
- Do not claim verification from static inspection or compilation alone where executable tests are required.

## 3. Verification contract

### V-01 Shared contracts

Run canonical schema/fixture regression and full shared crate verification:

```text
cargo test --manifest-path hydragrow-shared/Cargo.toml --test p1_9_schema_registry
cargo test --manifest-path hydragrow-shared/Cargo.toml
cargo clippy --manifest-path hydragrow-shared/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path hydragrow-shared/Cargo.toml -- --check
```

### V-02 Backend

Run authoritative backend build/test/lint/format commands from `.agent/verify.yml`.

Where integration tests require PostgreSQL, use only the isolated readiness database/container. Record exact migration and test results.

### V-03 Frontend

Run authoritative frontend build/test/lint/format commands from `.agent/verify.yml`.

Cross-system assertions must cover configuration mutation, query invalidation/status propagation, and explicit unavailable/error state handling where tests already encode these contracts.

### V-04 Firmware sensor

Run the Domain 8 native C++ fixture consumer test from repository root:

```text
pio test --project-dir ESP32-C3-SENSOR-NODE --environment native
```

Expected: canonical fixture consumer passes and fixture resolution is independent of process CWD.

### V-05 Firmware controller

Load ESP-IDF environment before any controller build/test:

```text
source ~/export-esp.sh
```

Then run the repository-supported controller build/check path. Run controller canonical fixture tests only where the configured target/runner can execute them. Compilation is not equivalent to test execution.

If no physical ESP32-C3 is available and `espflash` cannot execute unit tests, record the exact blocker. Do not substitute unsupported host-target execution and do not label Domain 9 fixture tests verified.

### V-06 Cross-system contract checks

Verify, using executable tests or deterministic scripts where available:

1. canonical fixture fields agree with firmware consumer types and shared types;
2. legacy command top-level fields are not emitted/accepted as canonical output;
3. configuration revision/status representation is consistent across backend API, frontend model/query state, MQTT/controller state;
4. readiness reports required synchronization dependencies consistently;
5. safety-data unavailable semantics remain explicit and fail-closed;
6. backup/restore uses ConfigurationSync rather than a parallel synchronization path.

Do not invent a new schema or API contract for these checks.

### V-07 Repository integrity

Run:

```text
git diff --check
git status --short
```

Confirm no unrelated dirty files were reset or reverted and no production PostgreSQL activity occurred.

## 4. Acceptance criteria

- **AC-01:** Shared canonical contract regression passes.
- **AC-02:** Backend authoritative verification passes, or each unresolved failure has exact executable evidence and classification.
- **AC-03:** Frontend authoritative verification passes, or each unresolved failure has exact executable evidence and classification.
- **AC-04:** Domain 8 sensor native canonical fixture consumer passes.
- **AC-05:** Controller firmware build/check uses `source ~/export-esp.sh` before execution.
- **AC-06:** Controller canonical fixture tests execute and pass, or physical/toolchain limitation is recorded without falsely claiming verification.
- **AC-07:** Cross-system configuration synchronization/status contracts are exercised by executable evidence.
- **AC-08:** Readiness and safety-data unavailable semantics retain their documented contracts.
- **AC-09:** Backup/restore reuse of ConfigurationSync remains covered by executable evidence.
- **AC-10:** `git diff --check` passes and dirty worktree is preserved.
- **AC-11:** No production PostgreSQL is touched.
- **AC-12:** Domain 10 does not begin Domain 11 HIL/physical E2E or Domain 12 evidence refresh work beyond this domain's own evidence artifact.

## 5. Non-goals

- No physical/HIL E2E.
- No release readiness audit.
- No evidence/traceability refresh for the whole roadmap.
- No shared-schema redesign.
- No production PostgreSQL migration/testing.
- No unrelated refactor.
- No Domain 7 rework.
- No new backend/frontend/firmware feature.

## 6. Execution order

1. Patch this spec before implementation.
2. Capture baseline worktree state.
3. Run shared verification.
4. Run isolated backend verification as required.
5. Run frontend verification.
6. Run sensor firmware verification.
7. `source ~/export-esp.sh`; run controller firmware build/check/test path.
8. Execute deterministic cross-system contract checks.
9. Run repository integrity checks.
10. Create/update Domain 10 evidence artifact.
11. Mark `COMPLETE / VERIFIED` only when every applicable acceptance criterion has executable evidence; otherwise mark `IMPLEMENTED / VERIFICATION BLOCKED` with exact blockers.

## 7. Evidence

Create:

`docs/evidence/DOMAIN-10-FULL-CROSS-SYSTEM-VERIFICATION-001.json`

Record exact commands, working directories, exit codes, test counts, environment setup, isolated PostgreSQL identity when used, firmware runner limitations, and acceptance status.
