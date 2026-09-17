# HYDRAGROW — Domain Spec 01: Migration Identity Reconciliation

**Domain ID:** `DB-MIGRATION-IDENTITY-001`  
**Domain:** PostgreSQL / SQLx migration identity and historical compatibility  
**Status:** `IMPLEMENTED / VERIFIED`  
**Scope:** Migration identity only

---

## 0. Purpose

This spec defines the canonical identity, ordering, compatibility, reconciliation, and verification rules for historical PostgreSQL migrations in HYDRAGROW.

The domain exists because SQLx derives migration identity from the numeric version in the migration filename. A historical migration filename therefore is not cosmetic metadata: changing its numeric prefix changes the migration identity seen by SQLx.

The goal is to make the repository migration source deterministic while preserving the historical SQL semantics and providing a controlled path for databases that already recorded the legacy migration version.

This spec does **not** define general database schema design, ConfigurationSync, safety semantics, readiness, firmware behavior, HIL, or release readiness.

---

# 1. Domain Boundary

## 1.1 In scope

- SQLx migration version identity.
- Historical migration filename normalization.
- Migration ordering.
- `_sqlx_migrations` compatibility.
- Legacy migration-version reconciliation.
- Migration checksum preservation.
- Clean PostgreSQL migration verification.
- Acceptance and handoff rules for this domain.

## 1.2 Out of scope

- New application features implemented by later migrations.
- Runtime database error semantics.
- Configuration synchronization behavior.
- Backup/Restore lifecycle.
- `/readyz` semantics.
- Firmware schema consumers.
- Cross-system or physical/HIL verification.
- Production deployment execution.

---

# 2. Problem Statement

The historical migration originally existed as:

```text
20260506_cleanup_unused_fields.sql
```

Its SQLx migration version was therefore:

```text
20260506
```

The repository also contains the initialization migration with version:

```text
20260312064526
```

Migration ordering is determined from the numeric versions. The legacy `20260506` identity was therefore unsafe for the intended clean-database ordering because the migration assumes the relevant schema already exists.

The canonical historical migration is now:

```text
20260506000000_cleanup_unused_fields.sql
```

with version:

```text
20260506000000
```

The SQL body is unchanged.

The problem has two separate dimensions:

```text
repository migration source
        !=
database historical metadata
```

A clean database sees only the canonical repository identity. A legacy database may already contain the old identity `20260506` in `_sqlx_migrations`.

Both cases must be handled explicitly.

---

# 3. Canonical Migration Identity

## 3.1 Identity rule

For this domain:

```text
migration identity = SQLx numeric migration version
```

The numeric version is derived from the migration filename.

The canonical migration is therefore:

```text
Filename:
20260506000000_cleanup_unused_fields.sql

Version:
20260506000000
```

## 3.2 Canonical checksum

The historical SQL content is preserved with checksum:

```text
8532ad360f743a69cad9178aa43ca9ac743e8a35456bdf9a5e128cbc95aa3750
```

Changing the filename must not be interpreted as permission to change the historical SQL body.

## 3.3 Uniqueness

The migration source must resolve each migration version exactly once.

The old and canonical identities must not coexist as two executable migrations representing the same historical change:

```text
20260506_...
20260506000000_...
```

must not both represent the same migration.

---

# 4. Canonical Ordering

Migration versions must preserve the intended dependency order.

The canonical historical sequence includes:

```text
20260312064526_...
...
20260506000000_cleanup_unused_fields.sql
...
```

The padded version is intentional. It is not a formatting-only rename.

The migration version must not be changed to an arbitrary value merely to make the filename look consistent. Any version change must be justified by historical execution order and compatibility requirements.

---

# 5. Applied-Migration Contract

SQLx records applied migrations in:

```text
_sqlx_migrations
```

The domain assumes that an applied migration's historical version is part of the database migration state.

Therefore this state:

```text
_sqlx_migrations.version = 20260506
```

is not equivalent to:

```text
_sqlx_migrations.version = 20260506000000
```

until an explicit reconciliation has occurred.

The repository rename alone must not be assumed to repair an existing database.

---

# 6. Legacy Identity Failure Mode

A database that previously applied:

```text
20260506
```

while the current migration source resolves only:

```text
20260506000000
```

must be treated as having a migration identity mismatch.

Expected SQLx behavior is a missing-applied-migration condition, equivalent to:

```text
migration 20260506 was previously applied
but is missing from the resolved migrations
```

This is an intentional safety signal.

It must not be silently converted into a successful migration state.

---

# 7. Reconciliation Strategy

## 7.1 Principle

Historical identity reconciliation is a controlled metadata repair, not a normal application migration.

The canonical flow is:

```text
Detect legacy identity
        |
        v
Verify historical equivalence
        |
        v
Verify checksum / SQL content
        |
        v
Verify no conflicting canonical migration
        |
        v
Reconcile _sqlx_migrations metadata
        |
        v
Run SQLx migration validation
        |
        v
Resume normal migration lifecycle
```

## 7.2 Do not use `ignore_missing`

`ignore_missing` is not the canonical compatibility strategy for this domain.

A missing applied migration must remain visible until the historical identity has been explicitly reconciled.

The repository must not use an ignore flag as a substitute for migration-history repair.

## 7.3 Reconciliation preconditions

Before reconciling `20260506` to `20260506000000`, all of the following must hold:

- The old version is known to represent this exact historical migration.
- The canonical migration is the same SQL change.
- The canonical checksum is verified.
- The canonical version does not conflict with another migration.
- The database is known to have applied the old migration.
- A suitable database backup/recovery path exists before production metadata repair.

If any precondition fails, reconciliation must stop.

---

# 8. Implemented Repository Change

The repository migration was normalized from:

```text
20260506_cleanup_unused_fields.sql
```

to:

```text
20260506000000_cleanup_unused_fields.sql
```

The migration SQL content was preserved.

The canonical checksum was verified as:

```text
8532ad360f743a69cad9178aa43ca9ac743e8a35456bdf9a5e128cbc95aa3750
```

The old filename is not present in the current repository migration source.

No duplicate migration was created.

---

# 9. Clean PostgreSQL Verification

An isolated PostgreSQL 16 database was used to verify the complete current migration source from an empty database.

Environment:

```text
PostgreSQL 16
Database: hydragrow_test
```

Migration source:

```text
hydragrow-backend/migrations
```

Command:

```text
sqlx migrate run --source hydragrow-backend/migrations
```

Result:

```text
55 migrations applied successfully
```

This establishes the clean-database invariant:

```text
empty PostgreSQL database
        |
        v
current repository migrations
        |
        v
55 migrations applied successfully
```

No migration-ordering failure occurred.

---

# 10. Legacy Database Simulation

The compatibility path was verified by simulating a historical database whose `_sqlx_migrations` metadata contained:

```text
version = 20260506
```

while the current repository source contained:

```text
version = 20260506000000
```

SQLx correctly rejected the unresolved historical identity.

This verifies that the problem is real and that the repository rename alone does not reconcile an already-applied legacy database.

---

# 11. Controlled Reconciliation Verification

In the isolated test database only, the historical metadata was reconciled:

```text
20260506
```

to:

```text
20260506000000
```

After reconciliation:

```text
sqlx migrate run
```

completed successfully.

This proves that a verified historical identity reconciliation is compatible with the canonical repository migration source.

No production database was modified during this verification.

---

# 12. Production Safety Contract

Production migration reconciliation must not be performed by an unconditional startup mutation.

The application must not silently execute:

```text
UPDATE _sqlx_migrations ...
```

against arbitrary databases.

A production reconciliation procedure must first establish:

1. database backup/recovery availability;
2. exact legacy version identity;
3. canonical migration equivalence;
4. checksum/content equivalence;
5. absence of conflicting versions;
6. expected post-reconciliation SQLx state;
7. verification after the metadata change.

The procedure should be explicit and auditable.

---

# 13. Failure Semantics

| Condition | Required behavior |
|---|---|
| Duplicate migration version | Fail migration validation |
| Applied legacy version missing from source | Detect and stop; do not silently ignore |
| Historical equivalence cannot be proven | Do not reconcile |
| Checksum/content mismatch | Do not reconcile; investigate migration history |
| Canonical version conflicts | Do not reconcile |
| Clean DB migration fails | Domain is not verified |
| Controlled reconciliation fails | Domain remains unresolved |

The domain must prefer a visible migration failure over silently accepting an ambiguous migration history.

---

# 14. Verification Matrix

| ID | Verification | Expected | Result |
|---|---|---|---|
| MIG-01 | Canonical migration filename | `20260506000000_cleanup_unused_fields.sql` exists | PASS |
| MIG-02 | Legacy filename removal | `20260506_cleanup_unused_fields.sql` absent | PASS |
| MIG-03 | Historical SQL preservation | SQL body unchanged | PASS |
| MIG-04 | Checksum | `8532ad360f743a69cad9178aa43ca9ac743e8a35456bdf9a5e128cbc95aa3750` | PASS |
| MIG-05 | Clean DB migration | Complete migration chain succeeds | PASS |
| MIG-06 | Migration count | 55 migrations applied | PASS |
| MIG-07 | Legacy identity simulation | SQLx detects missing applied identity | PASS |
| MIG-08 | Controlled reconciliation | Canonical source validates after reconciliation | PASS |
| MIG-09 | Production mutation | No production DB touched | PASS |

---

# 15. Acceptance Criteria

## AC-1 — Canonical identity

The historical cleanup migration is represented by exactly one canonical identity:

```text
20260506000000
```

**Status:** PASS

## AC-2 — Historical content preservation

The filename normalization does not alter the historical SQL content or its checksum.

**Status:** PASS

## AC-3 — Valid clean migration chain

An empty PostgreSQL database can apply the current migration source completely.

**Status:** PASS — 55/55 migrations

## AC-4 — Legacy mismatch is detectable

A database containing the legacy applied version `20260506` is correctly rejected by the current migration source until reconciliation.

**Status:** PASS

## AC-5 — Controlled reconciliation works

After verified metadata reconciliation, SQLx accepts the canonical migration source.

**Status:** PASS

## AC-6 — No silent compatibility bypass

`ignore_missing` is not used as the migration-history reconciliation mechanism.

**Status:** PASS

## AC-7 — Production isolation

Verification and reconciliation testing do not modify a production database.

**Status:** PASS

---

# 16. Current Domain Status

```text
DB-MIGRATION-IDENTITY-001
STATUS: COMPLETE / VERIFIED
```

The implementation described by this spec has already been performed in the repository and verified before this spec was written.

This document records the implementation as the canonical domain contract; it is not a proposal for the already-completed migration rename.

---

# 17. Handoff

The output of this domain is:

```text
Canonical migration identity
        +
Verified clean PostgreSQL baseline
        +
Known legacy reconciliation procedure
```

The next domain may assume the migration identity baseline is stable.

The next domain must not silently rename, reorder, duplicate, or rewrite this historical migration.

Future changes to the database schema must use a new migration with a new identity.

---

# 18. Traceability

```text
DB-MIGRATION-IDENTITY-001
        |
        +-- hydragrow-backend/migrations/20260506000000_cleanup_unused_fields.sql
        |
        +-- _sqlx_migrations historical identity contract
        |
        +-- Clean PostgreSQL migration verification
        |
        +-- Legacy 20260506 mismatch simulation
        |
        +-- Controlled 20260506 -> 20260506000000 reconciliation
        |
        +-- SQLx post-reconciliation validation
```

Domain invariant:

```text
Historical Migration Identity
        ==
Canonical Repository Identity
        ==
SQLx Applied-Migration Identity
```

for every database that has been explicitly reconciled under the controlled procedure.
