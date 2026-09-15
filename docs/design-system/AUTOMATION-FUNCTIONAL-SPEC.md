# HYDRAGROW — AUTOMATION FUNCTIONAL SPECIFICATION

## 0. Purpose

This document is the functional source of truth for the Automation surface, its rule-building model, execution semantics, safety boundary, and responsive interaction model.

Automation is a station-scoped orchestration surface. It defines and manages automatic logic; it is **not** the manual actuator surface and it is **not** the same thing as `Auto Mode`.

This is a UX and product contract, not an implementation specification.

Related specifications:

- `UX-RULES.md`
- `STATE-TRANSITION-SPEC.md`
- `COMPONENT-CONTRACT.md`
- `PAGE-CONTRACT.md`
- `UX-FLOW-TASK-FLOW-SPEC.md`
- `DASHBOARD-FUNCTIONAL-SPEC.md`
- `OPERATIONS-FUNCTIONAL-SPEC.md`
- `docs/discovery/2026-09-12-user-discovery.md`

---

## 1. Product Role

Automation answers:

**"Khi một sự kiện hoặc điều kiện xảy ra, hệ thống cần thực hiện workflow nào?"**

It owns:

- automatic rule/flow definition;
- trigger and condition definition;
- action definition;
- flow enable/disable;
- validation and dry run;
- execution inspection;
- chaining and advanced flow inspection;
- supported configuration overrides;
- templates and supported multi-device application;
- automation-specific audit and conflict information.

It does not own:

- manual actuator operation;
- persistent station/device configuration as a whole;
- full historical trace;
- continuous EC/pH regulation that belongs to the automatic controller;
- hardware safety interlocks.

---

## 2. Automation vs Operations vs Auto Mode

These concepts must remain distinct.

| Concept | Responsibility |
|---|---|
| Operations | Current runtime inspection and guarded physical control |
| Automation | Definition and management of event-driven workflows |
| Auto Mode | Runtime control-ownership state used by the controller/Operations |
| Settings | Persistent configuration |
| Journal | Canonical historical trace |

### 2.1 Auto Mode ownership

Auto Mode is the appropriate owner for closed-loop regulatory control such as maintaining EC/pH targets through the controller's control logic.

Automation must not become a second independent EC/pH controller.

Examples that belong to Auto Mode rather than a normal Automation rule:

- maintain EC at a target;
- continuously correct pH toward a target;
- repeatedly calculate dosing based on feedback to keep a control variable within target tolerance.

### 2.2 Automation dosing boundary

Automation may contain explicit, event-driven dosing when the intent is a workflow rather than continuous regulatory control.

Valid examples include:

- a defined dose after a supported stage/event;
- a predefined dose after a water-change workflow;
- an explicit experimental or protocol dose;
- a scheduled dose for an additive that is not owned by the regulatory controller.

If a user attempts to create an Automation whose apparent intent is to regulate EC/pH, the UI should explain that Auto Mode is the appropriate control surface.

Automation must not silently override the controller's ownership of regulatory control.

---

## 3. Station Context

Automation always operates against the currently selected station/device context.

The current frontend canonical context is `useDeviceStore.deviceId`.

Automation and Operations must consume the same selected device context.

The Automation surface must never silently select a different station.

When switching stations:

1. update selected station/device context;
2. refetch automation data for the new context;
3. discard stale list/detail state belonging to the previous station;
4. preserve unresolved mutations independently when needed;
5. render only automation data belonging to the selected station.

---

## 4. Core Mental Model

The canonical Automation model is:

```text
WHEN
    trigger

IF
    conditions

THEN
    actions

WITH
    execution and safety policy
```

Advanced flows may additionally contain:

```text
IF / ELSE
WAIT
WAIT UNTIL
REPEAT
RUN AUTOMATION
CHAIN
PARALLEL
JOIN
```

These are execution primitives. They are not required to be exposed as drag-and-drop nodes in the everyday UI.

---

## 5. Information Architecture

```text
Automation
├── Automation List
│   ├── Search
│   ├── Filter
│   └── Flow Card
│
├── Create / Edit
│   ├── Trigger
│   ├── Conditions
│   ├── Actions
│   └── Review & Safety
│
├── Automation Detail
│   ├── Overview
│   ├── Runtime
│   ├── Execution
│   ├── Conflicts
│   └── Configuration
│
├── Config Explorer
├── Templates
└── Advanced Flow
    └── Graph / Canvas
```

The landing page is list-first. Canvas is not the default authoring surface.

---

## 6. Source Boundary Labels

Every capability in this specification is classified as one of:

### SOURCE-BACKED

The current repository contains a verified frontend/backend implementation or contract for the capability.

### UX DECISION

The capability is a presentation or interaction decision that can use existing source-backed semantics.

### REQUIRES IMPLEMENTATION

The desired capability requires additional frontend/backend/runtime work before it may be treated as implemented.

The presence of a node in the canonical taxonomy does not mean that the current runtime already supports it.

---

## 7. Automation List

The first Automation screen is a list of flows for the selected station.

Primary actions:

- `+ Tạo Automation`;
- search;
- status filter;
- type/trigger filter;
- optional advanced view controls.

The list should answer quickly:

1. What does this Automation do?
2. When does it run?
3. What does it control?
4. Is it active?
5. What happened the last time it ran?

Technical implementation names such as `ACTION_COMMAND` may remain available in detail metadata but should not be the primary user-facing language.

---

## 8. Automation Card

Minimum information:

```text
┌──────────────────────────────────────────────┐
│ EC thấp → Bổ sung dinh dưỡng      ● Active  │
│ Khi EC < 1.1                                 │
│ Bơm A · 40% · 30s                             │
│ Lần cuối: 14:21 · Success                     │
│                                              │
│ [Chạy thử]                         [•••]     │
└──────────────────────────────────────────────┘
```

The card body opens detail.

The enable/disable control is independent from the card body.

The card must not claim physical actuator success from automation execution submission alone.

---

## 9. Automation Status

Automation status and actuator physical state are different concepts.

Automation may expose execution states such as:

- Draft;
- Active;
- Paused/Disabled;
- Waiting for trigger;
- Evaluating;
- Running;
- Completed;
- Safety blocked;
- Failed;
- Timeout;
- Unknown outcome.

Only states supported by the current execution contract should be rendered as authoritative. Additional states are `REQUIRES IMPLEMENTATION` until the runtime defines them.

---

## 10. Create / Edit Wizard

Everyday authoring uses four conceptual steps:

```text
1. Khi nào?
2. Điều kiện?
3. Làm gì?
4. Xác nhận
```

The builder should allow skipping empty sections where the rule does not need them.

The wizard is the canonical mobile authoring experience.

Desktop may show the same steps in a wider layout but must not require canvas interaction.

---

## 11. Trigger Taxonomy

### T1. Sensor Update — SOURCE-BACKED

Trigger evaluation may be driven by sensor data.

Supported telemetry fields include source-backed values such as:

- EC;
- pH;
- temperature;
- water level.

Sensor event and sensor comparison should remain conceptually separate:

```text
WHEN sensor data updates
IF EC < 1.1
```

The UI should normally express this as one readable rule rather than forcing users to understand internal trigger plumbing.

### T2. Schedule — SOURCE-BACKED

Schedule/Cron execution is supported.

Everyday UI may offer:

- daily;
- selected days;
- interval;
- specific date/time where supported.

Cron expressions remain an advanced representation.

Timezone must be explicit where the backend requires it.

`catchUpIfMissed` and `skipIfOffline` must not be presented as implemented configuration options until their persistence/runtime contract exists.

### T3. FSM / System Event — PARTIALLY SOURCE-BACKED

FSM-driven evaluation exists in the backend.

The product may expose system-event semantics such as:

- phase/state transition;
- fault-related event;
- recovery-related event;

but each concrete event must be verified against the authoritative FSM contract before being exposed as a selectable trigger.

### T4. Recipe / Stage Event — PARTIALLY SOURCE-BACKED

The runtime has stage/phase context and stage transition actions.

Useful event semantics include:

- entering a stage;
- leaving a stage;
- phase transition.

Concrete trigger availability requires implementation against the FSM/recipe event contract.

### T5. Webhook — SOURCE-BACKED

Incoming webhook triggering is supported.

Webhook authoring is an advanced trigger because it requires technical concepts such as payload mapping and dynamic values.

---

## 12. Condition Taxonomy

### C1. Compare — SOURCE-BACKED

Examples:

```text
EC < 1.10
pH >= 6.5
Water Level < 30%
```

### C2. Statistic / Time Window — SOURCE-BACKED

Supported concepts include:

- mean;
- minimum;
- maximum;
- evaluation window.

User-facing wording should prefer:

```text
Trong 15 phút qua, pH trung bình > 6.5
```

over exposing internal `mean`, `windowSec`, or IR terminology.

### C3. AND / OR — SOURCE-BACKED

Logical grouping is supported.

In the everyday builder, AND/OR should appear as readable condition groups rather than requiring explicit graph nodes.

### C4. Sensor Freshness / Quality — REQUIRES IMPLEMENTATION

Proposed semantics:

```text
pH reading must be fresh within 30 seconds
```

This is important for safe automation but must not be presented as implemented until the automation runtime exposes an authoritative freshness/quality contract.

### C5. Persistence / Debounce — REQUIRES IMPLEMENTATION

Example:

```text
pH > 7.5
for at least 2 minutes
```

This prevents transient sensor noise from repeatedly firing a rule.

### C6. Rate of Change — REQUIRES IMPLEMENTATION

Example:

```text
pH increased by > 0.5 within 10 minutes
```

This should be implemented using calibrated telemetry semantics rather than a hard-coded biological assumption.

### C7. System/Stage State — REQUIRES IMPLEMENTATION

Example:

```text
Cultivation stage = Stage 3
System phase = Monitoring
```

The underlying state exists in the system, but a general-purpose condition-node contract must be implemented before exposing it as a generic builder primitive.

### C8. Actuator State — REQUIRES IMPLEMENTATION

Example:

```text
PUMP_A is OFF
```

This is useful for idempotent workflows and should rely on authoritative observed state.

### C9. Hysteresis / Re-arm — REQUIRES IMPLEMENTATION

Example:

```text
Start when EC < 1.1
Re-arm when EC > 1.2
```

The goal is to avoid repeated firing around a threshold.

---

## 13. Action Taxonomy

### A1. Alert — SOURCE-BACKED

Supported automation notification behavior must use actual notification channels exposed by the backend.

Do not promise native mobile push where the current platform path is not implemented.

### A2. Event-Driven Dose — SOURCE-BACKED WITH OWNERSHIP GUARD

Dose actions exist in the backend.

They are valid for explicit workflow dosing, not for replacing closed-loop EC/pH regulation.

The action must pass the same authoritative safety constraints as other dosing commands.

The UI must not imply that a requested dose equals measured delivered volume.

### A3. Water On / Water Off — SOURCE-BACKED

Water actions are supported.

They remain subject to station availability, interlock, safety, and control-authority rules.

### A4. Actuator Control — REQUIRES IMPLEMENTATION

The general-purpose action should allow supported actuator commands where the actuator capability contract permits them.

The UI must not expose PWM or timed control for unsupported actuators.

### A5. Set PWM — REQUIRES IMPLEMENTATION AS A GENERAL AUTOMATION PRIMITIVE

PWM is source-backed for:

- `PUMP_A`;
- `PUMP_B`;
- `PH_UP`;
- `PH_DOWN`;
- `OSAKA`.

Current PWM contract:

- 20% minimum;
- 100% maximum;
- 5% step.

PWM remains a command parameter, not measured flow.

### A6. Emergency Stop — SOURCE-BACKED, CRITICAL

Automation may contain E-STOP only where the backend authorization and safety policy permit it.

E-STOP is a safety action, not an ordinary workflow action.

It must always pass through the highest-priority safety/control-authority path.

### A7. Advance Stage — SOURCE-BACKED

Stage advancement is available as an automation action.

It is a consequential cultivation-state mutation and must be permission- and state-guarded.

### A8. End Season — SOURCE-BACKED

Ending a season is a consequential lifecycle mutation.

The UI must communicate the target season and resulting state.

### A9. Reset/Recovery — REQUIRES IMPLEMENTATION

Fault recovery should be exposed only for fault classes where automatic recovery is explicitly safe and supported.

It must never imply that a reset request equals successful recovery.

---

## 14. Flow-Control Taxonomy

### F1. IF / ELSE — REQUIRES IMPLEMENTATION

Canonical branching primitive:

```text
IF water level < 30%
  YES: Water In
  NO: Continue
```

This is the highest-priority missing flow primitive.

### F2. Wait / Delay — REQUIRES IMPLEMENTATION

The frontend contains a delay concept, but it is not currently a complete execution primitive.

It must not be treated as implemented until the IR/compiler/runtime can preserve and execute its semantics.

### F3. Wait Until — REQUIRES IMPLEMENTATION

Example:

```text
Wait until water level > 70%
Timeout 10 minutes
```

### F4. Repeat — REQUIRES IMPLEMENTATION

Repeat must always have a bounded safety policy.

Unbounded repeat is prohibited.

### F5. Stop Flow — REQUIRES IMPLEMENTATION

Allows a branch to end execution without requiring an actuator action.

### F6. Run Automation / Chain — SOURCE-BACKED

Flow chaining exists through `next_flow_ids` and backend chain evaluation.

Current chain protections include depth/iteration handling and cycle detection.

Everyday UI should present this as:

`Chạy Automation khác`

rather than exposing `next_flow_ids`.

### F7. Parallel — REQUIRES IMPLEMENTATION

Parallel branches are useful for advanced workflows but require explicit backend execution semantics.

### F8. Join — REQUIRES IMPLEMENTATION

Required only if parallel execution is introduced.

---

## 15. Execution Policy

### X1. Cooldown — SOURCE-BACKED IN RELATED ACTION SEMANTICS / REQUIRES GENERAL FLOW POLICY

Alert actions have cooldown semantics. A general automation execution cooldown requires an explicit runtime contract.

### X2. Maximum Runs — REQUIRES IMPLEMENTATION

Example:

```text
Maximum 3 executions / hour
```

This is distinct from chemical dose limits.

### X3. Retry — REQUIRES IMPLEMENTATION

Retry must distinguish:

- rejected;
- safety blocked;
- failed;
- timeout;
- unknown physical outcome.

An unknown physical outcome must not be blindly retried when doing so could duplicate a physical action.

### X4. Timeout — REQUIRES IMPLEMENTATION

Timeout must be attached to operations that have a meaningful observable completion state.

### X5. Re-arm — REQUIRES IMPLEMENTATION

Re-arm controls when a condition may trigger again after an execution.

---

## 16. Configuration Nodes

### G1. Config Read — SOURCE-BACKED

Automation can read supported configuration values for evaluation/context.

### G2. Config Override — SOURCE-BACKED

The backend supports configuration overrides and their management.

Overrides must expose:

- target configuration;
- current value;
- override value;
- applicable scope;
- active state;
- supported revert behavior.

### G3. Restore Config — SOURCE-BACKED AT RUNTIME, UX NODE BOUNDARY REQUIRES VERIFICATION

The backend supports restoration/reconciliation behavior for overrides.

Do not expose a free-form Restore node until its exact authoring semantics are defined.

Persistent configuration remains owned by Settings.

---

## 17. Safety and Control Authority

Automation must not be a bypass around Operations safety controls.

The canonical execution path should be conceptually:

```text
Automation
    |
    v
Control Authority / Arbitration
    |
    v
Safety Gate
    |
    v
Actuator / System Action
```

The exact implementation is a backend architecture concern, but the UX contract requires the ownership boundary.

### 17.1 Regulatory control ownership

When Auto Mode owns regulatory control of EC/pH, an Automation rule must not silently compete for the same control loop.

If a workflow dose is explicitly permitted, it must be treated as a guarded exceptional/workflow action rather than a competing controller.

### 17.2 Interlocks

System interlocks remain authoritative.

Current source-backed actuator interlocks include:

- `PH_UP` blocked by `PH_DOWN`;
- `PH_DOWN` blocked by `PH_UP`;
- `WATER_PUMP_IN` blocked by `WATER_PUMP_OUT`;
- `WATER_PUMP_OUT` blocked by `WATER_PUMP_IN`.

Automation cannot redefine or bypass these through normal rule authoring.

### 17.3 Safety limits

Dose limits, cooldown, calibration requirements, and other source-backed safety policies remain authoritative.

Automation authoring must not provide a generic "disable safety" option.

---

## 18. Automation Safety Review

Before an Automation that can mutate system state is enabled, the user should be shown the effective behavior.

Example:

```text
SAFETY REVIEW

WHEN
EC < 1.10 mS/cm

AND
Water Level > 40%

THEN
PUMP_A
40% · 30 sec

Checks
✓ Rule valid
✓ Station selected
✓ Required capability available
✓ Safety policy available

⚠ Automation có thể tự động điều khiển thiết bị.

[Lưu nháp]        [Bật Automation]
```

If the rule conflicts with Auto Mode ownership, show a specific ownership warning rather than a generic error.

---

## 19. Validation

Validation is a first-class authoring action.

The UI should distinguish:

- syntax/structure valid;
- trigger valid;
- condition valid;
- action valid;
- safety validation;
- configuration validation;
- conflict validation.

Current script validation is source-backed.

The UI must not claim a rule is safe merely because its syntax is valid.

---

## 20. Dry Run / Test

Dry run is source-backed and must be prominent.

The result should explain:

```text
Current values
    EC       1.05
    pH       5.9
    Water    74%

Trigger
    ✓ matched

Conditions
    ✓ passed

Actions preview
    PUMP_A 40% for 30s

No physical command was sent.
```

The last statement is mandatory for a true dry run.

---

## 21. Runtime and Execution

Automation Detail should show the execution lifecycle at a useful level.

Preferred conceptual sequence:

```text
Trigger matched
    |
Condition evaluated
    |
Action evaluated
    |
Safety / authority checked
    |
Command dispatched
    |
Execution result
    |
Observed state when available
```

The UI must distinguish command acceptance from physical confirmation.

An automation execution that successfully submits a command must not automatically be described as proof that the actuator physically changed state.

---

## 22. Execution Result Vocabulary

The automation surface should reuse canonical command/runtime semantics where applicable:

- pending/sending;
- accepted;
- safety blocked;
- rate limited;
- rejected;
- failed;
- timeout;
- unknown outcome;
- completed.

The exact vocabulary should remain aligned with `STATE-TRANSITION-SPEC.md` and the backend result contract.

---

## 23. Automation Detail

Automation Detail is an entity detail surface, not another editor canvas.

Recommended sections:

```text
Overview
Rule
Runtime
Execution
Conflicts
Configuration
Audit
```

Primary actions:

- `Chạy thử`;
- `Sửa`;
- `Bật/Tạm dừng`;
- contextual menu.

---

## 24. Execution History

The automation detail surface may show a compact execution history from the source.

Example:

```text
14:21  Trigger matched     Success
14:02  Trigger matched     Success
13:41  Safety blocked      Blocked
```

Full historical trace remains in Journal where applicable.

The automation execution view must not duplicate the entire Journal.

---

## 25. Conflict Model

Current source-backed schedule conflict detection is limited and must be described honestly.

Current verified behavior includes conflict inspection for supported schedule/actuator overlap cases.

It must not be presented as a universal conflict engine.

Future conflict categories include:

- overlapping schedules;
- same-actuator contention;
- opposing actions;
- configuration-key contention;
- priority contention;
- safety conflicts;
- chain dependency conflicts.

These broader categories are `REQUIRES IMPLEMENTATION` unless a source contract is added.

---

## 26. Templates

Templates are source-backed.

Template flow:

```text
Create Automation
    |
Start from template
    |
Review parameters
    |
Validate
    |
Save as Draft / Enable
```

Templates must never silently enable a newly created automation without an explicit review state.

---

## 27. Multi-Device Application

The backend contains multi-device template/application support.

When exposed, the UI must clearly identify:

- source station/template;
- target stations;
- effective configuration/action;
- permission scope;
- validation result per target where available.

Bulk application must not silently change the selected station context.

---

## 28. Config Explorer

Config Explorer is an advanced Automation support surface.

It may show:

- active overrides;
- original value when available;
- effective value;
- source Automation;
- supported revert operation;
- relevant audit information.

It does not replace Settings.

---

## 29. Advanced Flow

Advanced Flow is the expert surface for execution topology.

It is appropriate for:

- flow chaining;
- branching;
- parallel execution when supported;
- complex dependency inspection;
- context propagation;
- execution debugging.

The existing ReactFlow/graph implementation may serve this surface after its execution semantics are aligned with the canonical node model.

Canvas must not be the required entry point for ordinary automation authoring.

---

## 30. Canvas Contract

Canvas is an advanced visualization/editor, not the default Automation UI.

### Desktop

Desktop may provide:

```text
[Simple Builder]   [Advanced Flow]
```

### Mobile

Mobile must not require graph editing.

For complex flows:

```text
Automation này sử dụng Flow nâng cao.

[Xem Flow]
```

If editing cannot be safely represented on mobile, the UI must preserve inspection and provide a clear desktop handoff rather than rendering an unusable miniature canvas.

---

## 31. Mobile Interaction Contract

The mobile authoring sequence is vertical:

```text
Automation List
    |
Automation Detail
    |
Edit
    |
Trigger
    |
Conditions
    |
Actions
    |
Safety Review
    |
Save / Enable
```

No drag-and-drop is required for core authoring.

No horizontal graph navigation is required for core authoring.

No multi-panel inspector is required for core authoring.

---

## 32. Desktop Interaction Contract

Desktop may use a two-pane layout:

```text
┌──────────────────────────────────┬────────────────────────────┐
│ Automation list                  │ Selected Automation        │
│                                  │                            │
│ EC thấp → Bơm A                  │ Overview                   │
│ pH cao → pH Down                 │ Rule                       │
│ Water protection                 │ Runtime                    │
│                                  │ Execution                  │
│                                  │ Conflicts                  │
│                                  │                            │
│                                  │ [Chạy thử] [Sửa]           │
└──────────────────────────────────┴────────────────────────────┘
```

The same rule semantics must remain understandable without the advanced graph.

---

## 33. Empty State

When no Automation exists:

```text
Chưa có Automation

Tạo quy tắc đầu tiên để hệ thống tự động xử lý một sự kiện hoặc workflow.

[+ Tạo Automation]
```

Templates may be offered as a secondary path.

---

## 34. Loading State

During station or automation loading:

- do not show fabricated flows;
- do not show stale automation under a new station;
- disable consequential mutations until required context exists;
- distinguish station loading from automation loading when observable.

---

## 35. Error State

Network or backend failure must preserve the last authoritative state where safe and clearly identify that fresh data may be unavailable.

Example:

```text
Không thể tải Automation.
Kiểm tra kết nối và thử lại.
```

Mutation failure must not optimistically leave an Automation in an unconfirmed enabled/disabled state.

---

## 36. Offline State

Offline does not automatically mean existing automation is stopped.

The UI must not claim runtime behavior that it cannot observe.

If the system has a source-backed controller-side schedule/runtime guarantee, the UI may state that guarantee explicitly. Otherwise show the limitation:

```text
Trạm đang ngoại tuyến.
Không thể xác nhận trạng thái thực thi hiện tại.
```

---

## 37. Unknown Outcome

When an action command is submitted but physical outcome cannot be established:

```text
Chưa thể xác nhận kết quả

Lệnh đã được gửi nhưng trạng thái thiết bị chưa thể xác nhận.
```

Do not automatically convert this to `Failed` or `Success`.

---

## 38. Permissions

Current source-backed roles:

| Capability | Admin | Operator | Viewer |
|---|---:|---:|---:|
| View Automation | Yes | Yes | Yes |
| Inspect Automation | Yes | Yes | Yes |
| Create Automation | Yes | Yes | No |
| Edit Automation | Yes | Yes | No |
| Enable/disable Automation | Yes | Yes | No |
| Run test | Yes | Yes | No |
| Config override/revert | Yes | Yes | No |

Backend authorization is authoritative.

Any finer-grained permission model is a future capability and must not be implied by this table.

---

## 39. Audit

Automation authoring and consequential mutation should be auditable.

Relevant events include:

- create;
- edit;
- delete;
- enable;
- disable;
- test/dry run;
- config override;
- config revert;
- multi-device application;
- execution result;
- safety block;
- failure.

Where available, audit data should include:

- actor;
- station;
- automation;
- timestamp;
- mutation/action;
- parameters;
- result;
- failure/rejection reason.

---

## 40. Data Integrity Rules

Automation must not invent:

- physical actuator success from HTTP acceptance;
- measured flow from PWM conversion;
- sensor freshness when the backend does not expose it;
- conflict detection broader than the implemented conflict engine;
- execution state not supplied by the runtime;
- controller ownership transfer merely because a Flow was enabled.

Automation must preserve station context on every read and mutation.

---

## 40A. Interaction Contract — Click, Open, Edit, Execute

This section is the authoritative interaction contract for Automation UI. It answers what happens after a user selects a visible Automation component, what surface opens, what information is shown, and which actions are available.

The interaction model follows the same principle as the Dashboard specification: **every consequential component must have an explicit post-click state**. A visual element must not look interactive unless its resulting behavior is defined here.

### 40A.1 Automation List Card

Primary click:

```text
Automation Card
    |
    v
Automation Detail
```

Automation Detail shows:

- Automation name and description;
- current status: Active / Disabled / Blocked / Conflict / Error when supplied;
- trigger summary;
- condition summary;
- action summary;
- last execution summary;
- execution success rate when available;
- station/device context;
- validation state;
- recent execution information.

Card-level controls must remain distinct from opening the detail:

- Enable/Disable: changes runtime activation state only after backend confirmation;
- `...`: Edit, Duplicate, Test/Dry Run, Enable/Disable, Delete according to capability and permission;
- status badge: opens the relevant status explanation when there is a diagnostic reason to inspect.

Delete is destructive and requires confirmation. Enable/Disable must not be represented as successful until the mutation is confirmed by the backend.

### 40A.2 Automation Detail

The detail surface is the canonical inspection surface for one Automation.

```text
Automation Detail
├─ Header / status
├─ Trigger
├─ Conditions
├─ Actions
├─ Flow / chaining
├─ Safety & ownership
├─ Validation
├─ Recent execution
└─ Actions: Sửa / Chạy thử / Bật-Tắt
```

Clicking a section opens its inspector/editor without losing the Automation context.

The detail surface must provide a clear return path to the Automation list and must preserve unsaved draft state when moving between inspectors.

### 40A.3 Create Automation

`+ Tạo Automation` opens the authoring wizard:

```text
1. Khi nào?
2. Điều kiện?
3. Làm gì?
4. Xác nhận
```

Each step is editable before final save. `Xác nhận` runs validation before the create mutation.

If validation fails:

- keep the draft;
- identify the exact invalid field/node;
- explain the constraint;
- do not create or enable the Automation.

If validation succeeds, save creates the Automation in the backend-authoritative state. Creation does not imply that the Automation is enabled unless the backend/source contract explicitly says so.

### 40A.4 Trigger Selection and Trigger Node

Selecting a Trigger type opens its configuration inspector.

The inspector shows only parameters supported by that trigger implementation.

At minimum, the interaction contract covers:

- trigger type;
- source sensor/event/schedule/webhook when applicable;
- trigger parameters;
- enabled/disabled context;
- validation state;
- test affordance only when a real source-backed test exists.

For unsupported or partially implemented trigger primitives, the UI must label them as unavailable or implementation-required rather than presenting a functional-looking editor.

### 40A.5 Condition Selection and Condition Node

Selecting a Condition opens its inspector.

For a comparison condition, the editor exposes only source-backed fields such as:

- sensor/value source;
- comparison operator;
- threshold/value;
- optional time-window/statistic parameters when supported.

AND/OR conditions expose their child conditions and logical operator.

Future primitives such as freshness, debounce, rate-of-change, actuator-state, and hysteresis must not be shown as implemented controls until their runtime semantics exist end-to-end.

### 40A.6 Action Selection and Action Node

Selecting an Action opens an action inspector containing:

- action type;
- target station/device/actuator where applicable;
- action parameters;
- duration/PWM parameters only when supported;
- safety and ownership status;
- validation errors or warnings;
- expected command/result semantics.

The editor must distinguish:

- command accepted by backend;
- safety blocked/rejected;
- execution failed;
- physical outcome confirmed;
- outcome unknown.

HTTP acceptance must never be presented as proof that the physical actuator completed the action.

### 40A.7 Event-Driven Dose Interaction

Selecting `Dose` must explicitly identify it as **Event-driven Dose** when used by Automation.

If the target represents continuous EC/pH regulation, the UI must warn that this belongs to Auto Mode and guide the user toward the Auto Mode target/configuration instead of creating a competing control loop.

Example warning:

```text
Automation Dose là tác vụ theo sự kiện.
Điều khiển EC/pH liên tục thuộc Auto Mode.
Hãy cấu hình mục tiêu EC/pH trong Auto Mode nếu mục đích là duy trì giá trị mục tiêu.
```

The warning must not be bypassed by merely changing the node label. Backend control authority remains authoritative.

### 40A.8 Water and Actuator Actions

Selecting a Water or Actuator action opens the same action-inspector contract.

The UI must identify:

- target actuator;
- requested operation;
- timing/PWM only when supported;
- applicable interlock/ownership constraints;
- resulting command state.

Water-level regulation owned by the controller must not be silently recreated as an Automation workflow. Event-driven drain/fill workflows may remain valid where their semantics are distinct from continuous regulation.

### 40A.9 Flow-Control Node

Selecting a flow-control primitive opens a primitive-specific inspector.

Examples:

- IF/ELSE: branch condition and branch contents;
- Wait/Delay: duration, only if runtime semantics are implemented;
- Wait Until: condition and timeout, only if implemented;
- Repeat: iteration policy and limits, only if implemented;
- Run Automation/Chain: target Flow and cycle/depth constraints;
- Parallel/Join: branch set and join semantics, only if implemented.

The UI must not expose a control as executable merely because it exists in the visual vocabulary. Runtime support is the source of truth.

### 40A.10 Enable / Disable

Enable/Disable is a state mutation, not navigation.

Interaction contract:

```text
Click Enable/Disable
    |
    v
Show pending mutation state
    |
    v
Backend result
    ├─ accepted  -> update status
    └─ rejected/error -> restore authoritative state + reason
```

While pending, duplicate mutation requests must be prevented.

### 40A.11 Test / Dry Run

`Chạy thử` opens a test surface showing the proposed execution path before the test begins.

Dry Run must clearly state whether commands are simulated or actually dispatched. A simulated Dry Run must never be shown as an actuator command having been sent.

Where validation is part of the test flow:

- show validation result first;
- show trigger/condition evaluation;
- show nodes that would execute;
- show actions that would be requested;
- show safety/ownership blocks;
- show that no physical command was issued when it is a true dry run.

### 40A.12 Execution Detail

Clicking an execution entry opens Execution Detail.

The surface should show, when supplied by runtime:

- execution ID/time;
- Automation and station context;
- trigger;
- condition evaluation;
- nodes executed/skipped;
- actions requested;
- accepted/rejected/safety-blocked results;
- timeout/failure information;
- unknown outcome;
- chain/next-flow information.

Do not fabricate missing execution stages.

### 40A.13 Conflict Detail

Clicking a conflict opens Conflict Detail.

It identifies, when available:

- conflicting Automations;
- time or trigger overlap;
- actuator/resource involved;
- ownership or priority reason;
- conflict status;
- available resolution action.

The UI must not imply universal conflict detection when the backend only detects a narrower class of conflicts.

### 40A.14 Config Override Interaction

Selecting a Config Override opens an inspector containing:

- current authoritative value;
- overridden value;
- source Automation;
- applicable scope;
- timestamp when supplied;
- Revert action when permitted.

Revert is a consequential mutation and must show pending, success, and failure states. Full persistent configuration remains owned by Settings.

### 40A.15 Template Interaction

Selecting a template opens a preview before application.

Preview must show:

- template identity;
- trigger/condition/action summary;
- target station/device scope;
- unsupported or incompatible fields;
- whether application creates a new Automation or mutates an existing one.

`Apply` must not silently overwrite existing Automation configuration.

### 40A.16 Search and Filter

Search/filter changes the list view only. It must not mutate Automation state.

Filters may include only fields actually available from the current data contract, such as status or source/type where supported.

Empty filtered results must distinguish:

```text
Không có Automation phù hợp với bộ lọc hiện tại.
```

from the true empty state where no Automation exists.

### 40A.17 Navigation and Back Behavior

Navigation must preserve the selected station/device context.

```text
Automation List
    |
    +-- Automation Detail
    |      +-- Node Inspector
    |      +-- Execution Detail
    |      +-- Conflict Detail
    |
    +-- Create/Edit Wizard
    |
    +-- Test/Dry Run
```

Back from an inspector returns to its parent Automation surface. Back from Detail returns to the Automation list. Leaving an unsaved editor must trigger the appropriate unsaved-draft protection rather than silently discarding changes.

### 40A.18 Mobile Interaction Contract

Mobile is Rule Builder first and must not require a graph canvas.

Tap on a Trigger, Condition, Action, or Flow primitive opens a full-screen editor or bottom sheet appropriate to the amount of configuration.

The editor must provide:

- clear title and context;
- editable parameters;
- validation errors adjacent to the relevant field;
- Save/Done;
- Cancel/Back without accidental data loss;
- Delete for removable nodes with confirmation.

Graph visualization is optional and must not be required to understand or edit the Automation.

### 40A.19 Desktop Interaction Contract

Desktop may use a secondary inspector/drawer or advanced canvas for node editing.

The selected node must remain visually identifiable while its inspector is open. Opening the inspector must not change runtime state.

Advanced Canvas remains subordinate to the semantic Rule Builder. Canvas interactions must map to the same execution primitives and validation rules.

### 40A.20 Consequential Action Feedback

Every consequential Automation mutation or runtime command must expose a distinguishable state:

```text
Idle
  |
Pending
  |
+-+--------------------+
| |                    |
Accepted            Rejected
|                    |
Execution result    Reason
|
+-- Success / Failed / Unknown
```

`Accepted` means the backend accepted the request. It does not by itself mean that the physical action completed.

### 40A.21 Interaction Anti-Patterns

The Automation UI must not:

- make the entire card toggle when only a small status control is intended;
- open an editor when the user intended to enable/disable;
- show unsupported node parameters as executable;
- present Automation Dose as a second EC/pH controller;
- report HTTP acceptance as physical completion;
- silently discard unsaved edits;
- lose selected station context when opening Detail or Test;
- treat Dry Run as a real actuator execution;
- show stale execution data as current without indicating its age/state.

---

## 41. Node Taxonomy Summary

The canonical future-facing execution vocabulary is:

```text
TRIGGERS
T1 Sensor Update
T2 Schedule
T3 System Event / FSM
T4 Recipe / Stage Event
T5 Webhook

CONDITIONS
C1 Compare
C2 Statistic / Time Window
C3 AND / OR
C4 Sensor Quality
C5 Persistence
C6 Rate of Change
C7 System / Stage State
C8 Actuator State
C9 Hysteresis / Re-arm

FLOW
F1 IF / ELSE
F2 Wait / Delay
F3 Wait Until
F4 Repeat
F5 Stop Flow
F6 Run Automation / Chain
F7 Parallel
F8 Join

ACTIONS
A1 Alert
A2 Event-Driven Dose
A3 Water On / Off
A4 Actuator Control
A5 Set PWM
A6 Emergency Stop
A7 Advance Stage
A8 End Season
A9 Recovery / Reset Fault

CONFIG
G1 Config Read
G2 Config Override
G3 Restore Config

EXECUTION POLICY
X1 Cooldown
X2 Maximum Runs
X3 Retry
X4 Timeout
X5 Re-arm

SAFETY
S1 Control Authority / Arbitration
S2 System Interlock
S3 Dose / Rate Limit
S4 Safety Gate
```

The taxonomy is an execution model, not a promise that every item is a currently available visual node.

---

## 42. Current Source Status

### Verified source-backed capabilities

- Automation CRUD;
- enable/disable;
- Sensor/FSM/Cron/Webhook trigger paths;
- comparison and grouped conditions;
- time-window statistics;
- Alert;
- Dose;
- Water actions;
- Emergency Stop;
- Advance Stage;
- End Season;
- Config Read;
- Config Override;
- Config Revert;
- Flow chaining;
- chain cycle/depth protection;
- validation;
- dry run/test;
- execution logging;
- execution success-rate calculation;
- templates;
- supported multi-device template application;
- supported schedule conflict inspection.

### Known implementation gaps

The following must not be represented as complete merely because a related UI concept exists:

- Delay execution semantics;
- general IF/ELSE branching;
- Wait Until;
- Repeat/loop authoring;
- general execution cooldown/re-arm;
- general retry policy;
- general timeout policy;
- sensor freshness condition;
- rate-of-change condition;
- actuator-state condition;
- generalized system/stage condition node;
- general-purpose automated actuator control;
- parallel execution and join;
- generalized conflict engine;
- controller/Automation control-authority arbitration;
- mobile-native push delivery where the current platform path is unavailable.

---

## 43. Product Rules

1. Automation is not Auto Mode.
2. Auto Mode owns closed-loop EC/pH regulatory control.
3. Automation owns event/workflow orchestration.
4. Automation dosing must be explicit and guarded; it must not silently become a second regulatory controller.
5. Every physical Automation action passes through authoritative safety policy.
6. System interlocks cannot be bypassed by normal Automation authoring.
7. Dry run never sends a physical command.
8. HTTP acceptance is not physical confirmation.
9. PWM conversion is not measured flow.
10. Canvas is advanced, not default.
11. Mobile core authoring is rule-builder-first and canvas-free.
12. Settings owns persistent configuration.
13. Journal owns the full historical trace.
14. Technical implementation identifiers should be hidden from the primary novice-facing language.
15. Unsupported runtime capabilities must be labeled as implementation work rather than implied to exist.

---

## 44. Anti-Patterns

Never:

- make Canvas the first screen for every Automation;
- require drag-and-drop to create a simple rule;
- present Auto Mode and Automation as equivalent controls;
- allow an Automation to silently compete with the controller's EC/pH regulation;
- imply that `Dose` means "maintain EC/pH";
- allow Automation to bypass hardware interlocks;
- claim a dry run sent a command;
- claim physical actuator success from command acceptance;
- show PWM-derived ml/min as measured flow;
- expose unsupported Cron controls as if persisted;
- expose a visual Delay node without executable delay semantics;
- call a partial schedule check a universal conflict engine;
- make mobile users manipulate a miniature desktop graph;
- duplicate the full Journal inside Automation Detail;
- duplicate persistent Settings forms inside Automation.

---

## 45. Definition of Done

### Information Architecture

- Automation is a separate station-scoped surface;
- list-first landing page exists;
- selected station context is explicit;
- Operations and Automation preserve the same station/device context;
- Canvas is not the default authoring surface.

### Everyday authoring

- user can create a simple trigger-condition-action rule without a canvas;
- mobile authoring is fully vertical;
- advanced technical concepts are progressively disclosed;
- validation is visible before enablement;
- dry run is available before consequential activation.

### Control boundary

- Auto Mode owns regulatory EC/pH control;
- Automation workflow actions cannot silently override controller ownership;
- all physical actions pass through authoritative safety policy;
- interlocks remain authoritative.

### Runtime

- automation execution state is distinguishable from actuator physical state;
- accepted command is not presented as physical confirmation;
- unknown outcome is represented honestly;
- execution history is inspectable.

### Advanced

- chaining is inspectable;
- advanced graph is optional;
- graph semantics match actual runtime semantics;
- unsupported node types are not presented as implemented.

### Configuration

- Config Override is visible where relevant;
- persistent configuration remains owned by Settings;
- revert preserves authoritative state semantics.

### Safety

- safety review exists for consequential automation;
- E-STOP retains critical safety semantics;
- no generic "disable safety" path exists;
- dose and rate limits remain authoritative.

### Data integrity

- no fabricated telemetry;
- no fabricated physical confirmation;
- no fabricated conflict coverage;
- no fabricated execution capabilities.

---

## 46. Locked Decisions

### 46.1 Automation is orchestration, not regulation

Automation is for event-driven workflows. Closed-loop EC/pH regulation belongs to Auto Mode/controller logic.

### 46.2 Event-driven dosing remains valid

Explicit dosing workflows remain a supported Automation use case when they do not attempt to replace the regulatory controller.

### 46.3 Control authority is required

Auto Mode, Automation, and Manual operations must not become independent competing writers to the same actuator. The system requires an authoritative control-authority/arbitration boundary before actuator execution.

### 46.4 Canvas is advanced

Canvas remains available for complex flow inspection/authoring, but simple Automation must be expressible without it.

### 46.5 Mobile is rule-builder-first

Mobile must not require graph manipulation for core Automation authoring.

### 46.6 Node is an execution primitive

The node taxonomy describes what the Automation engine can express. It does not prescribe that every primitive must be exposed as a visible drag-and-drop node.

---

## 47. Canonical Automation Formula

**Selected Station, Intent, Trigger, Conditions, Action, Control Authority, Safety Gate, Execution Policy, Observed Result, Recovery.**

---

## 48. PAGE-SPEC-CHECKLIST Compliance Contract

This section is the explicit audit contract against `PAGE-SPEC-CHECKLIST.md`. It does not create product capability. `Source-backed`, `Partially source-backed`, and `Implementation-required` statements remain authoritative for implementation status.

### 48.1 Page Identity & Responsibility

- Page: Automation.
- Route: `/automation`.
- Primary purpose: define, validate, test, enable, disable, inspect, and manage station-scoped event/workflow Automations.
- User problem: express repeatable event-driven orchestration without using manual runtime control or creating a second regulatory controller.
- Primary roles: Admin, Operator. Viewer is read/inspect only under the current role contract.
- Owned: Automation definition, workflow execution policy, validation, dry run, chaining, supported Config Override orchestration, templates, supported multi-device application, Automation-specific execution/conflict/audit inspection.
- Not owned: manual runtime actuator control (Operations), closed-loop EC/pH regulation (Auto Mode/controller), persistent configuration ownership (Settings), full historical trace (Journal), hardware safety interlocks.
- Entry: `/automation`, subject to authentication and selected device context.
- Exit: Operations, Dashboard, Cultivation, Journal, Settings, and supported internal Automation detail/editor surfaces. Exact destination must preserve selected device context where the destination is device-scoped.

### 48.2 Source of Truth

| Area | Source | Status / boundary |
|---|---|---|
| Page | `hydragrow-frontend/src/pages/Automation.tsx` | Source-backed current overview shell |
| Automation data/mutations | `hydragrow-frontend/src/hooks/useAutomationScripts.ts` | Source-backed CRUD, validate, test, template apply, Config Override, success-rate APIs |
| Builder state | `hydragrow-frontend/src/hooks/useAutomationBuilder.ts` | Source-backed current React Flow builder behavior |
| Overview canvas | `hydragrow-frontend/src/hooks/useFlowCanvas.ts` | Source-backed summary nodes and `next_flow_ids` edges |
| Editor | `hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx` | Source-backed current editor/test/save/delete behavior |
| Card | `hydragrow-frontend/src/components/automation/FlowOverviewCard.tsx` | Source-backed current card presentation/toggle behavior |
| IR/schema | `hydragrow-frontend/src/lib/automation/ir.ts` | Source-backed type, validation, ranges, trigger/action taxonomy currently admitted by schema |
| Compiler | `hydragrow-frontend/src/lib/automation/compileToRhai.ts` | Source-backed current Rhai compilation; currently compiles only the first action |
| Device context | `hydragrow-frontend/src/store/useDeviceStore.ts` | Source-backed `deviceId` selected context |
| API | `hydragrow-backend/src/api/script.rs` | Source-backed script/config endpoints, authorization scope checks, validation, cycle detection, template apply |
| Runtime model | `hydragrow-backend/src/models/script.rs` | Source-backed `UserScript`, inputs, outputs, `ScriptKind` |
| Runtime engine | `hydragrow-backend/src/services/script_engine.rs` | Source-backed Rhai compile/evaluation and execution result parsing |
| Runtime chain | `hydragrow-backend/src/mqtt/handlers/script_eval.rs` | Source-backed chain evaluation, sensor/FSM/cron/webhook separation, range-stat prefetch, context propagation, execution logging |
| Config ownership | `hydragrow-backend/src/services/config_context.rs`, `config_override.rs` | Source-backed Config Read/Overwrite resolution and reconciliation |
| Page route | `hydragrow-frontend/src/App.tsx` | Source-backed `/automation` route |

Known limitations remain explicit: current IR admits only `alert`, `recipe_override`, and `action_command` as `AutomationKind`; legacy `config_override` kind is rejected by backend and normalized on the frontend. Config Overwrite is represented as a separate IR block, not as a currently valid fourth `AutomationKind`.

### 48.3 Selected Context Contract

- Canonical runtime identity: `useDeviceStore.deviceId`.
- Automation data is fetched with the selected `deviceId`.
- No Automation UI may silently replace selected device context.
- On context change, previous list/detail/editor data must be invalidated or replaced before rendering it as current.
- Pending mutations must retain their own target identity; a response from device A must never update device B's visible state.
- Cross-page navigation to Dashboard, Operations, Cultivation, Journal, or Settings must preserve device context when that destination is device-scoped.
- Missing context: do not issue device-scoped Automation requests; current implementation shows the selected-device-required empty state.
- Loading context: distinguish missing context from Automation query loading.
- A richer station object must not be assumed until implemented; current source exposes `deviceId` as the canonical context.

### 48.4 Component Inventory Contract

| Component | Data/source | Interactive result | Relevant non-ready states |
|---|---|---|---|
| Page header | `deviceId`, page actions | create Flow, Config Explorer | no context, loading |
| Metrics banner | script/config/success-rate queries | inspect-only unless source action exists | loading, unavailable metric |
| Conflict banner | `findScheduleConflicts` result | open affected Flow/editor | no conflicts, limited coverage |
| Search input | local `searchQuery` | filters visible list only | empty query, filtered empty |
| Filter chips | local `filterKind` | changes list only | zero-result |
| Grid/Canvas switch | local `viewMode` | changes presentation only | mobile canvas restriction |
| Flow card | `UserScript` | open editor; status control toggles enabled state | disabled, mutation pending/error |
| Empty state | list result | create Flow | first-time empty |
| React Flow overview | `useFlowCanvas` | node click opens editor | empty graph, unsupported graph semantics |
| Multi-device panel | template source + owned devices | inspect/select/apply where supported | unavailable/unsupported apply |
| Config Explorer widget/view | Config Override API | inspect/revert where permitted | no overrides, load/error |
| Flow editor modal | `UserScript`/builder state | edit, save, delete, dry run, close | validation, pending, error, unsaved draft |
| Node palette/editor | IR schema + builder | add/select/edit node | unsupported primitive |
| Dry Run panel | `TestScriptRequest/Response` | submit simulation | validation/error/no physical command |
| Chain selector | `next_flow_ids` + scripts | select next Flow | self/cycle/empty candidate |

No card body, status badge, canvas node, chip, or inspector control may acquire additional behavior without an explicit contract here or in its subordinate component contract.

### 48.4.1 Component State Contract

Every consequential Automation component must expose a state model that is distinguishable from the Automation's persisted/runtime state.

| Component | Required states | State meaning boundary |
|---|---|---|
| Flow card enable/disable | idle, pending, success, rejected, error, unknown | mutation/result state; not proof of runtime execution |
| Flow editor | opening, ready, dirty, validating, saving, saved, rejected, error | editor/draft/mutation state; not persisted state until authoritative result |
| Delete | available, confirming, pending, deleted, rejected, error, unknown | destructive mutation state; entity remains authoritative until deletion is confirmed |
| Dry Run | idle, validating, running, result, rejected, error | simulation/test state; never physical execution |
| Config Revert | available, pending, success, rejected, error, unknown | configuration mutation state; not actuator observation |
| Template apply | idle, validating, pending, partial/result, rejected, error, unknown | target-application result scoped to requested devices |
| Node inspector | closed, opening, ready, unsupported, validation-error | presentation/editor state; selection does not execute the node |

For each state, implementation must define visible presentation, allowed actions, transition trigger, recovery path, and whether the state represents UI/draft state, request state, backend result, runtime execution, or physical observation.

`enabled`, `last_run_at`, `successRatePercent`, `accepted`, `saved`, and similar values must not be reused to imply a different state semantic without an explicit source contract.

### 48.4.2 Confirmation / Reversibility Contract

Every consequential Automation action must explicitly classify confirmation and reversibility.

- Create/update: no generic destructive confirmation; validation occurs before dispatch; unsaved draft remains available on failure.
- Enable/disable: no ad-hoc confirmation unless a higher-level product contract requires it; pending and authoritative result are explicit.
- Delete: explicit confirmation before dispatch; irreversible unless a source-backed recovery path exists.
- Config Revert: disclose the configuration consequence before dispatch where required; cancellation/reversal is not implied unless supported.
- Template apply: confirmation policy must reflect whether the selected target set will be mutated; no silent target expansion.
- Dry Run: no physical confirmation because the operation is simulation-only.
- Physical Automation action: use the canonical command/safety contract; do not invent a page-local confirmation flow.
- E-STOP, when exposed through Automation-adjacent UI, follows the canonical immediate safety contract rather than ordinary workflow confirmation.

Confirmation of user intent is never equivalent to backend acceptance, persistence, runtime execution, observed state, or physical confirmation.

### 48.4.3 Cross-Page Handoff Contract

Device-scoped handoff follows this minimum sequence:

`Automation -> selected device context -> user intent -> destination -> destination context restoration -> destination-owned task -> return/reconciliation`

Required rules:

- Preserve `deviceId` for device-scoped destinations.
- Do not silently substitute another device when the supplied context is invalid or unavailable.
- Dashboard handoff is monitoring/triage context only; Automation remains mutation owner.
- Operations handoff must identify the same selected device before a device-specific runtime task is presented.
- Cultivation handoff may consume supported stage/device context but must not create a second cultivation scheduler.
- Journal handoff may open compact execution/audit context, but Journal remains full-history owner.
- Settings handoff may inspect or change persistent configuration only through Settings-owned flows; Config Override remains Automation-owned workflow behavior.
- Browser/back from an internal Automation surface restores the parent context without silently changing device identity.
- Pending Automation mutations remain pending/reconcilable across navigation; successful navigation is not mutation success.

### 48.5 Exhaustive Interaction Contract

For every consequential interaction, implementation must follow:

```text
Trigger
  -> immediate UI state
  -> local validation / pending
  -> backend or local operation
  -> accepted / success / rejected / error / timeout / unknown
  -> authoritative reconciliation
```

Required mappings:

- Flow card body: open editor; no runtime mutation.
- Enable/Disable: guarded mutation against the selected script; disable duplicate requests; reconcile from backend response/query; error restores authoritative state.
- Search/filter: local list filtering only; no Automation mutation; filtered empty differs from true empty.
- Grid/Canvas: presentation switch only; no runtime mutation.
- Create: open editor with new draft; validate before create; backend create result is authoritative.
- Node select: open corresponding inspector; selection itself does not execute runtime behavior.
- Save: build IR, schema-validate, compile, call backend validation, then create/update; validation failure preserves draft; mutation failure does not imply saved state.
- Delete: destructive mutation; explicit confirmation; on failure retain the entity until backend state confirms deletion.
- Dry Run: call `/devices/{deviceId}/scripts/test`; show `will_fire`, condition trace, and action preview; true dry run sends no physical command.
- Chain selection: update draft `next_flow_ids`; reject self/cycle according to validation/backend cycle detection; save only after validation.
- Config Explorer open/back: presentation navigation only; preserve device context.
- Config Revert: mutation; pending/success/failure; backend result authoritative.
- Template preview/apply: identify source and targets; validate supported target set; do not silently overwrite or silently change selected context.
- Conflict inspection: show only conflicts returned by the supported conflict detector; never imply universal conflict coverage.
- Close/back from editor: preserve draft while moving between internal inspectors; leaving with unsaved changes must not silently discard them.

### 48.6 Data Contract and Integrity

Important fields:

- `UserScript.id`: UUID/string identifier; source backend model; missing means entity cannot be addressed.
- `device_id`: string device identity; source `UserScript`/request path; required for all device-scoped operations.
- `kind`: enum `alert | recipe_override | action_command` in the current valid frontend IR; backend rejects legacy `config_override` kind.
- `name`: string; required by product contract; editable in editor.
- `source`: Rhai source string; compiler/runtime source of execution semantics.
- `enabled`: boolean; authoritative persisted activation state.
- `ir_json`: optional Automation IR; null is valid for hand-authored legacy Rhai scripts.
- `next_flow_ids`: string UUID list; empty means no chain; backend validates cycles.
- `last_run_at`: optional timestamp for last successful fire; absence means no recorded successful run. It is not proof of current runtime execution.
- `cron_next_run_at`: backend runtime scheduling field; not a physical-state measurement.
- Sensor inputs `ph`, `ec`, `temp`, `water_level`: measured runtime inputs when supplied by `SensorSnapshot`; they are not estimates or commanded values.
- `stage_index`, `phase`, `elapsed_sec`: runtime/FSM context; not actuator confirmation.
- `doseMl`: requested dose parameter, not measured delivered volume.
- `pwm`: requested command parameter, not measured flow.
- `durationSec`: requested action duration, not proof of elapsed physical execution.
- `AlertOutput`: generated event/notification result, not sensor measurement.
- Config Override `originalValue`, `overrideValue`, `currentValue`: configuration values; `currentValue` is read from authoritative device configuration when available and must not be labeled as a physical actuator measurement.
- `successRatePercent`: derived execution-log statistic; null means unavailable, not zero success.
- Missing/unsupported values render as unavailable/unknown rather than invented zero/Off/Success.

Data distinction is mandatory:

```text
Measured != Estimated
Estimated != Commanded
Commanded != Confirmed
```

The current source does not establish a general physical actuator confirmation field for Automation execution. UI must not invent one.

### 48.7 State Model

| State | Contract |
|---|---|
| Initial | Context/query not yet established |
| Missing context | No `deviceId`; no device-scoped mutation/query |
| Loading | Query or context loading; no fabricated flows |
| Ready | Authoritative current data available |
| Empty | Selected device has no Automation |
| Filtered empty | Automation exists but current filter/search matches none |
| Partial | Some auxiliary data such as success rate/config overrides unavailable; preserve known data |
| Stale | Known prior data may remain visible only when explicitly identified as not fresh |
| Offline | Device availability cannot be inferred from Automation query alone; do not claim runtime stopped |
| Fault | Backend/runtime fault state is shown when supplied; no invented diagnosis |
| Unknown | Execution/mutation outcome cannot establish final authoritative state |
| Permission denied | Backend authorization rejects request or capability is not granted |
| Network error | Query/mutation failed without authoritative state change |
| Backend error | API returns failure; preserve authoritative prior state where safe |
| Mutation pending | Disable conflicting duplicate mutation and show pending feedback |
| Mutation success | Reconcile from backend/query; do not rely only on optimistic local state |
| Mutation rejected | Show reason when available and retain authoritative state |
| Timeout | Do not convert automatically to success/failure when final state is unknown |
| Recovery | Retry/reload/reconcile through supported operation; never imply recovery without authoritative result |

### 48.8 Action / Command Contract

Consequential Automation actions are owned by the Automation runtime path, not by the page UI itself.

- Create/update/delete Automation: target selected `deviceId` + script ID where applicable; backend authorization `script:write`; validation and cycle checks apply; backend response is authoritative.
- Enable/disable: mutation of `enabled`; same write authorization; no duplicate pending requests; backend confirmation required before final displayed state.
- Dry Run: target selected `deviceId`; request contains IR plus sample; backend returns `will_fire`, trace, and action preview; no physical command is sent by this endpoint.
- Config Revert: target selected `deviceId` + override ID; requires `script:write`; backend restores persisted configuration and marks override restored; UI must show failure if either authoritative step cannot be confirmed.
- Template apply: source script + target device list + overrides; backend endpoint is `/devices/{deviceId}/scripts/{scriptId}/apply-template`; target validation remains backend-authoritative.
- Runtime physical actions: action result can be accepted/generated/dispatched, but physical completion is confirmed only by an authoritative observed-state source if one exists.
- `doseMl` is positive; `pwm` is integer 1-100 in current IR schema. These are command parameters, not measured delivery.
- `durationSec` is positive integer for water-on actions.
- No general Automation `Set PWM` node is currently admitted as a standalone IR action; the source-backed `pwm` field exists on `dose` and runtime action output. A generalized PWM primitive remains implementation-required.

### 48.9 Safety & Control Authority

- Automation is never a bypass around Operations safety controls.
- Auto Mode owns closed-loop EC/pH regulation.
- Automation owns event/workflow orchestration.
- Manual runtime control remains Operations-owned.
- Physical Automation actions must pass authoritative Safety Gate/interlocks.
- Current source-backed interlocks include `PH_UP` vs `PH_DOWN` and `WATER_PUMP_IN` vs `WATER_PUMP_OUT`.
- Control Authority / Arbitration is a required architecture boundary when multiple writers can target the same controlled resource; the current spec must not present this conceptual boundary as a completed backend implementation unless source verifies it.
- E-STOP is critical safety action. It must use the canonical safety path and must not be treated as an ordinary workflow action. Generic confirmation must not be invented where the canonical safety contract requires immediate action.
- Offline/fault conditions must not produce optimistic physical-success UI.
- Recovery/reset is implementation-required unless the authoritative safety/runtime contract exposes a safe automatic recovery path.

### 48.10 Permission Contract

Current UI role mapping remains Admin / Operator / Viewer. Backend authorization is authoritative through `AuthContext` scopes.

- View/inspect: allowed by current page contract for all three roles.
- Create/edit/enable/disable/delete: write capability; Viewer must not execute these mutations.
- Dry Run/test: read/test capability; Viewer remains read/inspect unless a finer backend permission explicitly grants execution of the test operation.
- Config Override/Revert: write capability; Viewer must not mutate configuration.
- Template multi-device apply: requires write authorization and target scope validation; exact finer-grained target permission is not invented.
- Delete is destructive and requires explicit confirmation in UI; backend authorization remains authoritative.
- Permission failure must be observable and must not leave an optimistic mutation state.

### 48.11 Navigation Contract

- Entry: `/automation`.
- Internal surfaces: overview, Config Explorer, Flow editor, node inspector, Dry Run, chain/template/conflict inspection as supported by current components.
- Operations link: `/operations`; Automation must remain a separate surface, not an Operations tab/domain.
- Browser/back: return to the parent surface without silently changing selected `deviceId`.
- Missing context: do not fabricate a default device.
- Deep link with no valid selected context: show the required-context state; do not load another device implicitly.
- Pending mutation during navigation: do not silently discard the operation; surface pending/result or reconcile when returning.
- Unsaved editor: preserve draft or require explicit discard; never silently discard.
- Journal owns full historical trace; Automation only exposes compact execution information supplied by its source.
- Settings owns persistent configuration; Config Explorer does not become a second Settings surface.

### 48.12 Desktop & Mobile Contract

Desktop:

- List-first overview.
- Grid is the normal overview presentation.
- Canvas is advanced/secondary, not the required authoring model.
- Flow editor may use the large centered modal and secondary inspector/panel already implemented.
- Multi-device panel is desktop-only in the current `Automation.tsx` branch.
- Keyboard focus and visible focus are required for all actionable controls.

Mobile:

- Rule Builder semantics are primary; no miniature desktop canvas requirement.
- Trigger, condition, action, config, and chain editing must be reachable through touch-friendly inspector surfaces.
- Hover is not required.
- Touch targets must be adequate.
- Critical save/test/close actions remain reachable without horizontal graph navigation.
- If advanced graph inspection is retained, it is optional and must not be required for core authoring.

### 48.13 Forms & Validation

Current authoring form fields:

- Flow `name`: required non-empty string by product contract; current UI exposes editable text.
- `kind`: one of the current IR-supported kinds.
- Trigger/condition/action fields: constrained by the Zod IR schema and node editor.
- Conditions: at least one condition; comparison value numeric; non-instant statistic mode requires positive `windowSec`.
- Actions: at least one action; action type must match Automation kind.
- Dose: positive `doseMl`; `pwm` integer 1-100; supported pump enum only.
- Water On: positive integer `durationSec`; supported pump enum only.
- Config Overwrite: required key/value; priority integer; restore behavior follows IR/backend contract.
- Cron: non-empty six-field expression plus timezone; backend validates the trigger before persistence.
- Webhook mappings: non-empty `bodyPath` and `targetField` for each mapping.
- Chain: selected IDs are persisted as `next_flow_ids`; backend rejects cycles.

Validation layers: UI/schema, compiler, backend validation, and runtime safety/ownership. Passing syntax validation does not imply safety or physical execution success.

Submit: validate first, then create/update. Backend rejection keeps the draft. Cancel/close must not silently destroy unsaved work. Reset behavior must be explicit; no implicit reset after validation failure.

### 48.14 History & Audit

- Compact execution history may appear in Automation Detail when supplied by runtime/source.
- `last_run_at` records the latest successful fire; it is not a complete execution journal.
- Execution success rate is a derived summary, not full history.
- Config Override history is available through the Config Override endpoint and current Config Explorer data contract.
- Full historical trace remains Journal-owned.
- Consequential Automation events should be auditable where the authoritative audit/log system supports them.
- Audit identity should include actor, device, Automation, timestamp, requested operation/parameters, and result/rejection/error when supplied by the source. Do not invent absent fields.

### 48.15 Empty / Loading / Error / Unknown

- First-time empty: selected device has no Automations; provide create action.
- Filtered empty: Automations exist but current search/filter matches none; provide filter reset.
- Loading: skeleton/loading state; no fabricated metrics or flows.
- Partial loading: preserve available list while auxiliary metrics/config data load, if safe.
- Network/backend error: show explicit failure and retry path; retain known authoritative data where safe.
- Offline: do not claim Automations stopped or executed; show inability to observe current runtime when applicable.
- Unknown outcome: keep unknown distinct from failed/successful physical execution.
- Stale data: identify stale/unavailable freshness rather than presenting old execution state as current.
- Retry must re-fetch/reconcile authoritative state; it must not duplicate a consequential command.

### 48.16 Cross-Page Contract

```text
Dashboard
  = monitoring / triage

Operations
  = current runtime control

Auto Mode
  = closed-loop EC/pH regulation

Automation
  = event/workflow orchestration

Settings
  = persistent configuration

Journal
  = canonical historical trace
```

- Dashboard may navigate to Automation for workflow management but does not own Automation mutation.
- Operations owns current manual runtime control; Automation must not become an Operations tab.
- Auto Mode owns continuous EC/pH regulation; Automation event-driven Dose must not become a second controller.
- Settings owns persistent configuration; Automation Config Override is scoped workflow behavior, not general Settings ownership.
- Journal owns full history; Automation retains only useful compact execution/audit context.
- Cultivation owns cultivation-stage configuration/context; Automation may consume supported stage/FSM context but must not invent a separate cultivation scheduler.
- Fleet/provisioning owns device discovery/pairing. Automation may target supported devices through the template application contract without silently changing the selected device.

### 48.17 Accessibility Contract

- All actions use semantic controls appropriate to their behavior; a status indicator must not masquerade as a command.
- Every icon-only control has an accessible name.
- Selected filter/view state is programmatically distinguishable, not color-only.
- Focus is visible and keyboard navigation is supported on desktop.
- Modal/editor focus remains usable and has an explicit close path.
- Validation errors are associated with the relevant field and are discoverable without relying only on color.
- Pending/success/error status is communicated in text accessible to assistive technology.
- Destructive delete wording is explicit.
- Safety-critical E-STOP semantics must remain explicit and accessible when the action is exposed.
- Mobile controls meet adequate touch-target sizing.

### 48.18 Observability Contract

- User-visible status must distinguish Automation state, mutation state, execution state, and physical observed state.
- Mutation lifecycle must expose pending, accepted/success, rejected/error, and unknown where applicable.
- Runtime error code/reason is shown when supplied and useful.
- Execution timestamps are shown where source provides them.
- Execution/audit events use the authoritative logging system; page-local fake audit records are prohibited.
- Runtime logs/deep links may be exposed only when the source provides them.
- Unknown outcomes remain visible and recoverable; they must not be silently discarded or converted to failure.

### 48.19 Wireframe Contract

Required visual states for implementation:

1. Desktop overview: list/grid first, metrics, search/filter, optional advanced canvas, Config Explorer.
2. Mobile overview: compact list/rule-builder entry, no required desktop graph editing.
3. Flow editor: name/status/actions, node authoring, inspector, validation, save/close.
4. Critical interaction: enable/disable, save, dry run, Config Revert, destructive delete, physical action safety review where applicable.
5. Critical error: validation failure, backend failure, permission rejection, unknown execution result.
6. Empty: no Automation for selected device and filtered-empty variant.
7. Loading: context/query loading without fabricated data.
8. Dangerous action: E-STOP and other safety-sensitive actions use canonical safety semantics; no generic confirmation is added where immediate execution is required.

Wireframes must not add unsupported Delay execution, general branching, generalized PWM, freshness, retry, timeout, parallel/join, universal conflict detection, or other `REQUIRES IMPLEMENTATION` capabilities.

### 48.20 Acceptance Criteria

#### AC-1: Selected device context

Given a valid selected `deviceId`
When Automation loads
Then all Automation queries target that device
And no data from another device is rendered as current.

#### AC-2: Missing context

Given no selected `deviceId`
When the Automation page renders
Then no device-scoped Automation mutation is issued
And the UI explains that a device must be selected.

#### AC-3: Create validation

Given an invalid Automation draft
When the user saves
Then backend/schema validation prevents persistence
And the draft remains available with the relevant error.

#### AC-4: Enable/disable failure

Given an existing Automation
When enable/disable is requested and the backend rejects it
Then the UI shows the rejection/error
And does not leave the Automation in an unconfirmed mutated state.

#### AC-4a: Enable/disable component state

Given an Automation enable/disable mutation is dispatched
When the request is pending, accepted, rejected, or unknown
Then the Flow card exposes the corresponding mutation state
And the displayed `enabled` value is not treated as runtime execution proof.

#### AC-5: Dry Run

Given a valid Automation and sample values
When the user runs Dry Run
Then the UI shows trigger/condition/action preview from the test response
And no physical command is sent.

#### AC-5a: Dry Run state

Given a valid Dry Run request
When validation or execution is pending, succeeds, rejects, or errors
Then the Dry Run surface exposes the corresponding test state
And no test state is presented as physical execution.

#### AC-6: Unknown physical outcome

Given an Automation action request whose physical result cannot be established
When execution feedback is rendered
Then the UI shows unknown/unconfirmed
And does not label the actuator physically successful.

#### AC-7: Chain cycle

Given Flow A already participates in a chain
When the user creates a cycle through `next_flow_ids`
Then validation/backend cycle detection rejects the mutation
And the existing authoritative Flow remains unchanged.

#### AC-8: Station/device switch

Given Automation data for device A is visible
When selected context changes to device B
Then A data is invalidated or replaced before B data is rendered as current
And pending responses for A cannot overwrite B state.

#### AC-9: Config Revert

Given an active Config Override
When the user requests Revert and the backend confirms success
Then the UI reflects the restored authoritative configuration state
And the override is no longer presented as active.

#### AC-9a: Revert failure

Given an active Config Override
When Revert is rejected, errors, or reaches an unknown outcome
Then the UI does not present the configuration as restored
And the user has a supported recovery or reconciliation action.

#### AC-10: Permission failure

Given a Viewer without write authorization
When a consequential mutation is attempted
Then the backend denies it or the UI prevents unsupported execution
And no optimistic mutation remains visible as authoritative.

#### AC-10a: Delete confirmation and reversibility

Given the user attempts to delete an Automation
When deletion is initiated
Then explicit confirmation occurs before mutation dispatch
And no undo or rollback is implied unless a source-backed recovery path exists.

#### AC-11: Automation versus Auto Mode

Given a user defines an event-driven Dose
When the rule is intended to continuously regulate EC/pH
Then the UI identifies Auto Mode as the owning control surface
And Automation does not create a competing closed-loop controller.

#### AC-12: Empty and filtered empty

Given the selected device has no Automations
When the page loads
Then the true empty state is shown.

Given Automations exist
When search/filter matches none
Then the filtered-empty state is shown
And resetting the filter restores the list.

#### AC-13: Cross-page context handoff

Given device `A` is selected in Automation
When the user opens a device-scoped destination
Then the destination receives device `A` context
And it does not silently substitute another device.

#### AC-14: Return reconciliation

Given an Automation mutation is pending before navigation
When the user returns to Automation
Then the mutation is still represented as pending, resolved, rejected, or unknown according to authoritative state
And navigation itself is not treated as mutation success.

#### AC-15: Unsaved editor draft

Given the Flow editor contains unsaved changes
When the user closes or navigates away
Then the draft is preserved or explicit discard is required
And unsaved changes are not silently lost.

#### AC-16: Unsupported primitive

Given a requested flow primitive is marked `Implementation-required`
When the user attempts to configure or execute it
Then the UI does not expose it as a currently executable capability
And the limitation is explicit.

#### AC-17: Physical confirmation boundary

Given an Automation action is accepted or dispatched
When no authoritative observed-state source confirms physical completion
Then the UI shows the command/execution result without claiming physical confirmation.

#### AC-18: Conflict detector scope

Given the supported conflict detector returns no conflict
When the result is displayed
Then the UI presents only the detector's supported scope
And does not label the schedule universally conflict-free.

### 48.21 Implementation Mapping and Required Work

Current implementation mapping is source-backed for the overview shell, query/mutation hooks, IR schema, builder, canvas, editor, API endpoints, backend model, compiler, execution engine, chain execution, Config Override services, and `/automation` route listed in Section 48.2.

Implementation-required work remains where this spec explicitly says so, including:

- general Control Authority / Arbitration enforcement if not already provided by a canonical backend subsystem;
- end-to-end execution semantics for unsupported flow primitives;
- generalized retry/timeout/cooldown/re-arm policies;
- sensor freshness/quality, rate-of-change, actuator-state, and hysteresis conditions;
- generalized actuator/PWM automation primitives;
- parallel/join execution;
- universal conflict detection;
- mobile-safe advanced-flow editing if required beyond inspection;
- complete unsaved-draft/navigation protection if not already implemented by the editor lifecycle;
- explicit accessibility implementation coverage where current components do not yet satisfy this contract.

Documentation change and code change are separate. This section does not claim these implementation-required items are implemented.

### 48.22 Anti-Pattern Review

- No fabricated telemetry, diagnostics, physical measurements, delivered dose, or execution stages.
- No `last_run_at` or execution acceptance presented as current physical state.
- No command parameter presented as measured output.
- No unsupported node exposed as executable.
- No Dashboard/Operations/Automation/Settings/Journal ownership duplication.
- No hidden device-context switch.
- No stale device A data presented under device B.
- No destructive mutation without appropriate confirmation.
- No generic confirmation added to safety actions whose canonical contract requires immediate execution.
- No full persistent configuration duplicated inside Automation.
- No full Journal duplicated inside Automation Detail.
- No partial conflict detector presented as universal.
- No Dry Run presented as physical execution.

### 48.23 Definition of Done

Automation spec is checklist-compliant only when:

- page identity, route, responsibility, entry, and exit are explicit;
- source files, hooks, store, APIs, backend handlers/services, models/types, existing behavior, limitations, and implementation gaps are mapped;
- selected device context and switch semantics are explicit;
- component inventory and interaction outcomes are explicit;
- consequential components have explicit component-state contracts;
- confirmation, cancellation, reversibility, and rollback semantics are explicit for consequential actions;
- Data Contract distinguishes measured, estimated, commanded, and confirmed values;
- relevant initial/loading/ready/empty/partial/stale/offline/fault/unknown/permission/error/mutation/recovery states are explicit;
- consequential actions have command, permission, safety, failure, and unknown semantics;
- Control Authority ownership is explicit without claiming unverified implementation;
- navigation, pending mutations, unsaved drafts, and context preservation are explicit;
- desktop/mobile behavior is explicit;
- form and validation behavior is explicit;
- history/audit ownership and Journal boundary are explicit;
- empty/loading/error/unknown behavior is explicit;
- cross-page ownership boundaries are explicit;
- cross-page handoff defines context preservation, restoration, unavailable-context behavior, and return reconciliation;
- accessibility and observability requirements are explicit;
- wireframe states do not introduce unsupported capability;
- acceptance criteria are testable in Given/When/Then form;
- acceptance criteria cover component states, confirmation/reversibility, context handoff, pending navigation, and unsupported-capability boundaries;
- implementation-required work is separated from source-backed behavior;
- anti-pattern review passes;
- no unsupported capability is presented as implemented.

### 48.24 Canonical Page-Spec Formula

```text
PAGE SPEC
=
Responsibility
+ Source of Truth
+ Ownership
+ Selected Context
+ Information Architecture
+ Component Contract
+ Interaction Contract
+ Data Contract
+ State Model
+ Action / Command Contract
+ Safety / Control Authority
+ Permission
+ Navigation
+ Desktop / Mobile
+ Validation
+ History / Audit
+ Error / Unknown Handling
+ Cross-Page Contract
+ Wireframe Contract
+ Acceptance Criteria
+ Implementation Mapping
```

Final review question:

> Can another engineer implement Automation, its interactions, state transitions, safety boundaries, permissions, selected-device behavior, and cross-page behavior from this specification without inventing missing product behavior?
