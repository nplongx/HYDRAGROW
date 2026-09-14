# HydraGrow Frontend Wireframes

**Date:** 2026-09-12
**Based on:**
- `docs/discovery/2026-09-12-user-discovery.md`
- `docs/FRONTEND_FUNCTIONAL_SPEC.md`
- `hydragrow-frontend/DESIGN.md`
- Finalized Information Architecture and navigation/context model

**Status:** READY FOR VISUAL DESIGN

---

## Document Purpose

This document provides low-fidelity, structural wireframes for the HydraGrow frontend. These wireframes establish the structural UX before visual styling.

**Wireframe Principles:**
1. ONE coherent product
2. Stable navigation
3. Adaptive context
4. Progressive complexity
5. Station = organizational context
6. Device = operational context
7. Controller/Sensor = components of Device
8. Safety-critical information is never hidden
9. Do not infer UI mode from guessed persona
10. Do not redesign around the current legacy frontend layout

---

## Canonical Domain Model

```
User
  → 1..N Stations
        └── 1..N Devices
              ├── exactly 1 Controller Node
              └── exactly 1 Sensor Node
```

**Station** = organizational context
**Device** = operational context

---

## State Semantics

**Important distinction:**
- `zero` = measured value of 0
- `no data` = data never received
- `offline` = connection lost
- `stale` = data is old
- `error` = failure state
- `unknown` = state cannot be determined
- `not configured` = configuration missing

**Do not use "0" to represent unavailable telemetry.**

---

## Application Shell Structure

```
┌─────────────────────────────────────────────────────────────┐
│  HEADER                                                      │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Logo  |  [Navigation: Overview | Operations | ...]  │ │
│  │        │  [Station Selector]  [Device Selector]      │ │
│  │        │  [Emergency Stop]  [User Profile]           │ │
│  └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│  MAIN CONTENT                                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                                                        │ │
│  │  Page Content                                         │ │
│  │                                                        │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

**Navigation Items (Stable):**
- Overview (Dashboard)
- Operations
- Cultivation
- Journal
- Fleet
- Settings

**Contextual Elements (Progressively Revealed):**
- Station selector (visible when >1 Station)
- Device selector (visible when >1 Device in Station)
- Emergency stop (ALWAYS visible)
- Quick actions

---

## Wireframe Set

### Wireframe 1: First-time / No Station

**Page Purpose:** Onboarding entry point for users with no Stations

**Active Station Context:** None
**Active Device Context:** None

**Primary User Goal:** Add first Station to get started

**Primary Actions:**
- Create Station
- Pair Device (may create Station during pairing)

**Secondary Actions:**
- View account settings
- Logout

**Important Information:**
- Clear call-to-action: "Add your first Station"
- Brief explanation of what HydraGrow does

**State Transitions:**
- Empty state → Station created → Dashboard
- Empty state → Device paired → Dashboard

**Loading/Empty/Offline/Error States:**
- Empty state: No Stations available
- Loading: None (static page)

**Permission Behavior:**
- Requires user authentication

**Navigation Entry/Exit Points:**
- Entry: After authentication (if no Stations)
- Exit: To Dashboard (after Station creation/pairing)

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER                                                      │
├─────────────────────────────────────────────────────────────┤
│  EMPTY STATE                                                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                                                        │ │
│  │  Welcome to HydraGrow                                   │ │
│  │                                                        │ │
│  │  You don't have any Stations yet.                      │ │
│  │                                                        │ │
│  │  [Add your first Station]                              │ │
│  │  [Pair a Device]                                       │ │
│  │                                                        │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 2: Single Station + Single Device Dashboard

**Page Purpose:** Station-level situational awareness for simplest valid configuration

**Active Station Context:** Station (implicit, not prominent)
**Active Device Context:** Device (implicit, not prominent)

**Primary User Goal:** Monitor "my system" at a glance

**Primary Actions:**
- Quick actions (start/stop common operations)
- Navigate to Operations for detailed control
- Navigate to Cultivation for recipe status

**Secondary Actions:**
- View alerts
- View historical events

**Important Information:**
- Station state (aggregated from Device)
- Device telemetry summaries
- Active alerts
- Recommended action

**State Transitions:**
- Normal → Degraded (if Device goes offline)
- Dashboard → Operations (for detailed control)

**Loading/Empty/Offline/Error States:**
- Loading: Skeleton layout
- Offline: Degraded state with offline indicator
- Error: Error banner with retry option

**Permission Behavior:**
- Requires `station:read` permission

**Navigation Entry/Exit Points:**
- Entry: Default after login (auto-selected Station, implicit Device)
- Exit: To Operations, Cultivation, Journal, Fleet, Settings

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station selector hidden, Device selector hidden)   │
├─────────────────────────────────────────────────────────────┤
│  DASHBOARD                                                   │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Station: "My Station" (subtle)                         │ │
│  │ Status: [NORMAL]                                        │ │
│  │                                                        │ │
│  │ Quick Actions:                                         │ │
│  │ [Start] [Stop] [Calibrate]                             │ │
│  │                                                        │ │
│  │ Telemetry Summary:                                     │ │
│  │ EC: 1.5 mS/cm  pH: 6.2  Temp: 22°C  Water: 65%         │ │
│  │                                                        │ │
│  │ Active Alerts: [None]                                  │ │
│  │                                                        │ │
│  │ [View Details] → Operations                            │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 3: Single Station + Multiple Devices Dashboard

**Page Purpose:** Station-level situational awareness with Device summaries

**Active Station Context:** Station (implicit)
**Active Device Context:** None (until explicitly selected)

**Primary User Goal:** Monitor Station, select Device for operations

**Primary Actions:**
- Select Device
- Navigate to Operations for selected Device
- View Device summaries

**Secondary Actions:**
- Quick actions per Device
- Navigate to Cultivation, Journal

**Important Information:**
- Station state (aggregated from Devices)
- Device count
- Online/offline distribution
- Device status summaries
- Active alerts per Device

**State Transitions:**
- Dashboard → Device selection → Operations
- Dashboard → Fleet (if user acquires more Stations)

**Loading/Empty/Offline/Error States:**
- Loading: Skeleton layout
- Device offline: Degraded indicator in Device summary
- Mixed online/offline: Station status shows DEGRADED

**Permission Behavior:**
- Requires `station:read` permission

**Navigation Entry/Exit Points:**
- Entry: Default after login (auto-selected Station)
- Exit: To Operations (after Device selection), Cultivation, Journal, Fleet, Settings

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station selector hidden, Device selector visible)   │
├─────────────────────────────────────────────────────────────┤
│  DASHBOARD                                                   │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Station: "My Station"                                  │ │
│  │ Status: [NORMAL]                                        │ │
│  │ Devices: 3 (2 online, 1 offline)                         │ │
│  │                                                        │ │
│  │ Device Summaries:                                      │ │
│  │ ┌───────────────────────────────────────────────────┐ │ │
│  │ │ Device A1  [ONLINE]  EC: 1.5  pH: 6.2  [Select] │ │ │
│  │ │ Device A2  [OFFLINE] Last seen: 2h ago  [Select] │ │ │
│  │ │ Device A3  [ONLINE]  EC: 1.6  pH: 6.1  [Select] │ │ │
│  │ └───────────────────────────────────────────────────┘ │ │
│  │                                                        │ │
│  │ Active Alerts:                                         │ │
│  │ - Device A2: Sensor offline                           │ │
│  │                                                        │ │
│  │ [Manage Station] → Settings                            │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 4: Multi-Station Fleet

**Page Purpose:** Station list and station switching hub

**Active Station Context:** None (until selected)
**Active Device Context:** None

**Primary User Goal:** Select Station to monitor/manage

**Primary Actions:**
- Select Station
- Create Station
- Navigate to Pairing

**Secondary Actions:**
- View Station status summaries
- Delete Station (if permission)

**Important Information:**
- Station list with status summaries
- Device count per Station
- Online/offline distribution per Station
- Active alerts per Station

**State Transitions:**
- Fleet → Station selection → Dashboard
- Fleet → Create Station → Dashboard
- Fleet → Pairing → Dashboard

**Loading/Empty/Offline/Error States:**
- Loading: Skeleton layout
- Empty: No Stations (same as Wireframe 1)
- Station offline: Degraded indicator in Station summary

**Permission Behavior:**
- Requires user authentication
- Station management requires `station:manage` permission

**Navigation Entry/Exit Points:**
- Entry: From navigation, on first login (if >1 Stations)
- Exit: To Dashboard (after Station selection), Settings

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station selector visible)                            │
├─────────────────────────────────────────────────────────────┤
│  FLEET                                                       │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Your Stations                                           │ │
│  │                                                        │ │
│  │ ┌───────────────────────────────────────────────────┐ │ │
│  │ │ Station A  [NORMAL]  3 Devices  [Select]       │ │ │
│  │ │ Station B  [DEGRADED] 2 Devices  [Select]      │ │ │
│  │ │ Station C  [NORMAL]  1 Device   [Select]       │ │ │
│  │ └───────────────────────────────────────────────────┘ │ │
│  │                                                        │ │
│  │ [Add Station]                                         │ │
│  │ [Pair Device]                                          │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 5: Station Selected

**Page Purpose:** Station context activation (same as Dashboard, but explicit context shown)

**Active Station Context:** Station (explicit)
**Active Device Context:** None (or implicit if 1 Device)

**Primary User Goal:** Monitor Station-level state

**Primary Actions:**
- Navigate to Dashboard
- Navigate to Cultivation
- Navigate to Journal
- Switch Station (if >1 Station)

**Secondary Actions:**
- View Station settings
- View Device summaries

**Important Information:**
- Station identity (explicit)
- Station state
- Device count
- Active alerts

**State Transitions:**
- Station selected → Dashboard
- Station selected → Station switch → new Station selected

**Loading/Empty/Offline/Error States:**
- Loading: Skeleton layout
- Station offline: Degraded state with offline indicator

**Permission Behavior:**
- Requires `station:read` permission

**Navigation Entry/Exit Points:**
- Entry: From Fleet (after Station selection)
- Exit: To Dashboard, Cultivation, Journal, Settings

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station selector: [Station A ▼])                  │
├─────────────────────────────────────────────────────────────┤
│  STATION CONTEXT                                             │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Station: Station A                                      │ │
│  │ Status: [NORMAL]                                        │ │
│  │ Devices: 3                                              │ │
│  │                                                        │ │
│  │ [Go to Dashboard] → Overview                           │ │
│  │ [Cultivation] → Recipes                                │ │
│  │ [Journal] → Events                                     │ │
│  │ [Settings] → Configuration                            │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 6: Device Selected

**Page Purpose:** Device context activation (typically within Operations)

**Active Station Context:** Station (explicit)
**Active Device Context:** Device (explicit)

**Primary User Goal:** Control Device

**Primary Actions:**
- Control actuators (pumps, valves)
- View Device telemetry
- View FSM state
- Navigate to Operations

**Secondary Actions:**
- View Device settings
- View Controller/Sensor health

**Important Information:**
- Device identity (explicit, confirmable)
- Station identity (for context)
- Device telemetry
- FSM state
- Actuator states
- Controller/Sensor health

**State Transitions:**
- Device selected → Operations
- Device selected → Device switch → new Device selected

**Loading/Empty/Offline/Error States:**
- Loading: Skeleton layout
- Device offline: Degraded state with offline/stale semantics
- Sensor offline: Sensor-specific degraded state

**Permission Behavior:**
- Requires `device:read` permission
- Control requires `device:control` permission

**Navigation Entry/Exit Points:**
- Entry: From Dashboard (Device selection), Device selector
- Exit: To Dashboard, Settings, Journal

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station: [Station A] | Device: [Device A1 ▼])     │
├─────────────────────────────────────────────────────────────┤
│  DEVICE CONTEXT                                             │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Device: Device A1 (Station A)                          │ │
│  │ Status: [ONLINE]                                       │ │
│  │                                                        │ │
│  │ Telemetry:                                             │ │
│  │ EC: 1.5 mS/cm  pH: 6.2  Temp: 22°C  Water: 65%         │ │
│  │                                                        │ │
│  │ FSM State: [RUNNING]                                   │ │
│  │                                                        │ │
│  │ Actuators:                                             │ │
│  │ [Pump: ON]  [Valve: CLOSED]                            │ │
│  │                                                        │ │
│  │ [Control Actions] → Operations                         │ │
│  │ [Device Settings] → Settings                           │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 7: Operations / Device Control

**Page Purpose:** Device-scoped real-time control

**Active Station Context:** Station (explicit)
**Active Device Context:** Device (explicit)

**Primary User Goal:** Execute physical control operations on Device

**Primary Actions:**
- Start/stop actuators
- Adjust parameters
- View real-time telemetry
- View FSM state
- Emergency stop (always available)

**Secondary Actions:**
- View Controller/Sensor health
- View Device history
- Navigate to Device settings

**Important Information:**
- Device identity (confirmable)
- Station identity (for context)
- Real-time telemetry
- FSM state
- Actuator states
- Controller/Sensor health
- Active warnings

**State Transitions:**
- Operations → Dashboard
- Operations → Journal (event history)
- Operations → Settings (device configuration)

**Loading/Empty/Offline/Error States:**
- Loading: Skeleton layout
- Device offline: Degraded state with offline/stale semantics, do not imply command success
- Sensor offline: Sensor-specific degraded state, distinguish last-known from live telemetry
- Permission denied: Show read-only view or block dangerous controls

**Permission Behavior:**
- Requires `device:read` permission
- Control requires `device:control` permission
- Read-only view if user lacks `device:control`

**Navigation Entry/Exit Points:**
- Entry: From Dashboard (Device detail), navigation, Device alerts
- Exit: To Dashboard, Settings, Journal

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station: [Station A] | Device: [Device A1 ▼])     │
│  [Emergency Stop: Device A1]                                  │
├─────────────────────────────────────────────────────────────┤
│  OPERATIONS                                                  │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Device: Device A1 (Station A)                          │ │
│  │                                                        │ │
│  │ Real-time Telemetry:                                   │ │
│  │ EC: 1.5 mS/cm  pH: 6.2  Temp: 22°C  Water: 65%         │ │
│  │                                                        │ │
│  │ FSM State: [RUNNING]                                   │ │
│  │                                                        │ │
│  │ Actuator Controls:                                     │ │
│  │ [Pump: ON]  [Valve: CLOSED]  [Light: AUTO]           │ │
│  │                                                        │ │
│  │ Controller Health: [ONLINE]                            │ │
│  │ Sensor Health: [ONLINE]                               │ │
│  │                                                        │ │
│  │ [View History] → Journal                               │ │
│  │ [Device Settings] → Settings                           │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 8: Device Offline

**Page Purpose:** Device context with degraded state

**Active Station Context:** Station (explicit)
**Active Device Context:** Device (explicit, but offline)

**Primary User Goal:** Monitor Device (last-known state), understand offline status

**Primary Actions:**
- View last-known telemetry with explicit offline/stale semantics
- Switch to different Device
- View Device history

**Secondary Actions:**
- Navigate to Dashboard
- View Device settings

**Important Information:**
- Device identity (confirmable)
- Explicit offline/stale indicator
- Last-known telemetry
- Timestamp of last data
- Do NOT imply command success

**State Transitions:**
- Device offline → Device online (automatic refresh)
- Device offline → Switch Device

**Loading/Empty/Offline/Error States:**
- Current state: Offline (degraded)
- Last-known state displayed with explicit "offline" indicator
- Controls may be unavailable (depends on backend safety rules) [IMPLEMENTATION QUESTION]

**Permission Behavior:**
- Requires `device:read` permission
- Control availability depends on backend safety rules [IMPLEMENTATION QUESTION]

**Navigation Entry/Exit Points:**
- Entry: From Operations (when Device goes offline)
- Exit: To Dashboard, switch to different Device

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station: [Station A] | Device: [Device A1 ▼])     │
│  [Emergency Stop: Device A1]                                  │
├─────────────────────────────────────────────────────────────┤
│  OPERATIONS (DEVICE OFFLINE)                                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Device: Device A1 (Station A)                          │ │
│  │ Status: [OFFLINE] ⚠️                                    │
│  │ Last seen: 2 hours ago                                  │ │
│  │                                                        │ │
│  │ Last-known Telemetry (stale):                          │ │
│  │ EC: 1.5 mS/cm*  pH: 6.2*  Temp: 22°C*  Water: 65%*     │ │
│  │ *Last updated: 2 hours ago                              │ │
│  │                                                        │ │
│  │ FSM State: [UNKNOWN]                                   │ │
│  │                                                        │ │
│  │ Actuator Controls:                                     │ │
│  │ [Pump: UNAVAILABLE]  [Valve: UNAVAILABLE]               │ │
│  │                                                        │ │
│  │ [Switch Device] → Device selector                      │ │
│  │ [View History] → Journal                               │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 9: Sensor Offline

**Page Purpose:** Device context with Sensor unavailable

**Active Station Context:** Station (explicit)
**Active Device Context:** Device (explicit)
**Sensor State:** Offline

**Primary User Goal:** Monitor Device, understand Sensor unavailability

**Primary Actions:**
- View last-known sensor readings with explicit offline indicator
- View non-sensor telemetry (may be live)
- Controls may be available depending on safety rules [IMPLEMENTATION QUESTION]

**Secondary Actions:**
- View Controller health
- Navigate to Dashboard

**Important Information:**
- Explicit Sensor offline indicator
- Distinguish last-known sensor telemetry from live telemetry
- Device control availability depends on controller/FSM safety state [IMPLEMENTATION QUESTION]
- Do NOT imply operation is safe unless backend/controller state confirms it

**State Transitions:**
- Sensor offline → Sensor online (automatic refresh)
- Sensor offline → Continue with degraded control (if FSM allows) [IMPLEMENTATION QUESTION]

**Loading/Empty/Offline/Error States:**
- Current state: Sensor offline (degraded)
- Sensor telemetry: Last-known with explicit "sensor offline" indicator
- Non-sensor telemetry: May be live
- Control availability: Depends on device safety state [IMPLEMENTATION QUESTION]

**Permission Behavior:**
- Requires `device:read` permission
- Control availability depends on backend safety rules [IMPLEMENTATION QUESTION]

**Navigation Entry/Exit Points:**
- Entry: From Operations (when Sensor goes offline)
- Exit: To Dashboard

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station: [Station A] | Device: [Device A1 ▼])     │
│  [Emergency Stop: Device A1]                                  │
├─────────────────────────────────────────────────────────────┤
│  OPERATIONS (SENSOR OFFLINE)                                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Device: Device A1 (Station A)                          │ │
│  │ Status: [ONLINE]                                       │
│  │ Sensor: [OFFLINE] ⚠️                                    │
│  │                                                        │ │
│  │ Telemetry:                                             │ │
│  │ EC: 1.5 mS/cm* (last known 1h ago)                      │ │
│  │ pH: 6.2* (last known 1h ago)                           │ │
│  │ Temp: 22°C (live)                                      │ │
│  │ Water: 65% (live)                                      │ │
│  │                                                        │ │
│  │ FSM State: [RUNNING]                                   │ │
│  │                                                        │ │
│  │ Actuator Controls:                                     │ │
│  │ [Pump: ON]  [Valve: CLOSED]  (may be limited)          │ │
│  │                                                        │ │
│  │ Controller Health: [ONLINE]                            │ │
│  │ Sensor Health: [OFFLINE] ⚠️                             │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 10: Station with Zero Devices

**Page Purpose:** Provisioning state for empty Station

**Active Station Context:** Station (explicit)
**Active Device Context:** None (provisioning state)

**Primary User Goal:** Add Device to Station

**Primary Actions:**
- Navigate to Pairing
- Add Device

**Secondary Actions:**
- View Station-level configuration
- Navigate to Settings

**Important Information:**
- Station identity
- Station is valid but has no Devices
- Clear path to Pair/Add Device
- Device control NOT available

**State Transitions:**
- Station with zero Devices → Pairing → Device added → Station with Device

**Loading/Empty/Offline/Error States:**
- Current state: Provisioning (no Devices)
- Empty state with provisioning prompt

**Permission Behavior:**
- Requires `station:read` permission
- Pairing requires `station:manage` or `device:pair` permission [OPEN QUESTION]

**Navigation Entry/Exit Points:**
- Entry: From Dashboard (if Station has 0 Devices)
- Exit: To Pairing, Settings

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station: [Station A])                              │
├─────────────────────────────────────────────────────────────┤
│  STATION (ZERO DEVICES)                                     │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Station: Station A                                      │ │
│  │ Status: [NO DEVICES]                                    │ │
│  │                                                        │ │
│  │ This Station has no Devices yet.                       │ │
│  │                                                        │ │
│  │ [Add Device] → Pairing                                 │ │
│  │                                                        │ │
│  │ Station-level configuration available:                 │ │
│  │ [Station Settings] → Settings                           │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 11: Cultivation

**Page Purpose:** Station-level cultivation intent and recipe management

**Active Station Context:** Station (explicit)
**Active Device Context:** None (unless Device-specific capability)

**Primary User Goal:** Configure cultivation recipes and seasons

**Primary Actions:**
- Select recipe
- Configure season
- View cultivation status

**Secondary Actions:**
- View dosing history
- Navigate to Operations (for execution)

**Important Information:**
- **OPEN QUESTION [CULTIVATION SCOPE]:** Recipe/season ownership may be Station-level or Device-level
- Design supports both models
- If Station-level: Recipes apply to all Devices in Station
- If Device-level: Recipes apply to specific Device
- UI must remain consistent with final product decision

**State Transitions:**
- Cultivation → Dashboard (apply configuration)
- Cultivation → Operations (execution)

**Loading/Empty/Offline/Error States:**
- Loading: Skeleton layout
- Empty: No recipes configured
- Error: Configuration save failed

**Permission Behavior:**
- Requires `station:read` permission
- Configuration changes require `cultivation:edit` permission [REPOSITORY EVIDENCE]

**Navigation Entry/Exit Points:**
- Entry: From navigation, Dashboard (setup flow)
- Exit: To Dashboard, Operations, Journal

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station: [Station A])                              │
├─────────────────────────────────────────────────────────────┤
│  CULTIVATION [OPEN QUESTION: SCOPE]                         │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Cultivation for: Station A [or Device: Device A1]     │ │
│  │ *Scope depends on product decision*                     │ │
│  │                                                        │ │
│  │ Current Recipe: [Basil - Growth Stage]                 │ │
│  │ Season: [Spring 2026]                                   │ │
│  │                                                        │ │
│  │ [Select Recipe]                                         │ │
│  │ [Configure Season]                                      │ │
│  │ [View Dosing History] → Journal                         │ │
│  │                                                        │ │
│  │ [Apply to All Devices] (if Station-level)             │ │
│  │ [Apply to Device A1] (if device-level)                 │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 12: Journal

**Page Purpose:** Station + Device event history

**Active Station Context:** Station (explicit)
**Active Device Context:** None (filtering available)

**Primary User Goal:** Review event history, investigate issues

**Primary Actions:**
- View events
- Filter by Device
- Filter by event type
- Export events (for compliance)

**Secondary Actions:**
- Navigate to Operations (investigate issue)
- Navigate to Device source (drill down)

**Important Information:**
- Station-level events
- Device-level events with source identity
- Event filtering
- Event timestamps

**State Transitions:**
- Journal → Operations (investigate issue)
- Journal → Device source (drill down)

**Loading/Empty/Offline/Error States:**
- Loading: Skeleton layout
- Empty: No events
- Error: Export failed

**Permission Behavior:**
- Requires `station:read` permission
- Export may require additional permissions [OPEN QUESTION]

**Navigation Entry/Exit Points:**
- Entry: From navigation, alerts/events
- Exit: To Operations, Device source

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station: [Station A] | Device Filter: [All ▼])   │
├─────────────────────────────────────────────────────────────┤
│  JOURNAL                                                     │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Event History for Station A                             │ │
│  │                                                        │ │
│  │ Filter: [All] [Alerts] [Errors] [Info]                 │ │
│  │                                                        │ │
│  │ Events:                                                │ │
│  │ ┌───────────────────────────────────────────────────┐ │ │
│  │ │ 10:30 AM  Device A1  Sensor offline  [Alert]     │ │ │
│  │ │ 09:15 AM  Device A2  Pump started   [Info]        │ │ │
│  │ │ 08:00 AM  Station A   Recipe applied  [Info]       │ │ │
│  │ └───────────────────────────────────────────────────┘ │ │
│  │                                                        │ │
│  │ [Load More]                                            │ │
│  │ [Export] → CSV (if permission)                         │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 13: Settings

**Page Purpose:** Multi-scope configuration (User, Station, Device, Controller, Sensor)

**Active Station Context:** Depends on setting category
**Active Device Context:** Depends on setting category

**Primary User Goal:** Configure system at various scopes

**Primary Actions:**
- Navigate to setting category
- Modify settings
- Save changes

**Secondary Actions:**
- Navigate to previous page
- Cancel changes

**Important Information:**
- Settings grouped by scope:
  - USER-level: account, notifications
  - STATION-level: metadata, policies
  - DEVICE-level: thresholds, actuator config
  - CONTROLLER-level: FSM params
  - SENSOR-level: calibration

**State Transitions:**
- Settings → Return to previous page
- Settings → Dashboard

**Loading/Empty/Offline/Error States:**
- Loading: Skeleton layout
- Error: Save failed

**Permission Behavior:**
- USER-level: Always accessible
- STATION-level: Requires `station:config` permission [OPEN QUESTION]
- DEVICE-level: Requires `device:config` permission [REPOSITORY EVIDENCE]
- If user lacks permission: Show access denied, disable settings

**Navigation Entry/Exit Points:**
- Entry: From navigation, specific pages (contextual settings)
- Exit: To previous page, Dashboard

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station: [Station A] | Device: [Device A1 ▼])     │
├─────────────────────────────────────────────────────────────┤
│  SETTINGS                                                    │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Settings Scope: [Station A] [Device A1] [User]       │ │
│  │                                                        │ │
│  │ Categories:                                            │ │
│  │ • Account (User)                                       │ │
│  │ • Notifications (User)                                  │ │
│  │ • Station Configuration (Station)                       │ │
│  │ • Device Configuration (Device)                         │ │
│  │ • Controller Settings (Device)                           │ │
│  │ • Sensor Calibration (Device)                            │ │
│  │                                                        │ │
│  │ [Current Category Content]                              │ │
│  │                                                        │ │
│  │ [Save] [Cancel]                                         │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 14: Pairing

**Page Purpose:** Station → Device provisioning

**Active Station Context:** Station (optional, may create during pairing)
**Active Device Context:** None (until paired)

**Primary User Goal:** Add Device to Station

**Primary Actions:**
- Select Station (if multiple)
- Scan/identify Device
- Configure Controller/Sensor
- Validate connectivity

**Secondary Actions:**
- Cancel pairing
- View pairing history

**Important Information:**
- Station selection (if multiple)
- Device discovery status
- Controller/Sensor configuration steps
- Connectivity validation

**State Transitions:**
- Pairing → Device added → Station context updated
- Pairing failed → Retry or cancel

**Loading/Empty/Offline/Error States:**
- Loading: Discovery in progress
- Error: Pairing failed, show reason
- Device already paired: [OPEN QUESTION - Device movement between Stations]

**Permission Behavior:**
- Requires `station:manage` or `device:pair` permission [OPEN QUESTION]
- If user lacks permission: Show access denied, hide pairing option

**Navigation Entry/Exit Points:**
- Entry: From Fleet, Dashboard (provisioning prompt)
- Exit: To Dashboard (after successful pairing)

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station: [Station A ▼] or [Create Station])     │
├─────────────────────────────────────────────────────────────┤
│  PAIRING                                                     │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Step 1: Select Station                                  │ │
│  │ [Station A] (already selected)                          │ │
│  │                                                        │ │
│  │ Step 2: Discover Device                                 │ │
│  │ [Scan for Devices] [Enter Device ID]                   │ │
│  │                                                        │ │
│  │ Step 3: Configure Controller/Sensor                    │ │
│  │ [Controller Settings] [Sensor Calibration]               │ │
│  │                                                        │ │
│  │ Step 4: Validate Connectivity                            │ │
│  │ [Validate] ... [ONLINE] ✅                              │ │
│  │                                                        │ │
│  │ [Complete Pairing] → Dashboard                          │ │
│  │ [Cancel]                                               │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 15: Config Backup

**Page Purpose:** Explicit scope configuration export/import

**Active Station Context:** Depends on backup scope
**Active Device Context:** Depends on backup scope

**Primary User Goal:** Export or import configuration

**Primary Actions:**
- Select backup scope
- Export configuration
- Import configuration

**Secondary Actions:**
- Cancel operation
- View backup history

**Important Information:**
- **OPEN QUESTION [BACKUP SCOPE]:** Backup scope may be Station, Device, Controller, or combination
- Design supports all models
- UI must remain consistent with final product decision

**State Transitions:**
- Backup → Export complete
- Backup → Import complete

**Loading/Empty/Offline/Error States:**
- Loading: Export/import in progress
- Error: Export/import failed

**Permission Behavior:**
- Requires permission matching backup scope [OPEN QUESTION]
- If user lacks permission: Show access denied, disable backup

**Navigation Entry/Exit Points:**
- Entry: From Settings
- Exit: To Settings

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station: [Station A] | Device: [Device A1 ▼])     │
├─────────────────────────────────────────────────────────────┤
│  CONFIG BACKUP [OPEN QUESTION: SCOPE]                       │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Backup Scope: [Station A] [Device A1] [All]          │ │
│  │ *Scope depends on product decision*                     │ │
│  │                                                        │ │
│  │ Actions:                                                │ │
│  │ [Export Configuration] → Download JSON                   │ │
│  │ [Import Configuration] → Upload JSON                     │ │
│  │                                                        │ │
│  │ Backup History:                                        │ │
│  │ • 2026-09-10  Station A backup                         │ │
│  │ • 2026-09-05  Device A1 backup                         │ │
│  │                                                        │ │
│  │ [Cancel] → Settings                                     │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

### Wireframe 16: Roles / Access Management

**Page Purpose:** User and permission management

**Active Station Context:** User/access scope
**Active Device Context:** Station/Device (for permission assignment)

**Primary User Goal:** Manage users and permissions

**Primary Actions:**
- Add user
- Assign permissions
- Revoke permissions
- View access audit

**Secondary Actions:**
- Navigate to Settings
- Cancel changes

**Important Information:**
- User roles (Admin, Operator, Viewer) [REPOSITORY EVIDENCE]
- Station access permissions
- Device access permissions
- Permission inheritance/override behavior [OPEN QUESTION]

**State Transitions:**
- Roles → Settings
- Roles → Save changes

**Loading/Empty/Offline/Error States:**
- Loading: User list loading
- Error: Permission change failed

**Permission Behavior:**
- Requires `admin` permission [REPOSITORY EVIDENCE]
- If user lacks permission: Show access denied, redirect to Settings

**Navigation Entry/Exit Points:**
- Entry: From Settings
- Exit: To Settings

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER                                                      │
├─────────────────────────────────────────────────────────────┤
│  ROLES / ACCESS MANAGEMENT                                   │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Users:                                                 │ │
│  │ ┌───────────────────────────────────────────────────┐ │ │
│  │ │ user@example.com  [Admin]  [Edit] [Revoke]      │ │ │
│  │ │ operator@example.com  [Operator]  [Edit] [Revoke] │ │ │
│  │ │ viewer@example.com  [Viewer]  [Edit] [Revoke]   │ │ │
│  │ └───────────────────────────────────────────────────┘ │ │
│  │                                                        │ │
│  │ [Add User]                                             │ │
│  │                                                        │ │
│  │ Station Access:                                       │ │
│  │ [Station A]  [Station B]  [Station C]                   │ │
│  │                                                        │ │
│  │ Device Access:                                         │ │
│  │ [Device A1]  [Device A2]  [Device B1]                   │ │
│  │                                                        │ │
│  │ [Save] [Cancel] → Settings                              │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Navigation Flow

### Flow 1: User → Station → Device → Operations

```
User (authentication)
  ↓
Fleet (if >1 Stations) OR Dashboard (if 1 Station)
  ↓
Station selection (if >1 Stations)
  ↓
Dashboard (Station context active)
  ↓
Device selection (if >1 Devices)
  ↓
Operations (Station + Device context active)
```

### Flow 2: Fleet → Station → Device → Operations → Journal

```
Fleet (User context)
  ↓
Station selection
  ↓
Dashboard (Station context)
  ↓
Device selection
  ↓
Operations (Station + Device context)
  ↓
Journal (Station + Device context preserved)
```

---

## Context Transition Diagrams

### Transition 1: Station A + Device A1 → Station A + Device A2

```
Current: Station A, Device A1
User action: Change Device selector to Device A2
Result: Station A, Device A2
Context preserved: Station A
Context changed: Device A1 → Device A2
Page refresh: Operations shows Device A2 data
```

### Transition 2: Station A + Device A1 → Station B + Device B1

```
Current: Station A, Device A1
User action: Change Station selector to Station B
System: Clears Device A1 context, sets Station B context
Result: Station B, Device B1 (or implicit if 1 Device)
Context preserved: None (both cleared)
Context changed: Station A → Station B, Device A1 → Device B1
Page refresh: Dashboard shows Station B data
```

### Transition 3: Station A + no Device → Pairing → Station A + Device A1

```
Current: Station A, no Device (provisioning state)
User action: Navigate to Pairing, add Device
System: Associates Device with Station, provisions Controller/Sensor
Result: Station A, Device A1 (implicit)
Context preserved: Station A
Context changed: no Device → Device A1
Page refresh: Dashboard shows Device A1
```

---

## Page-to-Page Interaction Map

```
┌─────────────┐
│  Authentication │
└──────┬────────┘
       │
       ├─→ Fleet (if >1 Stations)
       │
       └─→ Dashboard (if 1 Station)
              │
              ├─→ Operations (Device context required)
              │    │
              │    ├─→ Journal (context preserved)
              │    │
              │    └─→ Settings (Device context preserved)
              │
              ├─→ Cultivation (Station context)
              │    │
              │    └─→ Journal
              │
              ├─→ Journal (Station context)
              │
              └─→ Settings (Station context)
                   │
                   ├─→ Pairing
                   │
                   ├─→ Config Backup
                   │
                   └─→ Roles / Access
```

---

## Edge-State Wireframes

### Edge State 1: Mixed Online/Offline Devices in Station

**Context:** Station with some Devices online, some offline

**Behavior:**
- Station status shows DEGRADED [TARGET SEMANTIC MODEL]
- Dashboard shows Device status mix
- User can select between online and offline Devices
- Offline Devices show last-known state with offline indicator
- Station-level alerts may be triggered based on severity [OPEN QUESTION]

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER (Station: [Station A] | Status: [DEGRADED])         │
├─────────────────────────────────────────────────────────────┤
│  DASHBOARD                                                   │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ Station: Station A  Status: [DEGRADED]                 │ │
│  │ Devices: 3 (2 online, 1 offline)                         │ │
│  │                                                        │ │
│  │ Device Summaries:                                      │ │
│  │ ┌───────────────────────────────────────────────────┐ │ │
│  │ │ Device A1  [ONLINE]  EC: 1.5  pH: 6.2  [Select] │ │ │
│  │ │ Device A2  [OFFLINE] Last seen: 2h ago  [Select] │ │ │
│  │ │ Device A3  [ONLINE]  EC: 1.6  pH: 6.1  [Select] │ │ │
│  │ └───────────────────────────────────────────────────┘ │ │
│  │                                                        │ │
│  │ Station Alert: "One Device offline"                   │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Edge State 2: Permission Denied

**Context:** User lacks permission for action

**Behavior:**
- Show permission denied error
- Explain which permission is required
- If action critical: Prompt to contact admin
- If action non-critical: Disable action or hide it
- Context remains unchanged

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER                                                      │
├─────────────────────────────────────────────────────────────┤
│  PERMISSION DENIED                                           │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                                                        │ │
│  │  ⚠️ Permission Denied                                 │ │
│  │                                                        │ │
│  │  You do not have permission to perform this action.   │ │
│  │                                                        │ │
│  │  Required permission: device:control                 │ │
│  │                                                        │ │
│  │  [Contact Administrator]                                │ │
│  │  [Go Back]                                              │ │
│  │                                                        │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Edge State 3: Deleted/Unavailable Station

**Context:** Selected Station deleted or access revoked

**Behavior:**
- Immediate notification: "Station no longer available"
- Redirect to Fleet
- Clear Station context
- Clear Device context
- Require new Station selection

**Layout:**
```
┌─────────────────────────────────────────────────────────────┐
│  HEADER                                                      │
├─────────────────────────────────────────────────────────────┤
│  STATION UNAVAILABLE                                       │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                                                        │ │
│  │  ⚠️ Station No Longer Available                        │ │
│  │                                                        │ │
│  │  The Station you were viewing is no longer available. │ │
│  │                                                        │ │
│  │  [Select Another Station] → Fleet                     │ │
│  │                                                        │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Unresolved UX/Product Dependencies

### Implementation Questions (Transport/Control Behavior)

1. **Device offline command behavior** [IMPLEMENTATION QUESTION]: Are commands rejected locally, queued, or dispatched and fail?
2. **Emergency stop behavior** [IMPLEMENTATION QUESTION]: Does emergency stop work when Device is offline? What are the safety rules?
3. **Sensor offline control availability** [IMPLEMENTATION QUESTION]: Which individual controls remain available when Sensor is offline? How is this derived from device safety state?
4. **Active operation during Device switch** [IMPLEMENTATION QUESTION]: What happens to an active operation when user switches Device? Is it cancelled or does it continue?
5. **Active operation during Station switch** [IMPLEMENTATION QUESTION]: What happens to an active operation when user switches Station? Is it cancelled or does it continue?
6. **Context persistence implementation** [IMPLEMENTATION QUESTION]: Should context be persisted locally or via backend user preferences?

### Open Questions (Product Decisions Required)

1. **Station status aggregation rules** [OPEN QUESTION]: Exact threshold rules for Station status (NORMAL/DEGRADED/CRITICAL/UNKNOWN)
2. **Device movement between Stations** [OPEN QUESTION]: Can a Device move between Stations?
3. **Station creation during pairing** [OPEN QUESTION]: Does pairing create a Station, or must Station exist first?
4. **Backup scope** [OPEN QUESTION]: Is backup scope Station, Device, Controller, or combination?
5. **Cultivation configuration scope** [OPEN QUESTION]: Are recipes/seasons Station-level or Device-level?
6. **Notification preferences** [OPEN QUESTION]: Exact notification preferences per category
7. **Emergency stop acknowledgement lifecycle** [OPEN QUESTION]: Exact acknowledgement requirements
8. **Permission inheritance/override behavior** [OPEN QUESTION]: Does Station access imply Device access?
9. **Health score semantics** [OPEN QUESTION]: Should health score be Device-level or Station-level?
10. **MQTT topic hierarchy** [IMPLEMENTATION QUESTION]: Does MQTT need Station identity?

---

## Final Report

### Screens Produced

16 wireframe scenarios:
1. First-time / no Station
2. Single Station + single Device Dashboard
3. Single Station + multiple Devices Dashboard
4. Multi-Station Fleet
5. Station selected
6. Device selected
7. Operations / Device control
8. Device offline
9. Sensor offline
10. Station with zero Devices
11. Cultivation
12. Journal
13. Settings
14. Pairing
15. Config Backup
16. Roles / access management

Plus:
- Navigation flow diagrams
- Context transition diagrams
- Page-to-page interaction map
- Edge-state wireframes

### Major UX Decisions

1. **ONE stable navigation architecture** with adaptive context (not radically adaptive navigation)
2. **Progressive complexity** based on actual context (Station count, Device count, permissions) rather than guessed persona
3. **Station = organizational context, Device = operational context** distinction enforced throughout
4. **Context persistence** rules defined (remember last valid Station/Device, never restore deleted/inaccessible)
5. **Device selection safety** - never allow operation to silently execute against wrong Device
6. **Emergency stop** always discoverable, requires explicit target, never executes against ambiguous target
7. **State semantics** explicitly distinguished (zero, no data, offline, stale, error, unknown, not configured)
8. **Offline behavior** - show last-known state with explicit offline/stale semantics, do not imply command success
9. **Cultivation scope** flagged as OPEN QUESTION, wireframe structured to support both Station-level and Device-level models
10. **Backup scope** flagged as OPEN QUESTION, wireframe structured to support all models

### Context Model Used

```
User
  → Station (organizational context)
      → Device (operational context)
          → Controller Node (component of Device)
          → Sensor Node (component of Device)
```

**Context States:**
- No Station Context
- Station Context Active, No Device Context
- Station Context Active, Device Context Implicit (1 Device)
- Station Context Active, Device Context Explicit (>1 Device)

**Context Lifecycle:**
- On Login: Auto-select or restore context
- On Station Switch: Clear Device context, resolve new Device
- On Device Switch: Update Device context, preserve Station
- On Context Invalidation: Notify user, clear context, redirect

### Remaining Unresolved Product Dependencies

**Implementation Questions (6):**
1. Device offline command behavior
2. Emergency stop behavior
3. Sensor offline control availability
4. Active operation during Device switch
5. Active operation during Station switch
6. Context persistence implementation

**Open Questions (10):**
1. Station status aggregation rules
2. Device movement between Stations
3. Station creation during pairing
4. Backup scope
5. Cultivation configuration scope
6. Notification preferences
7. Emergency stop acknowledgement lifecycle
8. Permission inheritance/override behavior
9. Health score semantics
10. MQTT topic hierarchy

### Conflicts with Functional Specification

**None.** The wireframes are fully aligned with `docs/FRONTEND_FUNCTIONAL_SPEC.md` and the finalized navigation architecture.

### Wireframe Phase Status

**READY FOR VISUAL DESIGN**

The wireframes establish the structural UX and are ready for visual styling. The navigation architecture is stable, context model is defined, and all edge cases are documented. The unresolved implementation questions and open questions are product/backend decisions that do not block visual design work.
