# P0.1 Canonical Station/Device Context Design Specification

## 1. Objective & Scope

The purpose of **P0.1 — Canonical Station/Device Context** is to establish one application-level source of truth for selected station/device identity and selection lifecycle in HYDRAGROW, and migrate existing main pages and components to consume it.

### In Scope
- Canonical `StationContext`, `StationProvider`, and `useStationContext` hook.
- Authoritative lifecycle states: `LoadingSelection`, `NoSelection`, `Selected`, `InvalidSelection`, `Unavailable`, `PermissionDenied`.
- Authoritative device source integration (`GET /devices` via `apiGet`).
- Selection lifecycle (`selectDevice`, `clearSelection`, `switchDevice`, `refreshAvailableDevices`).
- Persistence contract: stored preference validated against authoritative device list; no silent device-0 substitution; no fabricated device objects.
- Fleet-to-page handoff: selecting a station in Fleet updates canonical context and navigates to destination.
- Context switching invariant: switching from Device A to Device B guarantees device A data is never presented as device B; transient telemetry and state are reset on transition.
- Page migrations: Dashboard, Operations (ControlPanel), Automation, Cultivation (CropSeasons, RecipeBuilder, DosingHistory), Journal (SystemLog, Analytics), Settings, FleetView, DevicePairing.
- Reusable empty/warning state presentation for pages when selection is missing or invalid.
- Transitional compatibility adapters for `useDeviceStore` and `useDeviceSync`.
- Unit and integration tests for context invariants.

### Out of Scope (P0.2+)
- P0.2 authoritative telemetry/state architecture.
- P0.3 command lifecycle.
- P0.4 configuration consistency overhaul.
- MQTT protocol or backend API modifications.
- Firmware / simulator changes.

---

## 2. Target Context Model & Contract

### 2.1 Boundary
The context owns:
- Selected device identity (`selectedDeviceId`).
- Available devices collection (`availableDevices: DeviceInfo[]`).
- Selection lifecycle status (`status: StationSelectionStatus`).
- Selection validation and persistence.
- Selection transitions (`selectDevice`, `clearSelection`, `refreshAvailableDevices`).

The context does **NOT** own:
- Telemetry or sensor readings (`ec`, `ph`, `temp`, `water_level`).
- Device health scores or diagnostics.
- Command dispatch, queuing, or confirmation.
- Actuator / relay states.
- Automation scripts or execution states.
- Historical logs, journal entries, or crop cycles.

### 2.2 Status Model
```typescript
export type StationSelectionStatus =
  | 'LoadingSelection'   // Querying available devices / validating persisted preference
  | 'NoSelection'        // Available devices loaded, but no device is selected
  | 'Selected'           // A valid device is selected and confirmed in available devices
  | 'InvalidSelection'   // Persisted or requested device ID is not in available devices
  | 'Unavailable'        // Authoritative device API failed or network unreachable
  | 'PermissionDenied';  // API returned 401/403 or user lacks permission to access devices
```

### 2.3 Interface
```typescript
export interface StationContextValue {
  status: StationSelectionStatus;
  selectedDeviceId: string | null;
  selectedDevice: DeviceInfo | null;
  availableDevices: DeviceInfo[];
  error: Error | null;
  selectDevice: (deviceId: string) => void;
  clearSelection: () => void;
  refreshAvailableDevices: () => Promise<void>;
}
```

---

## 3. Selection Lifecycle & Persistence Contract

### 3.1 State Transitions
```text
[Mount / Boot]
       |
       v
LoadingSelection
       |
       +---> Available Devices Load Failure (401/403) ---> PermissionDenied
       +---> Available Devices Load Failure (other)   ---> Unavailable
       |
       +---> Available Devices Loaded
                 |
                 +---> No Persisted Preference        ---> NoSelection
                 |
                 +---> Persisted Preference Exists
                           |
                           +---> Found in List        ---> Selected
                           +---> NOT in List          ---> InvalidSelection

[User Action: selectDevice(id)]
       |
       +---> id in Available Devices                  ---> Selected (persists to storage)
       +---> id NOT in Available Devices              ---> InvalidSelection (does not fabricate)

[User Action: clearSelection()]                       ---> NoSelection (clears storage)

[Available Devices Refresh]
       |
       +---> Selected Device still in list            ---> Remains Selected
       +---> Selected Device missing from list        ---> InvalidSelection (does not auto-substitute)
```

### 3.2 Persistence Semantics
- Persistence storage key: `hydragrow_selected_device_id` (synchronizing with `hydragrow_app_settings.device_id`).
- Persisted identity is treated purely as a **user preference**, not proof of existence.
- On initialization, the stored preference is checked against `GET /devices`.
- If valid -> restored into `Selected`.
- If invalid or stale -> transitions to `InvalidSelection`. Under no circumstances is the first device (`devices[0]`) silently substituted.
- Missing preference -> deterministic `NoSelection`.
- Available devices load failure -> preserves `Unavailable` or `PermissionDenied`; never converted to empty list `[]`.

---

## 4. Context Switching Invariant (A -> B Isolation)

When switching from Device A to Device B:
1. `StationContext.selectDevice(idB)`:
   - Validates `idB` is in `availableDevices`.
   - Sets `selectedDeviceId = idB`.
   - Writes `idB` to persistence.
   - Clears transient telemetry in `useDeviceStore` via `resetTransientDeviceState()`. This resets:
     - `sensorData = null`
     - `deviceStatus = null`
     - `fsmState = null`
     - `controllerHealth = null`
     - `tankAlert = null`
   - Updates `useDeviceStore.deviceId = idB` for backward compatibility.
2. Device-dependent React Query keys:
   - Every device-scoped query includes `[..., selectedDeviceId]`.
   - Switching `selectedDeviceId` naturally unmounts/switches the cache key; stale cache for Device A is never returned for Device B queries.

---

## 5. Fleet Handoff & Page Migration

### 5.1 Fleet Handoff Flow
1. User navigates to Fleet (`/fleet`).
2. Fleet queries `useStationContext()`.
3. User clicks on a Station card.
4. Fleet handler executes:
   ```typescript
   selectDevice(device.device_id);
   navigate('/');
   ```
5. Target page (Dashboard `/`) mounts, reads `useStationContext()`, observes `status === 'Selected'` and `selectedDeviceId === device.device_id`.

### 5.2 Page Migration & Deterministic Empty States
All main pages are updated to consume `useStationContext()`:
- `Dashboard.tsx`:
  - If `status === 'NoSelection'`: renders deterministic `StationEmptyState` prompting user to select a station from Fleet.
  - If `status === 'InvalidSelection'`: renders warning that the previously selected device is not available.
  - If `status === 'Unavailable'`: renders network/service unavailable error with retry.
  - If `status === 'PermissionDenied'`: renders permission denied notice.
  - If `status === 'Selected'`: renders dashboard telemetry and controls.
- `ControlPanel.tsx` (Operations):
  - Consumes `useStationContext()`. Guarded with deterministic empty state if not `Selected`.
- `Automation.tsx`:
  - Consumes `useStationContext()`. Guarded with deterministic empty state if not `Selected`.
- `CropSeasons.tsx`, `RecipeBuilder.tsx`, `DosingHistory.tsx` (Cultivation):
  - Consumes `useStationContext()`. Passes validated `selectedDeviceId` to queries.
- `SystemLog.tsx`, `Analytics.tsx` (Journal):
  - Consumes `useStationContext()`. Shows deterministic empty state if no station is selected.
- `Settings.tsx`:
  - Consumes `useStationContext()`. Device settings tab uses `selectedDeviceId`.
- `FleetView.tsx`:
  - Consumes `useStationContext()`. Highlights currently selected device and uses `selectDevice` on card click.

---

## 6. Compatibility Strategy

### 6.1 `useDeviceStore` Transitional Adapter
- Retain `deviceId` and `setDeviceId` in `useDeviceStore` as a transitional mirror.
- When `StationContext` changes selection, it synchronizes `useDeviceStore.setState({ deviceId })`.
- Add `resetTransientDeviceState()` to `useDeviceStore` to clear old telemetry when switching devices.
- Mark `useDeviceStore.deviceId` as `@deprecated - Use useStationContext() instead`.

### 6.2 `useDeviceSync` Adaptation
- `useDeviceSync` will read `selectedDeviceId` from `useStationContext()` or sync with it.
- Remove silent auto-selection of `devices[0].device_id` when `device_id` is missing.

---

## 7. Test Plan

Comprehensive tests in `hydragrow-frontend/src/contexts/__tests__/StationContext.test.tsx`:
1. **Selection & Lifecycle:**
   - Starts in `LoadingSelection`, transitions to `NoSelection` when no preference is stored.
   - Transitions to `Selected` on valid `selectDevice(id)`.
   - Transitions to `NoSelection` on `clearSelection()`.
   - Transitions to `InvalidSelection` when selecting a non-existent device id.
   - Switching A -> B updates selected device and clears transient state.
2. **Persistence:**
   - Valid stored device ID is restored into `Selected` once available devices load.
   - Invalid stored device ID transitions to `InvalidSelection` without fabricating device info.
   - Missing stored device ID transitions to `NoSelection`.
   - API failure transitions to `Unavailable` (or `PermissionDenied` on 403) and does not convert to empty success or fabricate defaults.
3. **Isolation & Invariants:**
   - Device A telemetry is cleared on switch to Device B.
   - Disappearing device (device removed from available devices list on refresh) transitions to `InvalidSelection` rather than silently auto-selecting another device.
