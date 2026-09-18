# P2.2 Configuration Synchronization Platform

**Status:** PROPOSED
**Phase:** P2.2
**Depends on:** P0.4, P1.5, P1.10, Digital Twin configuration-sync behavior

## Goal

Unify all device configuration delivery under one durable reconciliation model without creating a second configuration database or protocol.

## Scope

The subsystem covers:

- unified controller configuration;
- WiFi/device connectivity configuration where the existing delivery state applies;
- backup restore;
- restart recovery;
- MQTT disconnect/reconnect;
- stale reported revision;
- unknown publish outcome;
- bounded retry.

## Canonical lifecycle

```text
PERSISTED_DESIRED
      |
      v
PENDING
      |
      v
PUBLISHED
      |
      v
ACCEPTED
      |
      v
APPLIED
      |
      v
VERIFIED
```

Failure branches:

```text
PENDING/PUBLISHED/ACCEPTED
        |
        +--> FAILED
        +--> UNKNOWN
        +--> retry -> PENDING
```

The exact existing DB vocabulary may be retained where compatible; the semantic predicates above are mandatory.

## Rules

- PostgreSQL desired configuration is durable authority.
- Device-reported applied revision is evidence of remote application, not cache state.
- MQTT publish success means transport accepted the publish, not device application.
- Backend restart must recover all pending/unknown sync work from durable state.
- Reconnect must trigger bounded reconciliation, not blind periodic republishing.
- Backup restore must reuse the same synchronization service.
- Version comparison must distinguish equal, older, newer, missing, and invalid reports.

## Acceptance criteria

- **AC-1:** Save/update creates durable desired revision before remote side effect.
- **AC-2:** Backend crash after DB commit but before MQTT publish leaves recoverable pending state.
- **AC-3:** Backend crash after ambiguous publish leaves recoverable unknown state with safe retry policy.
- **AC-4:** Controller restart causes desired configuration to converge without creating a second DB record.
- **AC-5:** Older controller-reported revision causes requeue; newer revision is not silently overwritten.
- **AC-6:** Duplicate retained/replayed config is idempotent.
- **AC-7:** Restore uses the same durable sync lifecycle.
- **AC-8:** No new MQTT topic/schema is introduced.
- **AC-9:** PostgreSQL integration tests prove transaction/recovery behavior on a real isolated database.

## Evidence

Include crash-window tests, reconnect/restart tests, real PostgreSQL + Mosquitto + Digital Twin E2E, and migration compatibility evidence where schema changes are needed.
