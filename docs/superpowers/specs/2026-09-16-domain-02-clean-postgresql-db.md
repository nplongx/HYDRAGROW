# HYDRAGROW — Domain Spec 02: Clean PostgreSQL DB

**Domain ID:** `DB-CLEAN-POSTGRES-001`  
**Domain:** Clean PostgreSQL database baseline and SQLx migration execution  
**Status:** `IMPLEMENTED / VERIFIED FOR BASELINE`  
**Scope:** PostgreSQL environment initialization and clean-database migration verification

---

## 0. Purpose

This spec defines the clean PostgreSQL database baseline used to verify that the current HYDRAGROW migration source can initialize an empty PostgreSQL database deterministically.

The domain answers one question:

> Starting from an empty PostgreSQL database, can the repository's canonical migration source create the expected database schema without relying on historical state, manual schema edits, or pre-existing application data?

This domain is deliberately narrower than general backend verification.

It covers:

- isolated PostgreSQL provisioning;
- empty-database initialization;
- SQLx migration execution;
- migration count and completion;
- clean-database validation;
- isolation and reproducibility rules;
- evidence needed before backend DB-backed testing.

It does **not** define migration identity reconciliation itself; that is `DB-MIGRATION-IDENTITY-001`.

It does **not** define the backend application test baseline; that is the next verification domain.

---

# 1. Domain Boundary

## 1.1 In scope

- PostgreSQL test instance lifecycle.
- Empty database creation.
- SQLx migration source resolution.
- Running all repository migrations against an empty database.
- Migration success/failure classification.
- Schema existence after migration.
- Isolation from production and development databases.
- Reproducibility of the clean migration run.
- Baseline evidence for subsequent backend tests.

## 1.2 Out of scope

- Historical migration identity reconciliation.
- Production database migration execution.
- Production backup/restore.
- Application authorization.
- ConfigurationSync behavior.
- Runtime DB error semantics.
- InfluxDB.
- MQTT.
- Firmware.
- Frontend.
- HIL / physical verification.
- Release readiness.

---

# 2. Clean-Database Definition

A database is considered **clean** when:

```text
PostgreSQL instance
    |
    +-- target database exists
    +-- target database contains no application schema/data
    +-- SQLx migration history is absent or newly initialized
    +-- no previous HYDRAGROW test run is being reused
```

The migration command must operate against that empty database using only the repository migration source.

A manually prepared database is **not** a clean baseline.

For example, this does not qualify:

```text
CREATE TABLE device_config ...
then
sqlx migrate run
```

because the migration chain was not responsible for creating the initial schema.

---

# 3. Environment Contract

The clean baseline requires:

1. PostgreSQL 16 or compatible supported PostgreSQL version.
2. A dedicated test database.
3. A database role with sufficient permissions for migration execution.
4. Network reachability from the backend/SQLx tooling.
5. The canonical repository migration directory:

```text
hydragrow-backend/migrations
```

6. No production database connection.
7. No shared mutable test database used concurrently by unrelated verification jobs.

The clean baseline must identify the actual database endpoint used by the verification run.

---

# 4. Isolated Verification Instance

The implemented baseline used an isolated PostgreSQL 16 container:

```text
Container:
hydragrow-readiness-postgres

Host port:
55432

Database:
hydragrow_test

User:
postgres
```

The instance was used only for readiness/migration verification.

No production PostgreSQL database was modified.

The important property is isolation, not the specific container name or host port.

Future verification environments may use different infrastructure, provided the same isolation and clean-state guarantees hold.

---

# 5. Canonical Migration Source

Migration execution uses:

```text
hydragrow-backend/migrations
```

The migration source must be the same source committed in the repository under test.

Do not copy individual migrations into the database environment.

Do not manually reorder migrations.

Do not omit migrations to make the clean run pass.

The migration source must be resolved by SQLx as one canonical ordered sequence.

---

# 6. Clean Migration Procedure

The canonical procedure is:

```text
Provision isolated PostgreSQL
        |
        v
Create empty target database
        |
        v
Verify database is clean
        |
        v
Run SQLx migration source
        |
        v
Verify all migrations applied
        |
        v
Verify resulting schema
        |
        v
Record evidence
```

The migration command used for the baseline was:

```bash
sqlx migrate run --source hydragrow-backend/migrations
```

The command must exit successfully.

A partial migration run is not a pass.

---

# 7. Implemented Baseline

The clean PostgreSQL baseline was executed against an empty PostgreSQL 16 database.

Result at the time of verification:

```text
55 migrations applied successfully
0 migration failures
```

This established that the canonical migration chain available at that verification point could initialize an empty database successfully.

The repository has subsequently accumulated additional migration files as later domains were implemented. Therefore `55` is **historical evidence for the clean baseline run**, not a claim about the current migration-file count.

Current migration count must always be measured from the repository at verification time rather than copied from historical evidence.

---

# 8. Migration Completion Invariant

A clean database passes only when:

```text
all resolved migrations
        ==
all successfully applied migrations
```

There must be no:

- missing migration;
- failed migration;
- partially applied migration;
- manually skipped migration;
- duplicate migration version;
- unexplained schema prerequisite.

The expected SQLx state after completion is represented by `_sqlx_migrations` containing the migration sequence that SQLx resolved from the repository source.

---

# 9. Schema Initialization Invariant

The clean migration chain must establish the schema required by the backend without manual pre-seeding.

At minimum, the verification must demonstrate that the migration chain can create the foundational application schema required by backend DB-backed tests.

Examples of foundational objects include tables used by:

```text
users
owned devices / device ownership
 device configuration
recipes / crop configuration
```

The exact object set is owned by the migration history; this domain verifies that the chain creates it rather than hardcoding a second schema definition.

---

# 10. Relationship to Migration Identity Reconciliation

This domain consumes the result of:

```text
DB-MIGRATION-IDENTITY-001
```

The canonical historical migration is:

```text
20260506000000_cleanup_unused_fields.sql
```

not the legacy repository filename:

```text
20260506_cleanup_unused_fields.sql
```

The clean database must use the canonical repository identity.

Legacy `_sqlx_migrations` metadata reconciliation is **not** part of the clean-database initialization path.

Clean DB and legacy DB are intentionally different verification cases:

```text
Clean DB
    |
    v
run canonical migration chain from zero

Legacy DB
    |
    v
reconcile historical identity when required
```

---

# 11. Failure Classification

Clean PostgreSQL verification must distinguish at least these cases.

### 11.1 Environment failure

Examples:

```text
cannot connect to PostgreSQL
permission denied
port unavailable
container unavailable
```

Classification:

```text
BLOCKED / ENVIRONMENT FAILURE
```

It is not a migration pass.

### 11.2 Migration resolution failure

Examples:

```text
duplicate version
invalid filename
unresolvable migration
```

Classification:

```text
FAIL — MIGRATION SOURCE
```

### 11.3 Migration execution failure

Examples:

```text
SQL syntax error
missing relation
constraint failure
migration dependency failure
```

Classification:

```text
FAIL — MIGRATION EXECUTION
```

### 11.4 Post-migration schema failure

If all migrations report success but required schema verification fails:

```text
FAIL — SCHEMA BASELINE
```

Do not convert this to PASS merely because `sqlx migrate run` exited zero.

---

# 12. Reproducibility Rules

A clean baseline must be reproducible.

A future verification run should:

1. destroy or discard the previous target database;
2. create a fresh empty target;
3. use the current repository migration source;
4. execute the declared migration command;
5. verify completion from the actual output/database state.

Do not reuse a previously migrated database and call it a clean run.

Do not rely on local application startup to implicitly perform migrations unless that is the declared verification command.

---

# 13. Test-Database Separation

The clean baseline database is not automatically equivalent to SQLx's temporary test databases.

A valid backend test lifecycle additionally requires the test runner to be able to create/drop its test databases and manage concurrent connections safely.

Therefore:

```text
Clean migration PASS
        !=
Full backend DB test PASS
```

The clean migration result is a prerequisite/evidence input for the backend test baseline, not a substitute for it.

---

# 14. Existing Verification Evidence

The clean baseline established the following facts:

```text
[PASS] PostgreSQL 16 instance available
[PASS] Dedicated test database available
[PASS] Empty database accepted migration chain
[PASS] Canonical migration source resolved
[PASS] 55 migrations applied at baseline time
[PASS] No migration execution failure
[PASS] No production database touched
```

The verification was performed against an isolated environment.

---

# 15. Acceptance Criteria

### AC-1 — Isolated PostgreSQL

A dedicated PostgreSQL instance/database is available for clean verification.

**Result:** PASS

### AC-2 — Empty starting state

The target database starts without the HYDRAGROW application schema.

**Result:** PASS

### AC-3 — Canonical migration source

SQLx executes `hydragrow-backend/migrations` as the repository migration source.

**Result:** PASS

### AC-4 — Full migration execution

Every migration resolved at the time of the baseline executes successfully.

**Result:** PASS — 55/55 at baseline time

### AC-5 — No manual schema prerequisite

The migration chain does not depend on manually creating the application schema beforehand.

**Result:** PASS

### AC-6 — Migration history recorded

SQLx records the successfully applied migration sequence in `_sqlx_migrations`.

**Result:** PASS

### AC-7 — Production isolation

No production database is used or modified during clean verification.

**Result:** PASS

### AC-8 — Evidence classification

Historical migration evidence is not presented as current migration-count evidence after new migrations are added.

**Result:** PASS / RULE ESTABLISHED

---

# 16. Current Status

```text
DOMAIN STATUS: COMPLETE / BASELINE ESTABLISHED
```

Important qualification:

The original clean run verified **55 migrations**. Later work added migrations, including ConfigurationSync. Therefore the next full verification must rerun the clean-database procedure against the **current** migration source before declaring the current repository's complete migration chain green.

This does not invalidate the original baseline; it means the evidence is versioned by verification time.

---

# 17. Handoff to Next Domain

This domain provides the database foundation for:

```text
DB-MIGRATION-IDENTITY-001
        |
        v
DB-CLEAN-POSTGRES-001
        |
        v
BACKEND-TEST-BASELINE
```

The next domain may assume that a clean PostgreSQL verification procedure exists, but must not assume that historical `55/55` evidence proves the current migration source remains green.

The next clean verification should use the current migration count and current repository state.

---

# 18. Traceability

```text
Canonical migration source
        |
        +--> isolated PostgreSQL
        |
        +--> empty database
        |
        +--> SQLx migration execution
        |
        +--> _sqlx_migrations
        |
        +--> resulting schema
        |
        +--> backend DB-test prerequisite
```

Domain invariant:

```text
Empty PostgreSQL Database
        |
        v
Canonical Repository Migration Source
        |
        v
Complete Migration Execution
        |
        v
Valid HYDRAGROW Database Baseline
```
