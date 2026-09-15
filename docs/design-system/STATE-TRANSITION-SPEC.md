# HYDRAGROW — B. STATE TRANSITION SPEC

## B0. Purpose

This document defines how HYDRAGROW operational states change over time.

It provides a canonical model for:

- system state
- station availability
- telemetry state
- actuator state
- command lifecycle
- cultivation state
- alerts and faults
- permission-related state
- emergency stop
- recovery
- timeout and failure behavior

The purpose is to prevent different pages or components from inventing different interpretations of the same operational state.

---

## B1. State Model Principles

### B1.1 State is authoritative

The UI must represent the latest authoritative state reported by the system.

A user action is a **request to transition state**, not proof that the transition occurred.

### B1.2 Requested state and actual state are separate

For physical operations:

**User intent → Command → System response → Physical state**

The interface must not collapse these into a single state.

### B1.3 Every transition has a trigger

A state should change only because of a defined trigger, such as:

- system event
- telemetry update
- command result
- timeout
- user action
- safety event
- configuration change
- connection change

### B1.4 Invalid transitions must be explicit

If a requested transition is not allowed, the system must:

- reject the transition
- preserve the current authoritative state
- provide an appropriate reason
- record the event when operationally significant

### B1.5 Safety transitions have priority

Safety-related transitions override normal operational transitions.

A normal command must never override an active safety condition.

---

# B2. Canonical State Categories

HYDRAGROW state is divided into several categories.

| Category | Purpose |
|---|---|
| System | Overall operational condition |
| Availability | Whether the target can currently be reached/operated |
| Telemetry | Quality and freshness of observed data |
| Actuator | Physical device state |
| Command | Lifecycle of a requested operation |
| Cultivation | Growing-process state |
| Alert | Operational attention state |
| Permission | Whether an action is authorized |
| Safety | Emergency/safety state |

These categories are related but must not be merged into a single status value.

---

# B3. System State Machine

The canonical system operational states are:

- `INITIALIZING`
- `READY`
- `RUNNING`
- `PAUSED`
- `WARNING`
- `FAULT`
- `STOPPED`
- `UNAVAILABLE`
- `ESTOP`

### B3.1 Initializing

System is starting or establishing sufficient state to become operational.

Allowed transitions:

- `INITIALIZING → READY`
- `INITIALIZING → WARNING`
- `INITIALIZING → FAULT`
- `INITIALIZING → UNAVAILABLE`
- `INITIALIZING → ESTOP`

The UI must not present the system as operational before the initialization requirements are satisfied.

### B3.2 Ready

System is available and able to operate but is not actively running.

Allowed transitions:

- `READY → RUNNING`
- `READY → WARNING`
- `READY → FAULT`
- `READY → UNAVAILABLE`
- `READY → ESTOP`
- `READY → STOPPED`

### B3.3 Running

System is actively executing its operational process.

Allowed transitions:

- `RUNNING → PAUSED`
- `RUNNING → WARNING`
- `RUNNING → FAULT`
- `RUNNING → STOPPED`
- `RUNNING → UNAVAILABLE`
- `RUNNING → ESTOP`

### B3.4 Paused

Operational process is intentionally suspended while remaining recoverable.

Allowed transitions:

- `PAUSED → RUNNING`
- `PAUSED → STOPPED`
- `PAUSED → WARNING`
- `PAUSED → FAULT`
- `PAUSED → UNAVAILABLE`
- `PAUSED → ESTOP`

### B3.5 Warning

System remains operational but one or more conditions require attention.

Allowed transitions:

- `WARNING → READY`
- `WARNING → RUNNING`
- `WARNING → PAUSED`
- `WARNING → FAULT`
- `WARNING → STOPPED`
- `WARNING → UNAVAILABLE`
- `WARNING → ESTOP`

Recovery from `WARNING` requires the underlying warning condition to clear.

### B3.6 Fault

A fault prevents normal operation or indicates that operation cannot be trusted.

Allowed transitions:

- `FAULT → READY`
- `FAULT → STOPPED`
- `FAULT → UNAVAILABLE`
- `FAULT → ESTOP`

Recovery must not be represented as successful merely because the user acknowledged the fault.

### B3.7 Stopped

System is intentionally not operating.

Allowed transitions:

- `STOPPED → READY`
- `STOPPED → RUNNING`
- `STOPPED → WARNING`
- `STOPPED → FAULT`
- `STOPPED → UNAVAILABLE`
- `STOPPED → ESTOP`

### B3.8 Unavailable

The system cannot currently provide a sufficiently trustworthy operational state.

Typical triggers:

- communication loss
- station unavailable
- required subsystem unavailable
- insufficient state synchronization

Allowed transitions:

- `UNAVAILABLE → READY`
- `UNAVAILABLE → WARNING`
- `UNAVAILABLE → FAULT`
- `UNAVAILABLE → ESTOP`
- `UNAVAILABLE → STOPPED`

The UI must distinguish unavailable state from stopped state.

### B3.9 E-STOP

Emergency stop is active.

E-STOP has priority over normal operational states.

Normal transitions must not bypass E-STOP.

Recovery requires:

1. emergency condition cleared
2. physical/system safety state confirmed
3. appropriate reset/recovery action
4. system re-establishes a valid operational state

Typical recovery:

`ESTOP → STOPPED → READY`

Do not automatically transition directly from `ESTOP → RUNNING`.

---

# B4. Transition Priority

When multiple events occur simultaneously, use this priority:

1. `ESTOP`
2. critical safety fault
3. loss of trustworthy system state
4. equipment fault
5. warning
6. normal operational transition
7. informational event

A lower-priority event must not overwrite a higher-priority state.

---

# B5. Command State Machine

Every physical command should use the following lifecycle:

`IDLE → REQUESTED → SENT → ACKNOWLEDGED → CONFIRMED`

Failure paths:

- `REQUESTED → REJECTED`
- `SENT → FAILED`
- `SENT → TIMEOUT`
- `ACKNOWLEDGED → FAILED`
- `ACKNOWLEDGED → TIMEOUT`

### B5.1 IDLE

No command is currently pending.

### B5.2 REQUESTED

A user or automated process has requested an operation.

At this point:

- intent exists
- physical state has not changed yet

### B5.3 SENT

The command has been transmitted to the target system.

Transmission does not imply execution.

### B5.4 ACKNOWLEDGED

The target has accepted the command.

Acknowledgement does not necessarily mean the physical action has completed.

### B5.5 CONFIRMED

The resulting physical/system state has been independently confirmed.

Only this state should be treated as successful completion of a physical command.

### B5.6 REJECTED

The command was intentionally refused.

Possible reasons:

- invalid state
- permission denied
- safety interlock
- invalid parameters
- unavailable target

### B5.7 FAILED

The command could not complete successfully.

### B5.8 TIMEOUT

Expected confirmation did not arrive within the defined timeout.

A timeout must not be displayed as success.

---

# B6. Command Transition Rules

### B6.1 Start

Typical lifecycle:

`IDLE → REQUESTED → SENT → ACKNOWLEDGED → CONFIRMED`

The system should reach `RUNNING` only when the relevant operational state is confirmed.

### B6.2 Stop

Typical lifecycle:

`IDLE → REQUESTED → SENT → ACKNOWLEDGED → CONFIRMED`

The resulting system state should be `STOPPED` or another explicitly reported safe state.

### B6.3 Pause

Typical lifecycle:

`IDLE → REQUESTED → SENT → ACKNOWLEDGED → CONFIRMED`

The resulting state should be `PAUSED`.

### B6.4 Resume

Typical lifecycle:

`IDLE → REQUESTED → SENT → ACKNOWLEDGED → CONFIRMED`

The resulting state should be explicitly reported before the UI treats the system as resumed.

---

# B7. Actuator State Machine

Actuators must distinguish requested state from actual state.

Canonical states:

- `OFF`
- `ON`
- `STARTING`
- `STOPPING`
- `FAULT`
- `UNAVAILABLE`
- `UNKNOWN`
- `LOCKED`

Typical transitions:

`OFF → STARTING → ON`

`ON → STOPPING → OFF`

Failure paths:

`STARTING → FAULT`

`STARTING → TIMEOUT`

`STOPPING → FAULT`

`STOPPING → TIMEOUT`

Communication failure:

`any operational state → UNKNOWN / UNAVAILABLE`

The UI must not immediately display `ON` after a user presses Start unless the physical/system state confirms it.

---

# B8. Telemetry State Machine

Telemetry has two dimensions:

1. value
2. quality

Canonical quality states:

- `LIVE`
- `STALE`
- `INVALID`
- `UNAVAILABLE`

### B8.1 LIVE

The value is current and valid.

### B8.2 STALE

The last known value exists but is older than the accepted freshness threshold.

The value may remain visible, but must be clearly identified as stale.

### B8.3 INVALID

A value exists but fails validation or cannot be trusted.

The UI must not treat it as normal telemetry.

### B8.4 UNAVAILABLE

No trustworthy value is currently available.

Do not substitute zero or another fabricated value.

### B8.5 Typical transitions

`LIVE → STALE`

when freshness threshold is exceeded.

`STALE → LIVE`

when a valid fresh reading arrives.

`LIVE → INVALID`

when the incoming value fails validation.

`LIVE → UNAVAILABLE`

when the data source becomes unavailable and no trustworthy value remains.

`INVALID → LIVE`

when a valid fresh value is received.

---

# B9. Telemetry Freshness Rules

Each telemetry stream should have a defined freshness policy.

Conceptually:

`fresh reading → LIVE`

`age > threshold → STALE`

`source unavailable → UNAVAILABLE`

`invalid reading → INVALID`

Freshness thresholds must be defined by the telemetry domain rather than assumed to be identical for every sensor.

---

# B10. Alert State Machine

Alerts should use explicit lifecycle states:

- `ACTIVE`
- `ACKNOWLEDGED`
- `CLEARED`
- `RESOLVED`

### B10.1 Active

A condition requiring attention currently exists.

### B10.2 Acknowledged

A user has acknowledged the alert.

Acknowledgement does **not** mean the underlying condition is fixed.

### B10.3 Cleared

The underlying condition is no longer detected.

### B10.4 Resolved

The alert lifecycle is complete according to system rules.

Typical sequence:

`ACTIVE → ACKNOWLEDGED → CLEARED → RESOLVED`

An alert may also clear without explicit acknowledgement:

`ACTIVE → CLEARED → RESOLVED`

The exact lifecycle should depend on the alert type.

---

# B11. Fault Recovery

Fault recovery must separate acknowledgement from recovery.

Incorrect:

`FAULT → ACKNOWLEDGED → RUNNING`

Correct conceptual flow:

`FAULT → ACKNOWLEDGED → CONDITION CLEARED → RECOVERY → READY`

Only after the system confirms readiness may normal operation resume.

If the fault remains:

`FAULT → ACKNOWLEDGED → FAULT`

The UI must not hide a persistent fault merely because it has been acknowledged.

---

# B12. Warning Recovery

Warnings may recover automatically when the underlying condition clears.

Typical flow:

`WARNING → condition clears → READY`

or:

`WARNING → condition clears → RUNNING`

depending on the authoritative system state.

The UI must not assume that clearing a warning necessarily means the system has returned to the state it had before the warning.

---

# B13. Availability State

Availability should be treated separately from operational state.

Canonical availability states:

- `AVAILABLE`
- `DEGRADED`
- `UNAVAILABLE`

### AVAILABLE

Target can be reached and its state can be trusted.

### DEGRADED

Target is reachable but one or more capabilities or data sources are impaired.

### UNAVAILABLE

Target cannot currently be trusted for normal operation.

Availability changes may force operational state changes.

Example:

`RUNNING + AVAILABLE`

becomes:

`RUNNING + UNAVAILABLE`

The UI may then represent the operational condition as `UNAVAILABLE` when the authoritative system state can no longer be trusted.

---

# B14. Permission State

Permission should be evaluated before command execution.

Canonical outcomes:

- `AUTHORIZED`
- `UNAUTHORIZED`
- `LOCKED`

Typical decision:

`Action requested → Permission check → Authorized / Unauthorized`

If unauthorized:

- no physical command is sent
- current system state remains unchanged
- the user receives permission feedback

Permission denial is not a system fault.

---

# B15. Safety Interlock

Safety interlocks prevent otherwise valid commands from executing.

Typical flow:

`Action requested → Safety check → Allowed / Blocked`

If blocked:

`REQUESTED → REJECTED`

The rejection reason should identify the relevant safety condition when appropriate.

Safety interlocks have higher priority than ordinary user intent.

---

# B16. E-STOP Transition Rules

When E-STOP activates:

`ANY NORMAL STATE → ESTOP`

This transition takes precedence over pending normal commands.

Pending commands must be marked according to their actual outcome.

Do not assume every pending command failed; record the actual system result when available.

After E-STOP:

`ESTOP → STOPPED`

only after the emergency condition is cleared and the system confirms a safe stopped state.

Then:

`STOPPED → READY`

after normal readiness checks succeed.

A later user action may request:

`READY → RUNNING`

The UI must never silently restart the system as part of E-STOP recovery.

---

# B17. Loss of Communication

When communication is lost:

1. Preserve the last known state where useful.
2. Mark its freshness explicitly.
3. Stop presenting it as live.
4. Update availability.
5. Prevent unsafe assumptions about physical state.
6. Apply system-defined fail-safe behavior.

Conceptual transition:

`LIVE / CONNECTED → COMMUNICATION LOST → UNAVAILABLE`

Recovery:

`UNAVAILABLE → CONNECTION RESTORED → STATE SYNCHRONIZATION → VALID STATE`

Do not immediately display the old state as current when communication returns.

---

# B18. Reconnection

Reconnection requires synchronization.

Typical flow:

`UNAVAILABLE → CONNECTING → SYNCHRONIZING → READY / RUNNING / WARNING / FAULT`

The UI should not treat connection establishment alone as proof that the system is operational.

---

# B19. State Synchronization

After reconnecting or loading a station:

1. establish connection
2. obtain authoritative system state
3. obtain relevant actuator states
4. obtain current telemetry
5. reconcile pending commands
6. update UI

During synchronization, the UI should use an explicit transitional state rather than guessing the final state.

---

# B20. Optimistic UI Rules

Optimistic UI must not be used for safety-critical physical state.

Allowed:

- local interaction feedback
- button press acknowledgement
- non-authoritative visual response

Not allowed:

- showing pump `ON` before confirmation
- showing system `RUNNING` before confirmation
- showing E-STOP released before confirmation
- showing a fault resolved merely because a reset request was sent

Physical state must remain authoritative.

---

# B21. Timeout Rules

Every asynchronous operational command should have a defined timeout policy.

On timeout:

`PENDING → TIMEOUT`

The UI should:

- stop showing the command as actively progressing
- preserve the last authoritative physical state
- identify that confirmation was not received
- provide an appropriate recovery action

A timeout is not equivalent to failure unless the system explicitly defines it that way.

---

# B22. Retry Rules

Retries must be intentional.

Automatic retry is appropriate only when:

- the operation is safe to retry
- duplicate execution cannot create an unsafe result
- retry behavior is explicitly defined

Do not automatically retry safety-sensitive or potentially destructive commands without a defined policy.

---

# B23. Cancellation

If a command supports cancellation:

`REQUESTED / SENT → CANCEL REQUESTED`

Cancellation itself requires confirmation from the system.

Do not represent:

`Cancel button pressed`

as:

`Command cancelled`

until the system confirms cancellation.

---

# B24. Concurrent Commands

When multiple commands target the same physical resource, the system must define whether they are:

- serialized
- rejected
- merged
- superseded

The UI must never imply that two mutually exclusive physical commands are executing successfully at the same time.

Example:

`START requested`

followed immediately by:

`STOP requested`

must resolve through a deterministic command policy.

---

# B25. Superseded Commands

If a newer command replaces an older pending command, the older command must receive an explicit lifecycle result such as:

`SUPERSEDED`

It must not silently disappear.

The journal/audit trail should preserve the relevant command history.

---

# B26. Cultivation State

Cultivation state should be modeled separately from equipment state.

Canonical cultivation states may include:

- `PLANNED`
- `ACTIVE`
- `PAUSED`
- `COMPLETED`
- `CANCELLED`

Typical transitions:

`PLANNED → ACTIVE`

`ACTIVE → PAUSED`

`PAUSED → ACTIVE`

`ACTIVE → COMPLETED`

`ACTIVE → CANCELLED`

`PAUSED → CANCELLED`

Equipment state must not automatically be interpreted as cultivation state.

For example:

`Pump OFF`

does not necessarily mean:

`Cultivation PAUSED`

unless the cultivation model explicitly defines that relationship.

---

# B27. Page-Level State Representation

Pages should derive their visible state from canonical state models.

A page must not create an independent state machine that contradicts system state.

For example:

- Overview reads system state.
- Operations reads command and actuator state.
- Cultivation reads cultivation plus relevant telemetry.
- Fleet reads station availability and operational state.
- Journal reads recorded lifecycle events.

Page-specific presentation may differ; state semantics must not.

---

# B28. Component State Contract

Every stateful component should define:

1. default state
2. loading state
3. unavailable state
4. disabled state
5. active state
6. warning state
7. error/fault state
8. pending state
9. success/confirmed state

For interactive components, additionally define:

- hover
- focus
- pressed
- disabled
- permission denied

The component must not invent semantic states that conflict with canonical application states.

---

# B29. Transition Event Contract

Each meaningful transition should be representable as an event containing, where applicable:

- timestamp
- source
- target
- previous state
- new state
- trigger
- actor
- command ID
- reason
- result

This information supports operational traceability.

---

# B30. State Persistence

State that represents physical or operational reality must come from an authoritative source.

UI-only state may be local.

Examples of UI-local state:

- expanded panel
- selected table row
- filter
- temporary form input

Examples of authoritative state:

- pump state
- system running state
- E-STOP
- station availability
- telemetry value
- active fault

Do not persist UI assumptions as physical truth.

---

# B31. State Restoration

After application reload:

1. restore UI preferences
2. reconnect to authoritative data
3. synchronize system state
4. reconcile commands
5. render current state

Do not restore stale physical state from local UI storage and present it as current.

---

# B32. Unknown State

When the system cannot determine the true state, use an explicit unknown/unavailable representation.

Do not infer state from:

- previous UI state
- last button pressed
- expected command outcome
- assumed device behavior

Unknown is preferable to false certainty.

---

# B33. State Conflict Resolution

If multiple sources report conflicting states:

1. use the defined authoritative source
2. identify the conflict
3. avoid silently selecting a convenient value
4. expose degraded/unknown state when authority cannot be established
5. record the conflict when operationally significant

The UI must prefer uncertainty over presenting an incorrect physical state.

---

# B34. Transition Guard

Before executing a transition, evaluate applicable guards:

- permission
- availability
- current state
- safety interlock
- parameter validity
- device capability
- conflicting command
- system readiness

Conceptually:

`Requested transition → Guards → Allowed / Rejected`

A rejected transition must leave authoritative state unchanged.

---

# B35. State Transition Table

| Current State | Trigger | Result |
|---|---|---|
| Initializing | initialization succeeds | Ready |
| Initializing | initialization warning | Warning |
| Initializing | initialization failure | Fault |
| Ready | Start confirmed | Running |
| Running | Pause confirmed | Paused |
| Paused | Resume confirmed | Running |
| Running | Stop confirmed | Stopped |
| Ready | Stop confirmed | Stopped |
| Any normal state | Critical safety event | E-STOP |
| Any operational state | Communication lost | Unavailable |
| Unavailable | State synchronized | Authoritative state |
| Fault | Condition cleared + recovery | Ready |
| Warning | Condition clears | Authoritative normal state |
| E-STOP | Safe recovery confirmed | Stopped |
| Stopped | Readiness confirmed | Ready |

This table is the canonical high-level transition reference.

---

# B36. UI Transition Rendering

When a transition is in progress, the UI should show:

- current authoritative state
- requested action
- pending status
- expected outcome when useful

Example:

**Current:** Stopped  
**Action:** Start requested  
**Command:** Pending  
**Physical state:** Not yet confirmed

Do not replace “Stopped” with “Running” merely because the Start command was issued.

---

# B37. Transition Feedback

Feedback should correspond to the lifecycle stage.

| Event | UI Feedback |
|---|---|
| Request accepted | Pending |
| Command sent | Sending / Pending |
| Command acknowledged | Awaiting confirmation |
| Physical state confirmed | Success / New state |
| Rejected | Rejected + reason |
| Failed | Failed + recovery |
| Timeout | Timeout + current known state |
| Safety blocked | Blocked + safety reason |

---

# B38. Journal Requirements

Operational transitions should be traceable.

At minimum, record important:

- starts
- stops
- pauses
- resumes
- faults
- E-STOP events
- configuration changes
- permission-sensitive actions
- command failures
- command timeouts
- recovery actions

The journal should describe the transition rather than only recording a button click.

---

# B39. Design Rule for State Changes

Whenever a component changes visually because of a state transition, the implementation must be able to answer:

**What event caused this state change?**

If there is no clear answer, the state is likely being inferred incorrectly.

---

# B40. Canonical State Formula

For HYDRAGROW:

**Observed State ≠ Requested State ≠ Command State**

These three concepts must remain distinguishable.

The canonical operational flow is:

**Intent → Guard → Command → Acknowledgement → Physical/System Confirmation → New State → Trace**

This model should be used consistently across Overview, Operations, Cultivation, Fleet, Journal, and device-level controls.
