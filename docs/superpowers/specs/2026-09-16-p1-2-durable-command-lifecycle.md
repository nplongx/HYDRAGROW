# P1.2 Durable Command Lifecycle

## Status

Spec ready; implementation not started.

P1.2 makes the existing correlated command lifecycle durable and recoverable across backend restart, MQTT reconnect, controller restart, and multi-instance deployment. It builds on P0.2's shared lifecycle contract and P1.1's authorization boundary. It does not reopen P0/P1.1 architecture.

## Requirement ID

`P1.2-DURABLE-COMMAND-LIFECYCLE`

## Change class

`C4` Cross-subsystem + `C5` Data persistence + `C6` Distributed state/reliability + `C7` Security-sensitive

## Purpose

A command is a durable user intent, not an in-memory map entry or a transient MQTT publish.

The system must answer, after process/device/network failure:

- what command was requested;
- which device it targets;
- who/which authenticated principal requested it;
- what lifecycle state is authoritative;
- whether the command was published;
- whether the controller acknowledged it;
- whether physical/runtime state confirmed it;
- whether it timed out or became permanently unknown;
- whether a retry would be a new command or a retry of the same intent;
- whether the command can still be acted upon safely.

The durable record must be the source of truth for command lifecycle history. In-memory state may be a cache/working set only.

---

## 1. Scope

P1.2 covers:

- durable command records in PostgreSQL;
- durable lifecycle transition history;
- idempotent command creation and correlation;
- transactional creation of command intent before MQTT publication;
- lifecycle transition validation;
- MQTT publish/retry semantics;
- controller acknowledgement correlation;
- physical/runtime confirmation correlation;
- timeout and `UNKNOWN` reconciliation;
- backend restart/recovery;
- MQTT reconnect/recovery;
- multi-instance backend command authority;
- durable command history API;
- WebSocket lifecycle delivery from durable state;
- frontend lifecycle consumption where required to expose durable state correctly;
- authorization integration from P1.1;
- negative/failure-path tests and evidence.

P1.2 does **not** redesign the FSM, telemetry architecture, scheduler, RBAC, Firebase, PostgreSQL, MQTT broker, backup lifecycle, or frontend data architecture.

---

## 2. Existing contract and implementation baseline

P0.2 already defines the shared command identity and lifecycle vocabulary in `hydragrow-shared/src/command.rs`:

```text
REQUESTED
   |
   v
SENT
   |
   v
ACKNOWLEDGED
   |
   v
CONFIRMED
```

Terminal outcomes are:

```text
REJECTED
FAILED
TIMEOUT
UNKNOWN
```

The current implementation already carries `command_id` through MQTT and emits lifecycle events. The backend currently keeps `CommandRecord` in:

```text
AppState.command_lifecycles: HashMap<(device_id, command_id), CommandRecord>
```

and mirrors lifecycle events into `system_events`.

That is insufficient for P1.2 because `system_events` is an event/audit stream, not a command aggregate, and the in-memory map disappears on process restart and is not authoritative across backend instances.

P1.2 must preserve the existing shared wire contract unless a concrete implementation defect requires a documented versioned change.

---

## 3. Core invariants

### 3.1 One command identity

Every externally initiated command has exactly one stable `command_id` for its logical execution attempt.

The `command_id` must:

- be generated before publication;
- be globally unique enough for the database uniqueness constraint;
- remain unchanged across MQTT retransmission of the same command;
- never be reused for a different device, action, or request;
- be present in durable storage before the first publish attempt.

A transport retry is **not** a new command.

If the user intentionally requests the same action again, that is a new command with a new `command_id` unless an explicit idempotency key identifies it as the same logical request.

### 3.2 Durable source of truth

PostgreSQL is authoritative for command lifecycle state.

The in-memory command map, if retained, is a cache and must be reconstructable from PostgreSQL. No correctness decision may depend on state that exists only in process memory.

### 3.3 Valid transitions only

Every lifecycle update must use the shared transition rules.

A stale, duplicate, reordered, or invalid event must not move a command backward or revive a terminal command.

Examples that must be rejected/ignored:

```text
CONFIRMED -> ACKNOWLEDGED
FAILED -> SENT
TIMEOUT -> CONFIRMED
UNKNOWN -> SENT
CONFIRMED -> CONFIRMED
```

Duplicate delivery of an already accepted state is idempotent and must not create a second logical transition.

### 3.4 Terminal means terminal

`CONFIRMED`, `REJECTED`, `FAILED`, `TIMEOUT`, and `UNKNOWN` are terminal.

Once terminal, the command cannot return to an active state.

If a later observation contradicts a terminal state, the system must record the contradiction as an audit/diagnostic event rather than silently rewriting history.

### 3.5 No false success

MQTT publish success means only that the broker accepted the message for publication.

It does **not** mean:

- controller received the command;
- controller applied it;
- actuator changed;
- physical state reached the requested state.

`SENT`, `ACKNOWLEDGED`, and `CONFIRMED` must remain distinct.

---

## 4. Durable data model

Create a dedicated command aggregate table rather than using `system_events` as the primary command store.

Suggested logical model:

```text
commands
--------
command_id             PK
idempotency_key        nullable, unique within principal/resource policy
principal_kind         user | service | internal
principal_id           nullable
service_key_label      nullable
session_id             nullable
user_id                nullable
 device_id             FK/reference as appropriate
action
request_payload        JSONB, redacted/non-secret
requested_state        nullable
requested_pwm          nullable
pump_id                nullable
lifecycle              canonical enum/string
created_at
authorized_at
sent_at                nullable
acknowledged_at        nullable
confirmed_at           nullable
terminal_at            nullable
next_retry_at          nullable
attempt_count
last_error             nullable
last_observed_at       nullable
version                optimistic concurrency/version field
```

Exact column names may follow repository conventions, but the following properties are mandatory:

- unique primary key on `command_id`;
- durable target `device_id`;
- durable authenticated principal attribution;
- durable action/request intent;
- durable current lifecycle;
- timestamps for lifecycle milestones where known;
- retry/attempt information;
- safe error/reason information without secrets;
- indexes for `(device_id, created_at)`, `(lifecycle, next_retry_at)`, and command lookup;
- ownership queries must continue to use P1.1's authoritative `device_ownership` relationship.

### 4.1 Lifecycle history table

Use a separate append-only history table, e.g. `command_lifecycle_events`, to preserve each accepted transition:

```text
id
command_id
sequence_no
from_lifecycle
lifecycle
device_id
reason
source
attempt_no
occurred_at
metadata
```

Required uniqueness:

```text
(command_id, sequence_no)
```

and/or an equivalent transition-event idempotency key.

History is append-only. Corrections are new diagnostic events, not destructive updates.

### 4.2 Request payload safety

Do not persist secrets in command payloads or lifecycle metadata.

Examples that must not be stored in cleartext:

- API keys;
- Firebase tokens;
- privileged-control tokens;
- WiFi passwords;
- webhook secrets;
- private signing material.

Persist only the minimum request fields needed for retry, audit, reconciliation, and UI history.

---

## 5. Command creation transaction

The command lifecycle starts in the database before MQTT publication.

Required order:

```text
authenticate
   |
resolve principal
   |
check capability
   |
check ownership
   |
generate command_id
   |
BEGIN DB transaction
   |
insert command REQUESTED
   |
insert lifecycle event REQUESTED
   |
COMMIT
   |
publish MQTT command with same command_id
```

The authorization decision is P1.1's boundary. P1.2 must not create a second authorization mechanism.

If durable creation fails, MQTT must not be published.

If MQTT publication fails after durable creation:

```text
REQUESTED -> FAILED
```

or a documented retryable state must be used. Do not report success while the durable record says the command never left the backend.

The preferred model is to retain `REQUESTED` with retry metadata for transient transport failure and use `FAILED` only after the retry policy declares publication permanently failed.

---

## 6. Outbound MQTT delivery

MQTT remains the transport, not the lifecycle database.

Each publication carries:

- `command_id`;
- target device identity;
- action;
- required command parameters;
- existing authentication/signature fields where applicable.

### 6.1 Delivery attempt

Each publish attempt increments `attempt_count` durably or updates it using an atomic operation.

The same `command_id` is reused for retries of the same logical command.

The backend must tolerate duplicate MQTT delivery. The controller must treat a repeated `command_id` as the same command identity rather than creating a new lifecycle identity.

### 6.2 Retry policy

Retry policy must be bounded.

Required properties:

- finite maximum attempts or finite retry deadline;
- bounded backoff;
- durable `next_retry_at`;
- retry after backend restart based on database state;
- no infinite retry loop;
- no retry after terminal state;
- no retry of commands that are explicitly non-repeatable unless the command class declares retry safety.

Command classes must explicitly identify whether transport retry is safe.

For dangerous actuator commands, the implementation must prefer controller-side idempotency/correlation over blind repeated actuation.

---

## 7. Lifecycle transition authority

Only the backend command service may mutate the durable aggregate lifecycle.

Inputs may originate from:

1. command creation;
2. successful/failed MQTT publication;
3. controller lifecycle acknowledgement/rejection;
4. runtime/telemetry confirmation;
5. timeout reconciliation;
6. startup/recovery reconciliation.

All inputs must pass through one transition function/service that:

- loads current durable state;
- validates the requested transition;
- performs an atomic compare/update or transaction;
- appends exactly one accepted history record;
- updates lifecycle timestamps;
- emits the application event after durable commit.

No handler may directly mutate the lifecycle column outside this boundary.

### 7.1 Concurrency

Multiple backend instances may receive the same MQTT lifecycle event.

The implementation must prevent:

- duplicate history rows;
- conflicting state overwrites;
- backward transitions;
- one instance acknowledging a transition that another instance already terminalized.

Use PostgreSQL row locking, optimistic versioning, or an equivalent atomic conditional update. The mechanism must be deterministic and tested under duplicate/concurrent transition attempts.

---

## 8. Controller acknowledgement

The controller's `ACKNOWLEDGED` event means:

> the controller accepted/recognized the command at its command boundary.

It does not mean physical success.

The backend must accept an acknowledgement only when:

- `command_id` exists;
- command `device_id` matches the MQTT topic/device identity;
- the transition is valid;
- the command is not terminal;
- the event is not stale/replayed beyond the documented correlation window.

Unknown command IDs must not create arbitrary command records from unsolicited device events.

An invalid cross-device lifecycle event must be rejected and audited without mutating the target command.

---

## 9. Physical/runtime confirmation

`CONFIRMED` is a physical/runtime outcome, not a transport acknowledgement.

The confirmation source remains the authoritative runtime/controller state already established by P0/P0.2.

The backend must only transition to `CONFIRMED` when the observed state satisfies the command's confirmation predicate.

Examples:

- pump ON command -> target pump state observed ON;
- pump OFF -> target pump state observed OFF;
- PWM command -> observed PWM matches required value/tolerance;
- emergency stop -> emergency state plus required pumps/actuators are safely off;
- reset fault -> documented healthy/monitoring state is observed.

A status snapshot must not confirm a command belonging to another device or unrelated action.

If a command is already terminal, a later matching observation is diagnostic only and must not reopen it.

---

## 10. Timeout and UNKNOWN policy

P1.2 must make timeout semantics explicit rather than leaving commands indefinitely in `SENT` or `ACKNOWLEDGED`.

### 10.1 Timeout

`TIMEOUT` means the system reached a defined deadline without obtaining the expected lifecycle evidence.

Timeout policy must be command-class aware where necessary.

At minimum define deadlines for:

- publication retry window;
- `SENT -> ACKNOWLEDGED`;
- `ACKNOWLEDGED -> CONFIRMED`.

The exact default values must be configuration, documented, bounded, and covered by tests rather than hard-coded in multiple handlers.

### 10.2 UNKNOWN

`UNKNOWN` is distinct from `TIMEOUT`.

Use `UNKNOWN` when the system cannot safely determine whether the command was applied, especially after a crash/reconnect boundary where delivery may have occurred but confirmation evidence is missing.

Examples:

```text
backend crashed after MQTT publish but before durable SENT update
broker/session state is unavailable
controller restarted with no retained command correlation
command deadline passed but telemetry continuity is insufficient
```

The system must not automatically convert `UNKNOWN` into success.

Whether an `UNKNOWN` command may be retried is command-class specific. Dangerous commands default to **no automatic retry** until an explicit reconciliation policy proves that repeating the action is safe.

---

## 11. Crash/restart recovery

### 11.1 Backend restart

On startup, the backend must load/reconcile non-terminal commands from PostgreSQL.

Recovery must:

- reconstruct the in-memory cache if one remains;
- find commands in `REQUESTED`, `SENT`, and `ACKNOWLEDGED`;
- evaluate deadlines and retry eligibility;
- resume bounded publication retry where safe;
- mark commands `TIMEOUT` or `UNKNOWN` when policy requires;
- avoid publishing commands that are already terminal;
- avoid duplicating lifecycle history.

Startup recovery must be idempotent: running it twice produces the same durable state.

### 11.2 MQTT reconnect

MQTT reconnect must not reset command identity or lifecycle state.

After reconnect, the backend reconciles from PostgreSQL and only schedules commands eligible for retry.

A reconnect is not evidence that a previous command was lost.

### 11.3 Controller restart

The controller must preserve enough command correlation state, where technically feasible, to prevent duplicate command execution after reconnect.

If controller-side persistence is not available for a command class, backend policy must treat ambiguous post-restart commands as `UNKNOWN` rather than assuming delivery or failure.

---

## 12. Multi-instance backend authority

P1.2 must work when two or more backend instances consume the same lifecycle stream or recover the same command set.

Requirements:

- PostgreSQL is the shared authority;
- no instance-local map is authoritative;
- duplicate lifecycle messages are safe;
- concurrent transition attempts are serialized or conditionally accepted;
- retry scheduling has one effective execution per command attempt, or duplicate publication is explicitly safe because `command_id` is idempotent;
- startup recovery may run on multiple instances without corrupting state.

A distributed lock may be used for scheduling, but correctness must not depend solely on a process-local mutex.

---

## 13. API contract

The existing device-scoped command history endpoint must read from the durable command store rather than reconstructing current state from the in-memory map or only from `system_events`.

Minimum API capabilities:

```text
GET /devices/{device_id}/control/commands
GET /devices/{device_id}/control/commands/{command_id}
```

Exact route naming may follow existing API conventions.

Responses must expose, at minimum:

- `command_id`;
- device ID;
- action;
- lifecycle;
- requested time;
- last update time;
- relevant request parameters that are safe to expose;
- terminal/error reason where present;
- retry/attempt status where useful.

Authorization remains P1.1's rule:

```text
authenticate -> principal -> capability -> ownership -> read durable command
```

A user must never read another user's command history.

A nonexistent command and an unauthorized command must preserve the repository's documented anti-enumeration/error semantics.

---

## 14. WebSocket/event delivery

WebSocket lifecycle events are notifications of durable state, not the authoritative store.

Required order:

```text
validate transition
     |
commit durable state + history
     |
publish AppEvent / WebSocket notification
```

If event-bus delivery fails after database commit, the command remains correct and can be recovered by API reload/reconciliation.

If a WebSocket client reconnects, it must not depend on receiving every historical event from the live stream. The client reloads current durable command state and then resumes live updates.

P1.1 ownership filtering remains mandatory.

---

## 15. Idempotency

P1.2 must support an optional caller-provided idempotency key for HTTP command creation where repeated client submission is reasonably expected.

Recommended semantic key:

```text
(principal, idempotency_key, device_id, action-class)
```

The exact uniqueness scope must prevent one principal from accidentally or maliciously reusing another principal's key.

Required behavior:

- same key + same logical request -> return/reuse the existing command;
- same key + materially different request -> `409 Conflict` or documented equivalent;
- key cannot bypass authorization;
- idempotency record survives backend restart;
- idempotency does not make two intentional user actions collapse into one unless the caller supplies the same key.

If idempotency is not exposed on a route, the route must still have stable `command_id` semantics for transport retry.

---

## 16. Bulk commands

Bulk command creation must remain all-or-nothing at the authorization boundary from P1.1.

For a multi-device command request:

```text
validate request
-> authorize_all(all targets)
-> create durable command records for all targets
-> commit atomically
-> publish each command with its own command_id
```

The implementation must define partial transport outcomes after the DB transaction commits.

A failure to publish command B must not erase durable command A. Each command receives its own lifecycle and retry policy.

The API must not report the whole bulk operation as successful merely because some commands were published.

---

## 17. Audit and observability

Every accepted lifecycle transition must be auditable with:

- command ID;
- device ID;
- lifecycle;
- transition source;
- timestamp;
- attempt number where relevant;
- safe reason code/message;
- authenticated principal attribution for the originating command.

Do not log secrets or full authentication credentials.

Metrics should cover at minimum:

- commands created;
- publish attempts;
- publish failures;
- acknowledgements;
- confirmations;
- rejections;
- failures;
- timeouts;
- unknown outcomes;
- retries;
- recovery actions;
- lifecycle transition conflicts/duplicates.

The metrics are operational signals only; PostgreSQL remains authoritative.

---

## 18. Security requirements

P1.2 must preserve P1.1 invariants:

- command creation requires valid authentication;
- capability is checked explicitly;
- ownership is checked from `device_ownership`;
- command records cannot be created for an unauthorized device;
- lifecycle events cannot mutate a different device's command;
- user-supplied `device_id`, MQTT topic, frontend selection, or command ID is not an ownership proof;
- service credentials remain explicitly classified;
- privileged control tokens are never persisted/logged as secrets;
- command history is device-scoped and ownership-protected;
- no lifecycle event can be used to manufacture an authorization grant.

A command ID is an identifier, not a bearer authorization credential.

---

## 19. Testing requirements

### 19.1 Shared contract tests

Preserve and extend tests for:

- legal transitions;
- illegal transitions;
- terminal immutability;
- canonical serialization;
- duplicate/replayed event behavior.

### 19.2 Backend persistence tests

Require database-backed tests for:

1. create `REQUESTED` transactionally;
2. create fails -> no MQTT publish;
3. publish success -> `SENT`;
4. publish failure -> retry/failure policy;
5. duplicate publish does not create a new command;
6. acknowledgement -> `ACKNOWLEDGED`;
7. physical state -> `CONFIRMED`;
8. rejection -> `REJECTED`;
9. failure -> `FAILED`;
10. deadline -> `TIMEOUT`;
11. ambiguous crash/reconnect -> `UNKNOWN`;
12. terminal state cannot reopen;
13. duplicate lifecycle event is idempotent;
14. concurrent transition attempts produce one accepted transition;
15. restart recovery reconstructs active commands;
16. recovery is idempotent;
17. retry resumes after restart only when eligible;
18. command history survives process restart;
19. command history remains ownership-protected;
20. unknown command lifecycle events do not create commands.

### 19.3 Integration tests

At least one end-to-end test must cover:

```text
HTTP command request
 -> durable REQUESTED
 -> MQTT publish
 -> SENT
 -> controller ACK
 -> ACKNOWLEDGED
 -> runtime state observation
 -> CONFIRMED
 -> API/WS observes final durable state
```

Failure-path integration must cover publish failure, controller rejection, timeout, and restart/reconnect ambiguity.

### 19.4 Concurrency tests

Run two workers/instances against the same command and verify:

- one logical command remains;
- no duplicate accepted transition;
- no invalid backward transition;
- history sequence is consistent;
- retry accounting is bounded.

### 19.5 Firmware/controller tests

Verify controller handling of duplicate `command_id` values:

- duplicate delivery is recognized as the same command;
- lifecycle is not emitted as a new command identity;
- safe idempotency behavior is preserved for each command class;
- a stale/invalid command does not corrupt current FSM state.

---

## 20. Migration strategy

Migration must preserve currently observable P0.2 lifecycle behavior while moving authority from memory to PostgreSQL.

Recommended order:

```text
1. Add commands + command_lifecycle_events tables
2. Add repository/service layer with transition transaction
3. Write REQUESTED before MQTT publish
4. Replace in-memory lifecycle mutation with durable transition service
5. Keep system_events as audit compatibility output
6. Load/reconcile active commands on startup
7. Add bounded retry/timeout/UNKNOWN worker
8. Switch command history API to durable store
9. Make WS events post-commit notifications
10. Add concurrency/restart integration tests
11. Remove in-memory map as correctness dependency
```

Existing `system_events` lifecycle rows may remain for historical compatibility. P1.2 must not silently treat historical audit rows as authoritative command aggregates unless an explicit backfill/migration guarantees complete command state.

Backfill, if needed, must classify incomplete historical commands conservatively rather than inventing `CONFIRMED` outcomes.

---

## 21. Acceptance criteria

### AC-1 - Durable command identity

Every command has a persistent unique `command_id` before MQTT publication.

### AC-2 - Durable lifecycle authority

Current lifecycle state and accepted transition history survive backend restart and are stored in PostgreSQL.

### AC-3 - Correct transition semantics

All lifecycle mutations use one validated transition boundary; invalid/backward/terminal transitions cannot mutate state.

### AC-4 - Idempotent transport

MQTT retransmission preserves command identity and cannot create duplicate logical commands.

### AC-5 - Acknowledgement correlation

`ACKNOWLEDGED` is accepted only for a known, matching device/command and does not imply physical confirmation.

### AC-6 - Physical confirmation

`CONFIRMED` is emitted only from the documented runtime/controller observation predicate.

### AC-7 - Timeout/UNKNOWN

Active commands cannot remain indefinitely unresolved. Deadline and ambiguity policy produces `TIMEOUT` or `UNKNOWN` according to explicit rules.

### AC-8 - Restart recovery

Backend and MQTT restart/reconnect recover active commands from PostgreSQL without losing identity, duplicating history, or falsely reporting success.

### AC-9 - Multi-instance safety

Concurrent backend workers cannot corrupt lifecycle state or produce duplicate accepted transitions.

### AC-10 - Durable history API

Command history/detail reads come from durable command state and remain protected by P1.1 capability + ownership checks.

### AC-11 - WebSocket consistency

WebSocket notifications are emitted after durable commit and clients can recover current state through API reload after missed events.

### AC-12 - Bulk semantics

Bulk command authorization is all-or-nothing before mutation; individual transport outcomes remain individually durable and visible.

### AC-13 - Secret safety

No command/lifecycle persistence or logs expose API keys, bearer tokens, privileged tokens, WiFi passwords, webhook secrets, or signing secrets.

### AC-14 - Failure-path verification

Database-backed tests cover publish failure, duplicate events, rejection, timeout, UNKNOWN, restart recovery, and concurrent transitions.

### AC-15 - Cross-system regression

Shared, backend, controller, simulator, and frontend verification commands declared by repository governance execute with failures classified. No P0/P1.1 security invariant is weakened.

### AC-16 - Traceability

`docs/project-state/TRACEABILITY.md` and `docs/project-state/CURRENT-STATUS.md` record P1.2 implementation status, evidence, residual risks, and the boundary to the next phase.

---

## 22. Non-goals

- redesigning the canonical FSM;
- replacing MQTT;
- replacing PostgreSQL;
- redesigning telemetry/Flux;
- redesigning frontend state architecture;
- scheduler redesign;
- new RBAC model;
- backup/restore lifecycle redesign;
- changing P1.1 ownership semantics;
- making every actuator operation automatically retryable;
- claiming physical success from MQTT delivery alone.

---

## 23. Implementation order

```text
P1.2 Durable Command Lifecycle
    |
    +-- 1. Freeze P0.2/P1.1 lifecycle + authorization contracts
    |
    +-- 2. Add durable command aggregate + lifecycle history schema
    |
    +-- 3. Implement repository/transactional transition service
    |
    +-- 4. Persist REQUESTED before MQTT publish
    |
    +-- 5. Move ACK/REJECT/FAIL/CONFIRM mutations to durable transition service
    |
    +-- 6. Add bounded retry + timeout/UNKNOWN reconciliation
    |
    +-- 7. Add startup/MQTT reconnect recovery
    |
    +-- 8. Make multi-instance transition/retry behavior atomic
    |
    +-- 9. Switch command history API + WS notification path to durable state
    |
    +-- 10. Add idempotency + bulk semantics where applicable
    |
    +-- 11. Add database/concurrency/restart/controller integration tests
    |
    +-- 12. Run cross-system verification
    |
    +-- 13. Record evidence + traceability
    |
    v
Next reliability phase
```

## 24. Risks and rollback

### Risks

- duplicate MQTT delivery can cause physical duplicate actuation if controller idempotency is incomplete;
- crash windows can leave delivery outcome ambiguous;
- retrying dangerous commands can be unsafe;
- multiple backend instances can race without database-level transition authority;
- historical `system_events` may be incomplete for command reconstruction;
- changing command persistence can expose assumptions in frontend/API tests.

### Rollback

Rollback must preserve command correlation and must not revert to an authorization bypass or silently resume unbounded retries.

If the durable worker is disabled during rollout, existing commands must remain readable from PostgreSQL and unresolved active commands must be marked/reconciled conservatively. Do not delete durable history to restore an older in-memory behavior.

---

## 25. Completion rule

P1.2 is complete only when command lifecycle correctness no longer depends on process memory and the following failure classes are demonstrated:

```text
normal delivery
publish failure
controller rejection
physical confirmation
backend restart
MQTT reconnect
controller restart / ambiguous delivery
concurrent backend workers
duplicate lifecycle events
bounded timeout/retry
```

A green happy-path test alone is insufficient.

The final evidence must demonstrate that after a backend restart, the system can still answer **which command exists, which device it targets, who requested it, what lifecycle state is authoritative, and whether the outcome is confirmed, failed, timed out, or unknown**.
