# HYDRAGROW — OPERATIONS / CONTROL FUNCTIONAL SPECIFICATION

## 0. Purpose

This document is the functional source of truth for the Operations page wireframe and interaction model.

Operations is the station-specific runtime control surface. It consumes the selected station context established by Dashboard and provides guarded control of supported physical actuators.

This is a UX and product contract, not an implementation specification.

Related specifications:

- `UX-RULES.md`
- `STATE-TRANSITION-SPEC.md`
- `COMPONENT-CONTRACT.md`
- `PAGE-CONTRACT.md`
- `DESIGN-TOKENS-VISUAL-SPEC.md`
- `UX-FLOW-TASK-FLOW-SPEC.md`
- `DASHBOARD-FUNCTIONAL-SPEC.md`

---

## 1. Core Principle

Operations means:

**Inspect first, control second, verify last.**

Every physical command must distinguish:

- current observed physical state;
- requested action;
- command lifecycle state;
- observed result.

A successful request submission is not by itself proof that a physical actuator changed state.

---

## 2. Page Responsibilities

Operations owns:

- station-specific actuator control;
- actuator state inspection;
- guarded manual intervention;
- safety interlock presentation;
- command lifecycle feedback;
- fault recovery actions supported by the source.

Operations does not own:

- fleet administration;
- full historical trace;
- cultivation planning;
- full system configuration;
- station pairing;
- long-term telemetry analytics.

---

## 3. Station Context Contract

Operations must always have an explicit selected station context.

The page header must make the active station unmistakable:

```text
Operations
Station C   ● Online   [Station C ▾]
```

Where device identity is necessary to disambiguate the station, show the relevant device identifier as secondary metadata.

The selected station context is inherited from Dashboard when Operations is entered through a station-specific action.

---

## 4. Station Switching

The station selector may switch the operational context without leaving Operations.

When the station changes:

1. update the selected station context;
2. load the authoritative state for the new station;
3. discard stale visual state from the previous station;
4. preserve unresolved command lifecycle state separately for the previous station when required;
5. render only controls and data belonging to the new station.

An actuator state from Station C must never appear under Station D.

---

## 5. Page Anatomy

```text
Application Shell
    Page Header
        Back / Dashboard
        Station Context
        Station Selector
    Operational Surface Navigation
        Điều khiển
        Tự động hóa
    System Condition
    Active Recipe
    Surface Content
    Page-level E-STOP
```

Operations and Automation are separate product surfaces. They share the same selected station context but do not share the same page mental model.

The safety surface remains visually available on Operations. Automation is not a runtime actuator-control surface and does not inherit the Operations E-STOP presentation merely by sharing station context.

---

## 6. Operations and Automation Surface Boundary

The selected-station operational area exposes two primary surfaces:

| Surface | Purpose |
|---|---|
| `Điều khiển` | Physical actuator control and operational state |
| `Tự động hóa` | Definition and management of automatic logic |

Default surface:

**Điều khiển**

These are navigation surfaces, not two modes of the same control panel.

Navigating between the surfaces does not change the selected station.

The distinction is explicit:

- **Operations:** điều khiển hệ thống hiện tại;
- **Automation:** định nghĩa và quản lý logic hệ thống tự động;
- **Auto Mode:** control-ownership state used by Operations.

`Auto Mode` must never be presented as equivalent to the Automation surface.

---

## 7. System Condition Region

The system condition region appears before physical controls.

It represents the highest-priority current condition relevant to operation:

- normal / ready;
- warning;
- fault;
- emergency;
- unavailable;
- relevant recovery condition.

Example normal state:

```text
┌──────────────────────────────────────────────────────────────┐
│ ● Hệ thống sẵn sàng                                         │
│ Có thể thực hiện thao tác thủ công.                         │
└──────────────────────────────────────────────────────────────┘
```

The message must be derived from authoritative state and available guidance. Do not invent fault causes.

---

## 8. Active Recipe Region

When an active recipe is available, Operations may show its operational summary before actuator controls.

The summary may contain source-backed values such as:

- recipe ID;
- revision;
- current stage;
- nutrient A:B ratio;
- EC target and tolerance;
- pH target and tolerance;
- water-level target;
- water-change interval when available.

The recipe region is informational. It is not a substitute for configuration editing.

Example:

```text
┌──────────────────────────────────────────────────────────────┐
│ QUY TRÌNH ĐANG CHẠY                                          │
│                                                              │
│ Recipe       Stage       A:B       EC / pH / Water          │
│ <value>      <value>     <value>   <source-backed values>   │
│                                                              │
│                                                   [Làm mới]  │
└──────────────────────────────────────────────────────────────┘
```

If no active recipe exists, display the canonical no-recipe state supplied by the product contract.

---

## 9. Control Mode

Operations must distinguish the current control ownership mode.

Current source-backed modes:

- `auto`;
- `manual`.

### Manual mode

Supported manual controls may be available subject to permission, availability, interlock, safety, and command guards.

### Auto mode

When `auto` is active, the automatic control path owns runtime control. Manual actuator commands are blocked unless the product explicitly supports an approved override path.

The UI must explain why a manual control is unavailable.

Auto mode is a control-ownership state, not a fault.

---

## 10. Actuator Groups

The current actuator surface is grouped into three functional areas.

### 10.1 Châm dinh dưỡng và pH

- `PUMP_A` — Bơm phân A
- `PUMP_B` — Bơm phân B
- `PH_UP` — Bơm pH Up
- `PH_DOWN` — Bơm pH Down

### 10.2 Cấp & Xả nước bồn

- `WATER_PUMP_IN` — Van cấp nước
- `WATER_PUMP_OUT` — Bơm xả thoát

### 10.3 Phun sương và Tuần hoàn

- `OSAKA` — Bơm tăng áp
- `MIST` — Van phun sương
- `MIX` — Van trộn

The implementation/source remains authoritative if the actuator inventory changes.

---

## 11. Actuator Card

Every actuator is represented by an Actuator Card.

Minimum anatomy:

```text
┌─────────────────────────────────────────────┐
│ [icon] Bơm phân A                           │
│        PUMP_A                                │
│                                              │
│ ● Đang hoạt động                 [ ON ]     │
│                                              │
│ Công suất                         <value>    │
│ ─────────────────────●──────────            │
│                                              │
│ [Điều khiển]              [Chi tiết]        │
│                                              │
│ ⌄ Tùy chỉnh kỹ thuật                         │
└─────────────────────────────────────────────┘
```

The card background is not itself a command target.

Clicking the card body must never toggle the actuator.

---

## 12. Actuator Detail

Each actuator may expose an Actuator Detail surface from Operations.

It is an entity detail surface, not a new top-level navigation item.

Desktop presentation should preferably use a drawer or secondary detail pane. Mobile may use a full-screen detail surface.

The detail surface contains:

1. current state;
2. identity and capability;
3. operational summary;
4. system diagnostics and availability;
5. configuration summary;
6. recent activity preview.

It must not become a second Operations page.

---

## 13. Actuator Detail — Current State

Current physical state is the primary information.

```text
CURRENT STATE

● Đang hoạt động

Command        Idle
Interlock      Clear
Control mode   Manual
Last observed  <timestamp>
```

Physical state and command state must remain visually distinct.

---

## 14. Actuator Detail — Identity & Capability

Show only source-backed identity and capabilities.

Current useful fields:

- actuator name;
- actuator ID;
- type;
- supported ON/OFF control;
- PWM support when applicable;
- timed-run support;
- supported control mode(s) when available.

For PWM-capable actuators, show the source-backed PWM contract:

- minimum 20%;
- maximum 100%;
- step 5%.

Do not infer hardware specifications from an actuator label or control capability.

---

## 15. Actuator Detail — Operational Summary

The operational summary is a compact troubleshooting snapshot, not a full event history.

Potential source-backed values:

- current state;
- current command state;
- current control ownership (`auto` / `manual`);
- interlock state;
- last observed state change when a timestamp exists;
- last command;
- last fault.

Only values actually available from the source may be rendered.

---

## 16. Actuator Detail — System Diagnostics & Availability

Diagnostics are limited to state, fault, command health, and availability signals that the current system actually exposes.

The detail surface may show:

- controller/device availability;
- actuator runtime state;
- command lifecycle status;
- FSM state and fault code when present;
- safety interlock state;
- control-mode restriction;
- permission/availability state;
- relevant timestamps when available.

The section explains why an actuator is currently unavailable or restricted when the source provides a reason.

Diagnostics are not a separate hardware telemetry layer. The UI must not claim physical health beyond the observed state and diagnostics actually exposed by the system.

---

## 17. Actuator Detail — Configuration Summary

Actuator Detail may show a compact summary of relevant configuration.

Example:

```text
CONFIGURATION

Control mode      Manual
PWM               Supported
Default PWM       <value>
Interlock         PH_DOWN
Availability      <source-backed reason when restricted>

[Mở trong Settings]
```

This is read-oriented context. Relevant dosing/calibration configuration may be summarized when already exposed by the source, including configured dosing-pump capacity or minimum/default PWM where applicable. Such values describe configuration/calibration, not measured physical flow.

Actuator Detail must not become an alternative configuration editor.

---

## 18. Configuration Ownership

Persistent configuration remains owned by **Settings**.

Configuration must not be duplicated as editable forms in Actuator Detail.

Recommended Settings hierarchy:

```text
Settings
├── Station Configuration
├── Device Configuration
│   ├── Controller
│   ├── Sensor Node
│   └── Actuators
│       ├── PUMP_A
│       ├── PUMP_B
│       ├── PH_UP
│       └── ...
├── Control & Safety
├── Automation
└── System
```

Actuator Detail may deep-link to the exact Settings scope when supported.

---

## 19. Runtime Control vs Persistent Configuration

Runtime controls belong to Operations.

Examples:

- current ON/OFF command;
- requested PWM for a command;
- timed-run duration.

Persistent configuration belongs to Settings.

The following distinction must remain explicit:

```text
Operations
    Current command parameters

Settings
    Persistent configuration
```

A runtime PWM command must not silently modify the persistent default PWM configuration.

---

## 20. Actuator Detail — Recent Activity

Actuator Detail may show a short recent-activity preview.

Example:

```text
RECENT ACTIVITY

22:14  Started manually
21:58  Stopped by automation
20:42  PWM changed
18:31  Command rejected

[Xem toàn bộ Journal]
```

This is a preview only.

Journal remains the historical source of truth.

When available, the preview should prioritize the latest actuator command/outcome and state-change events so the user can quickly understand what happened most recently.

---

## 21. Journal Deep Link

When the source supports the required filter, `Xem toàn bộ Journal` should open Journal with:

- selected station;
- relevant device;
- actuator scope when supported.

The user must not have to manually reconstruct the context when the application already knows it.

---

## 22. Actuator Control Action Classes

Operations controls belong to distinct action classes:

| Class | Examples |
|---|---|
| Inspect | Actuator Detail, Recipe summary |
| Command | ON/OFF, timed run, PWM command |
| Recovery | Reset fault |
| Safety | E-STOP, emergency force when explicitly supported |
| Configuration | Automation/config override actions |

Each class uses its appropriate feedback and guard contract.

---

## 23. Physical State

The current physical actuator state must be derived from authoritative observed data.

The UI may display states such as:

- ON / Đang hoạt động;
- OFF / Tắt;
- unknown;
- unavailable;
- other canonical semantic states supported by the source.

Do not set physical state solely because a command button was pressed.

---

## 24. Command State

Command lifecycle is independently represented.

Current source-backed UI statuses may include:

- `sending`;
- `accepted`;
- `safety_blocked`;
- `rate_limited`;
- HTTP failure;
- network failure.

The UI should map these into the canonical flow vocabulary defined by `UX-FLOW-TASK-FLOW-SPEC.md` and `STATE-TRANSITION-SPEC.md` without inventing unnecessary new states.

---

## 25. Command Lifecycle

Canonical lifecycle:

```text
Intent
Guard
Request
Pending
Result
Physical verification
Completion or recovery
```

The visual lifecycle must not skip from intent directly to confirmed physical state.

Example:

```text
Physical state: Tắt
Command: Đang gửi lệnh bật...
```

Only after observed state confirms the change:

```text
Physical state: Đang hoạt động
Command: Đã xác nhận
```

---

## 26. Dangerous Commands

Commands requiring stronger confirmation or guard treatment include source-defined dangerous operations such as:

- force-on;
- reset fault;
- PWM-setting commands;
- dosing-pump commands;
- emergency force actions.

The confirmation surface must identify the target and requested parameters.

---

## 27. Confirmation Contract

Where confirmation is required:

```text
┌──────────────────────────────────────────────┐
│ Xác nhận lệnh                                │
│                                              │
│ Station       Station C                      │
│ Thiết bị      Bơm phân A                     │
│ Hành động     Bật                            │
│ Thời lượng    <value>                        │
│ Công suất     <value>                        │
│                                              │
│ [Hủy]                     [Xác nhận]         │
└──────────────────────────────────────────────┘
```

Only display fields that actually apply to the selected command.

---

## 28. Safety Interlocks

Current source-backed interlocks include:

| Device | Blocked by |
|---|---|
| `PH_UP` | `PH_DOWN` |
| `PH_DOWN` | `PH_UP` |
| `WATER_PUMP_IN` | `WATER_PUMP_OUT` |
| `WATER_PUMP_OUT` | `WATER_PUMP_IN` |

When blocked, the actuator must explain the reason locally.

```text
LOCKED
Đã khóa vì Bơm pH Down đang chạy.
```

Do not only disable the control without an explanation.

---

## 29. Auto-Mode Guard

When control mode is `auto`:

- manual actuator commands are blocked according to the source contract;
- current physical state remains visible;
- the UI explains that automation owns control;
- the user is directed to the appropriate Settings surface if changing mode is permitted.

Do not hide the actuator merely because it cannot currently be controlled manually.

---

## 30. Offline Guard

When the selected station is unavailable:

```text
┌──────────────────────────────────────────────┐
│ HỆ THỐNG NGOẠI TUYẾN                         │
│ Không thể truyền lệnh.                       │
│                                              │
│ Controls disabled                            │
└──────────────────────────────────────────────┘
```

Last-known physical state may be shown only when clearly marked as not live.

Do not present stale ON/OFF state as confirmed live state.

---

## 31. Permission Guard

Viewer may inspect operational state but cannot execute control actions.

```text
Bạn không có quyền điều khiển thiết bị.
```

Controls should remain understandable and visibly locked rather than disappearing without explanation when the information itself is permitted to be viewed.

Backend authorization remains authoritative.

---

## 32. PWM Contract

Source-backed PWM support currently applies to:

- `PUMP_A`;
- `PUMP_B`;
- `PH_UP`;
- `PH_DOWN`;
- `OSAKA`.

The current control range is:

- minimum 20%;
- maximum 100%;
- step 5%.

The UI must not render PWM controls for actuators without PWM support.

---

## 33. PWM Calibration Boundary

The current source contains a PWM-to-flow conversion mapping intended as temporary calibration data.

It must not be presented as measured physical flow.

If a derived value is shown, it must be explicitly labeled as a configured or estimated conversion, not actual measured dosing flow.

Until authoritative calibration data exists, the safest default is to expose PWM percentage without claiming an actual ml/min measurement.

---

## 34. Timed Run

Timed run accepts duration in seconds.

Example:

```text
Thời gian hẹn giờ (Giây)
[ 30 ]

[ Chạy ]
```

Before execution, validate:

- station context;
- station availability;
- permission;
- control mode;
- interlock;
- relevant safety state;
- duration validity;
- command concurrency.

---

## 35. Emergency Force Run

Emergency force run is a privileged recovery/safety action, not a normal actuator control.

It must clearly state that normal safety checks may be bypassed according to the source contract.

The interaction must identify:

- station;
- actuator;
- requested action;
- PWM when applicable;
- duration when applicable;
- warning;
- confirmation requirement when defined by the authoritative safety contract.

---

## 36. E-STOP Contract

E-STOP is page-level safety control and remains available regardless of the selected Operations tab.

It must identify its target station/device when required by the source.

The final confirmation behavior is governed by the authoritative safety decision for HYDRAGROW.

For this specification, the product decision is now locked as:

**E-STOP is an immediate safety action and must not be delayed by a generic confirmation dialog.**

The UI should provide strong target visibility and immediate feedback rather than an extra confirmation step.

---

## 37. E-STOP Feedback

After E-STOP is invoked, the UI must enter the appropriate command/safety lifecycle and wait for authoritative system state.

It must not claim that all physical actuators are stopped merely because the request was accepted.

Example pending state:

```text
E-STOP requested
Waiting for system state confirmation...
```

---

## 38. Reset Fault

Reset Fault is a recovery request, not proof of recovery.

Example:

```text
[Khôi phục]
```

Confirmation, where required:

```text
Khôi phục trạng thái hoạt động?

[Hủy]                  [Khôi phục]
```

After execution, wait for authoritative FSM state.

Do not replace a fault state with normal state solely because the reset request succeeded at the transport layer.

---

## 38A. Operations Interaction Contract

This section is the authoritative click/action contract for the Operations surface. Every interactive component must define what opens, what changes, what is pending, and what constitutes a confirmed result.

### 38A.1 Page Header

```text
Operations
Station C   ● Online   [Station C ▾]
```

- Back/Dashboard returns to Dashboard without changing station context.
- Station selector opens the station-selection surface.
- Selecting another station changes context, then reloads authoritative runtime state.
- Station status is informational unless the source provides a specific diagnostic action.

### 38A.2 System Condition

Clicking a condition banner may open the relevant diagnostic/fault guidance when the source provides one.

The banner itself must not silently execute recovery.

For fault conditions, show the source-backed fault code/guide when available and expose the supported recovery action separately.

### 38A.3 Active Recipe

The recipe region is read-oriented.

- Click/inspect may open the relevant recipe/stage detail when that navigation exists.
- `Làm mới` refreshes authoritative recipe/runtime data.
- Recipe values do not become editable from Operations unless a separate source-backed edit surface explicitly owns the mutation.

Operations must not silently change recipe configuration when the user is only inspecting the active recipe.

### 38A.4 Control Mode

The current mode is visible independently from actuator state.

When `manual` is active, supported manual controls may be enabled after all guards pass.

When `auto` is active:

- manual controls remain visible where useful;
- conflicting manual commands are blocked;
- the UI explains that automatic control currently owns runtime control;
- changing control mode is not performed by clicking an actuator control.

If mode change is supported by the product, it must use an explicit mode-control interaction and its own command lifecycle.

### 38A.5 Actuator Card — Primary Interaction

The card body is inspect-only.

```text
Actuator Card
    |
    +-- [Điều khiển] --> Control surface
    +-- [Chi tiết]   --> Actuator Detail
    +-- [technical expand] --> technical controls/details
```

Clicking the card body must not toggle the actuator.

### 38A.6 ON/OFF Control

For a supported actuator, the ON/OFF control represents a command intent, not immediate physical state.

```text
Click ON
  |
  v
Validate guards
  |
  v
Sending / Pending
  |
  +-- Accepted --> wait for authoritative state
  +-- Blocked / Rejected --> show reason
  +-- Network / HTTP failure --> show failure state
  +-- Timeout --> Unknown outcome
```

The observed ON/OFF indicator changes to the new physical state only when authoritative observed data confirms it.

### 38A.7 PWM Control

Clicking or adjusting PWM changes the requested command parameter only; it does not change persistent configuration.

For supported PWM actuators:

- show 20% to 100%;
- step 5%;
- validate before dispatch;
- display the selected/requested value separately from observed actuator state.

If PWM is unavailable for an actuator, the control must not be rendered as an active control.

### 38A.8 Timed Run

`Chạy` on a timed-run control opens or uses the duration input, then validates the complete command context before dispatch.

The interaction must identify:

- target actuator;
- duration;
- PWM when applicable;
- control mode;
- interlock;
- safety/availability state.

While the command is unresolved, a second conflicting timed command must not be dispatched unless the authoritative command policy allows it.

### 38A.9 Advanced Device Control

Advanced controls are subordinate to the same Operations command contract.

They must not bypass:

- station context;
- permission;
- Auto Mode ownership;
- interlocks;
- safety gate;
- command lifecycle;
- physical verification.

The presence of an advanced control must not imply a hardware capability that is not exposed by the source.

### 38A.10 Actuator Detail

Clicking `Chi tiết` opens Actuator Detail.

Desktop: drawer or secondary pane.

Mobile: full-screen detail.

The detail surface must show the selected actuator identity and current station context and provide:

- current observed state;
- command state;
- capabilities;
- diagnostics;
- configuration summary;
- recent activity;
- Journal deep link when supported.

Closing Detail returns to the same Operations context without changing runtime state.

### 38A.11 Interlock Feedback

When a control is blocked by an interlock, clicking it must not dispatch a command.

The target control should remain understandable and explain the blocking condition locally.

Example:

```text
Không thể bật PH_UP
PH_DOWN đang hoạt động.
```

The UI must not imply that a blocked command entered the backend command lifecycle.

### 38A.12 Auto Mode Ownership Feedback

When an actuator command is blocked because `auto` owns control:

```text
Không thể điều khiển thủ công
Auto Mode đang kiểm soát thiết bị này.
```

This is an ownership restriction, not an actuator fault.

The UI must not suggest disabling Auto Mode as the only explanation when a more specific supported action exists.

### 38A.13 Reset Fault Interaction

Clicking `Khôi phục` opens the recovery confirmation when the authoritative safety contract requires confirmation.

After confirmation:

```text
Khôi phục
  |
  v
Pending
  |
  v
Refresh / observe FSM
  |
  +-- recovered
  +-- still faulted
  +-- unknown
```

The button result must not directly overwrite the FSM state.

### 38A.14 E-STOP Interaction

E-STOP is a page-level immediate safety action.

There is no generic confirmation dialog.

Invocation must immediately enter the safety/command lifecycle and provide strong feedback that the request is being processed.

```text
E-STOP
  |
  v
Requested / Pending
  |
  v
Authoritative system state
```

The UI must not claim that all physical actuators have stopped until the authoritative system state supports that conclusion.

### 38A.15 Loading Interaction

While required station/device state is loading:

- show loading placeholders for unavailable information;
- do not fabricate actuator state;
- disable consequential commands;
- preserve the selected station identity.

If only one subsection is loading, unaffected authoritative content may remain visible.

### 38A.16 Offline Interaction

When the station is offline/unavailable:

- disable commands that cannot be safely dispatched;
- preserve clearly marked last-known state when useful;
- explain that current physical state cannot be confirmed;
- do not queue commands silently unless a source-backed queue contract exists.

### 38A.17 Unknown Outcome Interaction

When a command was submitted but completion cannot be established, the UI enters `Unknown` rather than `Failed`.

Available next actions may include:

- refresh state;
- inspect actuator detail;
- retry when safe and permitted;
- recovery when supported.

Retry must not duplicate a potentially completed physical command without considering the authoritative safety/command policy.

### 38A.18 Permission Interaction

Viewer may inspect but cannot execute control commands.

When the user lacks execution permission:

- preserve readable state;
- visibly lock or disable command controls;
- explain the permission restriction;
- never rely on client-side hiding as the authorization mechanism.

### 38A.19 Station Switch During Pending Command

If the user changes station while a command is pending:

- detach the pending lifecycle from the newly selected station;
- never display the previous station's pending state as belonging to the new station;
- retain enough state to reconcile the previous command when returning to that station.

### 38A.20 Navigation with Unsaved Input

Runtime commands do not create persistent configuration drafts.

Where Operations contains an editable command form with unsaved parameters, navigating away must preserve or explicitly discard the draft according to the form contract. It must never silently dispatch the draft merely because the user navigated away.

---

## 38B. Control Authority and Safety Gate

Operations must model command ownership explicitly because Manual, Auto Mode, and Automation can all produce requests that may target physical resources.

The conceptual command path is:

```text
Manual / Auto Controller / Automation
                |
                v
      Control Authority / Arbitration
                |
                v
             Safety Gate
                |
                v
          Command Dispatch
                |
                v
       Authoritative State
```

The important product rule is that independent writers must not silently compete for the same actuator.

At minimum:

- Auto Mode owns closed-loop EC/pH regulation;
- Automation owns event/workflow orchestration;
- Manual control is a guarded intervention path;
- E-STOP and system protection have higher safety authority.

The exact backend priority algorithm remains an implementation concern, but the UI must expose ownership conflicts when the backend provides them.

---

## 39. Automation Surface Boundary

Automation is a separate station-scoped product surface. Operations only defines the boundary and navigation contract; the complete Automation functional specification lives in:

`docs/design-system/AUTOMATION-FUNCTIONAL-SPEC.md`

Operations must not duplicate Automation authoring semantics.

The boundary is:

```text
Operations
    = current runtime operation + manual control + safety/recovery

Automation
    = automatic logic definition + workflow orchestration

Auto Mode
    = runtime control-ownership state / closed-loop controller
```

Automation and Operations share selected station/device context but remain separate surfaces.

When navigating to Automation:

- preserve selected station/device;
- do not carry Operations actuator-control state as Automation UI state;
- do not reinterpret `Auto Mode` as the Automation page;
- use the Automation specification as the source of truth for Automation interactions.

---

## 40. Automation Navigation Contract

The Operations surface provides navigation to Automation, not an embedded Automation editor.

```text
Điều khiển
    |
    +-- [Tự động hóa] --> Automation
                              |
                              +-- List
                              +-- Detail
                              +-- Create/Edit
                              +-- Test/Dry Run
```

Navigation does not mutate actuator state, control mode, or Automation state.

The Automation page is responsible for its own header, cards, node inspectors, execution detail, conflict detail, and authoring interactions.

---

## 41. Automation Return Contract

Returning from Automation to Operations must restore the same selected station/device context.

Operations must refresh/reconcile authoritative runtime state when necessary rather than assuming that the state observed before navigation is still current.

Automation activity must not be rendered as a manual actuator command unless the authoritative runtime state records an actual command/action.

---

## 42. Loading State

During initial station/device loading:

- do not display fabricated actuator states;
- use LoadingState for content not yet available;
- disable consequential controls until required context exists.

The page must communicate whether it is loading station context, device state, recipe, or automation data when those loads are independently observable.

---

## 43. Unknown State

When physical state cannot be established:

```text
● Không xác định
```

Do not substitute `OFF` merely because no `ON` signal is available.

Unknown state must be treated as a meaningful guard condition for consequential actions.

---

## 44. Network Error

Network failure must be distinguishable from command rejection.

```text
Không thể gửi lệnh.
Kiểm tra kết nối và thử lại.
```

Do not claim that the physical device remained unchanged unless the authoritative state confirms that fact.

---

## 45. Rejection

When a command is rejected by a guard or backend policy:

```text
Lệnh bị từ chối
Lý do: <source-backed reason>
```

Where a specific safety interlock explains the rejection, show that explanation locally at the target actuator.

---

## 46. Timeout / Unknown Outcome

If command confirmation times out:

```text
Chưa nhận được xác nhận
Trạng thái thiết bị chưa thể kết luận.
```

The UI must not automatically convert timeout into failure of the physical action.

The next action may be refresh, retry, inspect state, or recovery according to the canonical flow contract.

---

## 47. Concurrent Commands

The same actuator should not accept a second conflicting command while a previous command remains unresolved unless the authoritative command policy explicitly permits it.

Example:

```text
PUMP_A
Lệnh đang chờ xác nhận
```

The UI should explain why the conflicting action is temporarily unavailable.

---

## 48. Station Switching During Pending Command

Changing station context must not destroy command lifecycle state belonging to the previous station.

When the user returns to that station, the UI must be able to reconcile the unresolved command with authoritative state.

Pending state from one station must never appear under another station.

---

## 49. Operations Interaction Map

```text
Selected Station
    |
    +-- Điều khiển
    |      |
    |      +-- Actuator Card
    |      |      +-- Toggle
    |      |      +-- PWM
    |      |      +-- Timed Run
    |      |      +-- Advanced Controls
    |      |      +-- Actuator Detail
    |      |
    |      +-- Reset Fault
    |      +-- E-STOP
    |
    +-- Tự động hóa
           +-- Navigate to separate Automation surface
```

---

## 50. Interaction Contract

| Element | Interaction | Result |
|---|---|---|
| Station selector | Select | Switch station context |
| Operational surface navigation | Click | Switch between Operations and Automation without changing station context |
| System condition | Inspect | State/recovery information |
| Recipe refresh | Click | Refetch recipe |
| Actuator card body | Click | Inspect only; no command |
| Toggle | Command | Guarded actuator command |
| PWM | Edit/commit | Guarded PWM command |
| Duration | Edit | Local pending value |
| Chạy | Command | Guarded timed/manual command |
| Advanced controls | Expand/collapse | Reveal technical controls |
| Actuator Detail | Click | Open actuator detail |
| Khôi phục | Command | Fault recovery request |
| E-STOP | Safety command | Immediate emergency action |
| Tự động hóa | Navigate | Open separate Automation surface |

---

## 51. Navigation Contract

Operations must support these context-preserving transitions:

```text
Dashboard Station Detail
    Operations
        Actuator Detail
        Journal
        Settings
        Automation

Operations
    Actuator Detail
        Settings
        Journal

Automation
    Flow Detail
    Config Explorer
    Operations
```

When navigating between Operations and Automation, preserve the selected station. The current frontend implementation stores the selected device identity in `useDeviceStore.deviceId`; this is the current canonical station/device context source until a richer station context model is introduced.

Automation must use that same context source. It must not independently select a device that differs from the selected Operations station.

---

## 52. Configuration Navigation Contract

Actuator Detail may expose:

`[Mở trong Settings]`

This must open the exact relevant configuration scope where supported.

It must not open a generic Settings landing page when the application can identify the actuator configuration location.

Operations runtime parameters remain local to Operations and must not silently become persistent Settings changes.

---

## 53. Journal Navigation Contract

Actuator Detail may expose:

`[Xem toàn bộ Journal]`

The navigation must preserve:

- station;
- device;
- actuator scope when supported.

Journal remains the canonical historical trace.

---

## 54. Role Matrix

| Capability | Admin | Operator | Viewer |
|---|---:|---:|---:|
| View Operations | Yes | Yes | Yes |
| View actuator state | Yes | Yes | Yes |
| Inspect Actuator Detail | Yes | Yes | Yes |
| Control actuator | Yes | Yes | No |
| Set PWM | Yes | Yes | No |
| Timed run | Yes | Yes | No |
| Emergency force | Yes | Yes | No |
| Reset fault | Yes | Yes | No |
| E-STOP | Yes | Yes | No |
Backend authorization remains authoritative.

---

## 55. Audit Requirements

Operationally consequential actions should be traceable.

Relevant events include:

- actuator start;
- actuator stop;
- timed run;
- PWM change;
- reset fault;
- E-STOP;
- emergency force;
- E-STOP and emergency safety actions.

Audit data should include, where available:

- actor;
- station;
- target;
- action;
- timestamp;
- requested parameters;
- result;
- failure, rejection, or timeout information.

---

## 56. Data Integrity Rules

Operations must not invent:

- physical command success from HTTP acceptance alone;
- physical confirmation from optimistic UI state;
- health claims not supported by observed diagnostics.

The source boundary is authoritative. Actuator Detail should use the runtime state, command lifecycle, FSM/fault, interlock, availability, permission, and configuration data already exposed by the system rather than introducing a speculative hardware-diagnostics model.

---

## 57. Desktop Wireframe Contract

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ OPERATIONS                                                                  │
│ [← Dashboard]   STATION C   ● Online   [Station C ▾]                        │
│                                                                             │
│ [ Điều khiển ]      [ Tự động hóa ]                                         │
├─────────────────────────────────────────────────────────────────────────────┤
│ [SYSTEM CONDITION / FAULT / AVAILABILITY]                                  │
│                                                                             │
│ [ACTIVE RECIPE SUMMARY]                                                     │
│                                                                             │
│ CHÂM DINH DƯỠNG VÀ pH                                                       │
│                                                                             │
│ ┌────────────────────┐  ┌────────────────────┐                             │
│ │ Bơm phân A         │  │ Bơm phân B         │                             │
│ │ state       [ON]   │  │ state       [OFF]  │                             │
│ │ PWM                │  │ PWM                │                             │
│ │ [Điều khiển]       │  │ [Điều khiển]       │                             │
│ │ [Chi tiết]         │  │ [Chi tiết]         │                             │
│ └────────────────────┘  └────────────────────┘                             │
│                                                                             │
│ ┌────────────────────┐  ┌────────────────────┐                             │
│ │ Bơm pH Up          │  │ Bơm pH Down        │                             │
│ │ state       [LOCK] │  │ state       [ON]   │                             │
│ │ interlock          │  │ PWM                │                             │
│ └────────────────────┘  └────────────────────┘                             │
│                                                                             │
│ CẤP & XẢ NƯỚC BỒN                                                           │
│ [Actuator cards]                                                            │
│                                                                             │
│ PHUN SƯƠNG VÀ TUẦN HOÀN                                                     │
│ [Actuator cards]                                                            │
│                                                                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                              [ E-STOP ]                                     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 58. Mobile Wireframe Contract

```text
┌───────────────────────────────┐
│ Operations                    │
│ [Station C ▾] ● Online        │
├───────────────────────────────┤
│ [Điều khiển] [Tự động hóa]    │
├───────────────────────────────┤
│ System condition              │
│ [status banner]               │
│                               │
│ Active Recipe                 │
│ [compact summary]             │
│                               │
│ Châm dinh dưỡng và pH         │
│ ┌───────────────────────────┐ │
│ │ Bơm phân A                │ │
│ │ Tắt                 [OFF] │ │
│ │ [Chi tiết]                │ │
│ └───────────────────────────┘ │
│                               │
│ [other actuator cards]        │
│                               │
├───────────────────────────────┤
│          [ E-STOP ]            │
└───────────────────────────────┘
```

Actuator Detail may occupy the full viewport on mobile.

---

## 59. Actuator Detail Wireframe Contract

Desktop:

```text
┌───────────────────────────────────────────────┬──────────────────────────┐
│ Operations                                    │ ACTUATOR DETAIL     [×] │
│                                               │                          │
│ [Actuator cards]                              │ Bơm phân A               │
│                                               │ ● RUNNING                │
│                                               │                          │
│                                               │ CURRENT STATE             │
│                                               │ IDENTITY & CAPABILITY     │
│                                               │ OPERATIONAL SUMMARY       │
│                                               │ DIAGNOSTICS               │
│                                               │ CONFIGURATION SUMMARY     │
│                                               │ RECENT ACTIVITY            │
│                                               │                          │
│                                               │ [Mở trong Settings]       │
│                                               │ [Xem Journal]             │
└───────────────────────────────────────────────┴──────────────────────────┘
```

The detail surface must not expose duplicate editable configuration forms.

---

## 60. Interaction Anti-Patterns

Never:

- toggle an actuator by clicking its card background;
- claim physical success immediately after request submission;
- hide a locked actuator without explaining why;
- mix one station's actuator state with another station's context;
- put the full configuration form into Actuator Detail;
- duplicate the full Journal inside Actuator Detail;
- infer physical health from online status alone;
- expose PWM-derived ml/min as measured flow when it is only calibration data;
- let runtime control silently modify persistent configuration;
- make E-STOP behave like a normal confirmation-gated command;
- show stale live-looking state while the station is unavailable.

---

## 61. Definition of Done

### Context

- selected station is always obvious;
- station switching is safe;
- stale state cannot cross station boundaries;
- navigation preserves station context.
- Operations and Automation consume the same selected station/device context.

### Control

- all source-backed actuators are represented;
- physical and command state are distinct;
- control actions have guards;
- PWM is limited to supported actuators;
- timed run validates duration;
- interlocks are visible;
- Actuator Detail exists as an entity detail surface.
- Actuator Detail shows only source-backed identity, capability, state, diagnostics/availability, configuration summary, and recent activity.

### Configuration

- Settings remains the single owner of persistent configuration;
- Actuator Detail only exposes configuration summary and deep links;
- runtime controls do not silently alter persistent configuration.

### History

- Actuator Detail may show recent activity;
- Journal remains the canonical historical trace;
- Journal deep links preserve station/device context.

### Safety

- E-STOP is immediately actionable;
- E-STOP result waits for authoritative state;
- emergency actions have explicit safety semantics;
- fault recovery does not imply successful recovery without state confirmation.

### Failure

- offline;
- unknown;
- permission denied;
- safety blocked;
- rejected;
- network error;
- timeout;
- pending

are distinguishable where relevant.

### Data integrity

- no fabricated telemetry;
- no speculative hardware diagnostics;
- no fabricated physical confirmation;
- temporary calibration is not presented as measured reality.

### Automation boundary

- Automation remains available as a separate station-scoped surface;
- Automation is not modeled as `Auto Mode`;
- Operations owns runtime control, not automation authoring;
- Automation owns flow/script definition and management;
- navigating between Operations and Automation does not silently change station/device context.

---

## 62. Locked Source Decisions

The following decisions are explicitly locked for the Operations UX contract:

### 62.1 HTTP acceptance is not physical confirmation

An accepted command request does not authorize the UI to claim that the actuator physically changed state.

The UI waits for authoritative observed state.

### 62.2 PWM-to-flow is not measured flow

Current PWM conversion data is treated as temporary calibration/estimation data, not measured dosing telemetry.

The UI must not label it as actual flow.

### 62.3 E-STOP is immediate

E-STOP is an immediate safety action. The interaction must not be delayed by a generic confirmation dialog.

Target visibility and post-command state feedback remain mandatory.

---

## 63. Canonical Operations Rule

Operations must always answer:

**Tôi đang điều khiển station nào?**

**Thiết bị hiện đang ở trạng thái nào?**

**Tôi có quyền điều khiển không?**

**Điều kiện an toàn nào đang chặn thao tác?**

**Lệnh đang ở lifecycle nào?**

**Thiết bị thực tế đã xác nhận thay đổi chưa?**

**Nếu không thành công, bước recovery là gì?**

---

## 64. Canonical Operations Formula

**Station Context, Current State, Guard, Explicit Action, Command Lifecycle, Physical Confirmation, Recovery.**

---

## 65. Source & Implementation Mapping

The current implementation evidence for this specification is primarily:

- `hydragrow-frontend/src/pages/ControlPanel.tsx` — current Operations/control surface;
- `hydragrow-frontend/src/components/control/AdvancedDeviceControl` — advanced actuator controls;
- `hydragrow-frontend/src/store/useDeviceStore.ts` — current selected device context;
- `hydragrow-backend` command/safety/FSM services — authoritative command and runtime behavior.

The current source-backed actuator inventory is:

`PUMP_A`, `PUMP_B`, `PH_UP`, `PH_DOWN`, `WATER_PUMP_IN`, `WATER_PUMP_OUT`, `OSAKA`, `MIST`, `MIX`.

Current source-backed control modes are `auto` and `manual`.

Current source-backed PWM support is limited to `PUMP_A`, `PUMP_B`, `PH_UP`, `PH_DOWN`, and `OSAKA`, with 20% to 100% and 5% step.

Current source-backed interlocks include the mutually exclusive pairs `PH_UP`/`PH_DOWN` and `WATER_PUMP_IN`/`WATER_PUMP_OUT`.

If implementation and this document disagree, source/runtime behavior must be investigated before either side is changed. The spec must not be used to invent unsupported hardware or backend capability.

---

## 66. Acceptance Checklist

### Page contract

- [ ] Operations has one explicit selected station/device context.
- [ ] Operations responsibility is limited to runtime operation, manual control, safety, and supported recovery.
- [ ] Automation remains a separate product surface.
- [ ] Auto Mode remains a control-ownership state, not an Automation page.

### Component and interaction contract

- [ ] Every interactive component has an explicit click/action result.
- [ ] Actuator card body is inspect-only.
- [ ] Control actions expose pending and result states.
- [ ] Actuator Detail has defined open/close behavior.
- [ ] Settings and Journal deep links preserve context.
- [ ] Station switching cannot leak previous-station state.

### State and command integrity

- [ ] Physical state is separate from command state.
- [ ] HTTP/backend acceptance is not physical confirmation.
- [ ] Timeout/unknown outcome is not silently converted to failure.
- [ ] Offline state disables unsupported consequential commands.
- [ ] Concurrent conflicting commands are guarded.

### Safety and ownership

- [ ] Permission guard is explicit.
- [ ] Auto Mode ownership is explicit.
- [ ] Source-backed interlocks are visible locally.
- [ ] E-STOP is immediately actionable.
- [ ] E-STOP completion waits for authoritative system state.
- [ ] Fault reset does not imply recovery without FSM/state confirmation.
- [ ] Control Authority / Arbitration is represented as the required architectural boundary before Safety Gate.

### Data integrity

- [ ] No unsupported actuator capability is rendered.
- [ ] PWM-to-flow conversion is not presented as measured flow.
- [ ] No speculative hardware diagnostics are shown.
- [ ] Last-known state is clearly distinguished from live state.

### Responsive behavior

- [ ] Desktop control and detail behavior is defined.
- [ ] Mobile control and detail behavior is defined.
- [ ] Critical actions remain reachable without hover.

### Verification

- [ ] Empty state defined where applicable.
- [ ] Loading state defined.
- [ ] Offline state defined.
- [ ] Error/rejection state defined.
- [ ] Unknown state defined.
- [ ] Audit requirements defined.
- [ ] Acceptance criteria can be converted into implementation tests.

This checklist is aligned with the common page-spec standard in `docs/design-system/PAGE-SPEC-CHECKLIST.md`.

---

## 67. Locked Operations Architecture

The Operations specification is now locked around these boundaries:

1. Operations is the runtime operational surface.
2. Automation is a separate station-scoped surface.
3. Auto Mode is runtime control ownership and closed-loop regulation, especially EC/pH.
4. Manual commands are guarded interventions, not an independent unguarded control path.
5. Independent command writers require Control Authority / Arbitration before the Safety Gate.
6. Physical state is authoritative and distinct from command acceptance.
7. E-STOP is immediate and its completion is verified from authoritative system state.
8. Persistent configuration belongs to Settings.
9. Full history belongs to Journal.
10. Actuator Detail is a secondary entity surface, not a new top-level page.

---

## 68. Component State Contract

Every consequential Operations component must expose a state model that is independent from the page-level state.

### Station context

States: `missing`, `loading`, `ready`, `switching`, `unavailable`, `error`.

- `missing`: no valid device/station context; consequential commands are unavailable.
- `loading`: context exists but authoritative runtime state is not yet ready.
- `ready`: current station/device context and required runtime state are available.
- `switching`: previous context must not remain visually authoritative for the new station.
- `unavailable/error`: show the reason when source-backed; do not fabricate live actuator state.

### Actuator control

States: `available`, `guarded`, `pending`, `accepted`, `rejected`, `error`, `timeout/unknown`, `observed`.

`accepted` means the control request was accepted by the backend. `observed` means authoritative runtime state supports the corresponding physical state. These states must not be collapsed.

### Actuator detail

States: `closed`, `opening`, `loading`, `ready`, `stale`, `error`, `unknown`.

Opening Detail never executes a command. Closing it returns to the same station and actuator context.

### Reset Fault

States: `available`, `confirming`, `pending`, `accepted`, `rejected`, `error`, `timeout/unknown`, `reconciling`, `recovered`, `still-faulted`.

`accepted` never directly replaces the authoritative FSM/fault state.

### E-STOP

States: `available`, `requested`, `pending`, `accepted`, `rejected`, `error`, `timeout/unknown`, `system-confirmed`.

E-STOP remains immediately actionable. `system-confirmed` requires authoritative system state; request acceptance alone is insufficient.

For every state, implementation must define visible presentation, allowed actions, transition trigger, recovery path, and semantic layer.

---

## 69. Mutation Lifecycle Contract

All consequential Operations actions follow:

```text
User intent
-> Guard / validation
-> Confirmation where required
-> Request dispatch
-> Pending
-> Backend acceptance / rejection
-> Runtime observation
-> Reconciliation
-> Completion or recovery
```

### ON/OFF, PWM, timed run, force-on

- identify device/station and actuator;
- validate availability, permission, mode, interlock, safety state, parameters, and concurrency;
- apply confirmation policy before dispatch;
- enter pending state;
- distinguish backend result from observed actuator state;
- reconcile authoritative runtime state;
- keep unresolved outcomes as `unknown`;
- prevent duplicate/conflicting commands unless the authoritative command policy permits them.

### Reset Fault

- validate station/device context and recovery eligibility;
- confirm when the authoritative safety contract requires it;
- dispatch recovery request;
- show pending;
- reconcile FSM/fault state;
- show recovered, still-faulted, rejected, error, or unknown based on authoritative evidence.

### E-STOP

- dispatch immediately without a generic confirmation dialog;
- show requested/pending feedback immediately;
- distinguish acceptance from system-wide stopped-state confirmation;
- reconcile authoritative safety/runtime state;
- if outcome is unknown, do not claim that all actuators stopped.

---

## 70. Confirmation / Reversibility Contract

| Action | Confirmation | Reversibility / cancellation | Result |
|---|---|---|---|
| ON/OFF | Product/safety policy; no ad-hoc confirmation | Reverse only through another supported command | Command + observed state |
| PWM | Required by current dangerous-command policy | Change again through supported command | Requested PWM + command result |
| Timed run | Required by current dangerous-command policy | No implied cancellation unless source supports it | Command result; physical state separately observed |
| Force-on | Confirm before dispatch | No implied rollback | Command result; physical state separately observed |
| Reset Fault | Confirm where required by safety contract | No implied undo | FSM/state reconciliation |
| E-STOP | Immediate; no generic confirmation | Recovery follows canonical safety path; no implied automatic undo | Safety request + authoritative system state |

Confirmation represents user intent only. It does not upgrade a request into persistence, observation, or physical confirmation.

---

## 71. Cross-Page Handoff Contract

Canonical flow:

```text
Operations origin
-> selected device/station context + intent
-> destination
-> destination state
-> action / inspection
-> return
-> runtime reconciliation
```

### Dashboard -> Operations

- preserve the selected `deviceId`;
- destination must not silently substitute another device;
- load authoritative runtime state before enabling consequential commands;
- returning to Dashboard may retain the same selection when supported.

### Operations -> Automation

- preserve the same selected device context;
- Automation owns workflow authoring/orchestration;
- Operations control state must not be copied into Automation as execution state;
- navigation does not mutate actuators or control mode.

### Operations -> Settings

- preserve device/actuator context when opening configuration;
- Settings owns persistent configuration;
- returning requires reconciliation if configuration can affect displayed operational state;
- runtime command parameters must not become persistent configuration silently.

### Operations -> Journal

- preserve device, station, and actuator scope when supported;
- Journal owns the full historical trace;
- returning does not imply that a pending command completed.

### Pending command / unsaved input

Navigation must not turn a pending command into success. If an editable runtime form has local unsaved parameters, the page must explicitly preserve or discard them according to its form contract and must never dispatch them merely because navigation occurred.

---

## 72. Ownership & Responsibility Matrix

| Capability / data | Data owner | Runtime/write owner | Operations role |
|---|---|---|---|
| Selected device context | Shared device store / application context | Context owner | Consume and preserve |
| Current actuator state | Runtime/device source | Runtime controller | Inspect |
| Manual actuator command | Control API/runtime | Operations guarded command path | Request |
| Auto regulation | Auto Controller | Auto Controller | Display ownership restriction |
| Automation workflows | Automation | Automation | Navigate/inspect boundary |
| Persistent configuration | Settings/config | Settings | Read summary/deep link |
| Full history | Journal/event source | Journal | Read preview/deep link |
| Safety gate / E-STOP | Safety/runtime source | Safety path | Invoke/observe according to canonical contract |

Reading shared state does not grant permission to mutate it.

---

## 73. Concurrency / Conflict Contract

Potential writers include manual Operations commands, Auto Controller, Automation, safety logic, and other clients.

- Operations must not assume last-write-wins or merge behavior.
- The authoritative backend/runtime policy decides whether conflicting commands are accepted, rejected, superseded, or otherwise handled.
- While a command is unresolved, a second conflicting command should remain unavailable unless the authoritative command policy explicitly permits it.
- A station switch must isolate pending lifecycle state by `deviceId`.
- A pending result for device A must never update the visible actuator state of device B.
- If the backend reports an ownership or conflict reason, show that reason at the affected control.
- Refresh/reconciliation must use the current device context.

`Implementation-required`: expose an explicit conflict/ownership result in the UI if the backend provides such a result but the current component does not surface it.

---

## 74. Source-to-UI Traceability

| UI behavior | Frontend source | Runtime/source boundary | Status |
|---|---|---|---|
| Selected device | `useDeviceStore.deviceId` | Application device context | Source-backed |
| Runtime actuator controls | `ControlPanel.tsx`, `AdvancedDeviceControl.tsx` | `useDeviceControl` -> `/api/devices/{deviceId}/control` | Source-backed |
| ON/OFF command | `AdvancedDeviceControl.handleToggle` | `togglePump` | Source-backed |
| PWM command | `AdvancedDeviceControl.applyPwm` | `setPwm` | Source-backed |
| Timed/force run | `AdvancedDeviceControl.handleAdvancedRun` | `forceOn` / control API | Source-backed |
| Interlock | `useDeviceControl.ensureInterlock` | `INTERLOCK_PAIRS` + Tauri safety check where applicable | Source-backed |
| Auto ownership guard | `ControlPanel`, `AdvancedDeviceControl` | `settings.control_mode` | Source-backed |
| Reset Fault | `ControlPanel` | `useDeviceControl.resetFault` | Source-backed |
| E-STOP | `useDeviceControl.emergencyStop` | Control API; page placement/UX integration | Partially source-backed |
| Station selector in Operations | Spec target; current ControlPanel has no selector | `useDeviceStore` only provides selected device identity | Implementation-required |
| Actuator Detail drawer/full-screen | Spec target | No verified current implementation in inspected Operations sources | Implementation-required |
| Journal/Settings deep-link actions | Spec target | Destination contracts required | Implementation-required |

Important current-source caveats:

- `ControlPanel.tsx` currently derives online state from `deviceStatus` and renders a disconnected banner only when controller status is known; this is not a general telemetry-freshness guarantee.
- `AdvancedDeviceControl` clears its local toggle-pending state after observed status matches or after a local timeout. The spec must not interpret that local timeout as physical failure.
- `useDeviceControl` returns `true` after an HTTP success and labels the command `accepted`; this is not physical confirmation.
- The current PWM display contains a PWM-to-ml/min approximation. This must remain explicitly non-measured calibration/estimation data.
- The current ControlPanel does not itself render E-STOP; the page-level E-STOP requirement therefore remains an implementation integration item unless provided by an enclosing surface.

---

## 75. Acceptance Criteria

### AC-01 Context integrity

Given device A is selected
When Operations opens
Then Operations displays A's authoritative runtime state
And no actuator data from another device is shown.

### AC-02 Station switch

Given device A is selected
When the user switches to device B
Then B becomes the active context
And A's pending/observed state is not rendered as B's state.

### AC-03 Manual ON/OFF

Given a supported actuator is controllable
When the user requests ON or OFF
Then guards run before dispatch
And the control enters pending
And backend acceptance is shown separately from observed physical state.

### AC-04 Interlock

Given `PH_DOWN` is running
When the user attempts to start `PH_UP`
Then no command is dispatched
And the UI identifies the interlock reason locally.

### AC-05 Auto Mode

Given control mode is `auto`
When the user attempts manual actuator control
Then the manual command is blocked
And current physical state remains visible
And the restriction is explained as ownership, not actuator fault.

### AC-06 PWM integrity

Given an actuator supports PWM
When the user commits a PWM value
Then the value is validated against the supported range
And the request state is distinct from persistent configuration
And any PWM-to-flow estimate is not labeled as measured flow.

### AC-07 Timed run

Given a timed-run request
When duration is invalid, unavailable, unsafe, unauthorized, or conflicting
Then dispatch is blocked with an appropriate reason
And no success state is shown.

### AC-08 Reset Fault

Given the system is faulted
When Reset Fault is confirmed and dispatched
Then the UI waits for authoritative FSM/runtime state
And does not declare recovery from request acceptance alone.

### AC-09 E-STOP

Given Operations is available
When the user invokes E-STOP
Then dispatch begins immediately without a generic confirmation dialog
And the UI shows the safety lifecycle
And stopped physical state is claimed only when authoritative state supports it.

### AC-10 Offline

Given the selected station is unavailable
When the user views Operations
Then unsupported consequential controls are disabled
And any last-known state is explicitly marked as non-live
And no command is silently queued without a source-backed queue contract.

### AC-11 Unknown command outcome

Given a command was submitted but confirmation cannot be established
When the timeout boundary is reached
Then the command is shown as unknown
And the UI offers only safe, supported next actions.

### AC-12 Pending station switch

Given a command for device A is pending
When the user switches to device B
Then B shows only B's state
And the A command remains attributable to A for later reconciliation.

### AC-13 Actuator Detail

Given an actuator is selected for inspection
When Detail opens
Then the selected actuator and device context are explicit
And opening Detail does not execute a command
And closing Detail restores the same Operations context.

### AC-14 Cross-page handoff

Given device A is selected
When the user opens Automation, Settings, or Journal
Then the destination receives/resolves A's context
And returning to Operations reconciles affected runtime/configuration state where applicable.

### AC-15 Permission

Given the user can inspect but cannot execute commands
When Operations renders
Then state remains readable
And command controls are visibly unavailable
And client-side hiding is not treated as authorization.

### AC-16 Duplicate/conflicting command

Given an actuator command is unresolved
When the user attempts a conflicting command
Then the second command is blocked unless the authoritative command policy permits it
And the reason is visible.

### AC-17 Physical confirmation boundary

Given the control API returns HTTP success
When the UI renders the result
Then it may show backend acceptance
But it must not claim physical actuator change until authoritative observed state supports that conclusion.

### AC-18 Navigation during pending/dirty state

Given a command is pending or runtime input is unsaved
When the user navigates away
Then navigation does not imply command completion
And unsaved input is preserved or explicitly discarded according to the applicable form contract.

---

## 76. Checklist Compliance Audit

| Checklist area | Status |
|---|---|
| 1 Page Identity / Responsibility | Covered |
| 2 Source of Truth | Covered; current integration gaps explicit |
| 3 Selected Context | Covered; selector integration remains implementation-required |
| 4 Information Architecture | Covered |
| 5 Component Inventory | Covered |
| 6 Interaction / Outcome | Covered |
| 6.2 Component State Contract | Covered |
| 7 Data Contract | Covered |
| 8 State Model | Covered |
| 9 Action / Command | Covered |
| 9.1 Mutation Lifecycle | Covered |
| 9.2 Confirmation / Reversibility | Covered |
| 10 Safety / Control Authority | Covered; backend arbitration implementation remains bounded |
| 11 Permission | Covered |
| 12 Navigation | Covered |
| 12.1 Cross-Page Handoff | Covered |
| 13 Desktop / Mobile | Covered |
| 14 Forms / Validation | Covered |
| 15 History / Audit | Covered |
| 16 Error / Unknown | Covered |
| 17 Cross-Page ownership | Covered |
| 18 Accessibility | Covered |
| 19 Observability | Covered |
| 20 Wireframe | Covered |
| 21 Acceptance Criteria | Covered |
| 22 Implementation Mapping | Covered |
| 22.1 Source-to-UI Traceability | Covered |
| 22.2 Ownership Matrix | Covered |
| 22.3 Concurrency / Conflict | Covered with implementation boundary |
| 23 Anti-Pattern | Covered |
| 24 Definition of Done | Covered |

### Implementation blockers before claiming full conformance

1. Operations station selector/context-switch UI is not present in the inspected `ControlPanel.tsx`; the existing shared `deviceId` context is source-backed, but the page-local selector is implementation-required.
2. Page-level E-STOP exists at the control-hook boundary but is not rendered by the inspected `ControlPanel.tsx`; integration must be verified or implemented.
3. Actuator Detail, Journal deep-link, and exact Settings-scope navigation are specified but not source-backed in the inspected Operations components.
4. The current local command timeout/pending behavior must be reconciled with authoritative runtime observation; local timeout is not physical failure.
5. Backend Control Authority / Arbitration remains an architectural boundary, not a claim that a complete priority algorithm is already implemented.

### Conclusion

**Operations functional spec is now checklist-complete at the documentation-contract level and is suitable as the implementation acceptance gate.** The implementation blockers above remain explicitly separated from the product contract and must not be silently treated as already implemented.
