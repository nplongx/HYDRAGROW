# HYDRAGROW P0 — React ↔ Visual Variant / State Matrix

React is the production source of truth. OpenPencil should mirror only states and variants that are backed by the current React contract.

## 1. Core controls

### Button

Source: `src/components/ui/Button.tsx`

| Axis | Contract |
|---|---|
| variant | `primary`, `secondary`, `danger`, `ghost`, `icon` |
| size | `sm`, `md`, `lg` |
| loading | `false`, `true` |
| disabled | `false`, `true` |
| fullWidth | `false`, `true` |
| behavioral rule | `loading` forces disabled and exposes `aria-busy` |

Visual matrix to expose in OpenPencil: **5 variant definitions × 3 size definitions**, with `loading` and `disabled` represented as state rows rather than separate components. `fullWidth` is a layout property, not a visual variant.

### Switch

Source: `src/components/ui/Switch.tsx`

| Axis | Contract |
|---|---|
| checked | `false`, `true` |
| disabled | `false`, `true` |
| size | `sm`, `md` |
| label | optional |

Visual definitions: `Switch/Off`, `Switch/On`; state matrix: enabled/disabled for both. `label` is content, not a component variant.

### InputGroup

Source: `src/components/ui/InputGroup.tsx`

| Axis | Contract |
|---|---|
| validation | default, error |
| disabled | enabled, disabled |
| helper | absent, present |
| unit | absent, present |
| control | native input or custom `children` |

Visual definitions: `InputGroup/Default`, `InputGroup/Invalid`. Disabled/helper/unit are state/content properties and should be represented in the state matrix, not as a combinatorial component explosion.

## 2. Status / feedback

### Device status pill

Source: `src/components/ui/DeviceStatePill.tsx`

| State | Label semantics |
|---|---|
| `online` | Trực tuyến |
| `offline` | Ngoại tuyến |
| `unknown` | Chưa rõ |
| `warning` | Cảnh báo |
| `dosing` | Đang châm |
| `auto` | Tự động |
| `manual` | Thủ công |

All seven are real React states. The previous five-state OpenPencil set is incomplete and should be replaced/extended rather than inventing new labels.

### Command status pill

Source: `src/components/ui/StatusPill.tsx`

| State | Source values |
|---|---|
| `sending` | `REQUESTED`, `SENT` |
| `acknowledged` | `ACKNOWLEDGED` |
| `confirmed` | `CONFIRMED` |
| `error` | any other non-empty command status |

This is a distinct status family from device status and should not be merged visually into the device-state variant names.

### Pump control status

Source: `src/components/ui/PumpControlStatePill.tsx`

| State | Label |
|---|---|
| `idle` | Sẵn sàng |
| `running` | Đang chạy |
| `locked` | Đã khoá |

`reason` is supporting state/copy and does not create a visual variant.

### FSM status badge

Source: `src/components/ui/FsmStatusBadge.tsx`

Semantic states currently include booting/info, manual/warning, monitoring/default, completed/success, emergency/fault, disconnected/fault, water/refill info, mixing info, stabilizing/warning, misting, calibration info/success, MIMO/mist, and arbitrary/default text. Fault codes are interactive and open a fault explanation sheet.

This should remain a separate component family from the compact `DeviceStatePill`; do not flatten its state machine into a single pill variant set.

### Banner

Source: `src/components/ui/Banner.tsx`

Contract variants: `info`, `warning`, `danger`. The current React implementation has **no success variant**. Therefore `Banner/Success` in OpenPencil is not contract-backed and should be removed or repurposed as a non-P0 concept.

State/content axes: title required, description optional, action optional.

### StateView

Source: `src/components/ui/StateView.tsx`

Contract tones: `neutral`, `info`, `warning`, `danger`.

Typical semantic use: empty → neutral, unavailable → info/neutral, error → danger, warning state → warning. The component is content-driven (`icon`, title, optional description/action), so `Empty/Unavailable/Error` are gallery examples rather than React variants.

## 3. Navigation / shell

### PageHeader

Source: `src/components/ui/PageHeader.tsx`

Axes: icon absent/present, subtitle absent/present, action absent/present. No explicit visual variant prop.

### TabShell

Source: `src/components/ui/TabShell.tsx`

Axes: controlled/uncontrolled active tab, 1+ tabs, optional subtitle/action. Visual tab state is `active` vs `inactive`; selected content is the panel state.

### AppShell

Source: `src/components/layout/AppShell.tsx`

Responsive states: desktop sidebar, mobile header, mobile bottom navigation. Connection status is `ONLINE`, `OFFLINE`, or unknown. Navigation item state is active/inactive with optional alert badge.

## 4. Operational display

### TelemetryCard

Current production analogue: `src/components/ui/SensorBentoCard.tsx`.

| Axis | Contract |
|---|---|
| theme | `blue`, `fuchsia`, `orange`, `cyan`, `rose`, `emerald` |
| statusTone | `good`, `warn`, `danger`, `info` |
| compact | `false`, `true` |
| value | number/string/null (`null` renders `--`) |
| unit | optional |
| statusLabel | optional |
| rangeLabel | optional |
| description | optional |
| sparkline | optional numeric percentage |

Do not create 6×4×2 component definitions. Use a canonical card plus a compact state and status examples. Theme is a content/metric identity axis; statusTone is the operational state axis.

### QuickActionBar

Source: `src/components/ui/QuickActionBar.tsx`

Fixed four actions: water now, dose, pause/resume pumps, view alerts. The only explicit visual state is `pumpsPaused=false/true`, which changes the third action label from pause to resume.

### ActuatorCard / TelemetryGroup / Panel

No dedicated production React component with those exact names currently exists in `src/components`. Treat these as structural composition patterns, not claim them as API-backed React variants until a concrete implementation is introduced.

## 5. Safety

### EmergencyStopButton

Source: `src/components/safety/EmergencyStopButton.tsx`

Contract variants: `floating`, `bar`.

Behavioral states: idle/open trigger, confirmation dialog open, submitting. The trigger itself has no disabled prop; submission state belongs to `EmergencyStopConfirmDialog`.

### EmergencyStopConfirmDialog

Source: `src/components/safety/EmergencyStopConfirmDialog.tsx`

State matrix: closed/open × submitting/not-submitting × actuator list empty/non-empty × telemetry state known/unknown. The visual component should expose the open + representative submitting state; actuator-list conditions are content states.

## 6. Components requiring contract correction in the existing OpenPencil library

1. `StatusPill/*`: extend from the previous 5 invented device states to the seven `DeviceStatePill` states, while keeping command and pump-control status families separate.
2. `Banner/Success`: remove from the P0 React-backed set because `BannerTone` is only `info | warning | danger`.
3. `StateView/*`: retain example instances, but label them as examples of the tone/content matrix rather than API variants.
4. `TelemetryCard/*`: align to `SensorBentoCard`; keep `good | warn | danger | info` as the state axis and avoid theme × tone × compact combinatorial variants.
5. `EmergencyStopButton/*`: keep `Floating` and `BottomBar` as the two contract-backed variants and add the dialog submitting state to the safety matrix.
6. `Panel`, `ActuatorCard`, `TelemetryGroup`, and generic `DataTable`/`Modal` definitions should be treated as structural design primitives until matching production React contracts exist.

## 7. OpenPencil implementation rule

Use component sets only for true visual variants. Use instance properties/content for copy, labels, metric values, units, helper text, icons, and list contents. Prefer a compact state matrix over generating every Cartesian combination.
