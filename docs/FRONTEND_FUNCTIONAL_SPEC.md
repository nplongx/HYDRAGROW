# HydraGrow Frontend Functional Specification

> **Document Purpose**: This document is the source of truth for FRONTEND FUNCTIONALITY and PRODUCT BEHAVIOR. It answers "what the product must do" for designers, implementers, QA agents, and future developers.
>
> **Complement to DESIGN.md**: DESIGN.md remains the source of truth for VISUAL LANGUAGE and DESIGN SYSTEM. This document does not prescribe colors, typography, layout, or visual hierarchy.
>
> **Repository Context**: Based on codebase audit of `nplongx/HYDRAGROW` at commit `bdf37b3` (2026-09-13). When this document differs from the current code, trust the code and update this document.

---

## 1. Purpose

This specification defines the complete functional behavior of the HydraGrow frontend application. It serves as the contract between:

1. **Product Design** - to design UI without inventing missing product behavior
2. **Frontend Implementation** - to build correct functionality
3. **QA/Testing** - to derive functional test cases
4. **Future Developers** - to understand the system without seeing the current UI

The specification is evidence-based, derived from actual repository code rather than assumptions.

---

## 2. Scope

**In Scope**:
- All current pages and their functional responsibilities
- Data models, state management, and API contracts
- User roles, permissions, and authorization
- Real-time data synchronization via WebSocket
- Device control and safety mechanisms
- Automation/dosing logic
- Configuration management
- Authentication flows

**Out of Scope**:
- Visual design (colors, typography, spacing, layout)
- Component implementation details
- CSS/styling decisions
- Responsive breakpoints
- Animation specifications

---

## 3. Product Model

### 3.1 Canonical Domain Model (USER DISCOVERY + PRODUCT DECISION)

**Entity Hierarchy**:
```
User
  └── 1..N Stations
        └── 1..N Devices
              ├── exactly 1 Controller Node
              └── exactly 1 Sensor Node
```

**Entity Definitions**:

**USER** [REPOSITORY EVIDENCE]:
- An authenticated person/account via Firebase Authentication
- A user may have access to multiple Stations
- Access may eventually be shared with other users depending on the authorization model
- Has a role: `admin`, `operator`, or `viewer`
- Has scopes (permissions) that determine API access

**STATION** [TARGET PRODUCT DECISION]:
- A first-class product entity
- Represents a physical/operational growing installation
- Owns/groups one or more Devices
- Is the primary organizational and management context
- Must NOT simply be treated as an alias for deviceId
- Identified by `station_id` (string, e.g., "hydra_station_01")
- Has connection state: online/offline/degraded
- Can have a user-assigned label (display name)

**DEVICE** [TARGET PRODUCT DECISION]:
- The primary independently controllable unit inside a Station
- Represents the operational unit for telemetry, control, configuration, and fault state
- A Station may contain multiple Devices
- Device identity must be distinct from Station identity
- Identified by `device_id` (string, e.g., "hydra_device_01")
- Has connection state: online/offline
- Has last-seen timestamp
- Can have a user-assigned label (display name)

**CONTROLLER NODE** [VERIFIED CURRENT INVARIANT]:
- Belongs to exactly one Device
- Is the primary control authority for that Device
- Physical ESP32-C3 device running firmware
- Executes FSM (Finite State Machine) for dosing cycles
- Controls actuators
- Consumes/uses Sensor Node telemetry
- Performs or participates in nutrient concentration regulation/dosing
- Reports health status, pump states, and sensor data
- Communicates via MQTT topics through backend
- Has firmware version and OTA update capability

**SENSOR NODE** [VERIFIED CURRENT INVARIANT]:
- Belongs to exactly one Device
- Provides telemetry to that Device/Controller
- Physical sensor hardware (may be separate from controller)
- Reports sensor readings via MQTT
- Has online/offline state
- Has error states (sensor failure, calibration needed)
- Is not an independent Station
- Is not an independently selectable operational unit in the primary UX
- Must not be treated as a peer of the Controller in the product hierarchy

### 3.2 Explicit Cardinality (PRODUCT DECISION)

| Relationship | Target Cardinality | Current Implementation | Status |
|--------------|--------------------|------------------------|--------|
| User → Station | 1:N | N/A (Station not implemented) | CURRENT GAP |
| Station → Device | 1:N | N/A (Device = Station) | CURRENT GAP |
| Device → Controller Node | 1:1 | VERIFIED (Controller scoped to device_id) | VERIFIED CURRENT INVARIANT |
| Device → Sensor Node | 1:1 | VERIFIED (Sensor scoped to device_id) | VERIFIED CURRENT INVARIANT |
| User → Device access | Derived through Station | Direct 1:N | CURRENT GAP |
| Station → Device ownership | 1:1..N | N/A (not implemented) | CURRENT GAP |

**Key Invariants**:
- A Device always has exactly one Controller Node [VERIFIED CURRENT INVARIANT]
- A Device always has exactly one Sensor Node [VERIFIED CURRENT INVARIANT]
- A User may own multiple Stations [TARGET PRODUCT DECISION]
- A Station may contain multiple Devices [TARGET PRODUCT DECISION]
- Device identity is distinct from Station identity [TARGET PRODUCT DECISION]

**Provenance**: VERIFIED CURRENT INVARIANT (Controller/Sensor), TARGET PRODUCT DECISION (Station/Device relationships), CURRENT GAP (Station entity, Station ownership)

### 3.3 Station vs Device Context (PRODUCT DECISION)

**Two Explicit Application Contexts**:

**STATION CONTEXT**:
- Which Station is currently being viewed/managed?
- Stored in app settings as `station_id`
- Global across all pages
- User switches Stations via Fleet page

**DEVICE CONTEXT**:
- Which Device inside that Station is currently being operated?
- Stored in app settings as `device_id`
- Subordinate to selected Station
- User selects Device within Station via Dashboard

**Context Resolution Rule (PRODUCT DECISION)**:

When a Station has exactly ONE Device:
- Device context may be automatically resolved
- User should not be forced through an unnecessary device-selection step
- UI may transparently show Device-level data without explicit selection

When a Station has MULTIPLE Devices:
- User must be able to select a specific Device
- Device-specific control requires a resolved Device context
- UI must provide Device selection mechanism
- Station-level pages may work without selected Device when their information/action is meaningful at Station scope

Device-level operations must have a resolved Device.

**Provenance**: TARGET PRODUCT DECISION

### 3.4 Page Scope Contract (PRODUCT DECISION)

| Page | Primary Scope | Secondary Scope | Can Control Device? | Can Manage Station? |
|------|---------------|-----------------|---------------------|---------------------|
| Dashboard | Station | Device summaries | No (view-only) | No (view-only) |
| Operations | Device | Controller/Sensor state | Yes | No |
| Cultivation | Station | Device where relevant | No | No |
| Journal | Station | Device/Event | No | No |
| Fleet | User | Station → Device hierarchy | No | Yes |
| Settings | Station or Device | Depends on setting | Yes (device settings) | Yes (station settings) |
| Pairing | Station → Device | Provisioning | No | Yes |
| Config Backup | Device | N/A | No | No |
| Roles | User | Authorization scope | N/A | N/A |

**Provenance**: TARGET PRODUCT DECISION (derived from domain model)

### 3.5 Current Implementation vs Target (CURRENT GAP ANALYSIS)

**Current Implementation** [REPOSITORY EVIDENCE]:
- PostgreSQL schema: Only `device_*` tables, no `station` table
- Backend ownership: `device_ownership` with `user_id, device_id` (no station_id)
- MQTT topics: All scoped to `device_id` only (e.g., `AGITECH/{device_id}/controller/config`)
- Frontend state: `useDeviceStore.ts` stores `deviceId` only (no station_id)
- Fleet page: Shows flat device list (no station grouping)
- Dashboard: Shows device-level data only (no station summary)

**Target Model** [USER DISCOVERY + PRODUCT DECISION]:
- User → Station → Device → Controller/Sensor hierarchy
- Station as first-class entity
- Station-level aggregation and management
- Device selection within Station
- Station-level and Device-level scoping

**Migration Required**:
1. Database schema: Add `station` table, `station_ownership` table
2. Backend: Add station CRUD endpoints, station_ownership endpoints
3. Frontend: Add station_id to state, Station selection UI, Device selection UI
4. Permissions: Add station-level and device-level scopes
5. Alerts: Add station-level aggregation

**Provenance**: REPOSITORY EVIDENCE (current implementation), USER DISCOVERY (target model), CURRENT GAP (migration required)

---

## 4. Terminology

**Station** [TARGET PRODUCT DECISION]:
- The physical/operational growing installation
- The primary organizational and management context
- Contains one or more Devices
- Identified by `station_id`
- **Current Implementation**: Not implemented (Device = Station)

**Device** [TARGET PRODUCT DECISION]:
- The primary independently controllable unit inside a Station
- The operational unit for telemetry, control, and device-level configuration
- Contains exactly one Controller Node and exactly one Sensor Node
- Identified by `device_id`
- **Current Implementation**: Exists but conflated with Station

**Controller Node** [VERIFIED CURRENT INVARIANT]:
- Belongs to exactly one Device
- The primary control authority for that Device
- Executes FSM logic, controls actuators, uses sensor telemetry
- **Current Implementation**: Implemented as part of device (verified via MQTT topics)

**Sensor Node** [VERIFIED CURRENT INVARIANT]:
- Belongs to exactly one Device
- Provides telemetry to that Device/Controller
- Not an independently selectable operational unit
- **Current Implementation**: Implemented as part of device (verified via MQTT topics)

**Station ID** [TARGET PRODUCT DECISION]:
- Unique identifier for a Station
- **Current Implementation**: Not implemented

**Device ID** [REPOSITORY EVIDENCE]:
- Unique identifier for a Device
- **Current Implementation**: Used for both Station and Device (conflated)

**Station Online** [TARGET PRODUCT DECISION]:
- At least one Device in Station is connected to backend
- **Current Implementation**: Not implemented

**Station Offline** [TARGET PRODUCT DECISION]:
- All Devices in Station are disconnected
- **Current Implementation**: Not implemented

**Station Degraded** [TARGET PRODUCT DECISION]:
- Some Devices online, some offline
- **Current Implementation**: Not implemented

**Device Online** [REPOSITORY EVIDENCE]:
- Device's Controller Node is connected to backend (MQTT active)
- **Current Implementation**: Implemented

**Device Offline** [REPOSITORY EVIDENCE]:
- Device's Controller Node not connected
- **Current Implementation**: Implemented

**Sensor Online** [REPOSITORY EVIDENCE]:
- Device's Sensor Node is reporting (within timeout)
- **Current Implementation**: Implemented

**Sensor Offline** [REPOSITORY EVIDENCE]:
- Device's Sensor Node not reporting
- **Current Implementation**: Implemented

**Sensor Stale** [REPOSITORY EVIDENCE]:
- Sensor data not received within timeout
- **Current Implementation**: Implemented

**Semantic Distinctions**:

- **Station ≠ Device**: Station is the installation; Device is the controllable unit within the station. Current implementation conflates these.
- **Device ID ≠ Station ID**: In target model, these are separate identifiers. Current implementation uses only device_id.
- **Controller ≠ Device**: Controller is the hardware node within a Device. Device is the container.
- **Sensor Node ≠ Device**: Sensor Node provides telemetry to a Device. Device contains Sensor Node.

**Actuators**:
- **Pump A**: Nutrient solution A pump
- **Pump B**: Nutrient solution B pump
- **pH Up Pump**: pH raising agent pump
- **pH Down Pump**: pH lowering agent pump
- **Osaka Pump**: High-pressure mixing pump
- **Water Pump In**: Water intake pump
- **Water Pump Out**: Water drain pump
- **Mist Valve**: Misting system valve
- **Mix Valve**: Mixing chamber valve
- Each actuator has on/off state and optional PWM control

**Cultivation Configuration**:
- **Recipe**: Multi-stage nutrient program with EC/pH targets
- **Season**: Crop lifecycle with start/end times
- **Crop Stage**: Individual growth phase with duration and targets
- **Targets**: EC, pH, water level, light hours, dosing limits

**Automation / Dosing Logic**:
- **Control Mode**: `auto` (MIMO algorithm) or `manual` (user control)
- **FSM States**: Booting, Monitoring, ManualMode, WaterRefilling, WaterDraining, MimoDosing, ActiveMixing, Stabilizing, Cooldown, SensorCalibration, Fault, EmergencyStop
- **Safety Limits**: Max dose per cycle, max dose per hour, cooldown periods
- **Adaptive Learning**: Auto-tuning of mixing/stabilize times based on fluid dynamics

**Events / Alerts / History**:
- **System Events**: Categorized logs (System, Dosing, Water, Calibration, Sensor, Alert, UserAction, Device, Automation)
- **Log Levels**: Info, Success, Warning, Critical
- **Alerts**: Real-time notifications for faults, warnings, and important events
- **Journal**: Historical event log with filtering and search

---

## 5. Application Map

### 5.1 Current Routes (TARGET MODEL)

**Primary Pages** (5 work groups):
- `/dashboard` - Dashboard (situational awareness for selected Station and Device)
- `/operations` - Operations (manual control + automation for selected Device)
- `/cultivation` - Cultivation (seasons + recipes + dosing history for selected Device)
- `/journal` - Journal (system events + analytics for selected Station/Device)
- `/settings` - Settings (configuration for selected Device)

**Utility Pages** (accessed from Settings):
- `/pairing` - Device Pairing (QR/manual device claiming within a Station)
- `/fleet` - Fleet View (multi-station management with nested devices)
- `/config-backup` - Configuration Backup/Restore (device-level)
- `/user-management` - User/Role Management (global)
- `/roles` - Role Management (same as user-management)

**Legacy Routes** (redirected to primary pages):
- `/control` → `/operations`
- `/automation` → `/operations`
- `/seasons`, `/crop-seasons`, `/recipes`, `/dosing-history` → `/cultivation`
- `/logs`, `/analytics` → `/journal`

**Authentication Flows**:
- Login screen (unauthenticated entry point)
- Register screen (new user registration)
- Forgot password screen (password reset)
- Session handling (automatic Firebase auth state sync)

### 5.2 Page Responsibility Matrix (TARGET MODEL)

| Page | Primary Responsibility | Secondary Responsibilities | Scope |
|------|----------------------|---------------------------|-------|
| Dashboard | Station-level situational awareness | Device selection within station | Station + Device |
| Operations | Device-level manual control | Device automation, FSM monitoring | Device |
| Cultivation | Device-level cultivation config | Recipe/season management for device | Device |
| Journal | Station-level event history | Device-level events where relevant | Station + Device |
| Settings | Device-level configuration | Station-level settings (future) | Device + Station |
| Pairing | Device claiming within station | Station creation/selection | Station + Device |
| Fleet | Multi-station overview | Device listing within stations | Station + Device |
| Config Backup | Device-level config backup | N/A | Device |
| Roles | User/role management | Permission matrix display | Global |

### 5.3 Context Model (TARGET MODEL)

**Selected Station Context**:
- Stored in app settings as `station_id`
- All pages operate within a selected station
- Fleet page allows station switching
- Station context preserved across navigation

**Selected Device Context**:
- Stored in app settings as `device_id`
- Subordinate to selected station
- Dashboard allows device selection within station
- Device context preserved across navigation
- Operations, Cultivation, Settings operate on selected device

**Current Implementation Gap**:
- Frontend currently only tracks `device_id`
- No `station_id` tracking
- Fleet page shows devices directly (no station grouping)
- Dashboard shows device-level data only (no station summary)

**Provenance**: USER DISCOVERY + PRODUCT DECISION

---

## 6. Global State Semantics

### 6.1 Connection States (TARGET MODEL)

| State | Meaning | Display Semantics |
|-------|---------|-------------------|
| **STATION_ONLINE** | At least one Device in Station is connected to backend | Green indicator, "Trạm Online" |
| **STATION_OFFLINE** | All Devices in Station are disconnected | Red indicator, "Trạm Offline" |
| **STATION_DEGRADED** | Some Devices online, some offline | Yellow indicator, "Trạm Giảm Chất Lượng" |
| **DEVICE_ONLINE** | Device's Controller Node is connected to backend (MQTT active) | Green indicator |
| **DEVICE_OFFLINE** | Device's Controller Node not connected | Red indicator |
| **SENSOR_ONLINE** | Device's Sensor Node is reporting (within timeout) | Green indicator |
| **SENSOR_OFFLINE** | Device's Sensor Node not reporting | Red indicator |
| **SENSOR_STALE** | Sensor data not received within timeout | Yellow warning |
| **UNKNOWN** | Connection state not yet determined | Loading state |
| **UNAVAILABLE** | Station/Device ID not configured | Error state, redirect to settings |

### 6.2 Data Semantics (TARGET MODEL)

| Value | Meaning | Distinction |
|-------|---------|-------------|
| **0** | Valid measured zero (e.g., EC = 0 ppm) | Real zero value |
| **null** | No data available | Sensor not reporting |
| **unknown** | Data not yet loaded | Initial state |
| **error** | Sensor hardware failure | `err_*` flag set |
| **offline** | Sensor node not connected | No MQTT data |

### 6.3 System Health States (TARGET MODEL)

| State | Meaning | User Action Required |
|-------|---------|---------------------|
| **NORMAL** | All systems operational, no warnings | None |
| **WARNING** | Non-critical issue (e.g., low tank level) | Monitor, plan maintenance |
| **CRITICAL** | System fault requiring intervention | Immediate action required |
| **EMERGENCY** | Emergency stop active | Acknowledge and resolve |
| **DEGRADED** | Partial functionality (e.g., one sensor failed) | Repair or recalibrate |

---

## 7. Station Context (TARGET MODEL)

### 7.1 Station Selection (TARGET MODEL)

**Initial Selection**:
- Web mode: Automatically selects first owned Station if none selected
- Tauri mode: Requires manual Station ID entry in settings
- Selection persisted in app settings (`station_id` field)

**Device Selection within Station**:
- After Station selected, user must select a Device within that Station
- Dashboard shows Device list for selected Station
- Device selection persisted in app settings (`device_id` field)
- If Station has only one Device, auto-select that Device

**Switching Stations**:
- Via Fleet page: Click "Chọn trạm này" on Station card
- Device selection resets (must select Device within new Station)
- WebSocket connection re-establishes with new Station context
- All page data invalidates and reloads

**Missing Selection**:
- Web mode: Auto-selects first Station if available
- Tauri mode: Blocks entire layout, redirects to settings
- Shows "Chưa chọn trạm" error state

**Deleted Station**:
- If current Station is unclaimed: `station_id` set to null
- `device_id` also set to null
- Redirects to Fleet for new Station selection
- Historical data preserved in backend

### 7.2 Multi-Station Users (TARGET MODEL)

**Fleet Management**:
- Users can own/operate multiple Stations
- Fleet page shows all owned Stations with nested Devices
- Stations grouped by location/crop type when applicable
- Warning-first sorting surfaces problematic Stations

**Context Switching**:
- Station context is global (stored in Zustand)
- Device context is global (stored in Zustand)
- All pages operate on currently selected Station and Device
- Switching Station invalidates Device selection
- WebSocket connection re-establishes with new context

**Permission Scope**:
- User roles apply across all owned Stations
- Station-specific permissions not implemented (future)
- All Stations share same user access level

### 7.3 Current Implementation Gap

**Missing Station Context**:
- Frontend currently has no Station concept
- Only Device selection exists
- Fleet page shows flat device list (no station grouping)
- No station_id in state management
- No station-level API endpoints

**Migration Required**:
- Add station_id to Zustand store
- Add Station selection UI to Fleet
- Add Device selection UI to Dashboard
- Update WebSocket to support station context
- Update all API calls to include station_id

**Provenance**: USER DISCOVERY + PRODUCT DECISION

---

## 8. Authentication

### 8.1 Authentication Flow (REPOSITORY EVIDENCE)

**Login**:
- Email/password authentication via Firebase
- Google OAuth authentication via Firebase
- Firebase ID token stored and attached to all API requests
- Session state managed by `AuthContext`
- Auto-refresh of ID token handled by Firebase SDK

**Registration**:
- Email/password registration via Firebase
- No additional profile data required initially
- Default role assigned by backend (requires admin assignment for full access)

**Forgot Password**:
- Email-based password reset via Firebase
- User receives reset link via email
- Password reset completed in Firebase flow

**Session Handling**:
- Firebase auth state listener auto-updates session
- Session expiration handled by Firebase SDK
- Unauthenticated state shows login screen
- Mock auth mode available for development (`mock_auth=true`)

**Session Expiration**:
- Firebase handles token refresh automatically
- Expired session triggers login screen
- User data preserved locally for re-authentication

### 8.2 Authorization (REPOSITORY EVIDENCE)

**Scopes**:
- Admin: `['*']` (full access)
- Operator: `['read:telemetry', 'write:config', 'control:pump', 'control:emergency', 'device:ota', 'device:network', 'script:write', 'recipe:write']`
- Viewer: `['read:telemetry']` (read-only)

**Scope Enforcement**:
- Backend middleware validates scopes on each API request
- Frontend uses role to conditionally show/hide UI elements
- Scope check performed before sensitive operations

**Role-Based Access**:
| Capability | Admin | Operator | Viewer |
|------------|-------|----------|--------|
| View dashboard | ✅ | ✅ | ✅ |
| View telemetry | ✅ | ✅ | ✅ |
| Control devices | ✅ | ✅ | ❌ |
| Emergency stop | ✅ | ✅ | ❌ |
| Edit configuration | ✅ | ✅ | ❌ |
| OTA updates | ✅ | ✅ | ❌ |
| Network configuration | ✅ | ✅ | ❌ |
| Write automation scripts | ✅ | ✅ | ❌ |
| Edit recipes | ✅ | ✅ | ❌ |
| Manage users | ✅ | ❌ | ❌ |
| Device pairing | ✅ | ✅ | ❌ |

**Current Implementation Gap**:
- Permissions are currently device-scoped only
- No station-level permissions
- No device-specific permissions within a station
- All devices in a station share the same access level

**Migration Required**:
- Add station-level permission scopes
- Add device-level permission scopes
- Update permission matrix to support hierarchical permissions
- Update backend middleware to check station/device context

**Provenance**: REPOSITORY EVIDENCE + PRODUCT DECISION

---

## 9. Alert / Fault Model

### 9.1 Alert Categories (REPOSITORY EVIDENCE)

**Informational**:
- System status changes
- Configuration updates
- OTA completion
- Log level: `info`

**Warning**:
- Tank level low
- Sensor near tolerance limit
- Minor performance degradation
- Log level: `warning`

**Fault**:
- Dosing failure (EC/pH not responding)
- Water refill/drain failure
- Sensor timeout
- Hardware fault detected
- Log level: `critical`

**Critical**:
- Emergency stop active
- Water level critical
- Multiple concurrent faults
- System failure
- Log level: `critical`

### 9.2 Alert Behavior (REPOSITORY EVIDENCE)

**Persistence**:
- Alerts stored in backend database
- Frontend maintains last 50 alerts in Zustand
- Older alerts scroll off as new ones arrive

**Acknowledgement**:
- No explicit acknowledgement mechanism
- Alerts auto-dismiss from toast after timeout
- Historical alerts always visible in Journal

**Resolution**:
- Faults require explicit reset action
- Warnings auto-resolve when condition clears
- Emergency stop requires user acknowledgement

**User Action**:
- Critical faults: immediate intervention required
- Warnings: monitoring advised
- Informational: no action required

**Dashboard Display**:
- Critical faults: red banner, immediate visibility
- Warnings: yellow banner, recommended action
- Informational: toast notification only

**Journal Display**:
- All alerts logged with full metadata
- Filterable by level, category, time range
- Exportable for analysis

### 9.3 Fault Codes (REPOSITORY EVIDENCE)

**EC_DOSING_FAILED**: Nutrient pump running but EC not changing
**PH_DOSING_FAILED**: pH pump running but pH not changing
**WATER_REFILL_FAILED**: Water pump running but level not rising
**WATER_DRAIN_FAILED**: Drain pump running but level not falling
**TOO_MANY_REFILLS**: Exceeded max refill cycles per hour
**TOO_MANY_DRAINS**: Exceeded max drain cycles per hour
**MAX_HOURLY_DOSE_EC**: Exceeded max EC dose per hour
**MAX_HOURLY_DOSE_PH**: Exceeded max pH dose per hour
**SENSOR_TIMEOUT**: Sensor not reporting within timeout
**EC_STAGNANT**: EC value not changing despite dosing
**PH_OSCILLATING**: pH value oscillating excessively
**WATER_LEVEL_CRITICAL**: Water level below critical minimum
**EMERGENCY_STOP**: Emergency stop triggered
**OSAKA_RUNNING_WITHOUT_VALVE**: Safety interlock violation

### 9.4 Alert Scoping (TARGET MODEL)

**Station-Level Alerts**:
- Station offline (all devices disconnected)
- Station degraded (some devices offline)
- Station-level configuration errors
- Station-level security alerts

**Device-Level Alerts**:
- Device offline
- Device fault
- Device emergency stop
- Device dosing failures
- Device sensor errors

**Current Implementation Gap**:
- All alerts are currently device-scoped
- No station-level alert aggregation
- No distinction between station and device alerts in UI

**Migration Required**:
- Add station-level alert aggregation logic
- Update alert model to include station_id
- Update Dashboard to show station-level alerts
- Update Journal to filter by station/device

**Provenance**: REPOSITORY EVIDENCE + PRODUCT DECISION

---

## 10. Command Lifecycle (REPOSITORY EVIDENCE)

### 10.1 Command States

```
requested → authorized → validated → dispatched → pending → acknowledged → executing → completed
```

**Possible Branches**:
- `rejected` (validation failed, permission denied)
- `timed out` (no response from device)
- `cancelled` (user cancelled or superseded)
- `failed` (device execution failed)
- `superseded` (new command replaced old command)
- `interrupted by safety mechanism` (interlock or emergency stop)
- `interrupted by emergency stop` (emergency stop activated)

### 10.2 Command Flow (REPOSITORY EVIDENCE)

**User Intent**:
- User clicks control button in UI
- Intent captured as command type and parameters

**Validation**:
- Frontend checks: device online, not in fault, not in auto mode
- Backend checks: user scope, safety limits, interlocks
- Device checks: FSM state, physical safety mechanisms

**Authorization**:
- User scope validated by backend middleware
- Firebase ID token attached to request
- API key validated for device access

**Dispatch**:
- Command sent via MQTT topic to device
- Command includes nonce and timestamp for idempotency
- Timeout set for device response

**Pending**:
- Command queued in device message buffer
- UI shows "pending" state
- No physical action yet

**Acknowledged**:
- Device confirms command receipt
- UI shows "acknowledged" state
- Physical execution begins

**Executing**:
- Device actuates hardware
- Real-time feedback via pump status updates
- UI shows current actuator state

**Completed**:
- Device reports successful execution
- UI updates to final state
- Command lifecycle ends

### 10.3 Failure Modes (REPOSITORY EVIDENCE)

**Device Offline**:
- Command rejected immediately
- Error message: "Hệ thống Ngoại tuyến"
- No command sent to device

**Command Duplicated**:
- Idempotency via nonce prevents duplicate execution
- Duplicate command ignored by device
- Original command proceeds

**Actuator No Response**:
- Device acknowledges but actuator doesn't change state
- Fault detection triggers after timeout
- System enters fault state

**State Feedback Disagrees**:
- Command sent but device reports different state
- Safety interlock may have blocked action
- User notified of conflict

**Safety Interlock Blocks**:
- Interlock prevents conflicting actuator states
- Command rejected at device level
- User shown interlock warning

**Emergency Stop Active**:
- All commands blocked except emergency stop acknowledge
- Emergency stop must be cleared first
- New commands rejected until resolved

**Automation Controlling Actuator**:
- Manual commands blocked in auto mode
- User must switch to manual mode first
- UI shows overlay blocking manual controls

---

## 11. Legacy and Migration Notes

### 11.1 Legacy Routes (REPOSITORY EVIDENCE)

**Current Legacy Routes** (redirected):
- `/control` → `/operations`
- `/automation` → `/operations`
- `/seasons` → `/cultivation`
- `/crop-seasons` → `/cultivation`
- `/recipes` → `/cultivation`
- `/dosing-history` → `/cultivation`
- `/logs` → `/journal`
- `/analytics` → `/journal`

**Migration Strategy**:
- Legacy routes retained as `<Navigate replace>` for backward compatibility
- Deep links from old notifications/bookmarks continue to work
- No deletion of legacy routes in current implementation
- Future: Consider removing legacy routes after deprecation period

**Missing Functionality**:
- No legacy functionality intentionally omitted
- All legacy features merged into new page groups
- No unimplemented features discovered

### 11.2 Domain Model Migration (PRODUCT DECISION REQUIRED)

**Current State**:
- User → Device (single level)
- No Station concept
- No Device as container for Controller/Sensor

**Target State**:
- User → Station → Device → Controller/Sensor
- Station as logical installation
- Device as controllable unit
- Controller/Sensor as hardware nodes

**Migration Steps**:

1. **Database Schema Migration**:
   - Add `station` table with `station_id`, `name`, `location`, `created_at`
   - Add `station_ownership` table with `user_id`, `station_id`
   - Add `station_id` column to `device_ownership` table
   - Add `station_id` column to all device_* tables
   - Create migration script to assign existing devices to default stations
   - Backfill station_id for existing device_ownership records

2. **Backend API Migration**:
   - Add station CRUD endpoints
   - Add station_ownership endpoints
   - Update device endpoints to require station_id context
   - Update MQTT topic hierarchy to support station-level topics
   - Add station-level aggregation endpoints

3. **Frontend State Migration**:
   - Add `station_id` to Zustand store
   - Add Station selection UI components
   - Update Fleet page to show station hierarchy
   - Update Dashboard to show station summary + device selection
   - Update all pages to pass station_id to API calls

4. **Permission Model Migration**:
   - Add station-level scopes (e.g., `station:read`, `station:write`)
   - Add device-level scopes (e.g., `device:control`, `device:configure`)
   - Update permission matrix
   - Update middleware to check hierarchical permissions

5. **Alert/Event Migration**:
   - Add station_id to alert model
   - Add station-level aggregation
   - Update Dashboard to show station-level alerts
   - Update Journal to filter by station/device

**Migration Risk**: HIGH
- Requires database schema changes
- Requires breaking API changes
- Requires frontend state model changes
- Permission model changes
- Data migration script required for existing devices

**Recommended Approach**:
- Phase 1: Add Station table and ownership (read-only)
- Phase 2: Migrate existing devices to default stations
- Phase 3: Add station-level UI (Fleet, Dashboard)
- Phase 4: Add device selection within stations
- Phase 5: Update permissions and API endpoints
- Phase 6: Remove legacy flat device model

**Provenance**: USER DISCOVERY + PRODUCT DECISION

---

## 12. Current vs Target Gap Analysis

### 12.1 Domain Model

| Aspect | Current Implementation | Target Behavior | Gap |
|--------|------------------------|-----------------|-----|
| User relationship | User owns Devices directly | User owns Stations | Station layer missing |
| Station contents | Device = Station | Station contains Devices | Station concept not implemented |
| Device contents | Hardware scoped by device_id | Device as container for Controller/Sensor | Device not conceptualized as container |
| MQTT hierarchy | Device ID only | Potential station/device context | No station-level scoping |
| Database | Device-centric tables | Station and Device entities | Station table missing |
| Ownership | device_ownership(device_id) | Station ownership, Device ownership under Station | Station ownership missing |
| Frontend context | User → Device (deviceId) | User → Station → Device | Station context missing |

### 12.2 Dashboard

| Area | Current Implementation | Target Behavior | Gap |
|------|------------------------|-----------------|-----|
| Scope | Device-level only | Station-level + Device selection | Station summary missing |
| Device selection | Not applicable (single device) | Device list within station | Device selection UI missing |
| Station-level alerts | Not applicable | Station offline/degraded alerts | Station alert aggregation missing |

### 12.3 Fleet

| Area | Current Implementation | Target Behavior | Gap |
|------|------------------------|-----------------|-----|
| Display | Flat device list | Station hierarchy with nested devices | Station grouping missing |
| Station switching | Device switching only | Station switching + device selection | Station selection UI missing |

### 12.4 Operations

| Area | Current Implementation | Target Behavior | Gap |
|------|------------------------|-----------------|-----|
| Scope | Device-level | Device-level (no change) | None |
| Context | deviceId only | station_id + deviceId | Context update required |

### 12.5 Cultivation

| Area | Current Implementation | Target Behavior | Gap |
|------|------------------------|-----------------|-----|
| Scope | Device-level | Device-level (no change) | None |
| Context | deviceId only | station_id + deviceId | Context update required |

### 12.6 Journal

| Area | Current Implementation | Target Behavior | Gap |
|------|------------------------|-----------------|-----|
| Scope | Device-level events | Station-level + Device-level events | Station-level filtering missing |

### 12.7 Settings

| Area | Current Implementation | Target Behavior | Gap |
|------|------------------------|-----------------|-----|
| Scope | Device-level | Device-level + Station-level (future) | Station settings missing |

### 12.8 Pairing

| Area | Current Implementation | Target Behavior | Gap |
|------|------------------------|-----------------|-----|
| Flow | Claim device directly | Select station → claim device within station | Station selection missing |

### 12.9 Permissions

| Area | Current Implementation | Target Behavior | Gap |
|------|------------------------|-----------------|-----|
| Scope | Device-level only | Station-level + Device-level | Hierarchical permissions missing |

---

## 13. Open Questions / Product Decisions

### 13.1 Resolved Decisions

**Q1: Station is a first-class entity** [RESOLVED - TARGET PRODUCT DECISION]
**Q2: User may have multiple Stations** [RESOLVED - TARGET PRODUCT DECISION]
**Q3: Station may have multiple Devices** [RESOLVED - TARGET PRODUCT DECISION]
**Q4: Device contains Controller + Sensor pair** [RESOLVED - VERIFIED CURRENT INVARIANT]
**Q5: Station is primary organizational context** [RESOLVED - TARGET PRODUCT DECISION]
**Q6: Device is primary operational control context** [RESOLVED - TARGET PRODUCT DECISION]

### 13.2 Remaining Open Questions

**MQTT Topic Hierarchy** [IMPLEMENTATION QUESTION]:
- Does MQTT need Station identity?
- Investigation required: Does the device/backend architecture actually needs station_id in MQTT topics?
- **Recommendation**: Transport-level representation is an implementation concern and must be evaluated independently. Only require Station information in MQTT if the device/backend architecture actually needs it.

**Permission Inheritance/Override** [OPEN QUESTION]:
- Does Station access imply Device access?
- Is Device-level override necessary?
- Can shared Station access be required?
- Can read-only access exist at Station level?
- Does control permission differ from configuration permission?

**Station Status Aggregation Rules** [OPEN QUESTION]:
- Exact threshold rules for Station status (NORMAL/DEGRADED/CRITICAL)
- Does one critical Device make the Station critical?
- Are weighted thresholds needed?
- Is aggregation computed frontend/backend?
- What remains the source of truth

**Device Movement Between Stations** [OPEN QUESTION]:
- Can a Device move between Stations?
- What happens to historical data when Device moves?
- Can a Device belong to multiple Stations simultaneously?

**Station Creation during Pairing** [OPEN QUESTION]:
- Does pairing create a Station?
- Can a Device be added to an existing Station?
- Is Station creation implicit during pairing?

**Backup Scope** [OPEN QUESTION]:
- Is backup scope Station, Device, Controller, or combination?
- What is included/excluded in backup?

**Cultivation Configuration Scope** [OPEN QUESTION]:
- Are recipes/seasons Station-level or Device-level?
- Does a recipe apply to all Devices in a Station or one Device?

**Notification Preferences** [OPEN QUESTION]:
- Exact notification preferences per category
- Can user opt in/out of specific alert types?

**Emergency Stop Acknowledgement Lifecycle** [OPEN QUESTION]:
- Exact acknowledgement requirements
- Does E-stop require manual reset or auto-clear?

### 13.3 Migration Timeline [ENGINEERING ROADMAP]

Migration timeline should be treated as an ENGINEERING ROADMAP item, not a product UX question.

### 13.4 Assumptions [REQUIRES VALIDATION]

**Multi-Device Stations as Future Requirement** [ASSUMPTION]:
- Multi-device stations may be architecturally supported without being empirically proven as the dominant customer model
- Requires validation from production data or user research

**Station-Level Operations Value** [ASSUMPTION]:
- Station-level operations are valuable
- Requires validation from user research

**Provenance Classification**:
- REPOSITORY EVIDENCE: Current implementation details
- USER DISCOVERY: Target domain model from docs/discovery/2026-09-12-user-discovery.md
- PRODUCT DECISION: Canonical domain model, page scope, context rules
- ASSUMPTION: Items requiring validation
- OPEN QUESTION: Genuine product decisions needed
- IMPLEMENTATION QUESTION: Technical implementation details (MQTT, backend architecture)

---

## 14. Global Acceptance Criteria

### 14.1 Page-Level Criteria (TARGET MODEL)

- **Dashboard**: Provides station-level situational awareness, allows device selection within station, distinguishes online/offline/stale, shows actionable alerts, respects permissions
- **Operations**: Enables device-level manual control with safety interlocks, displays FSM state, manages automation scripts, handles emergency stop
- **Cultivation**: Manages device-level seasons and recipes, allows recipe activation, shows dosing history, validates all inputs
- **Journal**: Displays station-level and device-level events with filtering, shows analytics, allows export, respects read-only access
- **Settings**: Configures device-level parameters, validates inputs, handles dangerous operations with confirmation, manages OTA and WiFi
- **Pairing**: Pairs devices within stations, validates device IDs, manages device list, handles camera permissions
- **Fleet**: Shows stations with nested devices, allows station switching and device selection, groups by crop type, respects permissions
- **Config Backup**: Exports and imports device-level configuration, validates files, requires confirmation for import, handles device offline
- **Roles**: Manages users and roles, shows permission matrix, enforces admin-only access, validates inputs

### 14.2 Cross-Page Criteria (TARGET MODEL)

- Every page has a clearly defined responsibility
- No two pages ambiguously own the same business action
- Physical control actions have explicit preconditions
- Offline and zero telemetry are semantically distinct
- Station context is consistent across pages
- Device context is consistent across pages
- Permissions are enforced consistently
- Critical faults have clearly defined behavior
- Every important user workflow can be traced across pages
- Legacy routes have an explicit migration strategy
- Designers can design UI without inventing missing product behavior

### 14.3 Data Integrity Criteria (TARGET MODEL)

- WebSocket updates preserve data semantics
- Telemetry of 0 is distinguished from no data
- Error flags are distinguished from offline state
- Stale data is distinguished from offline
- Last-seen semantics are consistent
- Controller and sensor online states are independent
- Station online state aggregates device online states

### 14.4 Safety Criteria (TARGET MODEL)

- Emergency stop always accessible
- Dangerous operations require confirmation
- Interlocks prevent conflicting commands
- Auto mode blocks manual control
- Fault state blocks normal operations
- Validation prevents invalid configurations
- Device offline blocks most operations

### 14.5 Permission Criteria (TARGET MODEL)

- All actions respect role-based permissions
- Scope checks enforced at API level
- Frontend UI reflects permissions
- Admin-only pages properly restricted
- Viewer cannot perform write operations
- Station-level permissions enforced where applicable
- Device-level permissions enforced where applicable

### 14.6 Error Handling Criteria (TARGET MODEL)

- Network errors are retryable with clear messages
- Backend errors show specific failure reasons
- Validation errors are field-specific
- Authorization errors explain permission denial
- Timeout errors allow retry
- Device offline states are clearly indicated
- Station offline states are clearly indicated
- Device selection errors are clearly indicated

### 14.7 State Management Criteria (TARGET MODEL)

- Loading states are distinct from empty states
- Error states are distinct from offline states
- State transitions are predictable
- Station context is preserved across navigation
- Device context is preserved across navigation
- WebSocket reconnection is automatic
- Cache invalidation is appropriate

### 14.8 Migration Criteria (TARGET MODEL)

- Domain model migration is planned and documented
- Database schema migration is backward-compatible
- API migration supports legacy clients during transition
- Frontend migration handles missing station_id gracefully
- Permission migration preserves existing access
- Alert migration preserves historical data
- Data migration script is tested on production-like data

---

## 15. Page Specifications

The following page specifications describe the TARGET MODEL behavior. Current implementation gaps are noted where applicable.

## 16. Dashboard

### 16.1 Purpose (PRODUCT DECISION)

Provide Station-level situational awareness. Dashboard is NOT:
- a Device configuration page
- a deep diagnostic tool
- a recipe editor
- a raw telemetry viewer
- a generic container for every existing component

### 16.2 User Job To Be Done (STATION-SCOPE)

"When I open the HydraGrow app, I want to immediately see if my selected Station is healthy and whether any action is required, so that I can quickly identify and address issues without navigating through multiple screens."

### 16.3 Primary User Questions (STATION-FIRST SEMANTICS - PRODUCT DECISION)

At Station level, Dashboard must answer:
1. Is the Station operational?
2. How many Devices belong to it?
3. Which Devices are online/offline?
4. Which Devices are actively running something?
5. Are there Station-level alerts?
6. Which Device(s) need attention?
7. What are the important current telemetry states across Devices?
8. Is there an immediate action required?

### 16.4 Primary Users / Roles

- **Admin**: Full Station situational awareness
- **Operator**: Full Station situational awareness
- **Viewer**: Read-only Station situational awareness

### 16.5 Entry Conditions (TARGET MODEL)

- User must be authenticated
- Station ID must be selected (auto-selected in web mode)
- Device ID selected automatically if Station has one Device
- Device ID must be explicitly selected if Station has multiple Devices
- WebSocket connection established (fallback to HTTP polling)
- If Station ID missing: show error state and redirect to Fleet

### 16.6 Data Inputs (TARGET MODEL)

| Data | Source | Purpose | Required | Update Behavior | Freshness |
|------|--------|---------|----------|-----------------|----------|
| Station ID | App settings | Target Station | Yes | User selection | Persistent |
| Device ID | App settings | Selected Device within Station | Yes | Auto or user selection | Persistent |
| Station summary | Backend | Station-level aggregated status | Yes | Periodic poll | 60 seconds |
| Device list | Backend | Devices in Station | Yes | On load + refresh | Cached 60s |
| Sensor data | WebSocket | Real-time readings per Device | Yes | Real-time push | <65 seconds |
| Device status | WebSocket | Connection state per Device | Yes | Real-time push | <65 seconds |
| FSM state | WebSocket | System state per Device | Yes | Real-time push | <65 seconds |
| Controller health | WebSocket | Health metrics per Device | Yes | Real-time push | <65 seconds |
| System events | WebSocket | Alerts per Device | No | Real-time push | <65 seconds |
| Tank alerts | WebSocket | Low tank warnings per Device | No | Real-time push | <65 seconds |

### 16.7 Data Displayed (STATION-FIRST SEMANTICS - PRODUCT DECISION)

**Station-Level Summary**:
- Station online/offline/degraded status
- Total Devices in Station
- Online/Offline Device count
- Station-level health indicator
- Station-level aggregated alerts

**Device-Level Summaries**:
- For each Device in Station:
  - Device online/offline status
  - Current FSM state
  - Key telemetry (EC, pH, water level)
  - Active actuators
  - Device-level alerts

**Single-Device Station Behavior (PRODUCT DECISION)**:
- If Station has exactly one Device:
  - Device context transparently resolved
  - Dashboard shows Device-level data directly
  - No unnecessary Device selection UI

**Multi-Device Station Behavior (PRODUCT DECISION)**:
- If Station has multiple Devices:
  - Dashboard shows Station summary first
  - Device list with status cards
  - User can identify which Device has the problem
  - User can select specific Device to view details
  - Dashboard must NOT silently collapse multiple Devices into one ambiguous "device status"

**Active Device Detail** (when Device selected):
- Sensor readings for selected Device
- Actuator status for selected Device
- FSM state for selected Device
- Device-specific alerts
- Device-specific quick actions

**Alerts**:
- Station-level alerts (aggregated from Devices)
- Device-level alerts (per Device, with Device identification)
- Tank level warnings per Device
- Fault state per Device with recovery action

**Quick Actions**:
- Navigate to Operations (for selected Device)
- Navigate to Journal (Station-level or Device-level filtering)
- Navigate to Settings (Station-level or Device-level)

**Provenance**: TARGET PRODUCT DECISION

### 16.8 Read/Write Boundary

**READ**:
- Station summary
- Device list
- Sensor data
- Device status
- FSM state
- Controller health
- System events
- Tank alerts

**WRITE**:
- Station selection (updates station_id in app settings)
- Device selection (updates device_id in app settings)

**SIDE EFFECTS**:
- WebSocket reconnection on Station/Device selection

### 16.9 State Model

**Loading**: Initial data fetch in progress
**Ready**: Station summary and Device list loaded
**Empty**: No Stations available
**Offline**: Station disconnected (all Devices offline)
**Degraded**: Some Devices offline
**Permission Denied**: User lacks permission to view Station

### 16.10 Empty State

**No Stations**: User has no Stations, redirect to Pairing
**No Devices in Station**: Station has no Devices, redirect to Pairing
**Station Not Selected**: Station ID missing, auto-select or redirect to Fleet

### 16.11 Error Handling

**Network Error**: Retry with message
**Backend Error**: Show specific error, allow retry
**Device Offline**: Show offline state, disable actions
**Validation Error**: Show field-specific error
**Authorization Error**: Show permission denied message

### 16.12 Safety Rules

No direct hardware control on Dashboard. All controls navigate to Operations.

### 16.13 Permission Model

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 16.14 Navigation Responsibilities

Navigate to:
- Operations (device control)
- Journal (alert investigation)
- Settings (configuration)
- Fleet (station switching)

### 16.15 Non-Responsibilities

- Device configuration (belongs to Settings)
- Deep diagnostics (belongs to Operations)
- Recipe editing (belongs to Cultivation)
- Actuator control (belongs to Operations)

### 16.16 Acceptance Criteria

- Dashboard displays Station-level summary
- Dashboard displays Device list within Station
- Device selection works for multi-Device Stations
- Single-Device Stations auto-resolve Device context
- Alerts are Station-aggregated with Device source preserved
- Offline/online states are semantically distinct
- Permissions are enforced

---

## 17. Operations

### 17.1 Purpose (REPOSITORY EVIDENCE)

Operations is the real-time control center for Device-level actuator control and automation.

### 17.2 User Job To Be Done

"When I need to manually control my growing system or monitor automation, I want to see real-time device state and issue commands, so that I can adjust parameters or respond to issues immediately."

### 17.3 Primary User Questions

1. What is the current FSM state?
2. Which actuators are running?
3. What are the current sensor readings?
4. Is the system in auto or manual mode?
5. Are there active faults?
6. Can I manually control actuators?
7. What automation scripts are active?
8. Can I edit automation scripts?

### 17.4 Primary Users / Roles

- **Admin**: Full control access
- **Operator**: Full control access
- **Viewer**: Read-only access

### 17.5 Entry Conditions

- User must be authenticated
- Device ID must be selected
- Device must be online
- WebSocket connection established

### 17.6 Data Inputs

| Data | Source | Purpose | Required | Update Behavior | Freshness |
|------|--------|---------|----------|-----------------|----------|
| Device ID | App settings | Target device | Yes | User selection | Persistent |
| Sensor data | WebSocket | Real-time readings | Yes | Real-time push | <65 seconds |
| Pump status | WebSocket | Actuator state | Yes | Real-time push | <65 seconds |
| FSM state | WebSocket | System state | Yes | Real-time push | <65 seconds |
| Controller health | WebSocket | Health metrics | Yes | Real-time push | <65 seconds |
| Automation scripts | Backend | Script definitions | No | On load + refresh | Cached 60s |

### 17.7 Data Displayed

**System State**:
- FSM state (e.g., Monitoring, MimoDosing, Fault)
- Control mode (auto/manual)
- Controller health score
- Last seen timestamp

**Sensor Readings**:
- EC value with target range
- pH value with target range
- Temperature value
- Water level percentage
- Error indicators

**Actuator Status**:
- Pump A/B status (on/off)
- pH Up/Down pump status
- Osaka pump status
- Water pump status
- Mist valve status
- Mix valve status
- PWM values where applicable

**Automation**:
- Active scripts list
- Script status (running/paused/error)
- Script editor
- Flow visualization

**Fault State**:
- Current fault code (if in fault)
- Fault description
- Recovery action

### 17.8 User Actions

**Action: Toggle Actuator**
- Purpose: Manually turn actuator on/off
- Trigger: Click actuator toggle button
- Preconditions: Device online, not in fault, manual mode
- Required permission: `control:pump`
- Parameters: Actuator ID, desired state
- Confirmation: None (quick action)
- Backend effect: Sends MQTT command to device
- Expected success: Actuator state changes
- Expected failure: Device offline, fault state, auto mode
- Retry behavior: User can retry after resolving issue
- Idempotency: No (duplicate commands toggle state)
- Audit/event: Manual action logged
- Navigation: None

**Action: Switch Control Mode**
- Purpose: Switch between auto and manual mode
- Trigger: Click mode toggle
- Preconditions: Device online, not in fault
- Required permission: `write:config`
- Parameters: Desired mode (auto/manual)
- Confirmation: Dialog confirmation
- Backend effect: Updates device configuration
- Expected success: Mode changes, FSM transitions
- Expected failure: Device offline, validation error
- Retry behavior: User can retry
- Idempotency: Yes
- Audit/event: Configuration change logged
- Navigation: None

**Action: Emergency Stop**
- Purpose: Immediately halt all actuators
- Trigger: Click emergency stop button
- Preconditions: Device online
- Required permission: `control:emergency`
- Parameters: None
- Confirmation: Dialog confirmation
- Backend effect: Sends emergency stop command
- Expected success: All actuators stop, FSM enters EmergencyStop state
- Expected failure: Device offline
- Retry behavior: User can retry
- Idempotency: Yes
- Audit/event: Emergency stop logged
- Navigation: None

**Action: Edit Automation Script**
- Purpose: Create or modify automation script
- Trigger: Click script edit button
- Preconditions: Device online, script:write permission
- Required permission: `script:write`
- Parameters: Script source code
- Confirmation: None
- Backend effect: Updates script in backend
- Expected success: Script saved
- Expected failure: Validation error, compilation error
- Retry behavior: User can fix and retry
- Idempotency: Yes
- Audit/event: Script change logged
- Navigation: None

### 17.9 Read/Write Boundary

**READ**:
- Device state
- Sensor data
- Pump status
- FSM state
- Automation scripts

**WRITE**:
- Actuator commands
- Control mode
- Automation scripts
- Emergency stop

**SIDE EFFECTS**:
- Device FSM transitions
- Actuator state changes
- Automation execution

### 17.10 State Model

**Loading**: Initial data fetch
**Ready**: All data loaded
**Offline**: Device not connected
**Manual Mode**: Manual control enabled
**Auto Mode**: Automation controlling
**Fault**: System in fault state
**Emergency Stop**: Emergency stop active

### 17.11 Empty State

**Device Not Selected**: Device ID missing, redirect to Dashboard
**Device Offline**: Device not connected, show offline state

### 17.12 Error Handling

**Network Error**: Retry with message
**Backend Error**: Show specific error
**Device Offline**: Show offline state, disable controls
**Validation Error**: Show field-specific error
**Authorization Error**: Show permission denied

### 17.13 Safety Rules

- Emergency stop always accessible regardless of mode
- Manual controls disabled in auto mode
- Fault state blocks normal operations
- Interlocks prevent conflicting actuator states
- Dangerous operations require confirmation

### 17.14 Permission Model

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 17.15 Navigation Responsibilities

Navigate to:
- Dashboard (situational awareness)
- Settings (configuration)
- Journal (event history)

### 17.16 Non-Responsibilities

- Station management (belongs to Fleet)
- Device configuration (belongs to Settings)
- Recipe editing (belongs to Cultivation)

### 17.17 Acceptance Criteria

- Operations displays real-time device state
- Manual controls work when conditions met
- Auto mode blocks manual controls
- Emergency stop always accessible
- Interlocks prevent conflicting commands
- Fault state blocks operations
- Permissions are enforced

---

## 18. Cultivation

### 18.1 Purpose (REPOSITORY EVIDENCE)

Cultivation represents agricultural intent and configuration for nutrient recipes and crop seasons.

### 18.2 User Job To Be Done

"When I need to configure nutrient targets and crop lifecycle, I want to create recipes and seasons, so that the system doses according to my cultivation plan."

### 18.3 Primary User Questions

1. What recipes are available?
2. What is the current active recipe?
3. What are the current nutrient targets?
4. What is the crop season schedule?
5. How has dosing been performing?
6. Can I create or edit recipes?

### 18.4 Primary Users / Roles

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 18.5 Entry Conditions

- User must be authenticated
- Device ID must be selected
- Device must be online

### 18.6 Data Inputs

| Data | Source | Purpose | Required | Update Behavior | Freshness |
|------|--------|---------|----------|-----------------|----------|
| Device ID | App settings | Target device | Yes | User selection | Persistent |
| Recipes | Backend | Nutrient programs | No | On load + refresh | Cached 60s |
| Seasons | Backend | Crop lifecycle | No | On load + refresh | Cached 60s |
| Dosing history | Backend | Historical dosing | No | On load + refresh | Cached 60s |

### 18.7 Data Displayed

**Active Recipe**:
- Recipe name
- Current stage
- EC target
- pH target
- Duration
- Time remaining

**Recipe List**:
- All available recipes
- Recipe names and descriptions
- Stage definitions

**Season Management**:
- Active season
- Season dates
- Crop type
- Stage schedule

**Dosing History**:
- Historical dosing events
- Timestamp
- EC/pH values
- Dosage amounts

### 18.8 User Actions

**Action: Create Recipe**
- Purpose: Create new nutrient recipe
- Trigger: Click create recipe button
- Preconditions: Authenticated
- Required permission: `recipe:write`
- Parameters: Recipe name, stages, targets
- Confirmation: None
- Backend effect: Creates recipe in database
- Expected success: Recipe created
- Expected failure: Validation error
- Retry behavior: User can fix and retry
- Idempotency: Yes
- Audit/event: Recipe creation logged
- Navigation: None

**Action: Activate Recipe**
- Purpose: Set active recipe for device
- Trigger: Click activate on recipe
- Preconditions: Device online
- Required permission: `recipe:write`
- Parameters: Recipe ID
- Confirmation: Dialog confirmation
- Backend effect: Updates device configuration
- Expected success: Recipe activated
- Expected failure: Device offline, validation error
- Retry behavior: User can retry
- Idempotency: Yes
- Audit/event: Recipe activation logged
- Navigation: None

### 18.9 Read/Write Boundary

**READ**:
- Recipes
- Seasons
- Dosing history

**WRITE**:
- Recipe creation/edit
- Recipe activation
- Season management

**SIDE EFFECTS**:
- Device configuration changes
- Active recipe update

### 18.10 State Model

**Loading**: Initial data fetch
**Ready**: All data loaded
**No Recipes**: No recipes available
**No Active Recipe**: No recipe currently active

### 18.11 Empty State

**No Recipes**: Show create recipe prompt
**No Seasons**: Show create season prompt

### 18.12 Error Handling

**Network Error**: Retry with message
**Backend Error**: Show specific error
**Validation Error**: Show field-specific error
**Authorization Error**: Show permission denied

### 18.13 Safety Rules

No direct hardware control in Cultivation.

### 18.14 Permission Model

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 18.15 Navigation Responsibilities

Navigate to:
- Operations (recipe execution)
- Settings (parameter reference)

### 18.16 Non-Responsibilities

- Device control (belongs to Operations)
- Station management (belongs to Fleet)

### 18.17 Acceptance Criteria

- Cultivation displays recipes and seasons
- Recipe creation works with validation
- Recipe activation updates device configuration
- Dosing history is displayed
- Permissions are enforced

---

## 19. Journal

### 19.1 Purpose (REPOSITORY EVIDENCE)

Journal is the historical/audit domain for system events and alerts.

### 19.2 User Job To Be Done

"When I need to investigate what happened in my system, I want to view event history and alerts, so that I can diagnose issues and understand system behavior."

### 19.3 Primary User Questions

1. What events occurred recently?
2. Were there any faults or warnings?
3. What caused a specific alert?
4. When did a configuration change happen?
5. What dosing events occurred?

### 19.4 Primary Users / Roles

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 19.5 Entry Conditions

- User must be authenticated
- Station ID must be selected
- Device ID may be selected for filtering

### 19.6 Data Inputs

| Data | Source | Purpose | Required | Update Behavior | Freshness |
|------|--------|---------|----------|-----------------|----------|
| Station ID | App settings | Target Station | Yes | User selection | Persistent |
| Device ID | App settings | Filter device | No | User selection | Persistent |
| Events | Backend | Historical events | No | On load + refresh | Cached 60s |

### 19.7 Data Displayed

**Event List**:
- Timestamp
- Category (System, Dosing, Water, Calibration, Sensor, Alert, UserAction, Device, Automation)
- Severity (Info, Success, Warning, Critical)
- Message
- Device ID (where applicable)
- Metadata (category-specific)

**Filters**:
- Time range
- Category
- Severity
- Device
- Station

**Analytics**:
- Event counts by category
- Alert trends over time

### 19.8 User Actions

**Action: Filter Events**
- Purpose: Narrow event list
- Trigger: Select filter criteria
- Preconditions: None
- Required permission: None
- Parameters: Filter values
- Confirmation: None
- Backend effect: Fetches filtered events
- Expected success: Filtered list displayed
- Expected failure: None
- Retry behavior: None
- Idempotency: Yes
- Audit/event: None
- Navigation: None

**Action: Export Events**
- Purpose: Export event data
- Trigger: Click export button
- Preconditions: Events available
- Required permission: `read:telemetry`
- Parameters: Export format
- Confirmation: None
- Backend effect: Generates export file
- Expected success: File downloaded
- Expected failure: Backend error
- Retry behavior: User can retry
- Idempotency: Yes
- Audit/event: Export logged
- Navigation: None

### 19.9 Read/Write Boundary

**READ**:
- Events
- Analytics

**WRITE**:
- None (read-only)

**SIDE EFFECTS**:
- None

### 19.10 State Model

**Loading**: Initial data fetch
**Ready**: All data loaded
**No Events**: No events available
**Filtered**: Filter criteria applied

### 19.11 Empty State

**No Events**: Show "No events available" message

### 19.12 Error Handling

**Network Error**: Retry with message
**Backend Error**: Show specific error
**Authorization Error**: Show permission denied

### 19.13 Safety Rules

No direct hardware control in Journal.

### 19.14 Permission Model

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 19.15 Navigation Responsibilities

Navigate to:
- Dashboard (alert investigation)
- Operations (fault diagnosis)
- Settings (configuration change history)

### 19.16 Non-Responsibilities

- Device control (belongs to Operations)
- Configuration changes (belongs to Settings)

### 19.17 Acceptance Criteria

- Journal displays events with filters
- Filtering works correctly
- Export functionality works
- Permissions are enforced

---

## 20. Fleet

### 20.1 Purpose (TARGET MODEL)

Fleet manages Station identity and selection with Station → Device hierarchy.

### 20.2 User Job To Be Done

"When I have multiple growing installations, I want to manage and switch between them, so that I can operate each Station independently."

### 20.3 Primary User Questions

1. What Stations do I own?
2. Which Station is currently selected?
3. How many Devices are in each Station?
4. What is the status of each Station?
5. Which Station has issues?

### 20.4 Primary Users / Roles

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 20.5 Entry Conditions

- User must be authenticated
- No Station/Device selection required initially

### 20.6 Data Inputs

| Data | Source | Purpose | Required | Update Behavior | Freshness |
|------|--------|---------|----------|-----------------|----------|
| Stations | Backend | User's Stations | Yes | On load + refresh | Cached 60s |
| Devices | Backend | Devices in each Station | Yes | On load + refresh | Cached 60s |

### 20.7 Data Displayed

**Station Cards**:
- Station name/label
- Station status (online/offline/degraded)
- Device count
- Online/Offline Device count
- Station-level alerts
- Select button

**Device Cards (within Station)**:
- Device name/label
- Device online/offline status
- Current FSM state
- Key telemetry (EC, pH, water level)
- Device-level alerts
- Select button

**Grouping**:
- Stations grouped by location/crop type when applicable
- Devices nested under Stations
- Collapsible Station groups

### 20.8 User Actions

**Action: Create Station**
- Purpose: Create new Station entity
- Trigger: Click create station button
- Preconditions: Authenticated
- Required permission: Admin
- Parameters: Station name, location, description
- Confirmation: None
- Backend effect: Creates station in database
- Expected success: Station created
- Expected failure: Validation error
- Retry behavior: User can fix and retry
- Idempotency: Yes
- Audit/event: Station creation logged
- Navigation: None

**Action: Select Station**
- Purpose: Switch to different Station
- Trigger: Click select on Station card
- Preconditions: Station exists
- Required permission: None
- Parameters: Station ID
- Confirmation: None
- Backend effect: Updates station_id in app settings
- Expected success: Station selected, WebSocket re-establishes
- Expected failure: Invalid station ID
- Retry behavior: User can retry
- Idempotency: Yes
- Audit/event: Station switch logged
- Navigation: Redirects to Dashboard

**Action: Rename Station**
- Purpose: Update Station display name
- Trigger: Click rename on Station card
- Preconditions: Station exists
- Required permission: Admin
- Parameters: New name
- Confirmation: None
- Backend effect: Updates station in database
- Expected success: Name updated
- Expected failure: Validation error
- Retry behavior: User can fix and retry
- Idempotency: Yes
- Audit/event: Rename logged
- Navigation: None

**Action: Delete Station**
- Purpose: Remove Station from account
- Trigger: Click delete on Station card
- Preconditions: Station exists
- Required permission: Admin
- Parameters: None
- Confirmation: Dialog confirmation
- Backend effect: Unclaims all Devices in Station, deletes station
- Expected success: Station deleted
- Expected failure: None
- Retry behavior: None
- Idempotency: No
- Audit/event: Station deletion logged
- Navigation: Redirects to Fleet

### 20.9 Read/Write Boundary

**READ**:
- Stations
- Devices

**WRITE**:
- Station creation
- Station rename
- Station deletion
- Device association

**SIDE EFFECTS**:
- Station context change
- Device context reset

### 20.10 State Model

**Loading**: Initial data fetch
**Ready**: All data loaded
**No Stations**: User has no Stations
**Station Selected**: Station context active

### 20.11 Empty State

**No Stations**: Show create station prompt, redirect to Pairing

### 20.12 Error Handling

**Network Error**: Retry with message
**Backend Error**: Show specific error
**Authorization Error**: Show permission denied

### 20.13 Safety Rules

No direct hardware control in Fleet.

### 20.14 Permission Model

- **Admin**: Full access
- **Operator**: View-only
- **Viewer**: View-only

### 20.15 Navigation Responsibilities

Navigate to:
- Dashboard (after Station selection)
- Pairing (to add Device)

### 20.16 Non-Responsibilities

- Device control (belongs to Operations)
- Device configuration (belongs to Settings)

### 20.17 Acceptance Criteria

- Fleet displays Stations with nested Devices
- Station selection works correctly
- Station creation/rename/delete work with proper permissions
- Device selection within Station works
- Permissions are enforced

---

## 21. Settings

### 21.1 Purpose (REPOSITORY EVIDENCE)

Settings provides configuration management for device-level and (future) station-level parameters.

### 21.2 User Job To Be Done

"When I need to configure my system parameters, I want to access settings, so that I can adjust targets, calibrate sensors, and manage updates."

### 21.3 Primary User Questions

1. What are the current EC/pH targets?
2. What are the safety limits?
3. Are sensors calibrated?
4. What is the WiFi configuration?
5. Is there a firmware update available?
6. What are the pump capacities?

### 21.4 Primary Users / Roles

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 21.5 Entry Conditions

- User must be authenticated
- Device ID must be selected
- Device must be online (for some settings)

### 21.6 Data Inputs

| Data | Source | Purpose | Required | Update Behavior | Freshness |
|------|--------|---------|----------|-----------------|----------|
| Device ID | App settings | Target device | Yes | User selection | Persistent |
| Device config | Backend | Current configuration | Yes | On load + refresh | Cached 60s |
| Calibration data | Backend | Sensor calibration | No | On load + refresh | Cached 60s |
| WiFi config | Backend | Network settings | No | On load + refresh | Cached 60s |
| Firmware version | Backend | OTA status | No | On load + refresh | Cached 60s |

### 21.7 Data Displayed

**Device Configuration**:
- EC target
- pH target
- Control mode
- Pump capacities
- Mixing/stabilize times
- Safety limits

**Sensor Calibration**:
- pH calibration points
- EC factor
- Temperature offset

**Water Configuration**:
- Water levels
- Circulation settings
- Misting settings

**Safety Configuration**:
- Max dose per cycle
- Max dose per hour
- Cooldown periods
- Emergency shutdown thresholds

**Network Configuration**:
- WiFi SSID
- WiFi password (masked)
- Connection status

**OTA Configuration**:
- Current firmware version
- Available update version
- Update status

### 21.8 User Actions

**Action: Update Configuration**
- Purpose: Change device parameter
- Trigger: Input field change + save
- Preconditions: Device online
- Required permission: `write:config`
- Parameters: Parameter name, value
- Confirmation: None (quick save) or dialog (dangerous)
- Backend effect: Updates device configuration
- Expected success: Configuration updated
- Expected failure: Validation error, device offline
- Retry behavior: User can fix and retry
- Idempotency: Yes
- Audit/event: Configuration change logged
- Navigation: None

**Action: Calibrate Sensor**
- Purpose: Calibrate sensor
- Trigger: Click calibrate button
- Preconditions: Device online, sensor online
- Required permission: `write:config`
- Parameters: Calibration type, reference values
- Confirmation: None
- Backend effect: Updates calibration
- Expected success: Calibration updated
- Expected failure: Sensor offline, validation error
- Retry behavior: User can retry
- Idempotency: Yes
- Audit/event: Calibration logged
- Navigation: None

**Action: Trigger OTA Update**
- Purpose: Update firmware
- Trigger: Click update button
- Preconditions: Device online, update available
- Required permission: `device:ota`
- Parameters: None
- Confirmation: Dialog confirmation
- Backend effect: Triggers OTA update
- Expected success: Update initiated
- Expected failure: Device offline, no update available
- Retry behavior: User can retry
- Idempotency: Yes
- Audit/event: OTA triggered logged
- Navigation: None

**Action: Update WiFi**
- Purpose: Change WiFi configuration
- Trigger: Input SSID/password + save
- Preconditions: Device online
- Required permission: `device:network`
- Parameters: SSID, password
- Confirmation: None
- Backend effect: Updates WiFi config
- Expected success: WiFi updated
- Expected failure: Validation error, device offline
- Retry behavior: User can retry
- Idempotency: Yes
- Audit/event: WiFi change logged
- Navigation: None

### 21.9 Read/Write Boundary

**READ**:
- Device configuration
- Calibration data
- WiFi config
- Firmware version

**WRITE**:
- Configuration updates
- Calibration updates
- WiFi updates
- OTA trigger

**SIDE EFFECTS**:
- Device configuration changes
- WiFi reconnection
- OTA update process

### 21.10 State Model

**Loading**: Initial data fetch
**Ready**: All data loaded
**Device Offline**: Device not connected, most settings disabled
**Updating**: Change in progress

### 21.11 Empty State

**Device Not Selected**: Device ID missing, redirect to Dashboard

### 21.12 Error Handling

**Network Error**: Retry with message
**Backend Error**: Show specific error
**Device Offline**: Show offline state, disable settings
**Validation Error**: Show field-specific error
**Authorization Error**: Show permission denied

### 21.13 Safety Rules

- Dangerous operations require confirmation
- Safety limits have validation
- Emergency stop settings require special confirmation

### 21.14 Permission Model

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 21.15 Navigation Responsibilities

Navigate to:
- Dashboard (return to overview)
- Operations (test configuration)

### 21.16 Non-Responsibilities

- Station management (belongs to Fleet)
- Device control (belongs to Operations)

### 21.17 Acceptance Criteria

- Settings displays all configuration sections
- Configuration updates work with validation
- Calibration works correctly
- OTA updates work correctly
- WiFi updates work correctly
- Dangerous operations require confirmation
- Permissions are enforced

---

## 22. Pairing

### 22.1 Purpose (REPOSITORY EVIDENCE)

Pairing enables device claiming and association with a Station.

### 22.2 User Job To Be Done

"When I need to add a new device to my system, I want to pair it, so that it becomes available for control and monitoring."

### 22.3 Primary User Questions

1. How do I add a new device?
2. What is the device ID?
3. Is the device already claimed?
4. Can I scan a QR code?
5. Which Station will the device belong to?

### 22.4 Primary Users / Roles

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 22.5 Entry Conditions

- User must be authenticated
- Station selected (for device association)

### 22.6 Data Inputs

| Data | Source | Purpose | Required | Update Behavior | Freshness |
|------|--------|---------|----------|-----------------|----------|
| Station ID | App settings | Target Station | Yes | User selection | Persistent |
| Device ID | User input/QR scan | Device to claim | Yes | User input | Immediate |
| Device list | Backend | Available devices | No | On load + refresh | Cached 60s |

### 22.7 Data Displayed

**Pairing Interface**:
- QR code scanner
- Manual device ID input
- Station selection
- Device status (claimed/unclaimed)
- Pairing status

### 22.8 User Actions

**Action: Claim Device**
- Purpose: Associate device with user's Station
- Trigger: Submit device ID or scan QR
- Preconditions: Device unclaimed, Station selected
- Required permission: Device pairing permission
- Parameters: Device ID, Station ID
- Confirmation: None
- Backend effect: Creates device_ownership record
- Expected success: Device claimed, MQTT credentials generated
- Expected failure: Device already claimed, invalid device ID
- Retry behavior: User can retry with different device
- Idempotency: No (duplicate claim rejected)
- Audit/event: Device claim logged
- Navigation: Redirects to Dashboard

**Action: Select Station for Pairing**
- Purpose: Choose which Station to add device to
- Trigger: Click Station selector
- Preconditions: Multiple Stations available
- Required permission: None
- Parameters: Station ID
- Confirmation: None
- Backend effect: Updates pairing context
- Expected success: Station selected
- Expected failure: None
- Retry behavior: None
- Idempotency: Yes
- Audit/event: None
- Navigation: None

### 22.9 Read/Write Boundary

**READ**:
- Station list
- Device list

**WRITE**:
- Device claim
- Station selection

**SIDE EFFECTS**:
- Device ownership created
- MQTT credentials generated

### 22.10 State Model

**Loading**: Initial data fetch
**Ready**: All data loaded
**Scanning**: QR scanner active
**Claiming**: Claim in progress
**Success**: Device claimed
**Failed**: Claim failed

### 22.11 Empty State

**No Stations**: User has no Stations, prompt to create Station first

### 22.12 Error Handling

**Network Error**: Retry with message
**Backend Error**: Show specific error
**Device Already Claimed**: Show error, suggest different device
**Invalid Device ID**: Show validation error
**Authorization Error**: Show permission denied

### 22.13 Safety Rules

No direct hardware control in Pairing.

### 22.14 Permission Model

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 22.15 Navigation Responsibilities

Navigate to:
- Dashboard (after successful pairing)
- Fleet (to manage Stations)

### 22.16 Non-Responsibilities

- Device control (belongs to Operations)
- Station creation (belongs to Fleet, future)

### 22.17 Acceptance Criteria

- Pairing accepts device ID or QR scan
- Device claim works when device is unclaimed
- Already-claimed devices are rejected
- Station selection works
- MQTT credentials are generated
- Permissions are enforced

---

## 23. Config Backup

### 23.1 Purpose (REPOSITORY EVIDENCE)

Config Backup enables configuration export/import for disaster recovery.

### 23.2 User Job To Be Done

"When I need to backup or restore my device configuration, I want to export/import settings, so that I can recover from failures or migrate devices."

### 23.3 Primary User Questions

1. What configuration can I backup?
2. How do I create a backup?
3. How do I restore from backup?
4. Is the backup compatible with this device?
5. What happens to current configuration on restore?

### 23.4 Primary Users / Roles

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 23.5 Entry Conditions

- User must be authenticated
- Device ID must be selected
- Device must be online (for restore)

### 23.6 Data Inputs

| Data | Source | Purpose | Required | Update Behavior | Freshness |
|------|--------|---------|----------|-----------------|----------|
| Device ID | App settings | Target device | Yes | User selection | Persistent |
| Backup file | User upload | Configuration to restore | No | User upload | Immediate |
| Device config | Backend | Current configuration | Yes | On load | Cached 60s |

### 23.7 Data Displayed

**Backup Interface**:
- Current configuration summary
- Backup history
- Upload interface for restore
- Download interface for export

### 23.8 User Actions

**Action: Create Backup**
- Purpose: Export device configuration
- Trigger: Click export button
- Preconditions: Device online
- Required permission: Backup permission
- Parameters: None
- Confirmation: None
- Backend effect: Generates backup file
- Expected success: File downloaded
- Expected failure: Device offline, backend error
- Retry behavior: User can retry
- Idempotency: Yes
- Audit/event: Backup created logged
- Navigation: None

**Action: Restore Backup**
- Purpose: Import device configuration
- Trigger: Upload backup file
- Preconditions: Device online, backup compatible
- Required permission: Backup permission
- Parameters: Backup file
- Confirmation: Dialog confirmation
- Backend effect: Restores configuration to device
- Expected success: Configuration restored, device reboots
- Expected failure: Device offline, incompatible backup, validation error
- Retry behavior: User can retry
- Idempotency: No
- Audit/event: Restore logged
- Navigation: None

### 23.9 Read/Write Boundary

**READ**:
- Device configuration
- Backup history

**WRITE**:
- Backup creation
- Configuration restore

**SIDE EFFECTS**:
- Device configuration changed
- Device reboot (on restore)

### 23.10 State Model

**Loading**: Initial data fetch
**Ready**: All data loaded
**Exporting**: Backup generation in progress
**Importing**: Restore in progress
**Success**: Operation completed
**Failed**: Operation failed

### 23.11 Empty State

**Device Not Selected**: Device ID missing, redirect to Dashboard

### 23.12 Error Handling

**Network Error**: Retry with message
**Backend Error**: Show specific error
**Device Offline**: Show offline state, disable restore
**Incompatible Backup**: Show compatibility error
**Validation Error**: Show validation error
**Authorization Error**: Show permission denied

### 23.13 Safety Rules

- Restore requires confirmation
- Incompatible backups are rejected
- Device must be online for restore

### 23.14 Permission Model

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 23.15 Navigation Responsibilities

Navigate to:
- Dashboard (after restore)

### 23.16 Non-Responsibilities

- Station management (belongs to Fleet)
- Device control (belongs to Operations)

### 23.17 Acceptance Criteria

- Backup exports configuration correctly
- Restore imports configuration correctly
- Incompatible backups are rejected
- Device offline state is handled
- Permissions are enforced

---

## 24. Roles

### 24.1 Purpose (REPOSITORY EVIDENCE)

Roles manages users and permission matrix.

### 24.2 User Job To Be Done

"When I need to manage team access, I want to add users and assign roles, so that team members have appropriate permissions."

### 24.3 Primary User Questions

1. What users exist?
2. What roles do they have?
3. What permissions does each role have?
4. Can I add a new user?
5. Can I change a user's role?

### 24.4 Primary Users / Roles

- **Admin**: Full access
- **Operator**: Full access
- **Viewer**: Read-only access

### 24.5 Entry Conditions

- User must be authenticated
- Must be Admin to manage users

### 24.6 Data Inputs

| Data | Source | Purpose | Required | Update Behavior | Freshness |
|------|--------|---------|----------|-----------------|----------|
| Users | Backend | User list | Yes | On load + refresh | Cached 60s |
| Roles | Backend | Role definitions | Yes | On load + refresh | Cached 60s |
| Permissions | Backend | Permission matrix | Yes | On load + refresh | Cached 60s |

### 24.7 Data Displayed

**User List**:
- User email
- User role
- User status

**Permission Matrix**:
- Role definitions
- Scope definitions
- Capability mapping

### 24.8 User Actions

**Action: Add User**
- Purpose: Create new user
- Trigger: Click add user button
- Preconditions: Authenticated, Admin
- Required permission: Admin
- Parameters: User email, role
- Confirmation: None
- Backend effect: Creates user in Firebase Auth and backend
- Expected success: User created
- Expected failure: User already exists, validation error
- Retry behavior: User can fix and retry
- Idempotency: No
- Audit/event: User creation logged
- Navigation: None

**Action: Change Role**
- Purpose: Update user role
- Trigger: Click role selector
- Preconditions: User exists
- Required permission: Admin
- Parameters: User ID, new role
- Confirmation: Dialog confirmation
- Backend effect: Updates user role in backend
- Expected success: Role updated
- Expected failure: Validation error
- Retry behavior: User can retry
- Idempotency: Yes
- Audit/event: Role change logged
- Navigation: None

**Action: Remove User**
- Purpose: Remove user from system
- Trigger: Click remove button
- Preconditions: User exists
- Required permission: Admin
- Parameters: User ID
- Confirmation: Dialog confirmation
- Backend effect: Removes user from backend and Firebase Auth
- Expected success: User removed
- Expected failure: None
- Retry behavior: None
- Idempotency: No
- Audit/event: User removal logged
- Navigation: None

### 24.9 Read/Write Boundary

**READ**:
- Users
- Roles
- Permissions

**WRITE**:
- User creation
- Role changes
- User removal

**SIDE EFFECTS**:
- Firebase Auth changes
- Backend user/role changes

### 24.10 State Model

**Loading**: Initial data fetch
**Ready**: All data loaded
**No Users**: No users exist

### 24.11 Empty State

**No Users**: Show "No users available" message

### 24.12 Error Handling

**Network Error**: Retry with message
**Backend Error**: Show specific error
**Authorization Error**: Show permission denied

### 24.13 Safety Rules

No direct hardware control in Roles.

### 24.14 Permission Model

- **Admin**: Full access
- **Operator**: View-only
- **Viewer**: View-only

### 24.15 Navigation Responsibilities

Navigate to:
- Dashboard (return to overview)

### 24.16 Non-Responsibilities

- Device control (belongs to Operations)
- Station management (belongs to Fleet)

### 24.17 Acceptance Criteria

- Roles displays users and permission matrix
- User addition works correctly
- Role changes work correctly
- User removal works correctly
- Only Admin can manage users
- Permissions are enforced

---

## 25. Cross-Page Workflows

### 25.1 Workflow A: User with One Station / One Device

**Starting Context**: User logged in, no Station/Device selected

**Steps**:
1. User logs in via Firebase Authentication
2. User authenticated (email/password or Google)
3. Dashboard auto-selects first owned Station
4. Dashboard auto-selects single Device within Station
5. Dashboard shows Device-level data directly
6. User can proceed to Operations, Cultivation, etc.

**Resulting Context**: Station and Device selected automatically

**Permissions**: Standard user permissions

**Failure Cases**:
- No Stations: Redirect to Pairing
- No Devices in Station: Redirect to Pairing

### 25.2 Workflow B: User with Multiple Stations

**Starting Context**: User logged in, no Station/Device selected

**Steps**:
1. User logs in
2. User navigates to Fleet
3. User selects Station from Station list
4. Station context updated (station_id)
5. Device context reset (device_id null)
6. Dashboard shows Station summary
7. If Station has one Device: Device auto-selected
8. If Station has multiple Devices: User selects Device
9. Device context updated (device_id)
10. User can proceed to Operations, Cultivation, etc.

**Resulting Context**: Station and Device selected explicitly

**Permissions**: Standard user permissions

**Failure Cases**:
- No Stations: Redirect to Pairing

### 25.3 Workflow C: Station with Multiple Devices

**Starting Context**: Station selected, no Device selected

**Steps**:
1. User on Dashboard
2. Dashboard shows Station summary
3. Dashboard shows Device list with status cards
4. User identifies problematic Device
5. User selects Device
6. Device context updated
7. Dashboard shows Device detail
8. User navigates to Operations for Device control

**Resulting Context**: Device selected within Station

**Permissions**: Standard user permissions

**Failure Cases**:
- No Devices in Station: Redirect to Pairing

### 25.4 Workflow D: Pair New Device into Existing Station

**Starting Context**: Station selected

**Steps**:
1. User navigates to Pairing
2. Station pre-selected
3. User scans QR or enters Device ID
4. Device verified
5. Device associated with selected Station
6. Device appears in Station's Device list
7. User navigates to Dashboard to verify

**Resulting Context**: Device added to Station

**Permissions**: Device pairing permission

**Failure Cases**:
- Device already claimed: Show error
- Invalid Device ID: Show validation error

### 25.5 Workflow E: Create Station and Add First Device

**Starting Context**: User logged in, no Stations

**Steps**:
1. User navigates to Fleet
2. User creates new Station
3. User assigns Station name, location, description
4. Station initially contains no Devices
5. User navigates to Pairing
6. User pairs Device
7. Device associated with Station
8. Station now has one Device
9. Device context auto-selected
10. User navigates to Dashboard

**Resulting Context**: Station created, Device added and auto-selected

**Permissions**: Station creation permission, Device pairing permission

**Failure Cases**:
- Validation error: Show error, allow retry

### 25.6 Workflow F: Switch Station

**Starting Context**: Station A selected, Device X selected

**Steps**:
1. User navigates to Fleet
2. User selects Station B
3. Station context updated to Station B
4. Device context reset
5. WebSocket re-establishes with new Station context
6. Dashboard shows Station B summary
7. If Station B has one Device: Device auto-selected
8. If Station B has multiple Devices: User selects Device
9. Device context updated
10. User can proceed to Operations, Cultivation, etc.

**Resulting Context**: Station B and Device selected

**Permissions**: Standard user permissions

**Failure Cases**:
- Invalid Station ID: Show error

### 25.7 Workflow G: Select Device within Station

**Starting Context**: Station selected, no Device selected

**Steps**:
1. User on Dashboard (multi-Device Station)
2. Dashboard shows Station summary
3. Dashboard shows Device list with status cards
4. User clicks Device card
5. Device context updated
6. Dashboard shows Device detail
7. User navigates to Operations for Device control

**Resulting Context**: Device selected

**Permissions**: Standard user permissions

**Failure Cases**:
- Device offline: Show offline state

### 25.8 Workflow H: Execute Device Operation

**Starting Context**: Device selected

**Steps**:
1. User on Operations
2. User triggers actuator command
3. Command validated and authorized
4. Command sent to Device/Controller
5. Device executes command
6. Result displayed in UI
7. Journal records event

**Resulting Context**: Device state updated

**Permissions**: Device control permission

**Failure Cases**:
- Device offline: Command rejected
- Fault state: Command blocked
- Auto mode: Manual control blocked

### 25.9 Workflow I: Station-Level Alert Caused by Device Failure

**Starting Context**: Station selected

**Steps**:
1. Device A2 goes offline/critical
2. Station status becomes DEGRADED/CRITICAL
3. Dashboard shows Station-level alert
4. Dashboard identifies Device A2 as source
5. User selects Device A2
6. User navigates to Operations/Journal to investigate

**Resulting Context**: Device A2 selected

**Permissions**: Standard user permissions

**Failure Cases**:
- None

### 25.10 Workflow J: Review Device Event in Journal

**Starting Context**: Device selected

**Steps**:
1. User navigates to Journal
2. User filters by Device
3. User views Device-specific events
4. User identifies root cause
5. User takes corrective action if needed

**Resulting Context**: None (informational)

**Permissions**: Standard user permissions

**Failure Cases**:
- None

### 25.11 Workflow K: Configure Cultivation for a Station

**Starting Context**: Station selected

**Steps**:
1. User navigates to Cultivation
2. User creates recipe/season
3. User assigns to Station (or Device within Station) [OPEN QUESTION]
4. Recipe activated on Device
5. Device executes cultivation configuration
6. Journal records dosing/events

**Resulting Context**: Cultivation configured

**Permissions**: Recipe write permission

**Failure Cases**:
- Validation error: Show error

### 25.12 Workflow L: Restore Configuration

**Starting Context**: Device selected

**Steps**:
1. User navigates to Config Backup
2. User selects Device backup
3. User validates compatibility
4. User authorizes restore
5. Restore triggered
6. Device applies configuration
7. Journal records restoration event

**Resulting Context**: Configuration restored

**Permissions**: Backup permission

**Failure Cases**:
- Incompatible backup: Show error
- Device offline: Show offline state

---

## 26. Telemetry Semantics (PRODUCT DECISION)

### 26.1 Global Semantic Contract

| Value | Meaning | Distinction |
|-------|---------|-------------|
| **0** | Valid measured zero (e.g., EC = 0 ppm) | Real zero value |
| **NULL** | No data available | Sensor not reporting |
| **UNKNOWN** | Data not yet loaded | Initial state |
| **OFFLINE** | Node not connected | No MQTT data |
| **STALE** | Data not received within timeout | No recent update |
| **ERROR** | Sensor hardware failure | `err_*` flag set |
| **NOT CONFIGURED** | Entity not set up | Missing configuration |

### 26.2 Critical Semantic

**A sensor value of 0 is NOT automatically equivalent to no sensor data.**

Every page must respect this semantic model.

**Provenance**: TARGET PRODUCT DECISION

---

## 27. Simplicity Rule (PRODUCT DECISION)

Explicitly encode progressive complexity:

**ONE STATION**:
- Simple experience
- Station selection transparent or minimal

**ONE DEVICE IN STATION**:
- Device context may be implicit
- No unnecessary Device selection UI

**MULTIPLE DEVICES**:
- Explicit Device selection required
- Device list with status cards

**MULTIPLE STATIONS**:
- Explicit Station selection required
- Station list with status cards

**MULTI-USER SHARING**:
- Explicit permission/access context

**Rule**: The user should not be forced to understand the full hierarchy when the hierarchy contains only one item at a given level.

**Provenance**: TARGET PRODUCT DECISION

---

## 28. Station-First Does Not Mean Station-Only (PRODUCT DECISION)

Make this principle explicit:

**Station is the organizational context.**
**Device is the operational context.**

Do NOT make every operation Station-wide.

**Example**:

- **Dashboard**: Station-centric
- **Operations**: Device-centric
- **Fleet**: Station → Device hierarchy

This distinction must be visible throughout the document.

**Provenance**: TARGET PRODUCT DECISION

---

## 29. MQTT / Device Contract Caution (PRODUCT DECISION)

Do NOT require MQTT topic hierarchy changes merely because Station was introduced.

**State**:

"Station-aware identity and scoping are product/domain requirements. The transport representation is an implementation concern and must be evaluated separately."

Only specify MQTT changes when required by actual architecture.

**Provenance**: IMPLEMENTATION QUESTION

---

## 30. Alert Aggregation (PRODUCT DECISION)

### 30.1 Target Semantic Model

**Station Alert**:
- An operational summary derived from one or more Device-level conditions

**Device Alert**:
- A specific issue belonging to a Device or one of its components

**Example**:

```
Station A
  - Device A1: NORMAL
  - Device A2: SENSOR OFFLINE

Station status: DEGRADED

Device A2: CRITICAL / SENSOR OFFLINE
```

### 30.2 Aggregation Rules [OPEN QUESTION]

The specification should explain:
- when a Station becomes degraded
- when a Station becomes critical
- whether one critical Device can make the Station critical
- whether aggregation is computed frontend/backend
- what remains the source of truth

Do not hard-code numerical alert thresholds unless they are verified/calibrated.

**Provenance**: TARGET PRODUCT DECISION (semantic model), OPEN QUESTION (threshold rules)

---

## 31. Permission Model (PRODUCT DECISION)

### 31.1 Reconciled Conceptual Model

**Preferred conceptual model**:

```
User
  → Station access
      → Device access (if needed)
```

### 31.2 Investigation Required [OPEN QUESTION]

Investigate whether:
- Station access implies Device access
- Device-level override is necessary
- shared Station access is required
- read-only access can exist at Station level
- control permission differs from configuration permission

### 31.3 Permission Matrix (REPOSITORY EVIDENCE + PRODUCT DECISION)

| Capability | Admin | Operator | Viewer | Scope |
|------------|-------|----------|--------|-------|
| View Station | ✅ | ✅ | ✅ | station:read |
| View Device | ✅ | ✅ | ✅ | device:read |
| Control Device | ✅ | ✅ | ❌ | device:control |
| Emergency Stop | ✅ | ✅ | ❌ | device:emergency |
| Edit Cultivation | ✅ | ✅ | ❌ | station:cultivation or device:cultivation [OPEN QUESTION] |
| Edit Configuration | ✅ | ✅ | ❌ | device:config |
| Pair Device | ✅ | ✅ | ❌ | station:device:pair |
| Remove Device | ✅ | ✅ | ❌ | station:device:remove |
| Manage Station | ✅ | ❌ | ❌ | station:manage |
| Manage Users | ✅ | ❌ | ❌ | * (global) |
| Manage Roles | ✅ | ❌ | ❌ | * (global) |
| Restore Backup | ✅ | ✅ | ❌ | device:backup |

**Current Implementation** [REPOSITORY EVIDENCE]:
- All scopes are device-scoped (e.g., `control:pump`, `write:config`)
- No station-level scopes exist
- Admin, Operator, Viewer roles defined
- Admin has `['*']` (full access)
- Operator has specific device-level scopes
- Viewer has `['read:telemetry']` (read-only)

**Provenance**: REPOSITORY EVIDENCE (current implementation), TARGET PRODUCT DECISION (conceptual model), OPEN QUESTION (inheritance/override behavior)

---

## 32. Design Readiness Review

### 32.1 Readiness Checklist

- [x] Canonical domain model is unambiguous
- [x] Station vs Device is unambiguous
- [x] Controller vs Sensor relationship is unambiguous
- [x] Active Station context is defined
- [x] Active Device context is defined
- [x] Single-device simplification is defined
- [x] Multi-device behavior is defined
- [x] Multi-station behavior is defined
- [x] Page responsibilities do not overlap ambiguously
- [x] Hardware control ownership is explicit
- [x] Permission model is sufficiently defined
- [x] Alert scope is defined
- [x] Telemetry state semantics are defined
- [x] Offline ≠ zero
- [x] Assumptions are marked
- [x] Open questions are marked
- [x] Legacy routes have a migration interpretation
- [x] No major contradiction remains unexplained

### 32.2 Final Readiness Status

**READY FOR DESIGN**

---

## 33. Final Report

### 33.1 Canonical Domain Model

```
User
  └── 1..N Stations
        └── 1..N Devices
              ├── exactly 1 Controller Node
              └── exactly 1 Sensor Node
```

**Station**: Physical/operational organizational unit
**Device**: Independently controllable operational unit
**Controller Node**: Control authority of a Device
**Sensor Node**: Telemetry provider belonging to that Device

### 33.2 Main Current vs Target Gaps

| Aspect | Current | Target | Migration Required |
|--------|---------|--------|-------------------|
| User → Station | Direct to Device | Via Station | HIGH |
| Station → Device | Device = Station | 1:N | HIGH |
| MQTT scoping | device_id only | Station + device_id | INVESTIGATION REQUIRED |
| Database | device_* tables only | station + device tables | HIGH |
| Frontend context | deviceId only | stationId + deviceId | HIGH |
| Permissions | device-scoped only | station + device scopes | HIGH |
| Alerts | device-level only | station + device aggregation | HIGH |

### 33.3 Resolved Product Decisions

1. Station is a first-class entity [RESOLVED - TARGET PRODUCT DECISION]
2. User may have multiple Stations [RESOLVED - TARGET PRODUCT DECISION]
3. Station may have multiple Devices [RESOLVED - TARGET PRODUCT DECISION]
4. Device contains Controller + Sensor pair [RESOLVED - VERIFIED CURRENT INVARIANT]
5. Station is primary organizational context [RESOLVED - TARGET PRODUCT DECISION]
6. Device is primary operational control context [RESOLVED - TARGET PRODUCT DECISION]
7. Single-device Stations auto-resolve Device context [RESOLVED - TARGET PRODUCT DECISION]
8. Multi-device Stations require explicit Device selection [RESOLVED - TARGET PRODUCT DECISION]
9. Station-first semantics (organization) vs Device-first (operation) [RESOLVED - TARGET PRODUCT DECISION]
10. Telemetry semantics (0 ≠ no data) [RESOLVED - TARGET PRODUCT DECISION]

### 33.4 Remaining Open Questions

1. **Permission inheritance/override behavior** [OPEN QUESTION]: Does Station access imply Device access? Is Device-level override necessary?
2. **Station status aggregation rules** [OPEN QUESTION]: Exact threshold rules for Station status (NORMAL/DEGRADED/CRITICAL)
3. **Device movement between Stations** [OPEN QUESTION]: Can a Device move between Stations?
4. **Station creation during pairing** [OPEN QUESTION]: Does pairing create a Station?
5. **Backup scope** [OPEN QUESTION]: Is backup scope Station, Device, Controller, or combination?
6. **Cultivation configuration scope** [OPEN QUESTION]: Are recipes/seasons Station-level or Device-level?
7. **Notification preferences** [OPEN QUESTION]: Exact notification preferences per category
8. **Emergency stop acknowledgement lifecycle** [OPEN QUESTION]: Exact acknowledgement requirements
9. **MQTT topic hierarchy** [IMPLEMENTATION QUESTION]: Does MQTT need Station identity?
10. **Health score semantics** [OPEN QUESTION]: Should health score be Device-level or Station-level?

### 33.5 Assumptions

1. **Multi-device stations as future requirement** [ASSUMPTION]: Requires validation from production data
2. **Station-level operations value** [ASSUMPTION]: Requires validation from user research

### 33.6 Migration Risks

**Overall Risk**: HIGH

- Database schema changes required
- Breaking API changes required
- Frontend state management changes required
- Permission model changes required
- Data migration script required
- Multi-phase migration approach recommended

### 33.7 Design Readiness

**READY FOR DESIGN**

The specification now provides a frozen product domain model and page responsibility model suitable for UI/UX design work. Designers can proceed with the understanding that:

- The target domain model is User → Station → Device → Controller/Sensor
- Station is the organizational context, Device is the operational context
- Current implementation requires migration to achieve target model
- All gaps are explicitly documented
- Assumptions are clearly marked
- Genuine product decisions are identified as open questions

**Next Phase**:
```
docs/discovery/2026-09-12-user-discovery.md
        +
docs/FRONTEND_FUNCTIONAL_SPEC.md
        +
docs/DESIGN.md
        ↓
OpenDesign / IA / wireframes / visual design
```
