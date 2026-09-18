# P2.1 Backend Domain / Application Decomposition

**Status:** PROPOSED
**Phase:** P2.1
**Depends on:** P1.1 authorization, P1.2 durable commands, P1.3 safety, P1.5 config sync, P1.6 journal, P1.7 API consolidation, P1.10 observability

## Goal

Reduce `AppState` and oversized handlers/services by extracting explicit application-service boundaries while keeping the existing modular-monolith deployment and database.

## Target boundary

```text
HTTP/MQTT/WebSocket transport
        |
        v
Application service
        |
        +--> domain policy
        +--> repository/persistence boundary
        +--> external transport port
        |
        v
PostgreSQL / InfluxDB / MQTT / event bus
```

Transport handlers own parsing/authentication/status mapping. Application services own orchestration. Domain policy owns invariants. Infrastructure owns SQL/MQTT/Influx details.

## Initial extraction order

1. Configuration synchronization.
2. Durable command orchestration.
3. Backup/restore orchestration.
4. Fleet read/query aggregation.
5. Sensor ingestion and authoritative telemetry merge.

Do not split `AppState` mechanically. Each extraction must remove a concrete responsibility and leave a smaller public interface.

## Acceptance criteria

- **AC-1:** Each extracted domain exposes an explicit service interface; handlers no longer orchestrate persistence + transport + policy in one function.
- **AC-2:** Authorization remains at the API boundary and is passed into the service as an authorized principal/context.
- **AC-3:** Command authority remains the existing durable command aggregate.
- **AC-4:** Safety policy remains fail-closed and cannot be bypassed by calling the extracted service directly.
- **AC-5:** Existing API behavior and canonical response/error contracts remain compatible unless a separately approved P2 contract change exists.
- **AC-6:** Unit tests cover domain/application decisions; integration tests cover persistence/transport effects.
- **AC-7:** No new database or microservice is introduced by decomposition.

## Required evidence

Record before/after module ownership, test results, `cargo clippy -D warnings`, `cargo fmt --check`, backend integration results, and a grep/static check showing removed duplicate orchestration paths.
