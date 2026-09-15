# HYDRAGROW — F. UX FLOW / TASK FLOW SPECIFICATION

## F0. Purpose

This document defines the canonical task flows for HYDRAGROW.

It standardizes how users enter a task, establish context, pass guards, perform actions, receive feedback, verify results, and recover from failure.

This is a UX contract, not an implementation specification.

## F1. Flow Principles

1. Goal first.
2. Context before consequential action.
3. Guard before execution.
4. Explicit intent for consequential actions.
5. Physical reality wins over optimistic UI state.
6. Recovery is part of every consequential flow.
7. Equivalent tasks use the same lifecycle and vocabulary.

## F2. Canonical Flow Anatomy

```text
Entry
Context
Preconditions
Guard
Intent
Confirmation
Execution
Pending
Result
Verification
Completion
Recovery
```

## F3. Flow State Vocabulary

Use the canonical states from `STATE-TRANSITION-SPEC.md`:

- `IDLE`
- `LOADING`
- `READY`
- `REQUESTED`
- `SENT`
- `ACKNOWLEDGED`
- `PENDING`
- `CONFIRMED`
- `FAILED`
- `REJECTED`
- `TIMEOUT`
- `CANCELLED`
- `UNAVAILABLE`
- `UNKNOWN`
- `COMPLETED`

Do not invent a flow-specific state when an existing canonical state is sufficient.

## F4. Context Contract

Before a consequential action, establish:

- current station;
- relevant zone or scope;
- target device or object;
- current physical/system state;
- availability state;
- effective permission;
- relevant safety state.

If required context is unknown, stop at the appropriate guard.

## F5. Guard Contract

| Guard | Failure state | User outcome |
|---|---|---|
| Station selected | No context | Select station |
| Station available | Unavailable | Block and explain |
| Permission | Permission denied | Block and explain |
| Current state | Invalid transition | Explain restriction |
| Telemetry freshness | Stale / unknown | Prevent unsafe decision when applicable |
| Safety interlock | Safety blocked | Block and explain |
| Parameters | Validation error | Preserve values and show field errors |
| Device capability | Unsupported | Explain limitation |
| Concurrent command | Conflict | Reconcile before continuing |
| System readiness | Not ready | Explain requirement |

A failed guard must not create a fake pending state.

## F6. Role-Based Flow Matrix

The current product defines three roles: `admin`, `operator`, and `viewer`.

| Flow | Admin | Operator | Viewer |
|---|---:|---:|---:|
| View telemetry | Yes | Yes | Yes |
| Inspect Overview | Yes | Yes | Yes |
| Inspect Operations | Yes | Yes | Yes |
| Control pumps / actuators | Yes | Yes | No |
| Trigger E-STOP | Yes | Yes | No |
| Edit cultivation / recipes | Yes | Yes | No |
| Change station configuration | Yes | Yes | No |
| OTA device update | Yes | Yes | No |
| Configure device network | Yes | Yes | No |
| Write scripts | Yes | Yes | No |
| View journal / history | Yes | Yes | Yes |
| Pair station | Yes | Yes | No |
| Backup | Yes | Yes | No |
| Restore | Yes | Yes | No |
| Manage members | Yes | No | No |
| Change roles | Yes | No | No |
| Activate / suspend member | Yes | No | No |

This matrix is the UX expectation layer. Backend authorization remains authoritative.

## F7. Flow-to-Page Matrix

| Flow | Primary page | Supporting surface |
|---|---|---|
| Station selection | Overview / Fleet | Shell selector |
| Health inspection | Overview | Fleet, Operations |
| Actuator control | Operations | Overview |
| E-STOP | Operations | Global operational shell |
| E-STOP recovery | Operations | Alert / fault surface |
| Fault recovery | Operations | Overview, alert detail |
| Telemetry inspection | Overview / Operations | Cultivation |
| Cultivation configuration | Cultivation | Settings |
| Journal entry | Journal | Contextual action surface |
| Alert investigation | Overview / Operations | Journal |
| Pairing | Pairing | Fleet |
| Backup | Backup | Settings |
| Restore | Backup | Settings |
| Role management | Roles | Settings |
| Zero Station | Zero Station | Pairing / Settings |

## F8. Flow x State Matrix

| Flow | Initial | Active | Success | Failure / degraded |
|---|---|---|---|---|
| Station selection | No context | Loading | Ready | Unavailable |
| Actuator command | Idle | Requested / Sent / Pending | Confirmed | Rejected / Failed / Timeout / Unknown |
| E-STOP | Normal state | Requested / Pending | E-STOP confirmed | Unknown / Failed |
| E-STOP recovery | E-STOP | Pending recovery | Stopped / Ready | Fault / Unavailable |
| Fault recovery | Fault | Recovery pending | Ready or normal state | Fault / Unavailable |
| Telemetry inspection | Loading | Fresh | Usable value | Stale / Invalid / Unavailable |
| Cultivation configuration | Current config | Pending save | Persisted | Validation error / Failed |
| Journal entry | Empty form | Saving | Persisted | Validation error / Failed |
| Pairing | Unpaired | Discovering / Connecting | Paired + synchronized | Failed / Unavailable |
| Backup | Ready | Running | Backup verified | Failed / Timeout |
| Restore | Backup selected | Restoring | Configuration verified | Failed / Conflict |
| Role change | Current role | Saving | Persisted | Permission denied / Failed |

The flow must map to an authoritative state in `STATE-TRANSITION-SPEC.md`.

## F9. Station Selection Flow

1. Open station selector.
2. Inspect station list.
3. Select station.
4. Validate availability.
5. Load station context.
6. Synchronize required state.
7. Display selected station.

Failure cases:

- no stations: empty state with pairing action;
- unavailable station: preserve selection and show unavailable state;
- synchronization failure: show unavailable or unknown state.

## F10. Overview Inspection Flow

The user should be able to determine:

1. whether the station is healthy;
2. whether anything is unsafe;
3. whether anything is abnormal;
4. which equipment is active;
5. what intervention is required.

The flow ends when no intervention is needed or a specific task is entered.

## F11. Operations Inspection Flow

1. Open Operations.
2. Confirm station context.
3. Inspect actuator state.
4. Inspect relevant telemetry.
5. Select target actuator.
6. Review available action.
7. Continue to command flow only when guards permit.

## F12. Normal Actuator Command Flow

```text
IDLE
REQUESTED
SENT
ACKNOWLEDGED
CONFIRMED
```

Failure states:

```text
REJECTED
FAILED
TIMEOUT
UNKNOWN
```

Required behavior:

1. Identify station and actuator.
2. Display current physical state.
3. Select command.
4. Evaluate guards.
5. Confirm when required.
6. Submit command.
7. Display pending state.
8. Receive result.
9. Verify physical/system state.
10. Display final state.
11. Record the operation when required.

Do not replace observed actuator state with requested state merely because the command was submitted.

## F13. E-STOP Flow

E-STOP is an immediate safety action.

State contract:

```text
Normal operational state
Requested
Pending
E-STOP confirmed
```

Rules:

- no confirmation modal;
- visually distinct control;
- affected station must be clear;
- normal controls become appropriately restricted after activation;
- post-activation state remains visible across navigation.

Completion means authoritative E-STOP state is confirmed.

## F14. E-STOP Recovery Flow

Preconditions:

- emergency condition cleared;
- safety state confirmed;
- appropriate permission available;
- station available;
- recovery operation allowed.

Flow:

1. Inspect emergency state.
2. Inspect cause when available.
3. Select recovery.
4. Evaluate guards.
5. Confirm recovery where required.
6. Submit recovery.
7. Display pending state.
8. Verify resulting state.
9. Return to controlled operation only after valid state is confirmed.

## F15. Fault Recovery Flow

1. Detect fault.
2. Identify affected subsystem.
3. Explain condition.
4. Identify supported recovery.
5. Check permission and safety guards.
6. Execute recovery.
7. Verify authoritative state.
8. Record outcome.

Possible outcomes: recovered, blocked, failed, or unknown.

Acknowledging a fault is not the same as resolving it.

## F16. Warning Flow

1. Display warning.
2. Explain condition.
3. Explain operational significance.
4. Identify whether action is required.
5. Provide recommended action when useful.
6. Monitor condition.
7. Mark resolved only when the underlying condition clears.

## F17. Permission-Denied Flow

1. User selects restricted action.
2. Permission guard fails.
3. Do not submit command.
4. Explain restriction.
5. Preserve page and target context.
6. Offer a safe alternative when available.

Authorization failure must remain distinct from technical failure.

## F18. Station Unavailable Flow

1. Detect unavailable state.
2. Preserve station identity.
3. Mark affected data unavailable or stale as appropriate.
4. Disable unsafe actions.
5. Explain availability condition.
6. Offer retry, reconnect, or diagnostics when supported.
7. Restore controls only after authoritative availability returns.

## F19. Communication Loss and Reconnection

During communication loss:

- freeze assumptions about physical state;
- mark telemetry stale or unknown;
- expose connection state;
- prevent unsafe commands;
- preserve command history.

After reconnection:

1. re-establish station identity;
2. request authoritative state;
3. refresh telemetry;
4. reconcile actuator state;
5. reconcile pending commands;
6. resolve alerts;
7. restore controls according to authoritative state.

## F20. Telemetry Inspection Flow

Inspect value, unit, timestamp/freshness, quality/state, and target range when applicable.

| Telemetry state | Operational treatment |
|---|---|
| Fresh | Usable within normal safety rules |
| Stale | Use cautiously and expose freshness |
| Invalid | Do not treat as trustworthy measurement |
| Unavailable | Do not infer a value |
| Disconnected | Treat current physical state as potentially unknown |

## F21. Cultivation Configuration Flow

1. Open Cultivation.
2. Select crop, zone, or configuration scope.
3. Inspect current configuration.
4. Edit parameter.
5. Validate value and range.
6. Review proposed change.
7. Save.
8. Show pending state.
9. Verify persistence.
10. Show resulting configuration.

For consequential changes, communicate current value, proposed value, affected scope, expected effect when known, and timing.

## F22. Journal Entry Flow

1. Open Journal.
2. Select Add Entry.
3. Enter information.
4. Validate required fields.
5. Save.
6. Confirm persistence.
7. Return to journal context.

System-generated events must remain distinguishable from user-created entries.

## F23. Alert Investigation and Resolution

Investigation:

1. Open alert.
2. Identify severity.
3. Identify station/subsystem.
4. Inspect current state.
5. Inspect relevant telemetry.
6. Determine supported action.
7. Execute permitted recovery when appropriate.
8. Verify outcome.

Distinguish acknowledged, dismissed, active, and resolved states.

## F24. Pairing Flow

```text
Unpaired
Discovering
Identifying
Validating
Connecting
Synchronizing
Paired
```

Failure states include not discovered, invalid pairing data, already paired, connection failed, synchronization failed, and unavailable.

Flow:

1. Open Pairing.
2. Start discovery.
3. Identify station.
4. Validate pairing information.
5. Confirm intended station.
6. Establish connection.
7. Synchronize state.
8. Validate connection.
9. Show paired state.

Retry must not create duplicate station records.

## F25. Backup Flow

1. Open Backup.
2. Confirm station and scope.
3. Review backup scope.
4. Start backup.
5. Show progress when meaningful.
6. Verify completion.
7. Display backup metadata.

## F26. Restore Flow

Restore is a consequential configuration operation.

1. Select backup.
2. Inspect metadata.
3. Inspect affected scope.
4. Review overwrite consequences.
5. Confirm restore.
6. Start restore.
7. Show pending/progress state.
8. Verify resulting configuration.
9. Display completion or failure.

Confirmation must identify source backup, target station, affected scope, and overwrite consequence.

## F27. Role and Permission Management Flow

1. Open Roles.
2. Select member.
3. Inspect current role and scopes.
4. Edit role or permission.
5. Validate authorization.
6. Review changes.
7. Confirm.
8. Save.
9. Verify persistence.

The backend remains authoritative for the final permission decision.

## F28. Zero Station Flow

1. Identify station.
2. Inspect initialization state.
3. Configure required settings.
4. Validate configuration.
5. Apply initialization.
6. Verify readiness.
7. Enter normal station operation.

Incomplete initialization must not be presented as normal healthy operation.

## F29. Form Submission and Unsaved Changes

```text
Edit
Validate
Review
Submit
Pending
Persisted / Failed
```

When leaving with meaningful unsaved changes, offer continue editing, save, or discard.

## F30. Cancellation Flow

1. Operation is pending.
2. User selects Cancel.
3. Check whether cancellation remains possible.
4. Submit cancellation when supported.
5. Show cancellation state.
6. Verify final state.

## F31. Retry Flow

Before retrying, check whether the original operation is pending, already completed, capable of duplication, still allowed by guards, and still targeted at an available station.

Never blindly resend a consequential command.

## F32. Timeout and Unknown Outcome

When confirmation times out:

1. mark outcome unresolved;
2. do not assume success or failure;
3. refresh authoritative state;
4. reconcile the command;
5. show the resolved result when evidence is available.

Unknown is preferable to false certainty.

## F33. Concurrent and Superseded Actions

If another user, automation process, or system event changes the target:

1. detect conflict;
2. stop relying on stale local state;
3. refresh authoritative state;
4. explain the conflict;
5. require reassessment when necessary.

For superseded commands, preserve history, identify the current command, suppress stale feedback, and show the authoritative final result.

## F34. Navigation During Pending Operations

Leaving a page must not automatically cancel an operation.

If it continues, preserve command state, surface important global feedback, allow return to the task, and prevent duplicate commands.

If navigation terminates the operation, inform the user before leaving.

## F35. Feedback Contract

| Stage | Required feedback |
|---|---|
| Action available | Clear actionable control |
| Guard failed | Blocking reason |
| Confirmation | Consequence and target |
| Submitted | Pending state |
| Executing | Active/progress state when meaningful |
| Confirmed | Final state |
| Failed | Failure reason and recovery |
| Timeout | Unresolved state and reconciliation |
| Unavailable | Availability state and next step |

## F36. Completion Contract

A task is complete only when the user can determine:

1. what happened;
2. whether it succeeded;
3. what the resulting state is;
4. whether further action is required.

For physical actions, completion requires sufficient physical/system confirmation.

## F37. Audit Contract

Consequential operations should be traceable where applicable.

Records should identify actor, target, requested action, timestamp, observed result, and failure/cancellation/timeout when applicable.

## F38. Accessibility Contract

Task completion must not depend solely on color, hover, animation, or precise pointer interaction.

Context, action, validation, confirmation, pending state, error, and recovery must remain available through accessible labels and logical focus/order.

## F39. Responsive Flow Contract

When width is constrained, preserve:

1. safety state;
2. current station/context;
3. primary action;
4. critical telemetry;
5. secondary information.

Responsive reflow must not create ambiguity about the physical target of a control.

## F40. Flow Anti-Patterns

Do not:

- execute a physical command without clear target context;
- represent command acceptance as physical confirmation;
- silently retry consequential commands;
- hide safety interlocks;
- require confirmation for E-STOP;
- treat stale telemetry as live;
- make unavailable controls look executable;
- make permission errors look like technical faults;
- discard consequential unsaved changes silently;
- report success without sufficient evidence;
- duplicate commands during retry;
- hide concurrent state changes;
- mark an active fault resolved only because it was dismissed.

## F41. Flow Definition Template

```text
Flow Name
Goal
Entry Points
User Role
Target
Preconditions
Initial State
Primary Action
Guards
Confirmation
Execution
Pending State
Success Condition
Verification
Failure Conditions
Recovery Actions
Cancellation
Timeout
Concurrent State Handling
Audit Requirement
Accessibility Requirement
Responsive Requirement
Completion Criteria
```

## F42. Flow Acceptance Criteria

A flow is ready for implementation when goal, entry points, context, preconditions, guards, permission, safety, confirmation, pending behavior, success criteria, verification, failure, recovery, timeout/cancellation where applicable, concurrent-state behavior, audit, accessibility, responsive behavior, and state-machine mapping are defined.

## F43. Three-Matrix Rule

The design system maintains three linked views:

1. **Role x Flow Matrix** — who may perform the task.
2. **Flow x State Matrix** — what states the task can occupy.
3. **Flow x Page Matrix** — where the task starts, continues, and completes.

A consequential flow is underspecified if one of these relationships is missing.

## F44. Canonical Task Flow Rule

Every consequential task must answer:

**Where am I?**

**What am I acting on?**

**What is the current state?**

**Am I authorized to act?**

**Is it safe to act?**

**What will happen if I continue?**

**Has the action actually completed?**

**What should I do if it did not?**

## F45. Canonical Flow Formula

HYDRAGROW task flows should consistently follow:

**Context, Goal, Guard, Action, Confirmation, Execution, Feedback, Verification, Recovery.**
