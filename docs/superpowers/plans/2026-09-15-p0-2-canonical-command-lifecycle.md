# P0.2 Canonical Command Lifecycle

## Contract

Canonical lifecycle:

`REQUESTED -> SENT -> ACKNOWLEDGED -> CONFIRMED`

Terminal/uncertain states:

- `REJECTED`: explicit rejection.
- `FAILED`: explicit failure.
- `TIMEOUT`: confirmation did not arrive before the deadline; not proof of physical failure.
- `UNKNOWN`: outcome cannot be safely established.

Rules:

- Command identity is `command_id`, scoped with `device_id`.
- HTTP/API success proves neither device acknowledgement nor physical confirmation.
- Generic controller/device status events are not command ACK without correlation.
- `CONFIRMED` requires authoritative runtime observation supporting the requested state.
- Consequential commands never silently retry.
- E-STOP remains immediate and safety-priority.
- Command lifecycle state remains separate from observed/physical state.

## Implementation sequence

1. Freeze lifecycle and transition contract in shared tests.
2. Add shared command identity/correlation metadata to MQTT command envelopes.
3. Create backend command registry keyed by `device_id + command_id`; enter `REQUESTED` before dispatch and `SENT` only after publication.
4. Correlate controller commands and publish `ACKNOWLEDGED` or `REJECTED` only for the originating command.
5. Reconcile `CONFIRMED` only from authoritative controller runtime observations; otherwise remain pending/unknown.
6. Persist lifecycle trace in existing `system_events`; do not add a migration unless existing persistence proves insufficient.
7. Expose lifecycle through WebSocket and a device-scoped read API for reload/reconnect reconciliation.
8. Replace frontend `sending` / `accepted` semantics with canonical lifecycle state and canonical StationContext identity.
9. Handle timeout/unknown, E-STOP, reset fault, and other consequential commands without silent retry.
10. Verify shared, backend, controller, frontend, and cross-device isolation behavior.

## Non-goals

- P0.3 telemetry redesign.
- P0.4 configuration consistency remediation.
- Generic event sourcing.
- Distributed/offline command queue.
- Automatic retry engine.
- New hardware feedback.
- Backend-wide decomposition.
- Firmware-wide rewrite.

## P0 closure boundary / P1 carry-over

The lifecycle semantics above remain the P0 contract. The current in-memory backend registry is not sufficient for the P1 durable lifecycle requirement: process restart, reconnect, timeout/UNKNOWN reconciliation, and multi-instance authority require a durable/reconcilable design.

Therefore:

- do not reopen the P0 lifecycle contract merely to fix P1 durability;
- do not infer physical actuator state from an HTTP response, MQTT publication, generic status event, or lifecycle event without authoritative observation;
- do not use an in-memory registry as durable command truth;
- P0 residual verification may require the simulator to compile against all lifecycle events, but durable persistence/reconciliation is P1 scope.

P1.0 must begin from a compiling cross-system baseline. The known simulator `PublishCommandLifecycle` exhaustiveness regression is a P0 residual blocker, while durable command persistence/reconciliation is a P1 implementation blocker.
