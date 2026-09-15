# HYDRAGROW Settings — Functional Specification

**Route:** `/settings`  
**Primary role:** persistent configuration owner for the selected device/station  
**Status:** Implementation-ready specification; existing source is partially aligned and has explicit implementation gaps.  
**Canonical checklist:** `docs/design-system/PAGE-SPEC-CHECKLIST.md`

---

## 0. Purpose

Settings is the configuration surface for persistent device behavior, safety limits, dosing calibration, sensor calibration, connectivity, device administration, account/session actions, and configuration backup/restore.

Settings changes **configuration state**. It does not become the owner of live actuator control, automation orchestration, cultivation lifecycle, or canonical historical records.

The page must never imply that saving a configuration proves the physical device has applied or executed it.

### Core semantic boundary

```text
User edit
-> validated configuration payload
-> backend accepted / rejected
-> persisted configuration
-> configuration delivery attempt
-> device-applied confirmation when a source-backed status exists
```

`Persisted != Delivered != Applied != Runtime-observed`.

---

## 1. Page Identity & Responsibility

### 1.1 Primary purpose

Provide one station-scoped place to inspect and modify persistent configuration without duplicating runtime control or historical domains.

### 1.2 User problem

Users need to change operating targets, water behavior, dosing parameters, sensor calibration, connectivity, and device administration while knowing exactly which station is being changed and whether the change was merely saved, accepted, synchronized, or physically confirmed.

### 1.3 Roles

- `admin`: full Settings capabilities subject to backend scope and device ownership.
- `operator`: configuration capabilities granted by `write:config`; device network/OTA/admin actions require their specific scopes.
- `viewer`: read-only configuration visibility where the endpoint permits access; no configuration mutation.

Backend authorization is authoritative. UI role labels must not be treated as permission enforcement.

### 1.4 Settings owns

- Persistent device configuration.
- Safety configuration values.
- Water-management configuration.
- Dosing calibration/configuration.
- Sensor calibration/configuration.
- Control mode configuration (`auto` / `manual`) as persisted runtime ownership setting.
- Application connection settings displayed/edited by the current implementation (`backend_url`, stored API key handling, optional Grafana URL).
- Device WiFi desired configuration.
- Firmware OTA trigger entry point.
- Device reboot and factory reset entry points.
- Configuration backup/export and restore entry points.

### 1.5 Settings does not own

- Live pump/actuator operation -> Operations.
- Automation workflow authoring/execution -> Automation.
- Crop season lifecycle -> Cultivation.
- Full event/history/audit browsing -> Journal.
- Member/role administration -> Roles.
- Device pairing/provisioning ownership -> Pairing/Fleet where applicable.
- Physical actuator confirmation unless an explicit device status source exists.

### 1.6 Entry points

- Main navigation -> `/settings`.
- Device-scoped navigation from Dashboard/Operations where a configuration action is relevant.
- Existing General section links to `/roles`, `/pairing`, `/config-backup`.

### 1.7 Exit points

- `/dashboard`
- `/operations`
- `/automation`
- `/journal`
- `/roles`
- `/pairing`
- `/config-backup`

Every exit must preserve the selected `deviceId` context where the destination is device-scoped.

---

## 2. Source of Truth

### 2.1 Frontend source

Primary files:

- `hydragrow-frontend/src/pages/Settings.tsx`
- `hydragrow-frontend/src/pages/settings/settingsData.ts`
- `hydragrow-frontend/src/pages/settings/GeneralSection.tsx`
- `hydragrow-frontend/src/pages/settings/ThresholdsSection.tsx`
- `hydragrow-frontend/src/pages/settings/ConnectivitySection.tsx`
- `hydragrow-frontend/src/pages/settings/DangerZoneSection.tsx`
- `hydragrow-frontend/src/pages/ConfigBackup.tsx`
- `hydragrow-frontend/src/store/useDeviceStore.ts`
- `hydragrow-frontend/src/hooks/useDeviceSync.ts`
- `hydragrow-frontend/src/platform/settings.ts`
- `hydragrow-frontend/src/platform/http.ts`

### 2.2 Backend source

- `hydragrow-backend/src/api/config.rs`
- `hydragrow-backend/src/api/config_backup.rs`
- `hydragrow-backend/src/api/device_admin.rs`
- `hydragrow-backend/src/api/device_pairing.rs`
- `hydragrow-backend/src/api/scope_definitions.rs`
- `hydragrow-backend/src/models/config.rs`
- `hydragrow-backend/src/services/config_registry.rs`
- `hydragrow-backend/src/services/config_override.rs`

### 2.3 Current API surface

Source-backed routes include:

```text
GET  /api/devices/{deviceId}/config/unified
PUT  /api/devices/{deviceId}/config/unified
GET  /api/devices/{deviceId}/config
PUT  /api/devices/{deviceId}/config
GET  /api/devices/{deviceId}/safety
POST /api/devices/{deviceId}/config/safety
GET  /api/devices/{deviceId}/config/water
POST /api/devices/{deviceId}/config/water
GET  /api/devices/{deviceId}/calibration/sensor
POST /api/devices/{deviceId}/calibration/sensor
POST /api/devices/{deviceId}/calibration/ph/finish
GET  /api/devices/{deviceId}/calibration/sensor/history
GET  /api/devices/{deviceId}/calibration/dosing
POST /api/devices/{deviceId}/calibration/dosing
GET  /api/devices/{deviceId}/ota/status
POST /api/devices/{deviceId}/ota/trigger
GET  /api/devices/{deviceId}/wifi
POST /api/devices/{deviceId}/wifi
POST /api/devices/{deviceId}/reboot
POST /api/devices/{deviceId}/factory-reset
GET  /api/devices/{deviceId}/admin/backup
POST /api/devices/{deviceId}/admin/restore
```

Exact route mounting/prefixes must be verified against the current application router before implementation; the paths above are the device-scoped route forms used by the inspected frontend/backend sources.

### 2.4 Backend persistence behavior

`PUT /config/unified`:

1. Requires `write:config`.
2. Requires device ownership.
3. Upserts device, safety, water, sensor, and dosing configuration.
4. Validates dosing constraints before dosing persistence.
5. Writes a `config_change` system event.
6. Attempts MQTT synchronization to the Controller Node and Sensor Node.
7. Returns `200 success` when DB persistence and sync path complete.
8. Returns `202 partial_success` when DB persistence succeeds but MQTT synchronization fails.

The `202 partial_success` result is not physical application confirmation.

### 2.5 Source-backed vs implementation-required

**Source-backed:** unified load/save, persistent configuration models, role scopes, device ownership checks, config audit event, MQTT sync attempt, WiFi metadata/delivery states, OTA status/trigger, reboot, factory reset, backup export/restore.

**Partially source-backed:** selected-device context, calibration lifecycle, configuration delivery confirmation, visible mutation reconciliation, application settings persistence, navigation protection for dirty drafts.

**Implementation-required:** explicit per-component state UI, authoritative applied-state reconciliation, conflict/version handling for concurrent edits, complete unsaved-draft navigation guard, truthful restore lifecycle, exact device-level scope presentation, and any new detail surfaces proposed by this spec.

---

## 3. Selected Device / Station Context

Settings is device-scoped.

### 3.1 Current source

The selected identity is `useDeviceStore().deviceId`. `useDeviceSync` also persists/loads the application `device_id` and hydrates unified configuration into the store.

No richer station object may be invented unless implemented.

### 3.2 Required context contract

Minimum context:

```text
deviceId
station label when available from an existing source
originating page/intent when navigation requires it
```

### 3.3 Rules

- Load configuration only for the current `deviceId`.
- On device switch, invalidate all editable configuration state before loading the new device.
- Do not display device A configuration under device B.
- Do not silently substitute another device when the requested device is unavailable.
- Missing device context is a distinct state, not an empty configuration.
- Loading a new device must show a loading/switching state before the new form becomes editable.
- Pending save state is keyed by `deviceId`.

### 3.4 Context switching

If a station selector is introduced/integrated:

```text
Current device
-> user selects another device
-> if dirty: apply unsaved-change policy
-> state = switching
-> clear/invalidate old form snapshot
-> load new device configuration
-> state = ready / unavailable / error
```

Current Settings source does not itself render a station selector. Selector integration is `Implementation-required` unless provided by the enclosing shell.

---

## 4. Information Architecture

Current four top-level tabs are canonical:

1. `Tổng quan`
2. `Ngưỡng, Nước & Cảm biến`
3. `Máy châm phân`
4. `Kết nối`

The four-tab structure is source-backed by `SETTINGS_TABS` and the existing Settings tests.

### 4.1 Tổng quan

- Account/session.
- Role display and Roles navigation.
- Control mode configuration.
- Pairing navigation.
- Backup/Restore navigation.
- Advanced/technical mode toggle.
- Device danger zone when applicable.

### 4.2 Ngưỡng, Nước & Cảm biến

Logical sections:

- Ngưỡng mục tiêu.
- Quản lý nước.
- Safety, visible only in advanced mode.
- Cảm biến & Hiệu chuẩn.

### 4.3 Máy châm phân

- Dosing model/calibration.
- Pump capacities.
- Step ratios.
- PWM and pulse parameters.
- Mixing/stabilization parameters.

### 4.4 Kết nối

- Node-RED integration link.
- Grafana URL.
- MQTT outbound integration topic.
- API/backend connection settings where exposed.
- WiFi desired configuration.
- OTA status/trigger.

### 4.5 Secondary surfaces

- PH calibration wizard is inline within Sensor section.
- Confirmation surface for destructive device actions.
- File picker for restore.
- Backup page remains a separate route today; Settings links to it.

---

## 5. Component Inventory

| Component | Source | Interactive | Ownership |
|---|---|---|---|
| Account card | `GeneralSection` | Logout | Auth/session |
| Role card | `GeneralSection` | Navigate | Roles |
| Control Mode | `GeneralSection` | Select | Settings/runtime ownership config |
| Pairing CTA | `GeneralSection` | Navigate | Pairing |
| Backup/Restore CTA | `GeneralSection` | Navigate | Config backup |
| Advanced Mode | `GeneralSection` | Toggle | Settings presentation |
| Save Changes | `Settings.tsx` | Submit | Settings |
| Target thresholds | `ThresholdsSection` | Edit | Settings |
| Water controls | `ThresholdsSection` | Edit/toggle | Settings |
| Schedule | `ThresholdsSection` | Edit | Settings |
| Safety limits | `ThresholdsSection` | Edit | Settings |
| Sensor enable flags | `ThresholdsSection` | Toggle | Settings |
| PH calibration wizard | `ThresholdsSection` + `Settings.tsx` | Measure/advance/save | Settings/calibration |
| Dosing calibration | `ThresholdsSection` | Edit | Settings |
| Node-RED URL | `ConnectivitySection` | Open link | Integration |
| Grafana URL | `ConnectivitySection` | Edit | Application setting |
| MQTT topic | `ConnectivitySection` | Copy | Integration |
| WiFi list | `ConnectivitySection` | Edit/save | Device network |
| OTA status/trigger | `ConnectivitySection` | Trigger | Device admin |
| Reboot | `DangerZoneSection` | Confirm/execute | Device admin |
| Factory Reset | `DangerZoneSection` | Confirm/execute | Device admin |

Any new clickable row, card, label, icon, status, or disclosure control must be added to this inventory before implementation.

---

## 6. Component State Contract

### 6.1 Page/config load

```text
initial
-> loading
-> ready
-> empty/missing-config
-> unavailable
-> error
```

- `loading`: form is non-editable; no stale previous-device values remain visible as current.
- `ready`: current device config is loaded and editable if permission allows.
- `missing-config`: backend explicitly indicates missing/default configuration; do not call this an empty device.
- `unavailable`: device/context cannot be resolved.
- `error`: request failed; preserve no misleading default values as authoritative.

### 6.2 Dirty form

```text
clean
-> dirty
-> validating
-> saving
-> saved / partial-success / rejected / error / unknown
-> clean after reconciliation
```

Dirty state must be derived from a stable loaded snapshot, not from a hard-coded default comparison.

### 6.3 Save button

- `disabled`: no permission, no device, invalid form, or save already pending.
- `ready`: dirty form is valid.
- `saving`: disable duplicate submission and show pending state.
- `accepted/persisted`: show saved result and begin reconciliation.
- `partial-success`: explicitly state DB persistence succeeded but device synchronization failed/was not confirmed.
- `rejected/error`: retain dirty values and show actionable error.
- `unknown`: do not silently reset the form; allow refresh/reconcile.

### 6.4 Advanced Mode

- `off`: advanced fields hidden.
- `on`: advanced fields visible.
- Toggling presentation mode must not mutate device configuration by itself.
- Advanced mode is a UI exposure mechanism, not a security boundary.

### 6.5 PH calibration wizard

States:

`blocked -> ready -> capturing -> captured -> next-point -> summary -> saving -> completed/rejected/unknown`.

Blocked conditions currently include sensor offline or PH error. A local countdown is not physical measurement confirmation.

### 6.6 WiFi

States:

`loading -> ready -> dirty -> saving -> pending -> applied / rolled_back / rejected / unknown / error`.

The existing backend exposes delivery states. Passwords are secret inputs and must never be rendered from persisted metadata.

### 6.7 OTA

States:

`status-loading -> no-update / update-available -> triggering -> accepted / rejected / error / unknown`.

`triggered` is not `installed` or `running`.

### 6.8 Reboot / Factory Reset

Reboot:

`available -> confirming -> dispatching -> accepted / rejected / error / unknown`.

Factory reset:

`available -> confirming -> dispatching -> accepted / rejected / error / unknown`.

Neither accepted command state proves physical reboot/reset completion.

---

## 7. Data Contract

### 7.1 Configuration groups

**DeviceConfig**

- `device_id`
- `ec_target`
- `ec_tolerance`
- `ph_target`
- `ph_tolerance`
- `control_mode`
- `is_enabled`
- `delay_between_a_and_b_sec`
- `last_updated`

**WaterConfig**

- tank height and water level thresholds/tolerance
- automatic refill/drain/dilution flags
- dilution amount
- scheduled water change and cron
- scheduled drain amount
- misting on/off timing
- high-temperature misting threshold/timing
- `last_updated`

**SafetyConfig**

- EC/pH/temperature minimum and maximum limits
- maximum EC/pH change per cycle
- maximum dose per cycle/hour
- cooldown
- critical water level
- refill/drain cycle limits
- refill/drain duration limits
- emergency shutdown flag
- acknowledgement thresholds
- `last_updated`

**SensorCalibration**

- pH calibration points/mode
- EC factor/offset
- temperature offset/compensation beta
- publish interval
- moving-average window
- sensor enable flags
- `last_calibrated`

**DosingCalibration**

- dose effect estimates (`ec_gain_per_ml`, pH shift values)
- mixing/stabilization timings
- step ratios
- pump capacities
- minimum PWM and pulse parameters
- mixing PWM values
- adaptive/tuner fields and learned matrices where returned by backend
- `last_calibrated`

### 7.2 Semantic classes

All values above are configuration values, not telemetry.

- `Configured`: persisted desired value loaded from backend.
- `Commanded`: a value included in a request to a runtime/device endpoint.
- `Persisted`: backend/database accepted and stored the configuration.
- `Delivered`: configuration was sent to a device integration path.
- `Applied`: device explicitly reports the configuration was applied.
- `Observed`: runtime/telemetry confirms a resulting operating state.
- `Estimated`: model/calibration value used to estimate behavior, not a sensor measurement.

Never display `saved` as `applied` without an explicit source.

### 7.3 Formatting

- Preserve backend numeric meaning and units.
- Percentages are displayed as `%`.
- Time fields retain explicit `ms` or `s` units.
- Volumes retain `ml`.
- Tank dimensions retain `cm` where source defines them.
- Do not infer a physical unit from a field name alone when the source is ambiguous.

### 7.4 Timestamps

- `last_updated` = configuration persistence timestamp.
- `last_calibrated` = calibration record timestamp.
- Neither is telemetry freshness.
- `last_seen` elsewhere in the system is device contact information, not configuration application proof.

---

## 8. State Model

| Page state | Meaning | UI behavior |
|---|---|---|
| Initial | Page mounted | Resolve context/settings |
| Loading | Reading configuration | Disable mutation; show loading |
| Ready | Valid config loaded | Show editable/read-only form by permission |
| Empty/missing | No authoritative config exists | Explicit missing state; do not imply device has defaults applied |
| Partial | Some config groups unavailable/defaulted | Mark affected groups; do not silently flatten to healthy state |
| Stale | Local form is older than backend/device state | Reconcile before claiming current |
| Offline | Device/network unavailable | Persistent config may still be saved if backend permits; delivery/applied status remains separate |
| Fault | Calibration/device fault blocks an operation | Explain blocking condition |
| Unknown | Outcome cannot be established | Preserve state; offer refresh/reconcile |
| Permission denied | Backend rejects scope/ownership | Read-only or blocked mutation |
| Network error | Request failed before authoritative result | Keep dirty values; retry |
| Backend error | Backend rejected/failed | Show error; do not mark persisted |
| Mutation pending | Save/admin operation in progress | Prevent duplicate conflicting action |
| Mutation success | Backend reports successful operation | Re-fetch/reconcile |
| Mutation rejected | Explicit rejection | Keep form state; explain reason |
| Timeout | No authoritative result | Mark unknown, not failed physical operation |
| Recovery | Refresh/retry/reconcile in progress | Resolve to ready or explicit failure |

---

## 9. Interaction Contract

### 9.1 Edit a configuration field

```text
User changes field
-> UI updates local draft
-> state = dirty
-> inline validation runs
-> Save becomes available only if valid and permitted
```

No backend mutation occurs merely because a field changes unless a future field is explicitly documented as immediate-save.

### 9.2 Save Changes

```text
User clicks Save
-> validate all client-side constraints
-> build unified payload
-> dispatch PUT /config/unified
-> state = saving
-> backend returns success / partial_success / rejection / error
-> re-fetch unified config
-> reconcile local snapshot
-> expose persisted/delivery state separately
```

Duplicate save while pending is blocked.

### 9.3 Control Mode

Selecting `auto` or `manual` changes the persisted `DeviceConfig.control_mode` on Save. It is not a live actuator command.

The setting defines runtime control ownership/configuration. Operations remains the runtime command surface.

### 9.4 WiFi save

User submits desired SSID list:

1. Validate SSID length, duplicate SSID, password rules, and maximum entries.
2. Preserve known SSIDs when password is blank using `secret_action: keep`.
3. Send the desired WiFi list through the existing device-network path.
4. Show pending state.
5. Reconcile using returned delivery metadata.

`applied` is authoritative only when returned by the backend/device status path.

### 9.5 OTA trigger

Requires the appropriate device OTA permission.

Confirmation is required because the current UI states that the device will reboot and temporarily stop control during update.

After dispatch, navigate to or expose Journal/runtime status where available; do not claim firmware installed from request acceptance alone.

### 9.6 Reboot

Current source asks for confirmation and sends `X-User-Confirmed: true`.

Required semantics:

```text
confirm
-> dispatch reboot
-> pending
-> accepted/rejected/error/unknown
```

No physical completion claim unless an authoritative reboot status is available.

### 9.7 Factory Reset

Immediate generic confirmation is not appropriate here; this is a destructive device-admin action and must use explicit confirmation before dispatch.

The confirmation must state the destructive scope supported by the product contract. The current UI says WiFi, recipe, and safety budget are deleted and the device restarts; implementation must verify that this statement matches actual firmware/backend behavior before release.

No undo is promised.

### 9.8 Backup export

Current backend exports `device_config` and active recipe in schema version 1. The UI creates a JSON download.

Export is read-only and needs no confirmation.

### 9.9 Backup restore

Restore is destructive because it can overwrite current configuration.

Required lifecycle:

```text
select file
-> parse JSON
-> validate schema/version/device target
-> show confirmation summary
-> dispatch restore
-> accepted / rejected / error / unknown
-> refresh configuration
```

Current backend returns `202 restore_triggered` after attempting MQTT command publication. That response must not be described as device-applied confirmation.

The current implementation does not persistently reconcile the restored configuration into the Settings draft; this is `Implementation-required`.

---

## 10. Mutation Lifecycle Contract

### 10.1 Unified Save

```text
User action
-> client validation
-> build payload
-> PUT /config/unified
-> pending
-> backend authorization + device ownership
-> DB upserts
-> audit event
-> MQTT sync attempt
-> response: success / partial_success / rejection / error
-> GET /config/unified
-> compare authoritative persisted snapshot
-> UI = reconciled
```

Important: the backend currently persists before attempting MQTT synchronization. A `partial_success` response therefore means persistence succeeded while device synchronization failed, not that the device rejected the configuration.

### 10.2 Calibration save

The PH wizard captures voltage/confidence locally, computes a summary, applies values to the draft, calls calibration-finish, and then invokes the unified save path.

The calibration summary is derived data. It is not itself a physical sensor-quality guarantee unless the backend/device source explicitly confirms it.

### 10.3 WiFi

WiFi has a stronger delivery-state source than generic configuration. Preserve:

```text
desired
-> command sent
-> pending
-> applied / rolled_back / rejected / unknown
```

Do not collapse these states into `saved`.

### 10.4 OTA / reboot / reset

All are command-style mutations. Their lifecycle must distinguish:

`requested != accepted != executed != physically observed`.

---

## 11. Confirmation / Reversibility Contract

| Action | Confirmation | Reversibility |
|---|---|---|
| Edit fields | No | Reversible before save via draft reset |
| Save config | No generic confirmation unless product introduces high-risk field-specific policy | Reversible by another config save |
| Control mode change | No generic confirmation | Reversible by changing mode |
| WiFi save | No generic confirmation; warn about connectivity effect | Reversible by saving another list; delivery may rollback |
| PH calibration save | Confirm in explicit wizard final action | Reversible by recalibration/save |
| OTA trigger | Confirm before execute | Not immediately reversible once started |
| Reboot | Confirm before execute | Not reversible once dispatched |
| Factory reset | Confirm before execute | Irreversible unless external backup exists |
| Backup export | No | N/A |
| Backup restore | Confirm before execute | Reversible only through another valid restore/configuration save |

Confirmation is about user intent. It does not upgrade backend acceptance into physical confirmation.

---

## 12. Safety & Control Authority

Settings can change safety boundaries and control ownership, so the page is safety-relevant even though it is not the runtime control owner.

### 12.1 Safety boundary

Safety values configure constraints used by runtime logic. Settings must not bypass Safety Gate or claim that editing a limit directly executes an actuator action.

### 12.2 Control ownership

```text
Settings
  = persistent configuration

Operations
  = runtime manual/operational control

Auto Mode
  = closed-loop runtime ownership

Automation
  = workflow orchestration

Safety Gate
  = final safety boundary
```

If multiple writers affect the same configuration/runtime value, Control Authority / Arbitration must be resolved by the backend/runtime architecture. Settings must not implement a private competing priority algorithm.

### 12.3 E-STOP

E-STOP remains an immediate safety action owned by the canonical safety/control path. Settings must not introduce a second E-STOP implementation.

### 12.4 Dangerous configuration

Advanced safety fields are presentation-gated but backend permission remains authoritative. A hidden field is not a security control.

---

## 13. Permission Contract

### 13.1 Configuration

- Read: authenticated user with device access/source-backed read capability.
- Write: `write:config` plus device ownership.
- Viewer: read-only; write request must be rejected by backend.

### 13.2 WiFi

- Write: `device:network` plus device ownership.
- Password is never re-displayed from persisted metadata.

### 13.3 OTA

- Write/execute: `device:ota` plus device ownership.

### 13.4 Reboot/factory reset

- Execute: `device:admin` plus device ownership.

### 13.5 Backup

- Export: backend currently allows `device:admin` or `write:config`.
- Restore: `device:admin`.

### 13.6 UI behavior

Permission denied must produce a distinct disabled/read-only state. Do not hide all evidence of configuration existence merely because mutation is unavailable unless product security policy requires it.

---

## 14. Forms & Validation

### 14.1 Validation layers

```text
field-level validation
-> cross-field validation
-> domain validation
-> backend validation
-> persistence
```

### 14.2 Cross-field invariants

At minimum, implementation must validate relationships that are explicit in source/domain logic, including:

- minimum < target < maximum where the relevant domain contract requires it;
- lower/upper pH and EC bounds remain coherent;
- dose and rate limits are non-negative and internally consistent;
- minimum PWM does not exceed configured operating PWM where the dosing validation requires this;
- pump capacity values are valid according to dosing validation;
- scheduled settings are valid before enabling their schedules;
- WiFi SSIDs are unique and within source-backed length limits;
- a new WiFi SSID cannot be sent with an empty password under the current contract;
- backup schema version is supported before restore.

Do not invent numeric limits that are not source-backed. Where backend validation is the authority, UI validation must defer to backend rejection rather than fabricate a different limit.

### 14.3 Draft behavior

- Draft is local until Save.
- Reset/cancel returns to the last authoritative loaded snapshot.
- Leaving the page with dirty state requires a navigation policy: confirm, discard, or stay.
- Browser back and tab navigation must follow the same policy where technically possible.

Current implementation does not fully provide this dirty-navigation guard; `Implementation-required`.

---

## 15. History & Audit

Settings does not own full history.

### 15.1 Audit events

Backend source already creates `config_change` system events for unified/base/safety configuration changes. The event includes device, actor context when available, timestamp, category, and selected configuration metadata.

Settings must not claim audit persistence beyond the actual backend event source.

### 15.2 Journal boundary

For detailed history, failures, runtime consequences, and event chronology -> Journal.

Settings may link to Journal for relevant configuration-change or device-admin events when the Journal supports an appropriate device/event filter.

---

## 16. Empty / Loading / Error / Unknown

### Configuration load error

Show an explicit error with Retry. Do not render hard-coded defaults as if they were the device's current configuration.

### Missing group

If one config table is absent and backend intentionally returns defaults, the UI must label the state as default/missing according to the response contract rather than implying those values are physically active.

### Offline

Offline does not automatically mean DB persistence is impossible. The page must separately show backend save result and device delivery/application state.

### Unknown save result

Do not show success toast. Preserve draft and provide Reconcile/Retry.

### Partial success

Show:

```text
Đã lưu cấu hình trên hệ thống.
Chưa xác nhận đồng bộ tới thiết bị.
```

Exact wording can be localized, but the semantic distinction is mandatory.

### WiFi unknown

Keep the last known delivery state and expose refresh. Never infer `applied` from a successful HTTP POST alone.

---

## 17. Cross-Page Contract

### Settings -> Operations

Changing control mode or runtime-related thresholds does not navigate automatically to Operations. A deliberate CTA may open Operations with the same `deviceId`.

### Settings -> Automation

Settings changes persistent configuration. Automation remains workflow owner. If a configuration override is active, Settings must not silently overwrite or claim ownership of the temporary override without a defined arbitration contract.

### Settings -> Journal

Configuration changes and device-admin actions can be inspected in Journal. Navigation preserves `deviceId` and relevant event intent where supported.

### Settings -> Roles

Role management remains Roles-owned. Returning preserves device context where possible.

### Settings -> Pairing

Pairing/provisioning is delegated. Settings does not duplicate pairing logic.

### Settings -> Config Backup

The existing separate `/config-backup` route remains the current backup UI owner until/unless it is intentionally folded into Settings.

---

## 18. Cross-Page Handoff Contract

Canonical form:

```text
Origin
-> deviceId / intent
-> destination
-> destination restores context
-> action / inspection
-> return
-> reconcile
```

Rules:

- Never silently change `deviceId` during navigation.
- A navigation success is not mutation success.
- Pending save/admin action continues to be represented after navigation where the application architecture supports it.
- Returning to Settings after an external mutation triggers reconciliation rather than trusting the old local snapshot.
- If destination context is unavailable, show an explicit unavailable state rather than substituting another station.

### 18.1 Component open/detail contract

Current Settings uses mostly inline sections rather than detail drawers. For each expandable section:

- trigger is the section header;
- surface is inline expansion;
- context is the current device and current draft;
- content is the section's configuration group;
- actions are field edits and section-specific commands;
- close is collapse with draft preserved;
- pending mutation does not disappear when the section collapses;
- mobile uses the same inline hierarchy, not a hidden hover interaction.

---

## 19. Desktop & Mobile Contract

### Desktop

- Header contains page title, current context, and primary Save action.
- Four tabs remain visible.
- Configuration groups use cards/subcards and accordions.
- Save action remains discoverable while editing.
- Advanced fields can increase information density without changing ownership.

### Mobile

- Tabs may scroll or wrap according to the existing responsive shell, but all four remain reachable.
- Form fields stack vertically.
- Primary Save action must remain reachable without excessive scrolling; a sticky bottom action is acceptable if it does not cover inputs.
- Destructive confirmations are full-width/modal or bottom-sheet appropriate.
- WiFi entries are stacked; password inputs remain masked.
- PH calibration steps remain linear and touch-friendly.
- No hover-only behavior.
- Touch targets must meet the accessibility contract.

---

## 20. Accessibility

- Every input has an accessible label and unit where applicable.
- Tab controls expose selected state.
- Accordion headers expose expanded/collapsed state.
- Validation errors are associated with the corresponding field.
- Save pending/success/error status is discoverable without relying on color alone.
- Disabled controls explain the blocking reason where useful.
- Confirmation dialogs expose explicit destructive consequences.
- Keyboard focus moves predictably into opened confirmation surfaces and returns to the trigger after close.
- WiFi password fields remain masked and are not copied into visible status text.

---

## 21. Observability

User-visible status must distinguish:

```text
Draft
Saving
Persisted
Partial / not synchronized
Applied (only with explicit source)
Unknown
Rejected
Error
```

For device-admin actions:

- show command request result;
- show error code/message where safe and useful;
- provide Journal/runtime status navigation when available;
- never claim physical completion from HTTP success alone.

For WiFi, expose source-backed delivery states: `pending`, `applied`, `rolled_back`, `rejected`, `unknown`.

### 21.1 Applied-state evidence gate

`Applied` is a source-backed state, not a UI inference.

The implementation may render `Applied` only when an explicit backend/device source identifies that the configuration was applied. The following are not sufficient evidence by themselves:

- HTTP `200` or another successful transport response;
- backend/database persistence;
- MQTT publish/sync attempt;
- `last_updated` timestamp;
- device `last_seen` contact;
- a successful request to a device-admin endpoint;
- the absence of an error response.

If no authoritative applied-state source exists, the UI must use an explicit state such as `Not confirmed`, `Sync pending`, or `Unknown`, according to the actual lifecycle state. This is a truthfulness constraint, not a requirement to invent a new applied-state API.

---

## 22. Ownership & Responsibility Matrix

| Capability/data | Data owner | UI owner | Mutation owner |
|---|---|---|---|
| Persistent device config | Config backend | Settings | Settings via config API |
| Runtime actuator state | Device/runtime | Operations | Operations/control path |
| Control mode | Device config/runtime policy | Settings | Config API/runtime |
| Automation workflow | Automation backend | Automation | Automation |
| Crop season | Cultivation backend | Cultivation | Cultivation |
| Full event history | Journal/event backend | Journal | Event-producing domains |
| Member roles | User/admin backend | Roles | Roles |
| Device pairing | Device pairing/Fleet | Pairing/Fleet | Pairing/Fleet |
| WiFi desired state | Device network config | Settings | Device network API |
| Firmware update | Device admin | Settings | Device admin API |
| Reboot/reset | Device admin | Settings | Device admin API |

Shared display does not imply shared mutation ownership.

---

## 23. Concurrency / Conflict Contract

Configuration is vulnerable to concurrent edits from multiple users, Automation overrides, device-side changes, and backup restore.

Rules:

- Do not assume last-write-wins is safe unless the backend explicitly defines it.
- Every loaded form represents a snapshot.
- Save should reconcile against authoritative backend state before clearing dirty state when conflict detection exists.
- If the backend introduces version/ETag/config revision, the client must send/check it.
- If no version exists, implementation must at minimum re-fetch after save and surface unexpected values rather than silently claiming all draft fields persisted.
- Temporary Automation config overrides must not be mistaken for the permanent Settings baseline.
- Device A save result must never update device B form state.

### 23.1 Automation override boundary

`config_override` in Automation is temporary runtime orchestration. Settings owns the persistent baseline. If an override is active, Settings may show the persistent baseline and an explicit active-override indicator only if a source-backed API provides it.

Do not silently write the temporary override back as the permanent baseline.

### 23.2 Conflict implementation gate

The preferred implementation is an authoritative configuration revision/ETag contract:

```text
load snapshot + revision
-> edit draft
-> save with expected revision
-> backend accepts OR returns conflict
-> reconcile authoritative state
```

If the backend does not expose a revision/ETag contract, the client must not describe re-fetch-after-save as full concurrency protection. The fallback is:

```text
save
-> re-fetch authoritative config
-> compare against intended draft
-> surface divergence
```

This fallback detects some divergence after a mutation but cannot prevent a concurrent overwrite. A future revision contract is therefore an explicit implementation gate, not an assumed capability.

---

## 24. Source-to-UI Traceability

| UI / action | Frontend source | Backend/source | Status |
|---|---|---|---|
| Selected device | `useDeviceStore.deviceId` | device ownership/config endpoints | Source-backed |
| Unified config load | `Settings.tsx` | `GET /config/unified` | Source-backed |
| Unified config save | `Settings.tsx` | `PUT /config/unified` | Source-backed |
| Control mode | `GeneralSection` | `DeviceConfig.control_mode` | Source-backed |
| Targets | `ThresholdsSection` | `DeviceConfig` | Source-backed |
| Water config | `ThresholdsSection` | `WaterConfig` | Source-backed |
| Safety config | `ThresholdsSection` | `SafetyConfig` | Source-backed |
| Dosing config | `ThresholdsSection` | `DosingCalibration` | Source-backed |
| Sensor calibration | `ThresholdsSection` | `SensorCalibration` + calibration endpoints | Partially source-backed |
| WiFi | `ConnectivitySection` | `device_admin` + device WiFi DB/status | Source-backed |
| OTA | `ConnectivitySection` | `device_admin` | Source-backed |
| Reboot | `DangerZoneSection` / `Settings.tsx` | device admin reboot endpoint | Source-backed |
| Factory reset | `DangerZoneSection` / `Settings.tsx` | device admin factory reset endpoint | Source-backed |
| Backup export | `ConfigBackup.tsx` | `config_backup::export_backup` | Source-backed |
| Backup restore | `ConfigBackup.tsx` | `config_backup::import_backup` | Partially source-backed |
| Dirty navigation guard | page-local state | no authoritative source | Implementation-required |
| Applied configuration status | no complete generic UI source | no generic applied-state source identified | Implementation-required |
| Config version/conflict detection | no source-backed page contract | no generic version contract identified | Implementation-required |

---

## 25. Wireframe Contract

### 25.1 Desktop overview

```text
┌──────────────────────────────────────────────────────────────────┐
│ Cài đặt hệ thống                         [Lưu thay đổi]          │
│ selected device / station context                                │
├──────────────────────────────────────────────────────────────────┤
│ Tổng quan | Ngưỡng, Nước & Cảm biến | Máy châm phân | Kết nối  │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│ Active tab content                                               │
│                                                                  │
│ cards / accordions / forms                                       │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│ mutation status / reconciliation                                 │
└──────────────────────────────────────────────────────────────────┘
```

### 25.2 Mobile overview

```text
┌─────────────────────────────┐
│ Cài đặt        [Lưu]       │
│ device context              │
├─────────────────────────────┤
│ tabs / horizontal scroll    │
├─────────────────────────────┤
│ section                     │
│ field                       │
│ field                       │
│ accordion                   │
│ ...                         │
├─────────────────────────────┤
│ save / mutation status      │
└─────────────────────────────┘
```

### 25.3 Critical states

The implementation must provide visual states for:

- configuration loading;
- missing/unavailable configuration;
- validation error;
- saving;
- partial success;
- unknown outcome;
- permission denied;
- PH calibration blocked;
- WiFi pending/rollback/rejected;
- OTA update available/pending;
- factory reset confirmation.

---

## 26. Acceptance Criteria

### AC-01 — Context integrity

**Given** Settings is opened for device A  
**When** configuration loads  
**Then** every displayed configuration value belongs to device A  
**And** device B data is never shown as current.

### AC-02 — Configuration load

**Given** a valid device context  
**When** unified config is requested  
**Then** the five configuration groups are reconciled into the form  
**And** missing/error groups are not represented as confirmed healthy values.

### AC-03 — Dirty draft

**Given** a loaded configuration  
**When** the user changes a field  
**Then** the page enters dirty state  
**And** no persistent mutation occurs until Save.

### AC-04 — Invalid configuration

**Given** a field violates client/domain validation  
**When** the user attempts Save  
**Then** Save is blocked  
**And** the affected fields expose actionable errors.

### AC-05 — Successful save

**Given** a valid dirty configuration and `write:config`  
**When** Save succeeds  
**Then** backend persistence is reported as successful  
**And** the page re-fetches authoritative configuration  
**And** the draft is cleared only after reconciliation.

### AC-06 — Partial success

**Given** DB persistence succeeds but MQTT sync fails  
**When** the backend returns `partial_success`  
**Then** the UI says configuration is persisted but device synchronization is not confirmed  
**And** the UI does not claim physical application.

### AC-07 — Permission

**Given** a viewer without `write:config`  
**When** the user edits/submits configuration  
**Then** the UI is read-only or blocked  
**And** backend `403` is handled as permission denied.

### AC-08 — Device switch

**Given** device A is displayed  
**When** the selected device changes to B  
**Then** old form data is invalidated before B data is rendered  
**And** pending operations remain associated with A.

### AC-09 — WiFi secrets

**Given** persisted WiFi metadata exists  
**When** Settings loads it  
**Then** passwords are not rendered  
**And** blank password for a known SSID means keep only under the current explicit contract.

### AC-10 — WiFi delivery

**Given** WiFi save is accepted  
**When** delivery state is pending/rolled back/rejected  
**Then** Settings shows that exact state  
**And** does not convert it to `applied`.

### AC-11 — PH calibration

**Given** sensor is offline or in PH error  
**When** calibration is opened  
**Then** measurement is blocked  
**And** the blocking condition is visible.

### AC-12 — Calibration completion

**Given** required points are captured  
**When** the user confirms calibration  
**Then** calibration values enter the unified configuration save path  
**And** the result is reconciled before the UI declares completion.

### AC-13 — OTA

**Given** an update is available and user has `device:ota`  
**When** the user confirms the update  
**Then** the OTA request is dispatched  
**And** the UI distinguishes trigger acceptance from firmware installation.

### AC-14 — Reboot

**Given** user has `device:admin`  
**When** the user confirms Reboot  
**Then** the command is dispatched  
**And** accepted is not displayed as physically rebooted.

### AC-15 — Factory Reset

**Given** user has `device:admin`  
**When** the user confirms Factory Reset  
**Then** the destructive request is dispatched once  
**And** no undo is promised  
**And** the UI distinguishes accepted from physically completed.

### AC-16 — Backup restore

**Given** a valid schema-version-1 backup file  
**When** the user confirms restore  
**Then** restore is dispatched  
**And** the UI refreshes configuration afterward  
**And** `202 restore_triggered` is not treated as device-applied confirmation.

### AC-17 — Unknown mutation outcome

**Given** Save or an admin action times out  
**When** no authoritative result is available  
**Then** state becomes unknown  
**And** the user can reconcile/retry  
**And** the page does not claim failure or success without evidence.

### AC-18 — Dirty navigation

**Given** Settings contains unsaved changes  
**When** the user attempts to leave or switch device  
**Then** the defined discard/stay policy is enforced  
**And** unsaved values are not silently lost.

### AC-19 — Cross-page handoff

**Given** Settings navigates to Operations, Journal, Roles, Pairing, or Backup  
**When** the destination opens  
**Then** device context is preserved where applicable  
**And** returning triggers reconciliation where the underlying data may have changed.

### AC-20 — No ownership leakage

**Given** a user wants to operate an actuator or inspect full history  
**When** Settings is used  
**Then** the user is directed to Operations or Journal  
**And** Settings does not duplicate their mutation/history ownership.

---

## 27. Implementation Mapping

### 27.1 Required implementation work

1. Add explicit selected-device/station context UI if not supplied by the application shell.
2. Add authoritative dirty-form snapshot and navigation/device-switch guard.
3. Replace broad `catch(() => null)` configuration loading with explicit missing/error states.
4. Reconcile unified save against authoritative backend state before clearing dirty state.
5. Surface `partial_success` distinctly from success.
6. Add explicit mutation state handling for save, WiFi, OTA, reboot, reset, restore, and calibration.
7. Preserve unknown outcomes rather than turning transport failures into generic success/error.
8. Add source-backed applied/delivery status where the backend/device exposes it; otherwise keep the status unknown/not confirmed.
9. Define conflict/version behavior if backend gains a config revision contract.
10. Make backup restore validate target device/schema before dispatch.
11. Reconcile Settings after returning from Backup/Restore and other mutation-owning pages.
12. Ensure factory-reset copy matches actual backend/firmware behavior.
13. Ensure permission gating mirrors backend scopes without treating UI hiding as authorization.
14. Ensure all clickable/expandable elements have the documented outcome contract.

### 27.1.1 Implementation gates

The following gates must be resolved before the corresponding behavior is considered complete:

1. **Dirty-state gate:** one authoritative loaded snapshot must drive dirty detection; navigation and device switching must use the same discard/stay policy.
2. **Load-error gate:** failed configuration reads must remain distinguishable from missing/default configuration; no fallback default may be presented as authoritative current state.
3. **Mutation lifecycle gate:** Save, WiFi, calibration, OTA, reboot, reset, and restore must distinguish request, pending, accepted/rejected, persistence/delivery where source-backed, and unknown outcomes.
4. **Applied-state gate:** do not add or display `Applied` without an explicit source. Where no source exists, the implementation must stop at `Persisted`, `Delivered`, `Not confirmed`, or `Unknown` as applicable.
5. **Restore gate:** restore must validate schema and target before dispatch and must re-fetch/reconcile afterward. `restore_triggered` is not device-applied confirmation.
6. **Factory-reset gate:** destructive copy and completion semantics must be verified against backend and firmware behavior before the UI contract is treated as final.
7. **Context gate:** all pending mutations and reconciliation results must remain bound to the originating `deviceId`; device switching must invalidate the previous editable snapshot before loading the new one.
8. **Conflict gate:** use a backend revision/ETag when available. Without one, re-fetch-and-detect-divergence is only a fallback detection mechanism, not a concurrency guarantee.

These gates are acceptance boundaries, not suggestions for adding unsupported product capabilities.

### 27.2 Deferred

- Full configuration version history in Settings.
- Generic per-field change history.
- A new Settings-owned audit viewer.
- New station metadata beyond existing device identity.
- New configuration primitives not represented in backend models.

---

## 28. Anti-Pattern Review

Do not:

- treat a successful `PUT` as proof the device physically applied every field;
- treat `partial_success` as full success;
- render hard-coded defaults as current device state after a failed load;
- use advanced mode as a security boundary;
- reveal persisted WiFi passwords;
- duplicate Operations actuator controls in Settings;
- duplicate Automation workflow authoring in Settings;
- duplicate Journal history in Settings;
- silently change selected device during navigation;
- overwrite persistent baseline with temporary Automation overrides;
- invent numeric ranges unsupported by source;
- invent device-applied telemetry/status fields;
- promise undo for factory reset or OTA;
- use a generic confirmation dialog as a substitute for mutation lifecycle state.

---

## 29. Definition of Done

The Settings page is implementation-complete only when:

- [ ] Route and ownership boundaries match this spec.
- [ ] Selected device context is stable and cannot leak stale data.
- [ ] All four canonical tabs are implemented.
- [ ] Every consequential component has explicit state, interaction, validation, and recovery behavior.
- [ ] Save lifecycle distinguishes validation, backend acceptance, persistence, synchronization, and applied/observed status.
- [ ] Partial success and unknown outcomes are visible.
- [ ] WiFi delivery states are preserved and secrets remain hidden.
- [ ] Calibration lifecycle is truthful and reconciled.
- [ ] OTA/reboot/factory reset have explicit confirmation and command-state handling.
- [ ] Backup restore has schema/target validation and reconciliation.
- [ ] Permissions are enforced by backend and reflected by UI.
- [ ] Dirty navigation/device switching is protected.
- [ ] Cross-page context and return reconciliation are implemented.
- [ ] Journal ownership is preserved.
- [ ] Accessibility requirements are verified.
- [ ] Acceptance criteria AC-01 through AC-20 pass.

---

## 30. Current Spec Audit

### 30.1 Checklist status

This document covers the canonical checklist at the documentation-contract level:

- Page identity/responsibility: **Covered**
- Source of truth: **Covered**
- Selected context: **Covered with implementation boundary**
- Information architecture: **Covered**
- Component inventory/state: **Covered**
- Interaction/open/detail contracts: **Covered**
- Data contract: **Covered**
- Page/component state model: **Covered**
- Mutation lifecycle: **Covered**
- Confirmation/reversibility: **Covered**
- Safety/control authority: **Covered**
- Permission: **Covered**
- Navigation/cross-page handoff: **Covered**
- Desktop/mobile: **Covered**
- Forms/validation: **Covered with backend-authority boundary**
- History/audit: **Covered**
- Empty/loading/error/unknown: **Covered**
- Accessibility: **Covered**
- Observability: **Covered**
- Wireframe: **Covered**
- Acceptance: **Covered**
- Implementation mapping: **Covered**
- Source-to-UI traceability: **Covered**
- Ownership matrix: **Covered**
- Concurrency/conflict: **Covered with implementation boundary**

### 30.2 Blocking implementation gaps

1. **Dirty-state/navigation protection** is not fully implemented in current Settings.
2. **Config load errors** are currently collapsed into `null`/default-like UI paths and need explicit error semantics.
3. **Unified save** needs explicit partial-success and unknown-result UI.
4. **Generic applied-state confirmation** is not currently source-backed; do not invent it.
5. **Backup restore** returns a trigger/accepted response and needs post-restore reconciliation.
6. **Factory reset wording** must be verified against actual firmware behavior before release.
7. **Selected station selector** is not present in the inspected Settings page; shared `deviceId` is source-backed but selector integration is implementation-required unless supplied by shell.
8. **Concurrent configuration conflict/versioning** has no generic source-backed contract and must not be implied.

### 30.2.1 Resolution gates for implementation

The blocking gaps above are resolved only when the implementation satisfies these evidence boundaries:

- Dirty state is snapshot-based and protected across page navigation and device switching.
- Load failures are explicit and cannot masquerade as default configuration.
- Mutation results preserve `Commanded != Backend Accepted != Persisted != Delivered != Applied != Observed` where the relevant states exist.
- `Applied` is rendered only from an explicit source; otherwise the UI says that application is not confirmed or unknown.
- Backup restore validates before dispatch and reconciles authoritative configuration afterward.
- Factory-reset wording is locked only after backend/firmware behavior has been verified.
- Selected-device context is preserved through navigation and async mutation completion.
- Conflict handling uses revision/ETag when available; otherwise the UI does not claim prevention of concurrent overwrite.

The implementation does not need to create a new generic applied-state endpoint merely to satisfy this spec. If the current platform has no such source, truthful `Not confirmed`/`Unknown` semantics satisfy the applied-state boundary.

### 30.3 Conclusion

The Settings functional specification is **checklist-complete at documentation-contract level and suitable as an implementation acceptance gate**. It is not a claim that the current Settings code already satisfies every contract above.

The next phase is implementation/audit of the source against this spec, with the blocking gaps above treated as explicit engineering work rather than hidden assumptions.

### 30.4 Implementation sequence

Implementation should proceed in this order so that individual Settings sections do not acquire incompatible lifecycle semantics:

1. **Context foundation:** selected `deviceId`, context switching, stale-data invalidation, pending-operation scoping.
2. **Load state foundation:** loading, missing, partial, unavailable, permission, network, and backend error states.
3. **Draft foundation:** authoritative snapshot, dirty detection, reset/cancel, navigation and device-switch protection.
4. **Mutation foundation:** shared request/pending/result/unknown model and reconciliation after Save.
5. **Device-action lifecycle:** WiFi, calibration, OTA, reboot, and factory reset using their source-backed result states.
6. **Backup/restore:** preflight validation, destructive confirmation, dispatch, and post-restore reconciliation.
7. **Conflict handling:** revision/ETag when the backend supports it; otherwise explicit divergence detection without claiming concurrency prevention.
8. **Cross-page acceptance audit:** verify context restoration and reconciliation for Operations, Automation, Journal, Roles, Pairing, and Backup.

This sequence is normative for implementation planning; it does not change page ownership or add new backend capabilities.
