# P1.10 Observability + Cross-System Synchronization

**Status:** Draft specification — implementation not started
**Phase:** P1.10
**Date:** 2026-09-16
**Depends on:** P1.0 Verification / Cross-System Baseline, P1.1 Authorization / Ownership Boundary, P1.2 Durable Command Lifecycle, P1.3 Safety / Failure Semantics, P1.4 Operations State Boundary, P1.5 Backup / Restore Lifecycle, P1.6 Journal / Event Model, P1.7 API Query / Mutation Consolidation, P1.8 Route / Navigation Contract, P1.9 Shared Schema Alignment

## 1. Purpose

P1.10 establishes two related but explicitly different capabilities:

1. **Observability** — make system behavior diagnosable through logs, metrics, and traces, with stable correlation and bounded cardinality.
2. **Cross-system synchronization** — make desired state, observed state, configuration, commands, caches, WebSocket delivery, and MQTT delivery converge according to explicit authority, freshness, ordering, retry, and reconciliation rules.

The governing rule is:

> Observability tells us what happened and how the system is behaving. Synchronization determines what state is authoritative, what is desired, what was observed, what is pending, and how independent systems converge.

Observability MUST NOT become an alternate state store. Synchronization MUST NOT depend on logs being complete. A missing log line cannot change authoritative state; a cache hit cannot prove synchronization; a successful MQTT publish cannot prove physical application.

P1.10 closes the remaining cross-system reliability gap after P1.0-P1.9 without inventing infrastructure that the repository does not currently operate.

---

## 2. Current repository facts

The following facts are the starting baseline. They are deliberately separated from the target design.

### 2.1 Backend and persistence

The backend is Rust/Actix-web and currently has:

- PostgreSQL through `sqlx` for users, device configuration, crop seasons, system events, and the durable command aggregate/lifecycle history introduced by P1.2.
- InfluxDB through `influxdb2` for sensor time-series data.
- MQTT through `rumqttc` for controller/sensor communication.
- an in-process Tokio broadcast event bus used for WebSocket fan-out.
- in-memory device/config-related caches in `AppState`; these are not generally durable authorities.

P1.2 already establishes PostgreSQL as the authoritative command lifecycle store, including idempotency, bounded retry, timeout, `UNKNOWN`, and startup reconciliation. P1.10 MUST build on that authority rather than create another command state store.

P1.4 establishes operational state as observed state with explicit contact/freshness/readiness/actuator knowledge. P1.10 MUST not replace that model with cache or transport state.

### 2.2 Prometheus metrics

The backend already depends on the Rust `prometheus` crate and exposes `GET /metrics`. The endpoint requires a bearer token from `METRICS_TOKEN`; it is not a public unauthenticated endpoint.

Existing metrics include HTTP request count/duration, active WebSocket connections, MQTT receive/error counts, sensor updates, event-bus lag, command lifecycle outcomes, safety decisions, Flux queries, backup/restore outcomes, controller resource telemetry, dosing, adaptive control, and other domain metrics.

Existing labels include bounded infrastructure/domain dimensions such as method, endpoint, status, topic suffix, error type, command lifecycle, reason code, operation, stage, device ID, channel, actuator, phase, pump, and axis. P1.10 MUST audit these labels for cardinality before adding more. Device IDs are already used by several metrics, so P1.10 MUST measure and govern their scale rather than pretend cardinality is currently zero.

Prometheus is a metrics system, not the Journal or log store. The specification does not introduce a second metrics backend.

### 2.3 Logs and tracing

The backend uses Rust `tracing` and `tracing-subscriber`, with `#[instrument]` already present on multiple HTTP, MQTT, PostgreSQL, InfluxDB, and service paths.

The backend also contains a `tracing-loki` integration controlled by `LOKI_URL`. The current Docker Compose file sets `LOKI_URL=http://loki:3100` for the backend, but the repository's current Compose file does **not** define a Loki service. Therefore P1.10 MUST describe Loki as an existing integration point, not as a currently verified/deployed repository-local observability service.

No backend OpenTelemetry exporter/tracer provider is established by the current backend `Cargo.toml`. The frontend lockfile contains an `@opentelemetry/api` package transitively, but that is not evidence of an active end-to-end tracing pipeline. P1.10 MUST NOT claim distributed tracing is already deployed.

The existing `tracing` spans are useful instrumentation, but they do not by themselves guarantee propagation across HTTP -> backend -> MQTT -> controller -> WebSocket -> frontend boundaries.

### 2.3.1 Current correlation gap

The P1.7 error contract can carry `request_id`, and P1.9 now has `correlation_id` on the Journal wire model, but there is no evidence of one middleware-level correlation context being installed and propagated through the whole request lifecycle. Existing `#[instrument]` fields are local to individual spans. P1.10 therefore treats end-to-end correlation as a gap, not an existing capability.

### 2.4 WebSocket

The backend exposes device-scoped WebSocket connections at `/api/devices/{device_id}/ws` and authenticates before/around handshake according to the existing authorization contract.

The backend fans out `AppEvent` values through a Tokio broadcast channel of capacity 256. A slow client can receive a `Lagged` error and therefore miss events. The server currently logs this condition rather than treating the live stream as durable history.

The frontend `useDeviceSync` reconnects after a 5-second delay, updates React Query for telemetry, dispatches legacy operational events, invalidates Journal/system-event queries for alerts, and treats malformed realtime frames as ignorable. P1.7/P1.8 already require server-state ownership and context isolation; P1.10 MUST preserve those boundaries.

Therefore WebSocket delivery is currently **best-effort notification/fan-out**, not an exactly-once durable synchronization channel.

The current frontend closes/reconnects the socket after transport failure, but does not perform a mandatory authoritative refetch on every reconnect. P1.10 therefore requires explicit reconnect recovery rather than treating successful reconnection as synchronization.

### 2.5 MQTT

The backend subscribes to device-scoped MQTT topics using a mix of QoS 0 (`sensors`) and QoS 1 (`status`, controller status, FSM, logs, lifecycle-related topics, dosing/water topics, etc.). The backend uses `clean_session(false)` and reinstalls subscriptions after successful broker connections.

The existing command path uses MQTT as transport, while P1.2 uses PostgreSQL as command lifecycle authority. MQTT publication therefore MUST remain distinct from command confirmation and physical observation.

MQTT reconnect currently restores subscriptions and waits for new observations; it does not fabricate telemetry or operational state. P1.10 MUST retain this P1.4 invariant.

### 2.6 PostgreSQL and InfluxDB roles

PostgreSQL is the authoritative durable store for relational/configuration/control/history domains already assigned to it, including P1.2 command lifecycle and P1.6 Journal/event history.

InfluxDB is the authoritative time-series store for sensor measurements used by the backend analytics/query paths. Existing writes convert shared `f32` sensor values to Influx `f64`, consistent with the P1.9 documented precision boundary.

P1.10 MUST NOT move telemetry into PostgreSQL merely to simplify synchronization, nor use InfluxDB as a command/configuration authority.

### 2.7 Confirmed synchronization gaps

The current code already has partial synchronization mechanisms, but they are domain-specific rather than one cross-system contract:

- P1.2 durable commands have `command_id`, lifecycle sequence, idempotency, retry budget, timeout, and restart reconciliation. P1.10 MUST instrument and connect this existing lifecycle; it MUST NOT create another command authority.
- Controller status currently confirms commands by comparing observed pump/FSM values, but the confirmation helper timestamps the observation with backend receipt time. P1.10 MUST preserve distinct observation/receipt semantics and use producer observation time when trustworthy.
- WiFi configuration already uses a durable `config_version` and rejects status for a mismatched device/version. This is a concrete model for versioned synchronization, not evidence that all configuration domains are versioned.
- General desired configuration does not yet have one demonstrated device-reported applied revision across all config sections. P1.10 MUST not label a successful PostgreSQL write as device-applied.
- `reconcile_config_overwrite_group` already reconciles competing automation overrides. P1.10 MUST distinguish this domain-specific arbitration from the broader desired/observed synchronization state model.
- `useDeviceSync` updates/invalidate React Query from realtime frames, but malformed/lagged/disconnected streams currently do not force a universal authoritative refetch. P1.10 MUST add recovery at the synchronization boundary.
- WebSocket `event_bus` lag is already observable through a bounded metric; the stream remains lossy and must stay non-authoritative.
- MQTT topic last-seen tracking exists in PostgreSQL, and watchdog code now checks both controller and sensor status topics. P1.10 MUST build freshness/diagnostic semantics on this existing evidence rather than recreate heartbeat storage.
- The backend currently emits some structured fields such as `device_id`, `firmware_version`, `category`, and `command_id`, but the fields are inconsistent across handlers. P1.10 MUST normalize context without requiring every log line to carry every field.

These findings are implementation gaps, not acceptance results. They describe the current source state at spec-writing time.

---

## 3. Scope

P1.10 covers:

- structured logs and log-level policy;
- metrics inventory, naming, cardinality, and alertable synchronization signals;
- trace/span correlation where the existing stack can support it;
- `correlation_id` and trace propagation across system boundaries;
- request/command/cycle/event/resource correlation semantics;
- freshness/staleness semantics for cross-system state;
- desired/observed configuration synchronization;
- desired/observed operational state distinction;
- command-to-observation correlation;
- PostgreSQL-backed synchronization authority;
- cache correctness and invalidation/revalidation;
- WebSocket notification semantics and recovery after missed frames;
- MQTT delivery, retry, ordering, and reconciliation semantics;
- idempotency and duplicate/replay handling;
- bounded retry and, only where necessary, a transactional outbox or equivalent durable handoff;
- health/readiness/liveness boundaries;
- synchronization diagnostics and evidence;
- observability and synchronization test matrices;
- rollout, migration, rollback, and operational evidence.

P1.10 MUST use existing PostgreSQL, InfluxDB, MQTT, WebSocket, Prometheus, tracing, and logging mechanisms as the first implementation surfaces. New infrastructure is permitted only when a concrete gap cannot be safely solved with existing repository infrastructure and the addition is explicitly approved/documented.

---

## 4. Explicit non-goals

P1.10 does **not**:

- replace PostgreSQL, InfluxDB, MQTT, WebSocket, Prometheus, or Actix-web;
- create a second command aggregate or lifecycle authority;
- turn Journal events into current state;
- make WebSocket exactly-once;
- claim MQTT exactly-once physical actuation;
- introduce a second frontend server-state cache;
- redesign P1.1 authorization or ownership;
- redesign P1.2 command lifecycle vocabulary;
- redesign P1.3 safety/failure policy;
- reopen P1.4 operational-state semantics;
- replace P1.6 Journal storage with a log platform;
- replace P1.7 React Query/API ownership with an event-store architecture;
- replace P1.8 route/context authority;
- create a second editable schema after P1.9;
- assume Kubernetes, Kafka, NATS, Redis, Grafana, Loki, Tempo, Jaeger, OpenTelemetry Collector, or another external platform is deployed unless repository evidence proves it;
- introduce exactly-once distributed transaction claims that the underlying transports cannot provide;
- use observability telemetry as authorization evidence;
- log secrets merely to make synchronization debugging easier.

---

## 5. Authority and state model

P1.10 MUST define every synchronized datum by **authority**, **desired/observed role**, **freshness**, **version/order**, and **delivery status**.

### 5.1 State classes

Use these distinct concepts:

```text
desired state       what an authorized caller/configuration says should be true
observed state      what the device/controller/backend has actually observed
applied state       what a command/configuration update has been accepted/applied as
cached state        a local copy used for performance, never authority by itself
transport state     whether a communication path is connected/available
sync state          whether desired and observed/applied state have converged
```

Examples:

```text
desired config = PostgreSQL configuration authority
observed config = controller-reported configuration/status, when available
command intent = P1.2 durable command aggregate
physical actuator = P1.4 authoritative runtime observation
sensor history = InfluxDB time-series authority
Journal history = P1.6 PostgreSQL event authority
frontend cache = React Query cache, never authority
WebSocket = notification/acceleration path, never authority
MQTT = transport, never general state authority
```

Exact ownership MUST be audited per resource before implementation. Where the current system has no authoritative observed-config representation, P1.10 MUST expose that as a synchronization gap rather than inventing one from the cache.

### 5.2 Sync state

The minimum cross-system synchronization state is:

```text
UNKNOWN
PENDING
IN_SYNC
STALE
CONFLICT
FAILED
```

These are semantic states, not UI colors.

- `UNKNOWN`: authority or observation is insufficient to decide convergence.
- `PENDING`: a desired mutation exists but required propagation/observation is not complete.
- `IN_SYNC`: required desired/applied/observed predicates agree within the contract.
- `STALE`: a previously valid observation has exceeded its freshness window.
- `CONFLICT`: valid sources disagree and cannot safely be reconciled automatically.
- `FAILED`: a defined delivery/reconciliation attempt has failed and policy does not currently permit further automatic progress.

`IN_SYNC` MUST never be inferred from a successful HTTP mutation, MQTT publish, WebSocket send, or cache write alone.

---

## 6. Correlation model

P1.10 standardizes correlation without conflating identifiers.

### 6.1 Identifier roles

| Identifier | Meaning | Lifetime | Authority |
|---|---|---|---|
| `request_id` | one HTTP/API request | request | API boundary |
| `correlation_id` | one logical cross-system operation | operation | operation initiator/backend |
| `trace_id` | distributed tracing trace | trace | tracing system/context |
| `span_id` | one tracing span | span | tracing implementation |
| `command_id` | one durable command intent | command | P1.2 PostgreSQL |
| `cycle_id` | one dosing/automation cycle | cycle | relevant domain |
| `event_id` | one historical Journal occurrence | event | P1.6 PostgreSQL |
| `device_id` | target/resource identity | resource | ownership/device registry |
| `config_version` | desired/applied config revision | config revision | configuration authority |
| `observation_version` | ordering marker for observed state | observation | producer/contract |

One ID MUST NOT be silently reused to mean another ID class.

### 6.2 `correlation_id`

Every externally initiated cross-system operation SHOULD receive a stable `correlation_id` at the first trusted application boundary.

The same `correlation_id` MUST be propagated through:

```text
HTTP request
   -> backend service
      -> PostgreSQL mutation / Journal event
      -> MQTT command or synchronization message
         -> controller/sensor processing where supported
      -> WebSocket notification
      -> frontend diagnostic context
```

Propagation metadata MUST be optional where an old producer cannot supply it, but the backend MUST NOT invent a new ID for every hop of one operation.

For P1.2 commands, `command_id` remains the durable command identity. `correlation_id` may group the wider user/request operation and MUST NOT replace `command_id`.

### 6.3 Trace propagation

If distributed tracing is introduced/connected, W3C Trace Context or an equivalent documented standard MUST be used rather than a proprietary per-service header.

At minimum, trusted backend-to-backend and backend-to-transport propagation SHOULD carry trace context when the transport supports metadata safely. MQTT payload/header conventions MUST be versioned through P1.9 rather than ad hoc fields scattered across command types.

The controller/sensor firmware does not need to become a full tracing implementation merely to carry a correlation reference. A bounded correlation identifier may be sufficient where resource constraints prevent spans.

Trace IDs MUST NOT be used as authorization credentials, idempotency keys, or durable command IDs.

---

## 7. Freshness and staleness

Freshness is a synchronization property, not merely a timestamp display.

### 7.1 Timestamp semantics

Where available, retain distinct:

- `observed_at` — when the source observed the state;
- `received_at` — when the backend received it;
- `applied_at` — when desired state was accepted/applied by the target boundary;
- `classified_at` — when backend derived synchronization/operational status;
- `updated_at` — persistence/update time where materially different.

P1.9's timestamp contract remains authoritative. Existing epoch-millisecond fields remain where their wire/storage contracts require them.

### 7.2 Freshness rule

Every synchronization-sensitive observed value MUST have a documented freshness policy:

```text
fresh if now - observation_time <= freshness_window
stale if observation_time exists but exceeds freshness_window
unknown if no trustworthy observation exists
```

The exact window MUST come from the existing operational behavior/contract and MUST NOT be invented globally by P1.10. Different resource classes may have different windows.

At minimum document windows for:

- sensor telemetry;
- controller health/contact;
- observed configuration;
- actuator state used for command confirmation;
- frontend realtime notification recovery.

### 7.3 Ordering

Newer valid observations MUST supersede older observations. An older delayed MQTT/WebSocket event MUST NOT restore freshness or overwrite a newer observation.

Ordering may use an existing monotonic sequence, device timestamp plus bounded clock assumptions, database version, or another established contract. Do not introduce two competing sequence authorities.

When ordering cannot be established safely, state becomes `UNKNOWN`/`CONFLICT` according to P1.4 rather than being normalized to “latest received”.

---

## 8. Desired/observed configuration synchronization

P1.10 must make configuration convergence explicit without pretending that the repository currently has a complete device-side config replica.

### 8.1 Configuration roles

The configuration model MUST distinguish:

```text
desired_config
    PostgreSQL/backend canonical configuration

pending_config
    desired revision not yet confirmed at the target

observed_config
    latest authoritative controller/device-reported configuration, when supported

effective_config
    the configuration actually used by the runtime, only where the producer reports it

config_cache
    local frontend/backend cache of one of the above
```

A cached desired configuration MUST NOT be labeled as observed/effective merely because the write succeeded.

### 8.2 Versioning

Each configuration mutation SHOULD have a monotonically increasing durable `config_version` or equivalent revision at the authority.

The target MUST report enough information to determine whether it has applied that revision where configuration synchronization is supported.

If the target cannot report a version, the system may use a bounded correlation/acknowledgement contract, but MUST label the result as acknowledged rather than fully observed/applied unless the runtime observation proves convergence.

### 8.3 Reconciliation

Reconciliation SHOULD follow:

```text
desired revision
      |
      v
delivery pending
      |
      v
target acknowledgement
      |
      v
observed/applied revision
      |
      +--> equal -> IN_SYNC
      |
      +--> older  -> PENDING / retry
      |
      +--> newer incompatible -> CONFLICT
      |
      +--> no observation before deadline -> STALE/FAILED according to policy
```

Reconciliation MUST be idempotent. Replaying the same desired revision MUST not produce a different logical configuration mutation.

### 8.4 Secrets

Secret-bearing configuration follows P1.9's redaction rules. Synchronization metadata may identify that a secret-bearing field is configured, but MUST NOT copy the secret into logs, metrics, traces, Journal events, WebSocket notifications, generic cache snapshots, or correlation metadata.

---

## 9. Command synchronization

P1.2 remains the command authority. P1.10 adds observability and cross-system visibility around it.

### 9.1 Command path

The observable synchronization chain is:

```text
authorized request
  -> durable REQUESTED
  -> MQTT publication attempt
  -> SENT
  -> controller ACK
  -> ACKNOWLEDGED
  -> authoritative runtime observation
  -> CONFIRMED
```

Terminal outcomes remain `REJECTED`, `FAILED`, `TIMEOUT`, and `UNKNOWN`.

P1.10 MUST correlate each step using `command_id`, and SHOULD additionally attach `correlation_id`/`trace_id` where available.

### 9.2 Delivery guarantees

The system MUST state guarantees per boundary instead of claiming one global delivery mode:

| Boundary | Current/target guarantee |
|---|---|
| PostgreSQL command creation | durable/transactional |
| PostgreSQL lifecycle transition | durable + idempotent transition |
| MQTT command publish | at-least-once transport where QoS 1 is used; publish result is not physical confirmation |
| Controller command handling | idempotent by `command_id` where command class supports retry |
| Runtime confirmation | observation-based, not delivery-based |
| Backend event bus | in-process best-effort broadcast; lag can drop frames |
| WebSocket | best-effort live notification; client recovery via authoritative query |
| Frontend React Query cache | local cache; never durable authority |
| Journal event persistence | P1.6 semantics; durable historical evidence |

Exactly-once delivery MUST NOT be claimed for MQTT, WebSocket, or the overall cross-system workflow unless a concrete implementation proves it for a specific boundary.

### 9.3 Retry

Retry MUST be classified as one of:

- safe/idempotent;
- conditionally safe with command/config version;
- unsafe without reconciliation;
- forbidden after terminal/ambiguous state.

P1.2's bounded command retry policy remains authoritative. P1.10 MUST not add a competing command retry loop.

---

## 10. Cache synchronization

### 10.1 Backend caches

Backend caches are acceleration layers only. A cache entry MUST carry or derive:

- resource identity;
- source/authority;
- version/order where applicable;
- observed timestamp;
- freshness state;
- invalidation/revalidation policy.

On backend restart, caches may be empty. Empty cache MUST NOT become an inferred default.

### 10.2 Frontend cache

P1.7's React Query ownership remains authoritative for frontend server state.

WebSocket messages may update/invalidate React Query only when the message is valid for the selected device/context and carries enough ordering/freshness evidence.

If a WebSocket event is missed, lagged, malformed, or received out of order, the frontend MUST recover by refetching authoritative API state rather than reconstructing history from the live stream.

Changing device context MUST invalidate or isolate old-device synchronization state according to P1.8/P1.7.

### 10.3 No optimistic synchronization completion

Mutation success may update UI mutation status, but MUST NOT set:

- observed actuator state;
- operational readiness;
- config-applied state;
- command confirmation;
- synchronization `IN_SYNC`

without authoritative evidence.

---

## 11. WebSocket synchronization contract

WebSocket is a low-latency notification path over authoritative backend state.

### 11.1 Message envelope

P1.9's canonical contracts remain the source of payload shape. P1.10 adds correlation/freshness metadata where required, without creating a second schema.

A realtime envelope SHOULD identify:

```text
message_id / event_id where an existing stable identity exists
device_id
event/type
occurred_at / observed_at where applicable
received_at where applicable
correlation_id when available
command_id when relevant
version/ordering marker when available
```

### 11.2 Missed messages

The current broadcast capacity is finite and already exposes `Lagged` behavior. P1.10 MUST treat this as an expected delivery failure mode.

Recovery:

```text
WS connected
    |
    +--> event received -> validate ordering -> apply/invalidate cache
    |
    +--> lag/close/reconnect -> mark live stream degraded
                         |
                         v
                  authoritative REST refetch
                         |
                         v
                  resume live notifications
```

The client MUST NOT assume that reconnect itself means state is synchronized.

### 11.3 Backpressure

P1.10 MUST measure WebSocket lag/drop/reconnect rates before increasing buffer sizes. Increasing an in-memory buffer without a recovery policy is not synchronization.

---

## 12. MQTT synchronization contract

### 12.1 Topic/payload

P1.9 owns canonical topic/payload definitions. P1.10 adds delivery and reconciliation semantics.

Every synchronization-sensitive MQTT message MUST have:

- resource/device identity from the existing topic contract;
- message/command identity where applicable;
- correlation identifier where the operation spans boundaries;
- observation time for state/telemetry messages;
- version/order marker where required to reject stale observations.

### 12.2 QoS

The existing QoS assignments remain the baseline. P1.10 MUST NOT blindly raise all topics to QoS 1 or QoS 2.

For each topic class document:

- why its current QoS is sufficient;
- whether duplicates are expected;
- whether loss is acceptable;
- how consumers recover from loss;
- whether retained state or explicit snapshot request is the recovery mechanism.

Sensor telemetry may tolerate loss because InfluxDB receives a time-series stream; command/configuration delivery requires stronger durable/reconciliation semantics.

### 12.3 Duplicate and replay handling

Consumers MUST be idempotent for messages that can be delivered more than once. Duplicate delivery MUST NOT create duplicate command identity, duplicate configuration mutation, or duplicate historical occurrence unless the source identity proves it is a genuinely new occurrence.

Replay of an old observation MUST be rejected/ignored when its ordering marker is older than the current authoritative observation.

---

## 13. Reconciliation and idempotency

### 13.1 Reconciliation loop

P1.10 SHOULD use bounded reconciliation workers for domains that can safely be reconciled from durable authority.

The loop is:

```text
load durable desired/pending state
        |
        v
load latest trustworthy observation
        |
        v
compare version + freshness + semantic content
        |
   +----+----+
   |         |
 equal     mismatch/unknown
   |         |
 IN_SYNC   classify
             |
       retry / request snapshot /
       mark stale / conflict / failed
```

The reconciliation loop MUST be idempotent and bounded. It MUST NOT repeatedly mutate the same state merely because a previous reconciliation already succeeded.

### 13.2 Idempotency keys

Use the strongest existing identity for each domain:

- `command_id` for P1.2 commands;
- configuration revision/version for configuration synchronization;
- producer event identity for Journal events where available;
- message identity/version for protocol-specific state delivery.

`correlation_id` alone MUST NOT be used as an idempotency key because one logical operation may contain multiple valid messages.

### 13.3 Conflict handling

Conflicting desired/observed state MUST be visible and bounded.

The system MUST NOT automatically choose “latest received” when the timestamps/versions do not establish authority.

For safety-critical actuator/configuration conflicts, automatic reconciliation MUST fail closed and require a fresh authoritative observation or an explicit authorized recovery operation.

---

## 14. Outbox / durable handoff decision

P1.10 does **not** mandate an outbox merely because one is a common distributed-systems pattern.

An outbox is required only for a boundary where all of the following are true:

1. a PostgreSQL transaction creates or changes authoritative state;
2. a side effect must be delivered after that commit;
3. losing the side effect between DB commit and transport publish would create an unrecoverable synchronization gap;
4. existing startup reconciliation cannot safely reconstruct the side effect from durable state;
5. the side effect is not already durably represented by P1.2 or another existing mechanism.

If those conditions hold, use a PostgreSQL transactional outbox or equivalent durable handoff rather than adding an external queue by default.

The outbox record MUST contain bounded, non-secret payload data plus:

- outbox/event ID;
- target resource;
- operation/type;
- `correlation_id`;
- related `command_id` or config version where applicable;
- creation time;
- attempt count;
- next retry time;
- terminal delivery status.

The outbox publisher MUST be idempotent. Reprocessing an outbox record MUST not create a second logical command/config mutation.

P1.2's existing durable command record already covers the command-intent-to-MQTT reliability boundary. An outbox MUST NOT duplicate that authority.

---

## 15. Health, readiness, and synchronization health

Health endpoints MUST distinguish process health from dependency readiness and synchronization health.

### 15.1 Liveness

Liveness answers:

> Is this process running and able to execute its basic event loop?

It MUST NOT claim PostgreSQL, InfluxDB, MQTT, or device synchronization is healthy merely because the process is alive.

### 15.2 Readiness

Readiness answers:

> Can this instance safely serve the capabilities it claims to serve now?

At minimum classify relevant dependencies:

```text
PostgreSQL
InfluxDB
MQTT transport
event/WebSocket fan-out
configuration synchronization worker
command reconciliation worker
```

The exact readiness response MUST follow existing health API conventions and P1.3 error semantics. A dependency failure MUST NOT be represented as an empty successful data response.

### 15.3 Device operational health

Device operational state remains P1.4 authority:

```text
MQTT connected != device contacted != telemetry fresh != runtime ready != actuator known
```

Backend readiness MUST NOT be substituted for device readiness.

### 15.4 Sync health signals

Expose bounded metrics/diagnostics for:

- age of oldest pending synchronization item;
- count of pending/failed/conflicted sync items;
- reconciliation success/failure count;
- MQTT delivery retry count;
- WebSocket reconnect/lag count;
- stale observation count;
- desired-vs-observed version mismatch count;
- command `UNKNOWN`/timeout counts;
- outbox backlog only if an outbox is actually implemented.

Do not expose per-message payloads or unbounded IDs as metric labels.

---

## 16. Observability design

### 16.1 Logs

Logs MUST be structured and useful for a single operation across boundaries.

Required fields where known:

```text
timestamp
level
service
environment
component
event_type
device_id (only where bounded/approved)
request_id
correlation_id
trace_id
command_id
config_version
outcome/reason_code
duration_ms where relevant
```

Not every log line needs every field. The rule is to propagate context, not to duplicate all state into every line.

Secrets MUST never appear in structured fields or formatted error strings.

Avoid logging full request/response/MQTT payloads by default. Payload sampling for diagnostics, if ever needed, MUST be bounded, redacted, explicitly enabled, and time-limited.

### 16.2 Metrics

Use counters for events/failures, gauges for current/backlog/state, and histograms for latency/duration distributions where appropriate.

Minimum cross-system metric families:

```text
hydragrow_sync_attempts_total
hydragrow_sync_failures_total
hydragrow_sync_reconciliations_total
hydragrow_sync_pending
hydragrow_sync_staleness_seconds
hydragrow_sync_conflicts_total
hydragrow_mqtt_delivery_total
hydragrow_mqtt_delivery_failures_total
hydragrow_ws_reconnects_total
hydragrow_ws_lagged_total
hydragrow_command_unknown_total
hydragrow_command_timeout_total
hydragrow_config_version_mismatch_total
```

These are target metric names, not permission to add every metric immediately. Existing equivalent metrics MUST be reused rather than duplicated. Where an existing metric already has a different stable name, P1.10 MUST either document the existing name as canonical or perform an explicit migration; it MUST not expose two permanently equivalent families.

Labels MUST be bounded and stable. Never label by:

- `correlation_id`;
- `trace_id`;
- `command_id`;
- raw `request_id`;
- event message;
- arbitrary URL/query string;
- secret;
- unbounded user input.

P1.10 therefore requires an explicit cardinality budget for every new label dimension. Device IDs already appear on many metrics, so the implementation MUST quantify expected series growth and define an operational upper bound rather than treating device identity as automatically safe.

### 16.3 Traces

Trace spans SHOULD cover the major cross-system boundaries:

```text
HTTP request
  -> authorization
  -> DB transaction
  -> MQTT publish
  -> controller handling (if propagation supported)
  -> observed confirmation
  -> Journal/event write
  -> WebSocket notification
```

Spans MUST record outcome, bounded resource identity, and timing. They MUST NOT record secrets or entire payloads.

The implementation MUST distinguish:

- trace context for diagnostics;
- durable correlation for business operations;
- command identity for lifecycle correctness.

If no production trace backend is available, P1.10 may first establish context propagation and local span evidence without claiming a complete trace backend.

### 16.4 Exemplars

If Prometheus/OpenMetrics exemplars are used, trace IDs may be attached as exemplars rather than high-cardinality metric labels. This remains optional and must not become a requirement for correctness.

---

## 17. Security and privacy

P1.10 preserves P1.1/P1.6/P1.9 security constraints.

### 17.1 Never log

- API keys;
- Firebase tokens;
- MQTT credentials;
- WiFi passwords;
- privileged control secrets;
- signing keys/private keys;
- session cookies;
- authorization headers;
- full backup artifacts;
- secret-bearing config values.

### 17.2 Correlation identifiers

Correlation IDs are identifiers, not credentials. They MUST be unguessability-safe enough for diagnostics but MUST NOT grant access to an API or device.

External caller-supplied correlation IDs MUST be validated for length/format and may be replaced or namespaced to avoid log-injection and cross-tenant confusion.

### 17.3 Authorization

Observability endpoints and synchronization diagnostics MUST not bypass device ownership. Metrics may be separately protected by the existing `METRICS_TOKEN` boundary.

Per-device operational diagnostics MUST use P1.1 ownership/capability checks.

WebSocket and MQTT resource identity MUST never become authorization proof by itself.

---

## 18. Failure and delivery semantics

Every cross-system operation MUST classify failure at the boundary where it occurs.

| Failure | Meaning | Required action |
|---|---|---|
| DB commit failed | desired/command authority not durable | do not publish side effect |
| MQTT publish failed | transport delivery not confirmed | retry/reconcile per command/config policy |
| MQTT publish succeeded | broker accepted publish | do not claim device applied |
| controller ACK absent | target receipt unknown | deadline/reconcile; do not infer success |
| observation absent | physical/applied state unknown | stale/unknown, then reconcile |
| WebSocket send failed | live notification missed | client/API recovery; authoritative state unchanged |
| event bus lagged | some realtime events dropped | mark stream degraded and refetch/reconcile |
| Influx write failed | telemetry history may be incomplete | metric/log failure; do not fabricate sample |
| Journal write failed | historical evidence unavailable | follow P1.6 criticality policy |
| cache write failed | acceleration unavailable | keep authority unchanged |
| readiness dependency failed | capability degraded | readiness reflects dependency, not fake success |

No failure path may convert “not delivered” into “delivered”, “delivered” into “applied”, or “cache updated” into “authoritative”.

---

## 19. Evidence and diagnostic model

P1.10 evidence MUST connect an observed symptom to a correlation chain.

Minimum diagnostic record:

```text
requirement_id:
operation:
request_id:
correlation_id:
trace_id:
command_id/config_version/event_id:
device_id:
desired_version/state:
observed_version/state:
observed_at:
received_at:
freshness:
delivery_attempt:
transport:
outcome:
reason_code:
test_command:
environment:
timestamp:
```

IDs that do not exist for a given operation are omitted, not fabricated.

Evidence MUST distinguish:

- executable test evidence;
- source/static evidence;
- production/runtime evidence;
- environment `BLOCKED` evidence;
- historical evidence.

No green synchronization claim may be based solely on a log line.

---

## 20. Test matrix

### 20.1 Observability

| Scenario | Expected evidence |
|---|---|
| HTTP request | request/correlation/trace context present without secrets |
| DB failure | error log + failure metric, no false success |
| MQTT failure | bounded retry/failure metric and correlation preserved |
| MQTT reconnect | reconnect metric/log; no fabricated device state |
| WebSocket connect/disconnect | active connection/reconnect metrics |
| WebSocket lag | lag metric/log + recovery path |
| command lifecycle | same `command_id` across lifecycle events |
| Journal event | event/correlation fields persisted per P1.6 |
| trace unavailable | operation remains correct without trace backend |
| high-cardinality input | not used as metric label |
| secret-bearing operation | no secret in logs/metrics/traces |

### 20.2 Synchronization

| Scenario | Expected result |
|---|---|
| desired config written | `PENDING`, not `IN_SYNC` merely from DB success |
| target confirms matching revision | `IN_SYNC` |
| target reports older revision | `PENDING` + bounded reconciliation |
| observation exceeds freshness window | `STALE` |
| no observation | `UNKNOWN` |
| conflicting valid revisions | `CONFLICT`, no unsafe auto-resolution |
| duplicate config delivery | one logical mutation |
| duplicate command delivery | same `command_id`, no duplicate logical command |
| old MQTT observation | ignored/rejected by ordering rule |
| MQTT publish succeeds | transport success only |
| physical observation matches | command may become `CONFIRMED` per P1.2 predicate |
| backend restart | durable pending work reconstructed/reconciled |
| WebSocket reconnect | API refetch/reconciliation before declaring current state |
| WebSocket lag | cache recovered from authoritative query |
| frontend cache stale | stale/refetch semantics preserved |
| Influx write failure | no fabricated telemetry/history |
| PostgreSQL unavailable | readiness/dependency error, no false sync success |

### 20.3 Cross-system matrix

At least one executable path MUST cover each chain:

```text
HTTP -> PostgreSQL -> MQTT -> controller observation -> PostgreSQL -> WS -> frontend
HTTP config mutation -> desired config -> MQTT -> observed config -> sync status
MQTT telemetry -> backend -> InfluxDB -> API -> frontend cache
controller/system event -> PostgreSQL Journal -> API/WS -> frontend Journal
```

The tests MUST verify identity/correlation, freshness, ordering, authorization, and recovery—not merely HTTP status codes.

---

## 21. Acceptance criteria

### AC-1 — Observability/synchronization boundary

Logs, metrics, and traces are documented as diagnostics; authoritative synchronization state remains in its declared domain store/producer.

### AC-2 — Current infrastructure truthfulness

The implementation documentation distinguishes the existing Prometheus `/metrics`, `tracing`/Loki integration, WebSocket broadcast, MQTT, PostgreSQL, and InfluxDB facts from proposed future integrations. No absent infrastructure is claimed as deployed.

### AC-3 — Correlation

Cross-system operations preserve a stable `correlation_id` where supported, while `request_id`, `trace_id`, `command_id`, `event_id`, and config version retain distinct meanings.

### AC-4 — Trace propagation

Trace context is propagated across supported backend boundaries without being used as business identity or authorization.

### AC-5 — Freshness

Synchronization-sensitive observations have explicit freshness/staleness rules and do not become current merely because they were cached or recently received.

### AC-6 — Desired versus observed

Configuration and operational state distinguish desired, observed/applied, cached, transport, and synchronization state.

### AC-7 — Ordering

Delayed/out-of-order observations cannot overwrite newer authoritative state or restore freshness incorrectly.

### AC-8 — Idempotency

Duplicate MQTT/WebSocket/reconciliation delivery does not create duplicate logical commands, configuration mutations, or historical occurrences where a stable identity exists.

### AC-9 — Delivery guarantees

Every relevant boundary documents its actual delivery guarantee. No exactly-once physical or WebSocket guarantee is claimed without evidence.

### AC-10 — Retry/reconciliation

Retry is bounded, command-class/config-class aware, and does not create a second retry authority. Reconciliation is idempotent and restart-safe.

### AC-11 — Outbox decision

An outbox is added only where the documented commit-to-side-effect gap cannot be safely recovered from existing durable state. If added, it is transactional, bounded, idempotent, and PostgreSQL-backed unless a separate infrastructure decision explicitly approves otherwise.

### AC-12 — WebSocket recovery

WebSocket lag, disconnect, and reconnect recover from authoritative REST/query state and never treat the live stream as complete history.

### AC-13 — MQTT recovery

MQTT reconnect restores subscriptions/transport without fabricating device state; pending desired/command work is reconciled from durable authority.

### AC-14 — Health/readiness

Process liveness, dependency readiness, device operational health, and synchronization health are distinct and testable.

### AC-15 — Metrics cardinality

New metric labels have explicit bounded cardinality. Correlation/request/command IDs and arbitrary payload values are not metric labels.

### AC-16 — Security

Secrets are absent from logs, metrics, traces, Journal correlation metadata, WebSocket diagnostics, and synchronization records except for existing explicitly redacted representations.

### AC-17 — Cache authority

Backend/frontend caches are demonstrably non-authoritative and carry/recover freshness/version semantics where required.

### AC-18 — Cross-system tests

The required HTTP/MQTT/PostgreSQL/InfluxDB/WebSocket/frontend paths have executable tests for success, duplication, staleness, ordering, failure, restart, and recovery as applicable.

### AC-19 — Existing phase invariants

P1.1 authorization, P1.2 command lifecycle, P1.3 safety semantics, P1.4 operational state, P1.6 Journal authority, P1.7 cache/API ownership, P1.8 context isolation, and P1.9 canonical schema ownership remain intact.

### AC-20 — Evidence integrity

Every claimed acceptance result is backed by current executable evidence or explicitly marked `BLOCKED`/`DIAGNOSED`; historical or static inspection is never represented as runtime PASS.

---

## 22. Implementation order

Implementation MUST proceed in this order:

```text
P1.10 Observability + Cross-System Synchronization
    |
    +-- 1. Freeze P1.0-P1.9 contracts and current worktree
    |
    +-- 2. Audit current logs/metrics/tracing and all sync producers/consumers
    |
    +-- 3. Build authority matrix: desired / observed / applied / cache / transport
    |
    +-- 4. Define correlation_id + request_id + trace_id propagation rules
    |
    +-- 5. Define freshness/order/version rules per resource class
    |
    +-- 6. Audit Prometheus labels/cardinality and existing metrics for reuse
    |
    +-- 7. Normalize structured log context and secret redaction
    |
    +-- 8. Add trace propagation where existing infrastructure supports it
    |
    +-- 9. Normalize WebSocket missed-event recovery
    |
    +-- 10. Normalize MQTT delivery/replay/reconciliation semantics
    |
    +-- 11. Define desired/observed configuration synchronization
    |
    +-- 12. Add bounded reconciliation workers where needed
    |
    +-- 13. Decide per boundary whether an outbox is actually necessary
    |
    +-- 14. Add health/readiness/synchronization signals
    |
    +-- 15. Add unit/integration/restart/duplicate/staleness tests
    |
    +-- 16. Run cross-system verification
    |
    +-- 17. Record evidence + traceability
    |
    v
P1 completion / operational hardening
```

No step may introduce a second source of truth merely because it is easier to instrument.

---

## 23. Rollout strategy

P1.10 SHOULD roll out in independently reversible increments.

### Phase A — instrumentation only

- add correlation context;
- audit/normalize logs;
- add/reuse bounded metrics;
- add trace propagation without changing state decisions;
- validate secret redaction and cardinality.

This phase MUST be behavior-neutral.

### Phase B — synchronization visibility

- expose desired/observed/version/freshness state;
- expose pending/conflict/stale diagnostics;
- measure WebSocket lag and MQTT retry/replay behavior;
- keep existing authority and delivery behavior unchanged.

### Phase C — reconciliation

- enable bounded reconciliation for configuration and other approved domains;
- enable restart recovery where durable state permits it;
- introduce outbox only for boundaries proven to require it;
- enable automatic retry only for explicitly safe/idempotent operations.

### Phase D — enforcement

- reject stale/conflicting observations where current code still accepts them;
- remove compatibility behavior only after evidence shows migration is complete;
- make readiness reflect synchronization blockers where operationally required.

### Rollback

Rollback MUST preserve authoritative PostgreSQL command/config/event state. Disable workers/features rather than deleting durable state.

Rollback MUST NOT:

- re-enable blind retries for dangerous commands;
- treat cache as authority;
- convert stale/unknown into healthy/current;
- discard correlation IDs needed for investigation;
- delete Journal or command history to hide synchronization defects.

---

## 24. Risks

- High-cardinality metrics can overload Prometheus even when individual metrics look harmless.
- Adding correlation fields to every log can increase volume and cost; fields must be bounded and useful.
- Device clock skew can make timestamp-only ordering unsafe.
- MQTT QoS 1 can produce duplicates and therefore requires idempotent consumers.
- WebSocket broadcast lag can make a UI look current while missing intermediate events unless recovery is explicit.
- Configuration desired/observed semantics may expose that the controller does not currently report a complete applied revision.
- Introducing an outbox where P1.2 already has durable command authority can create two competing delivery records.
- Reconciliation can accidentally become a hidden retry engine if action eligibility is not separated from state comparison.
- Logging payloads for debugging can leak secrets or personal data.
- Treating Loki integration as deployed when the repository Compose file has no Loki service would create false operational evidence.
- Treating frontend `@opentelemetry/api` dependency presence as distributed tracing would create false capability claims.

---

## 25. Evidence artifact

Create:

`docs/evidence/P1.10-OBSERVABILITY-CROSS-SYSTEM-SYNC.json`

The evidence MUST record:

- implementation/worktree timestamp;
- environment/runtime versions;
- current infrastructure inventory and verified deployment status;
- observability metric/log/trace inventory;
- metric cardinality audit;
- correlation propagation coverage;
- freshness/version rules by synchronized resource;
- desired/observed configuration evidence;
- MQTT delivery/retry/replay evidence;
- WebSocket lag/reconnect/recovery evidence;
- PostgreSQL reconciliation/restart evidence;
- Influx telemetry persistence evidence where relevant;
- outbox decision and justification, whether implemented or rejected;
- health/readiness results;
- security/redaction tests;
- cross-system test matrix results;
- AC-1 through AC-20 status;
- known gaps and `BLOCKED`/`DIAGNOSED` environment limitations.

Evidence MUST distinguish “instrumented” from “observable in production” and “synchronized” from “notification delivered”.

---

## 26. Definition of done

P1.10 is complete only when HYDRAGROW can answer, for a representative cross-system operation:

1. **What happened?** — logs/Journal provide bounded historical evidence.
2. **How often/how badly?** — metrics provide bounded operational signals.
3. **Which operation is this?** — correlation/trace context links the relevant hops.
4. **What was desired?** — the authoritative desired state is identifiable.
5. **What was observed/applied?** — authoritative observation and freshness are identifiable.
6. **Is it synchronized?** — `IN_SYNC`, `PENDING`, `STALE`, `CONFLICT`, `FAILED`, or `UNKNOWN` has a defined reason.
7. **What was delivered?** — MQTT/WebSocket delivery status is distinguished from application/physical confirmation.
8. **What happens after failure?** — retry/reconciliation is bounded, idempotent, and restart-safe.
9. **Can the UI recover?** — missed WebSocket events do not corrupt authoritative frontend state.
10. **Is it safe to operate?** — P1.3/P1.4 safety and readiness semantics remain authoritative and fail closed.

P1.10 is not complete because dashboards exist or because logs look detailed. It is complete when observability makes cross-system behavior diagnosable and synchronization semantics make authority, freshness, delivery, reconciliation, and recovery explicit and testable.

---

## 27. Traceability to P1.0-P1.9

| Prior phase | P1.10 dependency |
|---|---|
| P1.0 | verification/evidence integrity, environment classification, current-vs-historical evidence |
| P1.1 | authorization/ownership for diagnostics and device-scoped synchronization |
| P1.2 | durable command identity, lifecycle, retry, timeout, `UNKNOWN`, restart recovery |
| P1.3 | fail-closed failure/safety semantics and no fabricated state |
| P1.4 | operational state, freshness, contact/readiness/actuator knowledge |
| P1.5 | persistence/restore boundaries; no backup semantics pulled into observability |
| P1.6 | immutable Journal events, event identity, correlation metadata, redaction |
| P1.7 | API ownership, React Query cache, invalidation/retry rules |
| P1.8 | station/device context and isolation during realtime/cache recovery |
| P1.9 | canonical wire fields, timestamps, precision, correlation/version fields, one schema source |

P1.10 is the final cross-system reliability/operability phase in the P1 sequence. It consumes these contracts; it does not silently redefine them.

---

## 28. Repository constraints

- Preserve all existing dirty worktree changes.
- Do not reset, clean, stash, overwrite, or commit unrelated work.
- Do not create a second canonical schema after P1.9.
- Start with source/runtime audit before changing behavior.
- Reuse existing PostgreSQL/InfluxDB/MQTT/WebSocket/Prometheus/tracing infrastructure where practical.
- Treat absent infrastructure as absent; do not convert dependency declarations or code integrations into deployment claims.
- Record actual verification commands and outcomes.
- Keep synchronization authority explicit and durable where correctness depends on it.
- Never claim exactly-once physical actuation from transport-level delivery.
