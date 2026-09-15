# HYDRAGROW — DASHBOARD / OVERVIEW FUNCTIONAL SPECIFICATION

## 0. Purpose

This document is the functional source of truth for the Dashboard / Overview wireframe.

The Dashboard is the primary **monitoring surface** for HYDRAGROW. It must support two related tasks:

1. **All Stations Overview:** identify which accessible station needs attention.
2. **Selected Station Detail:** understand the current state and supported telemetry of one station.

The Dashboard is not the full actuator-control, cultivation, fleet-administration, configuration, analytics, or journal surface.

Related specifications:

- `UX-RULES.md`
- `STATE-TRANSITION-SPEC.md`
- `COMPONENT-CONTRACT.md`
- `PAGE-CONTRACT.md`
- `DESIGN-TOKENS-VISUAL-SPEC.md`
- `UX-FLOW-TASK-FLOW-SPEC.md`

---

## 1. Core Principle

Dashboard means:

**Monitor first, inspect second, operate third.**

When multiple stations are accessible, the default monitoring experience must let the user scan the stations before opening one station in detail.

The interface must never sacrifice data integrity to make the dashboard visually richer.

---

## 2. Two-Level Information Architecture

### 2.1 All Stations Overview

Primary question:

> Which station needs attention right now?

The overview presents compact summaries for accessible stations.

### 2.2 Selected Station Detail

Primary question:

> What is happening in this station, and what should I do next?

The detail view uses the current single-device Dashboard information model.

### 2.3 Single-station account

When the user has only one accessible station, the product may open directly into detail. The station identity must still be persistent and obvious.

### 2.4 Navigation model

```text
All Stations Overview
    Station Card
        Selected Station Detail
            Operations / Cultivation / Journal / Settings
```

The selected station must remain identifiable through station-specific navigation.

---

## 3. Current Source Boundary

### 3.1 Single-device Dashboard

Current implementation:

`hydragrow-frontend/src/pages/Dashboard.tsx`

The current page consumes:

| Domain | Supported data |
|---|---|
| Device | `deviceId` |
| Connection | `deviceStatus.is_online`, `last_seen` |
| System | `fsmState` |
| Sensor connection | `isSensorOnline` |
| Controller health | health score / diagnostics health score |
| Telemetry | `ec`, `ph`, `temp`, `water_level` |
| Sensor errors | `err_ec`, `err_ph`, `err_temp`, `err_water` |
| Actuators | `pump_status.*` |
| Configuration | EC, pH, temperature and water-level targets/limits used by the page |
| Tank alerts | four `tank_*_low` flags |
| Dosing summary | EC count, pH count, latest pH dosing timestamp |
| User | display name or email-derived name |
| Notification | FCM permission state |
| Fault | fault code and fault guide |
| Onboarding | onboarding visibility state |

### 3.2 Existing multi-station source

Current Fleet source:

`hydragrow-frontend/src/hooks/useFleetStatus.ts`

Current device fields:

| Field | Meaning |
|---|---|
| `device_id` | Device identity |
| `label` | User-facing station label when available |
| `is_online` | Connection state |
| `firmware_version` | Firmware version |
| `last_seen` | Last known device contact |

Current `/fleet/summary` fields used by Fleet:

| Field | Meaning |
|---|---|
| `device_id` | Device identity |
| `crop` | Crop label when available |
| `ec_latest` | Latest EC summary value when available |
| `ph_latest` | Latest pH summary value when available |
| `warning_count` | Warning count |

The existing `FleetStationCard` already establishes a practical baseline for multi-station monitoring.

Important implementation boundary: `useFleetStatus`, `FleetView`, and `FleetStationCard` are current Fleet implementation, not current Dashboard implementation. `Dashboard.tsx` does not currently render these Fleet overview components. Therefore the All Stations Overview described in Part I is a target Dashboard capability and remains `Implementation-required` until integrated into Dashboard.

---

## 4. Data Integrity Rule

A wireframe may only show a value when a source contract provides that value.

A value available for Selected Station Detail must not be assumed to be available for every station in All Stations Overview.

If a summary value is unavailable, show an unavailable state such as `—` rather than inventing a value.

---

## 5. Data That Must Not Be Invented

Unless an explicit contract is added, do not assume:

- humidity;
- CO2;
- light level;
- pressure;
- flow rate;
- separate water temperature;
- nutrient volume;
- tank percentage or volume;
- actuator runtime;
- actuator power;
- dosage volume;
- synthetic fleet health score;
- uptime summary;
- arbitrary trend history;
- per-reading freshness text;
- additional sensor units;
- station location;
- network address;
- growth stage;
- physical health claims not supported by source data.

---

## 6. Dashboard Mode Contract

### All Stations Overview

Optimized for:

- scanning;
- comparison;
- anomaly detection;
- station selection.

### Selected Station Detail

Optimized for:

- context;
- state inspection;
- telemetry inspection;
- next action;
- transition to operational tasks.

The two modes must feel like one product while having clearly different information density.

---

## 7. Global Station Context

Station identity is a first-class part of the Dashboard.

When a station is selected, the UI must expose at least:

- station label when available;
- device ID;
- online/offline state;
- current page context.

Recommended header pattern:

```text
Dashboard

[← Tất cả trạm]   Station C   ● Online
                    ID: device_xxx
```

The user must never need to infer the selected station from telemetry values.

---

# PART I — ALL STATIONS OVERVIEW

## 8. Overview Functional Regions

The All Stations Overview contains:

1. Dashboard header;
2. fleet summary strip;
3. monitoring controls;
4. station grid/list;
5. station summary cards;
6. global feedback/error state;
7. empty/loading states;
8. optional supporting content.

It must not render the full detailed telemetry dashboard for every station.

---

## 9. Region A1 — Dashboard Header

### Purpose

Establish the page as the system monitoring surface.

### Content

- Dashboard title;
- concise monitoring description;
- station count when available;
- refresh action;
- add-station action when permitted.

The header should not contain detailed actuator controls.

---

## 10. Region A2 — Fleet Summary Strip

### Purpose

Give the user an immediate fleet-level scan before individual cards.

### Currently supportable values

From the existing Fleet source:

- total accessible stations;
- number of stations reported online;
- warning count available from `/fleet/summary`.

Example structure:

```text
6 trạm       5 trực tuyến       2 cần chú ý
```

The numbers must always come from actual returned data.

### Rule

Do not create a fleet health score by averaging station values unless the product defines that metric explicitly.

---

## 11. Region A3 — Monitoring Controls

The current Fleet implementation supports:

- `Tất cả`;
- `Cần chú ý`;
- refresh;
- add new station.

The Dashboard overview should retain the monitoring-oriented parts of this behavior.

Current-source status: these controls are implemented in `FleetView`, not `Dashboard.tsx`. Dashboard does not currently contain the fleet filter, fleet refresh, or add-station controls.

The controls are not a replacement for Fleet administration.

---

## 12. Region A4 — Station Grid

The station grid is the primary monitoring surface when multiple stations are accessible.

Each accessible station receives one compact summary card.

### Ordering

The current Fleet implementation sorts warning-first:

1. higher warning count first;
2. online stations before offline stations when warning counts tie.

Dashboard should retain this ordering unless a stronger product requirement replaces it.

### Rationale

The monitoring surface should surface abnormal stations before normal stations.

---

## 13. Region A5 — Station Summary Card

The card must answer:

1. Which station is this?
2. Is it online?
3. Does it need attention?
4. Which important summary metrics are currently available?
5. How do I inspect it?

### Required identity

- `label` when available;
- fallback to `device_id`;
- device ID as supporting identity;
- connection state.

### Current multi-station summary metrics

The existing `/fleet/summary` provides:

- EC latest;
- pH latest;
- warning count;
- crop label when available.

These are the canonical candidates for the current multi-station card.

### Important boundary

Do not display temperature or water level for every station unless the fleet summary contract is extended to provide them.

The fact that these metrics exist in Selected Station Detail does not make them available to the fleet overview.

---

## 14. Station Card Information Hierarchy

Recommended structure:

```text
┌──────────────────────────────────┐
│ ● Station C                 ! 2  │
│ device_xxx       🌱 Lettuce      │
│                                  │
│ EC              pH               │
│ 1,240           6.1              │
│                                  │
│ ⚠ 2 cảnh báo                     │
│                                  │
│                         Xem  ›    │
└──────────────────────────────────┘
```

Priority:

1. station identity;
2. connection state;
3. warning state;
4. primary available metrics;
5. crop context when available;
6. detail entry.

The values in the example are placeholders for layout only and must never be copied as real sensor values.

---

## 15. Station Card — EC

### Source

`/fleet/summary.ec_latest`

### Presentation

Show the latest EC value when available.

The current Fleet card does not display an EC unit.

The single-station Dashboard currently displays `ppm`.

This discrepancy must not be silently reconciled by the wireframe.

Until the domain unit is explicitly approved, preserve the source component's presentation or omit the unit.

---

## 16. Station Card — pH

### Source

`/fleet/summary.ph_latest`

Show numeric pH when available.

Do not add a unit.

When unavailable, show `—` or another approved unavailable representation.

---

## 17. Station Card — Warning Count

### Source

`/fleet/summary.warning_count`

When greater than zero, show a visible attention indicator and count.

The count supports:

- sorting;
- filtering;
- visual prioritization.

The count does not provide warning detail by itself.

Do not fabricate a warning description from the count.

---

## 18. Station Card — Crop

### Source

`/fleet/summary.crop`

Show only when a crop label exists.

Crop is supporting context only.

Do not infer growth stage, yield, health, or cultivation state from crop name.

---

## 19. Station Card — Connection

### Online

Show an explicit online state.

### Offline

Show offline state and `last_seen` when available.

The existing Fleet card uses relative last-seen wording.

### Rule

Offline stations must not visually resemble healthy online stations.

Online does not mean healthy.

---

## 20. Station Selection

Selecting a station card must:

1. identify the selected station;
2. preserve station identity;
3. load station-specific authoritative data;
4. enter Selected Station Detail;
5. expose selected station identity in the detail header.

The user must never need to remember which card they selected.

Current-source status: `Implementation-required` on Dashboard. The existing selection flow is `FleetView -> setDeviceId(deviceId) -> navigate('/')`; it is not implemented by `Dashboard.tsx` itself.

---

# PART II — SELECTED STATION DETAIL

## 21. Detail Functional Regions

Selected Station Detail contains:

1. station context header;
2. system state / next action;
3. health score;
4. sensor connectivity;
5. tank alerts;
6. quick actions;
7. onboarding when applicable;
8. four detailed telemetry cards;
9. active devices;
10. dosing summary;
11. persistent E-STOP.

This is the detailed single-device Dashboard model supported by the current `Dashboard.tsx` source.

---

## 22. Region B1 — Station Context Header

The header must show:

```text
Dashboard

[← Tất cả trạm]   Station C
                    ● Trạm Online
                    ID: device_xxx
```

Control mode may be shown when available.

The return control must make it obvious that the user is leaving station detail and returning to fleet monitoring.

---

## 23. Region B2 — System State / Next Action

### Inputs

- `fsmState`;
- `isOnline`;
- `isSensorOnline`;
- fault code and fault guide;
- notification permission.

### Current priority

1. device offline;
2. sensor node offline;
3. actionable fault;
4. notification permission not granted;
5. no intervention required.

### Current action text

- Offline: `Kiểm tra nguồn Wi-Fi trạm điều khiển.`
- Sensor offline: `Đang mất tín hiệu cảm biến. Kiểm tra nguồn node cảm biến.`
- Fault: use `faultGuide.action`.
- Notification permission: `Bật thông báo để nhận cảnh báo tức thì.`
- Otherwise: `Không cần thao tác. Tiếp tục theo dõi.`

The source of any recommended action must remain traceable.

---

## 24. Region B3 — Health Score

### Source

`controllerHealth.health_score_percent`, with diagnostics fallback.

### Current semantics

- 80 or above: good;
- 50–79: warning;
- below 50: critical;
- no usable score under the component contract: offline.

Health score is a summary indicator only.

It must not be presented as proof that every sensor, actuator, or physical process is healthy.

---

## 25. Region B4 — Sensor Connectivity

### Source

`isSensorOnline`.

### Current presentation

Online:

- `Tốt`;
- `Đang đo`.

Offline:

- `Mất`;
- `Cần kiểm tra`.

This is aggregate sensor-node connectivity, not proof that every individual sensor is healthy.

---

## 26. Region B5 — Tank Alerts

Supported flags:

| Flag | Label |
|---|---|
| `tank_a_low` | Cạn Dinh Dưỡng A |
| `tank_b_low` | Cạn Dinh Dưỡng B |
| `tank_ph_up_low` | Cạn pH Up |
| `tank_ph_down_low` | Cạn pH Down |

Show the region only when at least one flag is true.

Do not invent tank percentage, volume, or remaining-time information.

---

## 27. Region B6 — Quick Actions

Current actions:

1. `Tưới ngay`;
2. `Châm dinh dưỡng`;
3. `Tạm dừng bơm`;
4. `Xem cảnh báo`.

Current behavior:

| Action | Current behavior |
|---|---|
| Tưới ngay | `forceOn('WATER_PUMP_IN', 30)` |
| Châm dinh dưỡng | `/operations` |
| Tạm dừng bơm | `/operations` |
| Xem cảnh báo | `/journal` |

`Tưới ngay` is a physical command. It must follow the command lifecycle, guards, pending state, result, and physical-state verification defined by B and F.

A button click is never proof that watering physically occurred.

---

## 28. Region B7 — Onboarding

Show `OnboardingWizard` only when `shouldShowOnboarding` is true.

Onboarding is supporting setup guidance and must not outrank safety, fault, availability, or critical telemetry.

---

## 29. Region B8 — Live Telemetry

The detailed Dashboard currently has exactly four telemetry metrics:

1. EC;
2. pH;
3. Temperature;
4. Water Level.

No additional telemetry is implied.

---

## 30. Telemetry — EC

Inputs:

- `sensorData.ec`;
- `sensorData.err_ec`;
- EC target/tolerance and limits.

Current presentation:

- title: `Dinh dưỡng EC`;
- normal unit: `ppm`;
- error value: `Bảo trì`;
- description: `Nồng độ dinh dưỡng bồn chứa.`;
- error description: `Lỗi cảm biến EC.`.

Do not reinterpret the unit independently of the domain contract.

---

## 31. Telemetry — pH

Inputs:

- `sensorData.ph`;
- `sensorData.err_ph`;
- pH target/tolerance and limits.

Current presentation:

- title: `Độ pH`;
- no displayed unit;
- error value: `Lỗi`;
- description: `Độ cân bằng axit/kiềm.`;
- error description: `Cần hiệu chuẩn pH.`.

---

## 32. Telemetry — Temperature

Inputs:

- `sensorData.temp`;
- `sensorData.err_temp`;
- minimum and maximum temperature limits.

Current presentation:

- title: `Nhiệt độ`;
- normal unit: `°C`;
- error value: `Lỗi`;
- range: `An toàn <min>-<max>°C`;
- description: `Nhiệt độ dung dịch bồn chứa.`.

---

## 33. Telemetry — Water Level

Inputs:

- `sensorData.water_level`;
- `sensorData.err_water`;
- water-level target.

Current presentation:

- title: `Mực nước`;
- normal unit: `%`;
- error value: `Lỗi phao`;
- target: `Giữ quanh <target>%`;
- description: `Đảm bảo bơm không chạy khô.`;
- error description: `Kiểm tra phao siêu âm.`.

---

## 34. Telemetry Trust

Each detailed telemetry card must distinguish:

- actual value;
- unit when supported;
- target or limit when supported;
- sensor error/quality state.

`SensorData.time` exists in the model, but the current Dashboard does not render freshness text.

Do not invent `Live`, `5s ago`, or similar labels until freshness presentation is explicitly defined.

When the sensor node is offline, the UI must not imply that an old value is current.

---

## 35. Region B9 — Active Devices

### Source

`sensorData.pump_status`.

Supported keys currently include:

- `pump_a`;
- `pump_b`;
- `ph_up`;
- `ph_down`;
- `osaka_pump`;
- `mist_valve`;
- `mix_valve`;
- `water_pump_in`;
- `water_pump_out`.

Show entries currently reported as `true`.

If none are true:

`Không có bơm hoặc van nào đang chạy`

This is a summary, not a control panel.

Do not add runtime, power, flow, PWM, or actuator-health claims without source support.

---

## 36. Active Device Semantics

`pump_status.* === true` means the source reports the device as active/running.

It does not independently prove:

- command success;
- actual fluid flow;
- actuator health;
- expected physical output.

The UI must preserve the distinction between requested state, command state, and observed state.

---

## 37. Region B10 — Dosing Summary

Source:

`useSystemHealthSummary(deviceId)`.

Inputs:

- `ec_dosing_count`;
- `ph_dosing_count`;
- `latest_ph_dosing_at`.

Calculation:

`dosingTotalCount = ec_dosing_count + ph_dosing_count`.

`water_operation_count` is not part of dosing count.

Presentation:

- with timestamp: `<count> lần · lần cuối <time>`;
- without timestamp: `Chưa ghi nhận lần châm nào hôm nay`.

Do not add dose volume, ml totals, average dose, or success rate.

---

## 38. Region B11 — Emergency Stop

`EmergencyStopButton` currently uses `variant="floating"`.

The current implementation still includes `EmergencyStopConfirmDialog` before `emergencyStop()`.

The canonical A/F safety contract states that E-STOP is an immediate safety action and must not use a generic confirmation step that delays execution. The Dashboard functional contract therefore treats the current confirmation dialog as an implementation gap, not an open product decision.

An active E-STOP condition must remain visible across navigation and restrict normal controls according to the safety contract.

---

# PART III — STATES AND FLOWS

## 39. All Stations Loading

When fleet data is loading and no usable station data exists:

- show station-card skeletons;
- do not invent names;
- do not invent metrics;
- do not invent status.

If previously authoritative data exists, refresh behavior may preserve it while indicating that refresh is in progress.

---

## 40. No Stations

If the user has no accessible stations:

```text
Chưa có trạm nào

Tài khoản của bạn chưa liên kết trạm thủy canh nào.

[Liên kết thiết bị mới]
```

This is a missing fleet state, not an offline station.

---

## 41. Warning Filter Empty

If `Cần chú ý` is selected and no station matches:

```text
Tất cả trạm đang ổn định

Không có trạm nào đang ghi nhận cảnh báo.

[Xem tất cả trạm]
```

The wording must follow the actual warning source semantics.

---

## 42. Fleet Retrieval Error

If fleet retrieval fails:

- explain that the station list could not be loaded;
- preserve retry/refresh;
- do not represent the error as a station fault;
- do not represent missing data as healthy data.

A fleet API failure and a station being offline are different states.

---

## 43. Selected Station Loading

When station-specific data is loading:

- preserve selected station identity;
- show detail loading state;
- do not reuse another station's telemetry;
- do not fabricate current state.

---

## 44. Selected Station Offline

If a selected station becomes unavailable:

1. preserve station identity;
2. show offline/unavailable state;
3. stop treating stale telemetry as current;
4. restrict unsafe actions;
5. provide a recovery or return-to-overview path.

---

## 45. Station Switching

A selected station should be switchable without forcing the user back to Fleet.

Recommended control:

```text
[ Station C ▾ ]
```

The selector should expose:

- station label;
- device ID;
- online/offline state;
- warning indicator when available.

The selector is a context control, not a fleet administration surface.

---

## 46. Context Persistence

When moving from Selected Station Detail to:

- Operations;
- Cultivation;
- Journal;
- Settings;

the selected station context must remain identifiable.

The user should be able to tell:

> I am operating Station C.

The visual context is required; storing an internal ID alone is not sufficient.

---

## 47. All Stations to Detail Flow

```text
Open Dashboard
Inspect fleet summary
Inspect station cards
Identify station needing attention
Select station
Load station-specific state
Show selected station context
Inspect detailed state and telemetry
Choose next task
```

The selected station must remain explicit throughout the flow.

---

## 48. Detail to Operations Flow

```text
Selected Station Detail
Inspect state
Identify operational issue
Select Operations
Preserve selected station
Inspect control state
Perform guarded operation
Return with station context preserved
```

Dashboard should not duplicate the complete Operations page.

---

# PART IV — LAYOUT

## 49. Desktop — All Stations Overview

Recommended structure:

```text
Application Shell
└── Dashboard
    ├── Dashboard Header
    ├── Fleet Summary Strip
    ├── Monitoring Controls
    ├── Station Grid
    │   ├── Station Card
    │   ├── Station Card
    │   └── ...
    └── Global / Supporting Feedback
```

Three columns are appropriate at typical desktop widths when card content remains readable.

The grid must scale to additional stations without making cards excessively wide.

---

## 50. Desktop — Selected Station Detail

```text
Application Shell
└── Dashboard
    ├── Station Context Header
    ├── System State / Next Action
    ├── Health + Sensor Summary
    ├── Tank Alerts when applicable
    ├── Quick Actions
    ├── Onboarding when applicable
    ├── Live Telemetry: EC / pH / Temperature / Water Level
    ├── Active Devices
    ├── Dosing Summary
    └── Floating E-STOP
```

---

## 51. Mobile — All Stations

Preserve:

1. Dashboard identity;
2. fleet summary;
3. warning filter;
4. station cards;
5. refresh/add actions when permitted.

Station cards become a readable single-column list or compact list.

Each card must still expose:

- station identity;
- connection state;
- warning state;
- primary available metrics.

---

## 52. Mobile — Selected Station

Preserve:

1. selected station context;
2. system state / next action;
3. health and sensor connectivity;
4. critical warnings;
5. quick actions;
6. onboarding when applicable;
7. four telemetry cards;
8. active devices;
9. dosing summary;
10. E-STOP.

The selected station context may be sticky or condensed, but must remain identifiable.

---

# PART V — SYSTEM BOUNDARIES

## 53. Dashboard vs Fleet

### Dashboard

Optimized for:

- continuous monitoring;
- anomaly detection;
- quick station selection;
- operational awareness.

### Fleet

Optimized for:

- station administration;
- pairing;
- firmware/device metadata;
- fleet management.

Dashboard may reuse fleet summary data but should not become a duplicate Fleet administration page.

---

## 54. Dashboard vs Operations

Dashboard may:

- show active devices;
- show warnings;
- expose limited quick actions;
- link to Operations.

Dashboard must not become:

- a complete actuator matrix;
- a PWM editor;
- a dosing configuration editor;
- a cultivation scheduler;
- a full event history viewer.

---

## 55. Fleet Summary Contract Gap

The current multi-station source supports a useful monitoring card with:

- station identity;
- online/offline;
- last seen;
- EC latest;
- pH latest;
- warning count;
- crop label.

It does not currently expose the complete detailed station state used by `Dashboard.tsx`, including:

- temperature;
- water level;
- individual sensor errors;
- sensor-node connectivity;
- controller health score;
- detailed FSM state;
- active actuator summary;
- tank alert flags;
- dosing summary.

Do not solve this gap by fabricating client-side values.

---

## 56. Proposed Future Fleet Summary Extension

This is a proposal, not a claim about current source data.

If product requirements demand richer station cards, the fleet summary contract may explicitly expose fields such as:

```text
device_id
label
is_online
last_seen
warning_count
ec_latest
ph_latest
temp_latest
water_level_latest
sensor_online
system_state
```

Each new field must define semantics, freshness, unit, and authority before it appears in a wireframe.

---

## 57. Freshness Rules

`last_seen` describes device communication.

It must not automatically be described as telemetry freshness.

A station being online does not prove that every telemetry value is current.

If independent telemetry freshness is needed, the source contract must provide it explicitly.

---

## 58. Health vs Connectivity

The Dashboard must distinguish:

- online;
- healthy;
- telemetry trustworthy.

The current multi-station source does not provide a fleet-level controller health score.

Therefore no synthetic fleet health percentage should be displayed.

---

## 59. Warning Semantics

`warning_count` supports:

- sorting;
- filtering;
- attention indication.

It does not automatically support:

- warning category;
- severity;
- warning description;
- recommended recovery action.

Those details require additional source data.

---

# PART VI — ACCEPTANCE

## 60. All Stations Acceptance Criteria

### Monitoring

- Multiple accessible stations can be scanned from Dashboard.
- Stations needing attention are visually prioritized.
- Station identity is immediately visible.
- Online/offline state is explicit.
- Warning count is visible when available.
- EC and pH are shown when summary data provides them.
- Missing values are represented as unavailable.

### Data integrity

- No unsupported metric is fabricated.
- No unsupported unit is invented.
- No synthetic health score is shown.
- `last_seen` is not mislabeled as telemetry freshness.

### Interaction

- A station card can be selected.
- Selection enters station detail.
- Selected station identity remains visible.
- User can return to All Stations.

---

## 61. Selected Station Acceptance Criteria

### Context

- Selected station is always identifiable.
- Device ID is available for disambiguation.
- Online/offline state is explicit.
- Station context remains visible through station-specific navigation.

### State

- FSM state is authoritative.
- Faults are visible.
- Sensor-node loss is distinct from device loss.

### Telemetry

- Exactly four source-backed detailed metrics are represented.
- No metric is fabricated.
- Current unit contracts are preserved.
- Targets/limits are distinct from actual values.
- Sensor errors are explicit.
- Freshness is not fabricated.

### Physical control

- Physical actions follow command lifecycle rules.
- Click does not equal physical confirmation.
- Unsafe actions are guarded.
- E-STOP remains accessible.

---

## 62. Station Context Acceptance Criteria

- User always knows whether they are monitoring all stations or one selected station.
- Selected station label is visible when available.
- Device ID is available for disambiguation.
- Online/offline state is visible.
- Station context survives navigation to station-specific pages.
- Switching stations does not silently reuse another station's telemetry.
- Returning to All Stations restores the fleet monitoring context.

---

## 63. Anti-Patterns

Do not:

- make Dashboard single-station-only for multi-station accounts;
- force users to open Fleet merely to discover which station has a warning;
- show a complete detailed telemetry dashboard for every station;
- invent temperature or water-level values for fleet cards;
- show synthetic health scores;
- label every online station as healthy;
- hide the selected station identity;
- rely on the user remembering which station they selected;
- make station cards look like control panels;
- use warning count as warning detail;
- treat `last_seen` as telemetry freshness;
- copy stale fleet values into detailed station state without reconciliation;
- add metrics solely to make the wireframe look richer.

---

## 64. Open Product Decisions

These decisions remain explicit:

1. Whether multi-station accounts always open Dashboard in All Stations Overview.
2. Whether single-station accounts open directly into Selected Station Detail.
3. Whether selected station context is represented in the URL or another persistent navigation state.
4. Whether the fleet summary should be extended beyond EC, pH, crop, and warning count.
5. Whether EC `ppm` is the final domain-approved unit.
6. The final guard and feedback contract for `Tưới ngay` within the canonical command contract.
7. Whether `SensorData.time` becomes a visible freshness indicator.

---

## 65. Canonical Dashboard Rule

The Dashboard is the primary HYDRAGROW **monitoring surface**.

When multiple stations are accessible, it must first help the user determine **which station needs attention**.

When one station is selected, it must then help the user determine **what is happening in that station and what can safely be done next**.

Every visible value, label, status, action, and unit must have a traceable source in the current product contract.

The canonical model is:

**All Stations, Select Station, Confirm Context, Inspect State, Inspect Supported Telemetry, Take the Appropriate Next Task.**

---

## 66. Interaction and Drill-down Contract

This section defines what happens when a user interacts with a Dashboard region. It complements the visual and data contracts above.

Every interactive element must define:

1. interaction target;
2. interaction type;
3. visible response;
4. destination or overlay, if any;
5. station context behavior;
6. return behavior;
7. permission and safety guard, when applicable.

The Dashboard must not make a region appear actionable unless its interaction contract is defined.

### 66.1 Interaction classes

| Class | Meaning | Typical result |
|---|---|---|
| Read-only | Informational content | No interaction |
| Inspect | User requests more information | Popover, dialog, drawer, or inline expansion |
| Navigate | User continues a task elsewhere | Destination page with station context |
| Filter | User changes monitoring scope | Overview list is filtered in place |
| Command | User requests physical/system action | Guarded command lifecycle |

### 66.2 Click does not imply command

A telemetry card, status card, active-device indicator, or warning summary must not be interpreted as a physical command merely because it is interactive.

Physical commands are only available through explicit command controls and must follow `STATE-TRANSITION-SPEC.md` and `UX-FLOW-TASK-FLOW-SPEC.md`.

---

## 67. All Stations Overview Interaction Map

The overview is primarily a monitoring and selection surface.

### 67.1 Station Card

**Interaction:** click/tap the primary card surface.

**Result:** open Selected Station Detail for that station.

**No modal is required.** The transition is a page/view-level change within Dashboard.

The selected station must become the active station context before detail content is rendered.

### 67.2 Station Card action affordance

If the card contains a secondary action such as `Xem`, it must produce the same result as clicking the primary card surface.

Do not create multiple different destinations from the same card unless each action has an explicit label.

### 67.3 Warning count on Station Card

The warning count is a summary value, not warning detail.

Until the fleet API exposes warning records, clicking the warning count must open the Selected Station Detail rather than fabricate or display an invented warning list.

### 67.4 Fleet Summary

Summary counters may act as filters when their underlying data supports filtering.

Recommended behavior:

- `Tất cả trạm`: clear monitoring filters;
- `Online`: filter to online stations;
- `Cần chú ý`: filter to stations whose source-backed attention condition is present.

These interactions update the overview in place and do not change station context.

### 67.5 Monitoring Filter

The filter must visibly identify the active filter.

When the filter returns zero stations, show an empty result state with an explicit way to restore `Tất cả`.

### 67.6 Refresh

Refresh is an in-place data operation.

During refresh:

- preserve station cards where safe;
- show a visible pending indicator;
- do not fabricate new values;
- do not clear valid content merely to show a spinner unless required by the data-loading contract.

After refresh, the overview must reconcile the latest source-backed station summaries.

### 67.7 Add Station

`Thêm trạm` is a navigation action to the pairing/provisioning flow when the user has permission.

It must not open Selected Station Detail because no station has been selected yet.

---

## 68. Station Selection and Context Contract

Station selection is a first-class Dashboard state.

### 68.1 Selection sequence

The canonical interaction is:

```text
All Stations Overview
    Station Card selected
    Selected station context established
    Selected Station Detail rendered
```

### 68.2 Context header

Selected Station Detail must expose, at minimum when available:

- station label;
- device ID for disambiguation;
- online/offline state;
- current operational state when available.

Example:

```text
┌─────────────────────────────────────────────────────────────┐
│ Dashboard                                                   │
│                                                             │
│ [← Tất cả trạm]   STATION C       ● Online                 │
│                    device_xxx                               │
└─────────────────────────────────────────────────────────────┘
```

### 68.3 Switching station

The station selector may be used from Selected Station Detail.

Selecting another station must:

1. replace the selected station context;
2. clear or invalidate station-specific transient state;
3. request/render data for the new station;
4. never display the previous station's telemetry as if it belonged to the new station.

### 68.4 Returning to All Stations

`← Tất cả trạm` returns to the monitoring overview.

The overview should restore the fleet monitoring context and may preserve the user's filter and scroll position when technically appropriate.

### 68.5 Cross-page station context

When a Dashboard action navigates to Operations, Cultivation, Journal, or Settings for a selected station, the destination must receive the selected station context explicitly.

The user must not be required to manually infer or reselect the station when the originating action is station-specific.

---

## 69. Selected Station Detail Interaction Map

The detail view contains both informational and actionable regions.

### 69.1 System State / Next Action

**Normal state:** read-only.

**Actionable fault/warning:** expose the source-backed recovery action when available.

The action may navigate to Operations or another defined recovery surface. The Dashboard must not invent a recovery procedure.

### 69.2 Health Score

Health Score is read-only in the current contract.

It must not open a fabricated diagnostics view containing CPU, memory, uptime, subsystem scores, or other unsupported data.

If a future diagnostics contract is added, Health Score may become an Inspect interaction.

### 69.3 Sensor Connectivity

Sensor connectivity is read-only in the current contract because the Dashboard has an aggregate sensor-node availability signal rather than an individual sensor diagnostic contract.

Do not create a sensor diagnostic panel from unsupported assumptions.

### 69.4 Tank Alerts

Tank alerts are inspectable and actionable.

Source correction: the current Dashboard renders tank alerts as non-interactive `Badge` elements inside a `Banner`; there is no click handler or overlay. Any tank-alert inspect/corrective-action surface is `Implementation-required`.

Clicking an individual source-backed tank alert may open a compact detail overlay containing:

- alert name;
- affected tank identifier/name;
- current alert state;
- supported corrective-action CTA.

The overlay must not invent tank volume, percentage, remaining time, or other unavailable measurements.

The corrective-action CTA navigates to Operations with the selected station context.

### 69.5 Quick Actions

Quick Actions are explicit action controls.

#### Tưới ngay

This is a physical command and must follow the command lifecycle.

The interaction must support:

```text
Available
    Guard check
    Pending
    Result
    Observed state reconciliation
```

The UI must distinguish command acceptance from observed physical state.

The Dashboard must not treat command acceptance as physical confirmation. The command lifecycle uses the canonical `REQUESTED`, `SENT`, `ACKNOWLEDGED`, `PENDING`, `CONFIRMED`, `REJECTED`, `FAILED`, `TIMEOUT`, and `UNKNOWN` semantics where applicable. If the authoritative physical state is not observed after command submission, the result remains unconfirmed/unknown and the UI must provide recovery or re-check behavior rather than claiming completion.

#### Châm dinh dưỡng

Navigate to Operations with the selected station context.

#### Tạm dừng bơm

Navigate to Operations with the selected station context unless a direct command contract is explicitly introduced.

#### Xem cảnh báo

Navigate to Journal with the selected station context.

### 69.6 Telemetry Card

Telemetry cards are inspectable but are not physical controls.

Source correction: `SensorBentoCard` is currently a non-interactive `div`. Inspect/drill-down behavior is `Implementation-required` unless a handler and destination are added.

An Inspect interaction may expose the currently supported information:

- metric name;
- current value;
- current unit;
- target or limit when source-backed;
- tolerance when source-backed;
- semantic state;
- sensor error state when applicable.

Do not add historical charts, trends, calibration data, or additional measurements unless their contracts are defined elsewhere.

### 69.7 Active Devices

Active device entries are observational summaries.

Clicking an active device navigates to Operations with:

Source correction: the current `ActiveDeviceTag` is a non-interactive `span`. Operations navigation from an active-device entry is `Implementation-required`.

- selected station context;
- selected device/actuator context when supported.

The active-device indicator itself must not directly start or stop the device.

### 69.8 Dosing Summary

The dosing summary is an Inspect/Navigate interaction.

Source correction: the current `DosingSummaryCard` is a non-interactive `div`. Journal navigation from this card is `Implementation-required`.

Clicking it may navigate to Journal with:

- selected station context;
- dosing-related event filter when supported.

Do not add dosing volume or dosing analytics not present in the source contract.

### 69.9 E-STOP

E-STOP is a dedicated safety command, not a normal Dashboard navigation.

Its interaction must follow the canonical safety contract and must remain available when appropriate under the system state. E-STOP is an immediate safety action and must not use a generic confirmation step that delays execution. The current `EmergencyStopButton` implementation still opens `EmergencyStopConfirmDialog`; this is an implementation gap against the locked Dashboard safety contract, not a reason for the functional spec to preserve generic confirmation.

After invocation, the UI must expose the safety command lifecycle and distinguish command/result state from confirmed physical state. Failure or unknown outcome must remain observable and must not be rendered as successful E-STOP completion.

### 69.10 Onboarding

Onboarding controls operate inside the onboarding flow.

They must not alter selected station context unexpectedly.

Onboarding must remain secondary to safety-critical and availability states.

---

## 70. Overlay Contract

Where an interaction uses a popover, dialog, drawer, or inline expansion, the overlay must define its content explicitly.

### 70.1 Overlay content rules

Every overlay must contain:

- clear title;
- affected station when station-specific;
- concise source-backed information;
- available action, if any;
- explicit close/cancel behavior.

### 70.2 Overlay hierarchy

Safety and command confirmation overlays outrank informational overlays.

An informational overlay must never obscure an active critical-state presentation indefinitely.

### 70.3 Overlay and station switching

If the user switches station while an overlay is open, the overlay must be closed or revalidated before displaying station-specific content for the new station.

---

## 71. Return and Back Behavior

Navigation must preserve the user's mental model.

### From Station Card

`All Stations Overview` back context is restored.

### From Selected Station Detail to All Stations

Return to the monitoring overview rather than another unrelated page.

### From Operations / Journal / Cultivation / Settings

When entered from a station-specific Dashboard action, return should preserve the originating station context when supported by the application's navigation model.

### Browser Back

Browser/back navigation must not silently switch the selected station to a different station.

---

## 72. Interaction Feedback Contract

Every interaction needs an observable response.

### Selection

- selected card is visually distinct;
- detail header confirms station identity.

### Navigation

- destination renders the correct station context;
- no stale station data is presented as current.

### Filter

- active filter is visible;
- result count/content updates.

### Refresh

- pending state is visible;
- completion reconciles data.

### Command

- command pending state is visible;
- success/error result is visible;
- observed physical state is reconciled separately when applicable.

---

## 73. Interaction and Permission Matrix

| Interaction | Viewer | Operator | Admin | Station context |
|---|---:|---:|---:|---|
| View station cards | Yes | Yes | Yes | Overview |
| Select station | Yes | Yes | Yes | Selected station |
| Filter overview | Yes | Yes | Yes | Overview |
| Refresh monitoring data | Yes | Yes | Yes | Current scope |
| View telemetry | Yes | Yes | Yes | Selected station |
| View alerts | Yes | Yes | Yes | Selected station |
| Navigate to Journal | Yes | Yes | Yes | Selected station |
| Navigate to Operations | Yes | Yes | Yes | Selected station |
| Tưới ngay | Guarded by command permission | Guarded by command permission | Guarded by command permission | Selected station |
| E-STOP | Safety policy | Safety policy | Safety policy | Selected station |
| Add station | No unless explicitly granted | No unless explicitly granted | Yes | No station yet |

The matrix is a Dashboard interaction summary. The authoritative permission rules remain in the role and command contracts.

---

## 74. Interaction Anti-Patterns

Do not:

- make every card clickable without a meaningful result;
- make a status indicator look like a command button;
- open a generic modal when navigation is the clearer action;
- navigate to Operations without preserving station context;
- display previous-station telemetry after switching stations;
- turn warning counts into invented warning details;
- fabricate diagnostics after clicking Health Score;
- fabricate sensor diagnostics after clicking Sensor Connectivity;
- add telemetry history merely because a card is clickable;
- use click feedback as proof that a physical command succeeded;
- let a low-priority overlay hide a critical state;
- require the user to remember which station they selected;
- silently reset the station context during cross-page navigation.

---

## 75. Interaction Definition of Done

The Dashboard interaction layer is complete only when:

- every clickable region has a documented result;
- every non-clickable region is intentionally read-only;
- Station Card selection opens the correct station detail;
- selected station identity is persistent and obvious;
- station switching cannot leak stale telemetry;
- cross-page actions preserve station context;
- overlays contain only source-backed information;
- physical commands follow command lifecycle rules;
- permissions are enforced before commands are presented/executed;
- E-STOP follows the final approved safety contract;
- browser/back behavior preserves station context;
- filter, refresh, navigation, inspect, and command feedback are visible;
- no interaction requires invented data to make the UI feel complete.

---

## 76. Canonical Dashboard Interaction Formula

**Discover, Select, Confirm Context, Inspect, Navigate or Command, Receive Feedback, Preserve Context.**

---

## 77. PAGE-SPEC-CHECKLIST Compliance Contract

This section reconciles the Dashboard against `PAGE-SPEC-CHECKLIST.md`. It is an implementation-readiness contract, not a request to add unsupported product capability.

### 77.1 Page identity and ownership

- **Page:** Dashboard / Overview.
- **Primary purpose:** monitoring and triage across accessible stations, followed by selected-station inspection and transition into the owning task surface.
- **User problem:** identify which station needs attention and understand the selected station's current source-backed state.
- **Primary roles:** `admin`, `operator`, `viewer`; backend authorization remains authoritative.
- **Dashboard owns:** overview summaries, selected-station monitoring, inspection, limited quick-action entry points, station-context handoff, and monitoring feedback.
- **Dashboard does not own:** full actuator control, closed-loop EC/pH regulation, Automation orchestration, persistent configuration, cultivation configuration, fleet administration, or full historical trace.
- **Entry:** application Dashboard/Overview entry and station-specific navigation that resolves to Dashboard where supported by the existing router.
- **Exit:** Operations, Cultivation, Journal, Settings, Fleet/provisioning, or All Stations Overview according to the existing navigation model.

### 77.2 Source of truth and implementation status

| Area | Status | Source / contract |
|---|---|---|
| Selected-device Dashboard state | `Source-backed` | `hydragrow-frontend/src/pages/Dashboard.tsx`, `useDeviceStore`, Dashboard-related Gleam modules |
| Device identity/status | `Source-backed` | `useDeviceStore`; `deviceId`, `deviceStatus.is_online`, `last_seen` |
| Telemetry / actuator state | `Source-backed` | `useDeviceStore.sensorData` |
| Settings-derived targets/limits | `Source-backed` | `useDeviceStore.settings` |
| Health summary | `Source-backed` | `useSystemHealthSummary` and controller-health state |
| Fleet device list/status | `Source-backed` | `hydragrow-frontend/src/hooks/useFleetStatus.ts` via `/devices` and `/devices/{device_id}/status` |
| Fleet summary EC/pH/warnings | `Source-backed where endpoint supplies value` | `hydragrow-frontend/src/pages/FleetView.tsx` via `/fleet/summary` |
| Dashboard All Stations Overview | `Implementation-required` | Current `Dashboard.tsx` does not consume `useFleetStatus` or render `FleetStationCard`; FleetView currently owns this implementation |
| Dashboard overview filters/refresh/add station | `Implementation-required` | Current controls are implemented in `FleetView.tsx`, not `Dashboard.tsx` |
| Rich per-station fleet telemetry | `Implementation-required` | Must not be implied by current Fleet source |
| Station-context persistence across pages | `Implementation-required` | Current `FleetView` writes `deviceId` then navigates to Dashboard; Dashboard currently consumes only one `deviceId`, so multi-station switching/handoff still requires implementation work |
| E-STOP immediate-action semantics | `Implementation-required` | Functional contract; current `EmergencyStopButton` still opens `EmergencyStopConfirmDialog` |
| Full history/analytics | `Deferred` | Journal owns canonical historical trace |

`last_seen` means last known device contact. It must not be labeled as telemetry freshness. `SensorData.time` must not be exposed as visible freshness unless a separate product contract establishes that behavior.

### 77.3 Selected Station Context

The minimum runtime context is:

- `deviceId`;
- station `label` when available from fleet data;
- connection/availability state;
- current operational/system state;
- originating page/action when needed to preserve return context.

Context rules:

1. Select station before station-specific inspection or command.
2. On station switch, invalidate station-specific transient state before rendering new station data.
3. Never render old station telemetry under the new station identity.
4. If station/device context is missing, show an explicit no-context state and delegate pairing/configuration to the owning surface.
5. Cross-page station-specific navigation passes the selected context explicitly.
6. Browser/back must not silently replace selected station context.
7. An open station-specific overlay must close or be revalidated when context changes.

### 77.4 Component inventory and interaction ownership

| Component | Data/source | Interaction |
|---|---|---|
| Dashboard header | device/status/mode/user | inspect; device-ID link navigates to Fleet; station context/navigation where supported |
| Station overview/card | FleetView `FleetStationCard` | current selection is implemented in FleetView; Dashboard integration is `Implementation-required` |
| Fleet summary strip | FleetView source | target Dashboard interaction; current Dashboard implementation absent |
| Overview filters | FleetView source | target Dashboard interaction; current Dashboard implementation absent |
| Refresh | FleetView `useFleetStatus.refresh` | target Dashboard interaction; current Dashboard implementation absent |
| Loading/empty/error feedback | page data-loading/error state | read-only feedback; retry/recovery where supported |
| System state / Next Action | FSM, availability, fault guide | inspect; source-backed recovery navigation |
| Health Score | controller health | read-only |
| Sensor Connectivity | aggregate sensor-node availability | read-only |
| Tank Alerts | `tankAlert.*` | current Dashboard is read-only; inspect/corrective navigation is `Implementation-required` |
| Telemetry cards | `sensorData.*`, settings limits | current Dashboard is read-only; inspect surface is `Implementation-required` |
| Active Devices | `pump_status.*` | current Dashboard is read-only; Operations navigation is `Implementation-required` |
| Dosing Summary | health-summary dosing counters/timestamp | current Dashboard is read-only; Journal navigation is `Implementation-required` |
| Quick Actions | explicit action controls | navigate or physical command according to action contract |
| E-STOP | safety control + actuator state | immediate safety command |
| Notification CTA | FCM permission | request/enable notification permission |
| Onboarding | onboarding hook/flow | remain within onboarding; no silent station switch |
| Overlays | source-backed selected-station data | inspect/action/close according to overlay contract |

Every interactive component must expose a defined disabled, pending, error, and unknown outcome where that state is applicable. Read-only status must not look like a command control.

### 77.4.1 Component State Contract

Each interactive Dashboard component must define its applicable runtime states explicitly. The component state is UI state and must not be inferred as physical device state unless a separate source-backed observation confirms it.

| Component | Required states | State meaning / UI rule |
|---|---|---|
| Station Card | idle, selected, unavailable, loading | `selected` identifies UI context only; unavailable metrics remain unavailable |
| Station switch | idle, pending, success, error, unknown | invalidate prior station transient state before accepting new station data as current |
| Refresh | idle, pending, success, error | pending feedback is visible; completion reconciles source-backed data |
| `Tưới ngay` | available, disabled, pending, accepted, rejected, timeout/unknown | command result must not be presented as physical confirmation |
| E-STOP | available, executing, accepted/rejected/unknown | immediate safety action; no generic confirmation state may block dispatch |
| Notification CTA | available, requesting, enabled, denied, unavailable/error | browser/backend permission result must be shown truthfully |
| Overlay/detail | closed, opening/loading, ready, error, stale | station-specific content must be revalidated or closed when context changes |

For every applicable state, the implementation must define:

1. visible UI;
2. allowed actions;
3. transition trigger;
4. recovery path;
5. whether the state represents request status, backend result, or observed device state.

`accepted`, `success`, `online`, `running`, or similar UI labels must not be used as shorthand for physical confirmation unless the source contract explicitly establishes that meaning.

### 77.5 Exhaustive interaction contract

For each interaction, the minimum contract is:

`Trigger -> immediate UI response -> guard/local validation -> operation or navigation -> success/rejection/error/timeout/unknown -> recovery -> context preservation`.

Dashboard-specific rules:

- **Station select:** select identity, show pending context load, render only matching station data, preserve selection on failure/unavailable state.
- **Device-ID link:** navigate to `/fleet`; this is fleet administration navigation, not a station-detail command, and must not be presented as physical control.
- **Filter:** target Dashboard interaction; current implementation exists in FleetView, not Dashboard.
- **Refresh:** target Dashboard interaction; current refresh implementation exists in FleetView, not Dashboard.
- **Inspect:** no backend mutation unless the specific interaction is explicitly a command.
- **Navigate:** destination must receive station context for station-specific actions.
- **Notification CTA:** request notification permission. Current `useFCM` has silent return paths when backend/device/token prerequisites are missing and logs some failures without a user-visible error; visible failure feedback is `Implementation-required`.
- **Tưới ngay:** guarded physical command; pending/result/unknown must be visible; backend acceptance is not physical confirmation.
- **Châm dinh dưỡng / Tạm dừng bơm / Xem cảnh báo:** navigation to owning surface with selected station context.
- **E-STOP:** immediate safety command; no generic confirmation delay; result and unknown/failure remain observable.
- **Duplicate/conflicting commands:** Dashboard must not create a second control authority; defer arbitration/interlock decisions to canonical control/safety contracts.

### 77.6 Data contract

| Data class | Dashboard rule |
|---|---|
| Measured | `ec`, `ph`, `temp`, `water_level` and source-backed sensor readings; preserve source unit/precision rules |
| Estimated | None currently specified; do not infer estimates |
| Commanded | Requested actuator/action state; never display as confirmed physical state |
| Confirmed/observed | Only source-backed authoritative state after reconciliation |
| Connection | `is_online`; distinct from sensor availability |
| Contact | `last_seen`; last known device contact, not telemetry freshness |
| Alert | Source-backed `tank_*_low`, fault/state signals, and fleet `warning_count` |
| Health | Existing controller/diagnostics health score; no fabricated subsystem diagnostics |
| Actuator | `sensorData.pump_status.*`; observational on Dashboard except explicit command controls |
| Dosing summary | EC/pH counts and latest pH dosing timestamp from source |
| Unavailable | `—` or explicit unavailable/unknown state; never meaningful zero/Off without source justification |

For every displayed value, missing/null behavior, stale/unknown behavior, formatting, unit, and precision follow the underlying source contract. Derived display labels such as semantic sensor status are identified as derived, not measured values.

### 77.7 State model

Dashboard must intentionally classify these states:

- initial/loading;
- all-stations ready;
- no accessible stations;
- filtered zero-result;
- fleet network/backend error;
- selected-station loading;
- selected-station ready;
- selected-station partial data;
- selected-station offline/unavailable;
- sensor-node unavailable, distinct from device offline;
- FSM warning/fault/emergency state;
- stale/invalidated station state during switching;
- permission denied;
- command pending;
- command accepted/acknowledged;
- command rejected/failed;
- command timeout/unknown;
- confirmed/observed physical state;
- recovery after error or unavailable state.

Missing information must never be rendered as a confirmed negative state.

### 77.7.1 Confirmation / Reversibility Contract

Every consequential Dashboard action must explicitly classify whether confirmation is required and whether the action can be cancelled or reversed.

| Action | Confirmation | Reversibility | Required behavior |
|---|---|---|---|
| Station selection / navigation | No | N/A | Transition immediately and preserve station context. |
| Refresh | No | N/A | Show pending state and reconcile the result. |
| `Tưới ngay` | Only if required by the canonical command contract | Command-specific | Do not add an ad-hoc confirmation merely to mask command uncertainty. |
| E-STOP | No generic confirmation | Safety action | Dispatch immediately according to the canonical safety path. |
| Destructive delegated action | Yes when required by the owning surface | Explicitly defined by owning contract | Disclose consequence before mutation. |

Rules:

1. Confirmation occurs before dispatch, never after the command has already been sent.
2. Confirmation does not convert `accepted` into physical confirmation.
3. If an action is cancellable, the cancellation window and resulting state must be explicit.
4. If an action is not reversible, the UI must not imply that undo is available.
5. Unsaved Dashboard-local state may require a navigation guard, but Dashboard must not invent persistent draft behavior.
6. E-STOP is governed by the safety contract and must not inherit a generic destructive-action confirmation pattern.

### 77.8 Command, safety, and control authority

Dashboard is not a control-logic owner.

- **Operations:** current runtime/system control.
- **Auto Mode:** closed-loop EC/pH regulation.
- **Automation:** event/workflow orchestration; may trigger supported dosing actions but must not become a second EC/pH controller.
- **Control Authority / Arbitration:** resolves competing writers before the Safety Gate.
- **Safety Gate:** blocks unsafe operations according to canonical safety rules.
- **Dashboard:** invokes supported commands and displays lifecycle/result; it does not bypass arbitration or safety.

For `Tưới ngay`, the current Dashboard source calls `forceOn('WATER_PUMP_IN', 30)`, which maps to the `force_on` command with `duration_sec: 30` and no Dashboard-supplied PWM value. These are implementation/source facts, not permission for Dashboard to expose a new parameter editor. Guards, permission, interlocks, rate limits, execution semantics, and physical confirmation follow the canonical command contract.

The current `useDeviceControl` implementation marks a successful HTTP response as `accepted` and returns `true`; Dashboard does not independently establish `CONFIRMED` physical state. Therefore the Dashboard command contract must not present the current implementation as physical confirmation. Confirmation/reconciliation remains implementation-required against the canonical command/state contract.

E-STOP has safety priority. It must not be blocked by an ordinary confirmation pattern that delays the safety action. The UI must not claim physical stop solely from a successful request/HTTP response.

### 77.9 Permission contract

The product roles are `admin`, `operator`, `viewer`.

- View/inspect Dashboard: all three roles, subject to backend access to the station.
- Station selection/filter/refresh: all three where the station is accessible.
- Navigation to Journal/Operations: all three where destination access permits.
- Physical commands: execute permission is authoritative; Viewer is not granted actuator control by Dashboard UI.
- E-STOP: follow canonical safety permission contract; current UX flow defines Admin/Operator yes, Viewer no.
- Pairing/Add Station: delegated to Fleet/pairing and backend authorization; Dashboard currently does not render an Add Station control.

Permission denial is a guarded outcome, not a fake pending state.

### 77.10 Navigation and cross-page contract

Dashboard handoff rules:

- **Operations:** selected station context; current operational task owns detailed control.
- **Automation:** separate surface/domain; Dashboard must not present Automation as an Operations tab or own automation logic.
- **Cultivation:** selected station context where cultivation task is station-specific.
- **Journal:** selected station context; dosing/alert filter only when supported.
- **Settings:** selected station context for persistent configuration; Dashboard does not edit persistent configuration.
- **Fleet/pairing:** station administration/pairing ownership remains outside Dashboard.

### 77.10.1 Cross-Page Handoff Contract

Every station-specific Dashboard handoff must define the complete context transfer:

```text
Origin
-> selected station/device context
-> user intent/action
-> destination
-> destination context restoration
-> result or mutation handoff
-> return/reconciliation
```

Minimum handoff context:

- `deviceId`;
- station label when available;
- originating Dashboard context/action when needed to determine return behavior;
- destination-specific intent when the destination supports more than one entry mode.

Rules:

1. Dashboard must not navigate to a station-specific destination while silently dropping `deviceId`.
2. The destination must not silently substitute another station when supplied context is valid but unavailable; it must expose an explicit unavailable/no-context state according to its own contract.
3. On return, Dashboard must reconcile the selected station against current source data rather than assuming pre-navigation values are still current.
4. A pending Dashboard mutation must not be reported as completed merely because navigation succeeded.
5. Browser/back navigation must preserve station context where supported by the application navigation model.
6. Cross-page ownership remains with the destination surface after handoff; Dashboard owns only its originating context and presentation.
7. Fleet provisioning/pairing remains Fleet-owned; Dashboard may hand off to Fleet but does not absorb Fleet configuration ownership.

### 77.10.2 Contract Application to Dashboard Interactions

The contracts above are normative for the concrete Dashboard interactions below.

#### Station Card

```text
idle
-> selected/pending
-> detail loading
-> detail ready | unavailable | error
```

- Selection changes the active `deviceId` before station-specific content is rendered.
- A failed detail load must retain the selected identity while exposing the error/recovery state.
- Switching again invalidates the prior station's transient data before rendering the next station.
- No station-card selection is a physical command and therefore no confirmation is required.

#### Fleet filter / Refresh

- Filter changes are local overview state; they do not mutate selected station context.
- Refresh exposes a pending state and reconciles returned source data.
- Refresh failure must remain distinguishable from a valid empty fleet result.
- A filter returning zero stations is an empty result, not a fleet-load failure.

#### `Tưới ngay`

- Guard check occurs before command dispatch.
- If the canonical command contract requires confirmation, confirmation occurs before dispatch; otherwise no ad-hoc confirmation is added.
- UI state after dispatch must identify the command result (`accepted`, `rejected`, `timeout`, `unknown`, or equivalent canonical state) separately from observed actuator state.
- Navigation or local click feedback must never convert an accepted command into physical confirmation.
- If the command is not reversible, the UI must not imply an undo action.

#### E-STOP

- E-STOP dispatch is immediate and must not wait for a generic confirmation dialog.
- The UI exposes execution/result state after dispatch.
- `accepted` or successful request submission is not itself proof that all physical actuators have stopped.
- Failure or unknown outcome remains visible and actionable according to the canonical safety recovery path.

#### Station-specific handoff

For Operations, Cultivation, Journal, or Settings:

```text
Dashboard
-> selected deviceId
-> user intent
-> destination
-> destination restores context
-> destination owns the task
-> Dashboard reconciles on return
```

The destination must not silently fall back to another station. If the supplied station is unavailable, the destination exposes its explicit unavailable/no-context state.

#### Read-only cards

Health Score, Sensor Connectivity, Active Devices, Dosing Summary, and telemetry cards remain read-only unless a concrete source-backed interaction is implemented. A future click target must first define its destination/overlay, states, permissions, and recovery path; the visual treatment alone must not imply an interaction.

Current application routes verified in `hydragrow-frontend/src/App.tsx`:

- Dashboard: `/dashboard` (with `/` redirecting to `/dashboard`)
- Operations: `/operations`
- Automation: `/automation`
- Cultivation: `/cultivation`
- Journal: `/journal`
- Settings: `/settings`
- Pairing: `/pairing`
- Fleet: `/fleet`

The functional contract does not require a specific URL encoding for selected station context. The current router does not encode `deviceId` in the route; explicit station-context persistence across station-specific navigation is therefore implementation-required.

Pending Dashboard commands must not be represented as completed merely because navigation occurred. If navigation happens during an active consequential operation, the canonical command state remains authoritative and the destination must reconcile it when supported.

### 77.11 Desktop/mobile contract

**Desktop**

- All Stations Overview is scan-oriented.
- Selected Station Detail uses the primary content area; secondary inspection/overlay surfaces are subordinate.
- Actuator detail is not top-level Dashboard navigation; use drawer/secondary pane where implemented.
- Hover may enhance discoverability but must not be the only interaction cue.
- Keyboard focus and action order must follow semantic page order.

**Mobile**

- Overview remains scan/select first; do not merely shrink the desktop canvas.
- Selected Station Detail becomes a vertically ordered inspection surface.
- Secondary/drawer detail becomes full-screen when needed.
- Critical actions, including E-STOP, remain reachable without hover.
- Touch targets must be adequate; sticky actions are used only where required for safety or task completion.

No graph/canvas interaction is required by the Dashboard contract.

### 77.12 Forms and validation

Dashboard owns no persistent configuration form.

- Filters have only source-backed allowed values.
- Physical command parameters are not edited on Dashboard unless a separate command contract explicitly assigns ownership here.
- Add Station validation/submission is delegated to Fleet/pairing; it is not a current Dashboard control.
- Backend validation remains authoritative.
- No Dashboard draft persistence is required.

### 77.13 History and audit

Dashboard does not own full history.

- Dosing summary and alert actions may navigate to Journal.
- Consequential Dashboard commands rely on the canonical command/audit system where supported.
- Dashboard must not invent local audit persistence, actor fields, timestamps, or event IDs.
- Full historical trace remains owned by Journal.

### 77.14 Empty/loading/error/unknown contract

- **Initial/loading:** explicit loading state; no fabricated values.
- **No station:** explicit no-context/empty state with owning pairing/configuration action.
- **Filtered empty:** target Dashboard overview behavior; current filtered-empty implementation exists in FleetView, not Dashboard.
- **Fleet error:** target Dashboard overview behavior; current FleetView exposes an error, while Dashboard does not currently retrieve fleet data.
- **Selected station loading:** identify station context while data loads.
- **Offline:** show offline/unavailable state; do not imply stopped/healthy state.
- **Sensor unavailable:** show sensor-node unavailable separately from device offline.
- **Partial data:** render available source-backed fields and mark unavailable fields explicitly.
- **Unknown command outcome:** do not claim success; retain observable unknown state and provide supported retry/reconciliation path.
- **Fault:** expose source-backed fault information/recovery only.

### 77.15 Accessibility and observability

Accessibility requirements:

- semantic buttons/links for actions/navigation;
- visible keyboard focus;
- accessible names for station selection, filters, quick actions, and E-STOP;
- state not communicated by color alone;
- pending/error/unknown feedback discoverable to assistive technology where appropriate;
- adequate touch targets.

Observability requirements:

- user-visible command lifecycle status;
- source-backed error/fault code where useful;
- relevant timestamps only where source-backed;
- unknown outcomes remain observable;
- consequential command/audit events use the canonical runtime/audit path where available.

No new analytics event names or telemetry pipeline are implied by this spec.

### 77.16 Wireframe, acceptance, and implementation mapping

Wireframes must cover, without adding unsupported data:

1. All Stations Overview desktop.
2. All Stations Overview mobile.
3. Selected Station Detail desktop.
4. Selected Station Detail mobile.
5. Critical/fault state.
6. Loading state.
7. Empty/no-station state.
8. Consequential command pending/result/unknown state.
9. E-STOP immediate-action state.

Acceptance criteria:

**Station selection**

Given the All Stations Overview is ready,
When the user selects station `S`,
Then `S` becomes selected and its detail loads,
And no telemetry from another station is shown as current.

**Station switch**

Given station `A` is selected and station-specific data is visible,
When the user selects station `B`,
Then station-specific transient state from `A` is invalidated,
And `B` identity is shown before `B` data is treated as current.

**Station selection state**

Given the user selects station `S`,
When the selection is being applied,
Then the UI exposes the applicable pending/loading state,
And a selection request state is not presented as proof of station telemetry freshness or physical device state.

**Station selection error**

Given station `S` is selected but its detail data cannot be loaded,
When the load fails,
Then `S` remains identifiable,
And the UI exposes an error/recovery state,
And no stale telemetry from the previously selected station is shown as current.

**Filter**

Given the overview has stations,
When the user applies a supported filter,
Then only matching source-backed stations remain visible,
And selected station context is preserved.

**Unavailable summary**

Given a fleet summary field is unavailable,
When the station card renders,
Then it shows an explicit unavailable state such as `—`,
And it does not derive a Selected Station Detail value for that station.

**Refresh**

Given Dashboard data is visible,
When the user refreshes,
Then pending feedback is visible,
And completion reconciles source-backed values without fabricating missing data.

**Refresh error versus empty**

Given Dashboard overview data cannot be retrieved,
When refresh fails,
Then the UI shows an error/recovery state,
And it does not present the result as a valid empty fleet.

Given the overview retrieval succeeds but the active filter matches no stations,
When the filtered result renders,
Then the UI shows the filtered-empty state,
And the user can restore the unfiltered overview.

**Operations handoff**

Given station `S` is selected,
When the user opens a station-specific Operations action,
Then Operations receives station `S` context,
And the user is not required to infer or manually reselect `S`.

**Physical command**

Given `Tưới ngay` is permitted and all required guards pass,
When the user invokes it,
Then the command enters its canonical lifecycle,
And request acceptance is not displayed as physical confirmation.

**Physical command reversibility**

Given `Tưới ngay` has been dispatched,
When the command has no supported cancellation/reversal path,
Then the UI does not expose or imply an undo action,
And any later observed state is reported separately from command acceptance.

**E-STOP**

Given the E-STOP action is available,
When the user invokes E-STOP,
Then the safety command is initiated without a generic confirmation delay,
And success/unknown/failure remains distinguishable from observed physical stop state.

**Permission denial**

Given the current user lacks execute permission,
When a physical command is attempted,
Then execution is blocked,
And the UI does not create a fake pending state.

**Unknown outcome**

Given a consequential command cannot be classified as success or failure,
When the operation reaches timeout/unknown,
Then the Dashboard shows unknown state,
And it does not claim physical completion.

**Back/context**

Given the user entered another page from station `S`,
When the user returns via supported back/browser navigation,
Then station context remains `S` where the application navigation model supports persistence,
And the Dashboard does not silently switch stations.

**Cross-page handoff unavailable**

Given station `S` is the selected station,
When a station-specific destination cannot restore `S` context,
Then the destination exposes an explicit unavailable/no-context state,
And it does not silently substitute another station.

**Pending command during navigation**

Given a consequential Dashboard command is pending,
When the user navigates to another page,
Then navigation success does not mark the command complete,
And the canonical command state remains authoritative for reconciliation.

Implementation mapping:

- `hydragrow-frontend/src/pages/Dashboard.tsx`: current single-device Dashboard composition, derived state, navigation, notification/onboarding integration, quick actions, telemetry, health, alerts, and E-STOP entry point.
- `hydragrow-frontend/src/store/useDeviceStore.ts`: selected `deviceId`, sensor data, device status, controller health, FSM state, settings, sensor availability, tank alert, and owned-device state.
- `hydragrow-frontend/src/hooks/useFleetStatus.ts`: fleet device list/status retrieval via `/devices` and `/devices/{device_id}/status`.
- `hydragrow-frontend/src/components/ui/QuickActionBar.tsx`: Dashboard quick-action controls.
- `hydragrow-frontend/src/components/safety/EmergencyStopButton.tsx`: current E-STOP UI and command entry; implementation currently opens `EmergencyStopConfirmDialog`, which must be reconciled with the immediate-action contract.
- `hydragrow-frontend/src/hooks/useDeviceControl.ts`: command execution boundary used by Dashboard/E-STOP; current `forceOn` maps to `force_on`, and current successful HTTP response is represented as `accepted`, not physical confirmation.
- `useSystemHealthSummary`, `useFCM`, `useOnboardingState`: supporting Dashboard data/permission/onboarding hooks.
- `SensorBentoCard`, `DosingSummaryCard`, `HealthScore`, `DeviceStatePill`, `Banner`, `LoadingState`: current Dashboard presentation components.

Required implementation changes:

1. Reconcile E-STOP UI with the immediate safety-action contract; do not preserve generic confirmation merely because the current component has it.
2. Ensure station-specific Dashboard navigation passes/preserves selected context and invalidates old station data during switching.
3. Ensure command lifecycle feedback distinguishes accepted/requested state from observed physical state.
4. Keep unsupported fleet summary/detail fields unavailable rather than deriving them from richer single-device data.
5. Ensure `Tưới ngay` remains the source-backed `force_on` action with `duration_sec: 30`; do not expose an unsupported Dashboard parameter editor.

Deferred:

- richer fleet telemetry/diagnostics;
- telemetry freshness presentation;
- Dashboard historical charts/analytics;
- full configuration editing;
- local Dashboard audit persistence;
- unsupported diagnostic drill-downs.

### 77.17 Checklist DoD

Dashboard is implementation-ready only when the requirements above are satisfied and verified against the repository source. In particular:

- ownership is unambiguous;
- source-backed vs implementation-required behavior is explicit;
- selected station context cannot leak or disappear silently;
- every interactive component has a defined outcome and applicable runtime state;
- component state transitions identify whether they represent UI state, request state, backend result, or observed device state;
- consequential actions define confirmation policy and reversibility/cancellation behavior;
- measured, estimated, commanded, and confirmed data are not collapsed;
- relevant state/error/unknown paths are defined;
- physical commands respect permission, safety, and Control Authority boundaries;
- E-STOP follows immediate safety semantics;
- cross-page navigation preserves context;
- cross-page handoff defines context restoration, unavailable/no-context behavior, and return reconciliation;
- mobile and desktop behavior are functionally defined;
- Journal, Settings, Operations, Automation, and Fleet ownership boundaries remain intact;
- wireframes cannot add unsupported capabilities;
- acceptance criteria are testable;
- implementation mapping identifies current code and required changes.

Final review question:

> Can another engineer implement the Dashboard, its interactions, state transitions, safety boundaries, and cross-page behavior from this specification without inventing missing product behavior?

---

## 78. PAGE-SPEC-CHECKLIST Audit Result — 2026-09-15

This audit applies the current `PAGE-SPEC-CHECKLIST.md`, including Component Open / Detail Contract, Mutation Lifecycle Contract, Source-to-UI Traceability, Ownership Matrix, and Concurrency / Conflict Contract.

### 78.1 Status summary

| Checklist area | Status | Finding |
|---|---|---|
| Page identity / ownership | `Pass` | Responsibility and cross-page ownership are explicit. |
| Source of truth | `Partial` | Selected-device Dashboard is source-backed; All Stations Overview is currently implemented in Fleet, not Dashboard. |
| Selected context | `Partial` | `deviceId` is source-backed, but persistent multi-station Dashboard context/switching is not implemented. |
| Information architecture | `Partial` | Target two-level Dashboard IA is defined; current source still renders single-device Dashboard only. |
| Component inventory | `Partial` | Major current components are mapped; target overview components are Fleet-owned rather than Dashboard-owned. |
| Component Open / Detail | `Partial` | Navigation/command outcomes are documented, while unsupported inspect surfaces are now explicitly marked `Implementation-required`. |
| Interaction / Outcome | `Partial` | Target flows are detailed; several overview interactions are not currently wired into Dashboard. |
| Data contract | `Pass` | Measured/commanded/confirmed distinctions and unavailable rules are explicit. |
| State model | `Partial` | Target states are comprehensive; current Dashboard has no multi-station state model. |
| Action / Command | `Pass` | `Tưới ngay` source path is exact; backend acceptance is not physical confirmation. |
| Mutation lifecycle | `Partial` | Contract is explicit, but `useDeviceControl` exposes `accepted`/boolean rather than a complete observed-state reconciliation model. |
| Safety / Control Authority | `Partial` | Boundaries are explicit; current E-STOP UI still violates immediate-action semantics. |
| Permission | `Partial` | Roles are documented, but command authorization remains canonical/backend-dependent. |
| Navigation | `Partial` | Routes are verified, but selected station context is not encoded/persisted by the current router. |
| Desktop / Mobile | `Pass` | Both target modes are functionally defined. |
| Forms / validation | `Pass` | Dashboard owns no persistent configuration form; delegated flows are bounded. |
| History / audit | `Pass` | Journal remains full-history owner. |
| Empty / loading / error / unknown | `Partial` | Target states are defined; current Dashboard does not implement fleet retrieval/filter/error surfaces. |
| Cross-page | `Partial` | Ownership is clear, but context handoff requires implementation. |
| Accessibility | `Partial` | Requirements are defined; implementation verification remains required for target interactions and E-STOP. |
| Observability | `Partial` | Contract is defined, but current command UI does not expose the complete lifecycle and `useFCM` has silent failure paths. |
| Wireframe | `Partial` | Target states are specified, but must not be treated as current implementation. |
| Acceptance | `Partial` | Given/When/Then coverage is strong; overview tests are target tests until Dashboard integrates fleet source. |
| Implementation mapping | `Pass` | Current Dashboard/Fleet/control/component paths and required changes are mapped. |
| Ownership matrix | `Pass` | Dashboard/Fleet/Operations/Automation/Settings/Journal boundaries are explicit. |
| Concurrency / conflict | `Partial` | Control arbitration and stale-context rules are defined conceptually; implementation is absent. |

Acceptance coverage now explicitly includes component-state transitions, confirmation/reversibility, error-versus-empty distinction, unavailable handoff, and pending-command navigation reconciliation. These tests remain `Implementation-required` where the corresponding behavior is not yet present in the repository.

### 78.2 Source-to-UI traceability findings

- `Dashboard.tsx` is currently a single-device page. It does not consume `useFleetStatus` and does not render `FleetStationCard`.
- `FleetView.tsx` currently owns fleet loading, filtering, sorting, grouping, summary retrieval, station selection, refresh, and pairing navigation.
- `FleetStationCard.tsx` is a real clickable button and selects a device through `FleetView`.
- `SensorBentoCard.tsx` is a non-interactive `div`; telemetry drill-down is not currently implemented.
- `ActiveDeviceTag` in `Dashboard.tsx` is a non-interactive `span`; Operations drill-down is not currently implemented.
- `DosingSummaryCard.tsx` is a non-interactive `div`; Journal drill-down is not currently implemented.
- Tank alert badges in `Dashboard.tsx` are non-interactive; the previously described overlay/CTA is target behavior only.
- `EmergencyStopButton.tsx` opens `EmergencyStopConfirmDialog`; this conflicts with the locked immediate-action safety contract.
- `useDeviceControl.ts` marks HTTP success as `accepted` and returns `true`; it does not establish physical confirmation.
- `useFCM.ts` contains silent prerequisite/failure paths, so visible notification failure feedback is not fully source-backed.

### 78.3 Blocking gaps

1. Integrate the fleet overview source into Dashboard, or explicitly move All Stations Overview ownership back to Fleet. The current spec chooses Dashboard ownership, so integration is required.
2. Implement selected-station context persistence and switching without stale-data leakage.
3. Reconcile E-STOP UI with the immediate-action safety contract.
4. Implement or remove target inspect/drill-down interactions that are not currently source-backed.
5. Implement truthful command lifecycle feedback for `Tưới ngay`; `accepted` must not be rendered as physical confirmation.
6. Define visible failure feedback for notification enablement where current `useFCM` can fail silently.

### 78.3.1 Contract Coverage Follow-up

The newly added Component State, Confirmation / Reversibility, and Cross-Page Handoff contracts are now applied to the concrete Dashboard interaction classes in Section 77.10.2.

This does **not** change implementation status. The following remain implementation-required where the current source does not yet provide them:

- multi-station Dashboard overview and station switching;
- selected-station persistence and stale-data invalidation;
- immediate-action E-STOP behavior;
- complete `Tưới ngay` command/result/reconciliation feedback;
- source-backed inspect/drill-down surfaces;
- visible notification permission failure feedback.

Acceptance tests for these behaviors must verify the contract state transitions rather than only the presence of controls or navigation.

### 78.4 Audit conclusion

The Dashboard spec is **checklist-complete at documentation-contract level but not implementation-ready**.

The main correction from this audit is important: the spec no longer conflates the existing Fleet implementation with the existing Dashboard implementation. Target multi-station behavior is explicitly labeled `Implementation-required`, while the current single-device Dashboard behavior remains source-backed.
