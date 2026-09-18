# P2.3 Realtime Transport / Event Router / Recovery Boundary

**Status:** PROPOSED
**Phase:** P2.3
**Depends on:** P1.4, P1.7, P1.8, P1.10

## Goal

Separate WebSocket transport, event routing, and authoritative recovery while keeping WebSocket best-effort and React Query authoritative for server state.

## Target architecture

```text
WebSocket transport
       |
       v
Realtime event router
       |
       +--> telemetry updater
       +--> command lifecycle updater
       +--> journal invalidation
       +--> operational-state notification
       |
       v
Authoritative recovery coordinator
       |
       v
REST/React Query refetch
```

The router MUST not own durable state.

## Recovery rules

- Socket connect is not proof of synchronized state.
- Socket reconnect triggers an authoritative recovery sequence for the selected device.
- Broadcast lag triggers recovery; it does not fabricate missed events.
- Older telemetry is rejected according to P1.4 ordering rules.
- Malformed frames are ignored/logged with bounded diagnostics and cannot corrupt cache state.
- Command lifecycle updates use `command_id` and never infer confirmation from a generic status frame.

## Acceptance criteria

- **AC-1:** Transport code contains no domain-specific query mutation logic.
- **AC-2:** Event router has deterministic routing tests for telemetry, status, lifecycle, journal, and malformed frames.
- **AC-3:** Reconnect performs authoritative recovery and tests prove stale local state is corrected.
- **AC-4:** Broadcast lag produces a recovery action rather than silent state acceptance.
- **AC-5:** No second server-state cache is introduced.
- **AC-6:** Backend + frontend reconnect E2E remains green.
