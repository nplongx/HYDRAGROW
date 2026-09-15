# HYDRAGROW Journal — Functional Specification

**Route:** `/journal`  
**Primary role:** canonical historical trace and event-inspection surface for the selected device/station  
**Status:** Implementation-ready specification; current source is partially aligned and has explicit implementation gaps.  
**Canonical checklist:** `docs/design-system/PAGE-SPEC-CHECKLIST.md`

---

## 0. Purpose

Journal is the canonical historical trace for `system_events` exposed by the backend. It lets users inspect recorded events, chronology, severity, category, technical context, and event-record resolution state.

Journal does not own current physical state, live actuator control, persistent configuration, Automation authoring, or cultivation lifecycle.

### Core semantic boundary

Historical event record is not current device state, current telemetry, physical truth, or a guarantee of complete backend history.

Event existence does not prove that the underlying physical condition still exists or that a requested action physically completed.

---

## 1. Page Identity & Responsibility

### 1.1 Primary purpose

Provide one device-scoped place to inspect historical events, filter/search loaded records, inspect event details, group related cycle events, mark event records resolved/reopened, export loaded records, and inspect supported analytics.

### 1.2 User problem

Journal must let users answer what happened, when it was recorded, which device was involved, what severity/category/source it has, what technical context is available, and whether the event record is marked resolved.

### 1.3 Roles

- `admin`: inspect and use event-write capabilities allowed by backend authorization.
- `operator`: inspect and acknowledge/reopen when granted `events:write`.
- `viewer`: inspect; no acknowledge/reopen without backend authorization.

Backend authorization is authoritative. UI role labels are not a security boundary.

### 1.4 Journal owns

- Historical `system_events` inspection.
- Timeline, category filtering, local search, Important/All Technical presentation.
- Cycle grouping from source-backed cycle identifiers.
- Event detail and metadata inspection.
- Event-record acknowledge/reopen.
- CSV export of currently loaded records.
- Event-derived recent health summary.
- Embedded supported Analytics surface.

### 1.5 Journal does not own

- Live actuator control -> Operations.
- Automatic workflow authoring/execution -> Automation.
- Persistent configuration -> Settings.
- Cultivation lifecycle -> Cultivation.
- Device pairing/provisioning -> Pairing/Fleet.
- Member/role administration -> Roles.
- Physical confirmation of an actuator/process.

### 1.6 Entry / exit

- Main navigation -> `/journal`.
- `/logs` redirects to `/journal`.
- `/analytics` redirects to `/journal`.
- Device-scoped navigation may enter Journal for historical inspection.
- Device-scoped exits preserve selected `deviceId`.

---

## 2. Core Product Rules

1. Journal is the canonical historical event trace.
2. Event history is not current state.
3. No event is not proof of health.
4. Resolved event record is not proof of physical recovery.
5. Event `success` is not physical completion without explicit source evidence.
6. Loaded history is not automatically complete backend history.
7. Local search is not backend-wide search.
8. Loaded CSV is not automatically a full-history export.
9. `last_seen` is device contact information, not event freshness.
10. Event `timestamp` is event-record time, not generic telemetry freshness.
11. Missing data is not silently converted to zero, Off, healthy, or success.
12. Shared data does not transfer mutation ownership to Journal.

---

## 3. Source of Truth

### 3.1 Frontend

- `hydragrow-frontend/src/pages/Journal.tsx`
- `hydragrow-frontend/src/pages/SystemLog.tsx`
- `hydragrow-frontend/src/pages/Analytics.tsx`
- `hydragrow-frontend/src/components/logs/EventLogCard.tsx`
- `hydragrow-frontend/src/components/logs/EventDetailDrawer.tsx`
- `hydragrow-frontend/src/components/logs/CycleEventCard.tsx`
- `hydragrow-frontend/src/components/logs/HealthSummaryBar.tsx`
- `hydragrow-frontend/src/lib/logs/eventGrouping.ts`
- `hydragrow-frontend/src/hooks/useSystemHealthSummary.ts`
- `hydragrow-frontend/src/hooks/useHealthHistory.ts`
- `hydragrow-frontend/src/store/useDeviceStore.ts`
- `hydragrow-frontend/src/types/models.ts`
- `hydragrow-frontend/src/App.tsx`

### 3.2 Backend

- `hydragrow-backend/src/api/alert.rs`
- `hydragrow-backend/src/api/analytics.rs`
- `hydragrow-backend/src/db/postgres.rs`
- `hydragrow-backend/src/main.rs`
- relevant event-producing MQTT/API handlers and `system_events` migrations.

### 3.3 Current API surface

- `GET /api/devices/{device_id}/events`
- `GET /api/devices/{device_id}/events/cycle/{cycle_id}`
- `PUT /api/devices/{device_id}/events/{event_id}/acknowledge`
- `GET /api/devices/{device_id}/health-summary`
- `GET /api/devices/{device_id}/analytics/health`
- `GET /api/devices/{device_id}/analytics/dosing-history`

The API is mounted below the authenticated `/api` scope and device scope. Backend authorization remains authoritative.

### 3.4 Source status vocabulary

- `Source-backed`: verified current repository/backend behavior.
- `Partially source-backed`: source exists but UI/semantics are incomplete.
- `Implementation-required`: additional work required.
- `Deferred`: intentionally outside current contract.

The specification never presents `Implementation-required` as implemented.

---

## 4. Selected Device / Station Context

Current canonical identity is `useDeviceStore().deviceId`.

Rules:

- Event query, acknowledge/reopen, health-summary, and device-health use selected device.
- Device A events must never render as Device B events.
- Device A async results must never update Device B UI.
- Missing context is not an empty Journal.
- Context switching invalidates/replaces old event presentation before new-device data is current.
- Pending mutation identity is at minimum `(deviceId, eventId)`.
- No richer station object may be invented.

The current Journal source has no page-local station selector. Selector integration is `Implementation-required` unless supplied by the shell.

---

## 5. Information Architecture

Journal has two canonical tabs:

1. `Sự kiện`
2. `Phân tích`

`Sự kiện` contains Health Summary, Search, Important/All Technical, category filters, Event Timeline, Cycle Groups, Event Detail, and CSV Export.

`Phân tích` contains Hardware Health, Session Health Trends, Weekly Report Preference, and Grafana Dashboard.

`Journal.tsx` owns the tab shell. Event detail is a secondary surface, not a new top-level page.

---

## 6. Component Inventory

| Component | Source | Interaction | Status |
|---|---|---|---|
| Journal tabs | `Journal.tsx` | Switch tab | Source-backed |
| Health Summary | `useSystemHealthSummary` | Inspect | Source-backed; error UI incomplete |
| Search | `filterEventsBySearch` | Filter loaded records | Source-backed |
| Category filters | `SystemLog.tsx` | Query/filter | Source-backed |
| Important/Technical | `eventGrouping.ts` | Presentation switch | Source-backed |
| CSV export | `handleExportCSV` | Generate local file | Source-backed; loaded-data scope |
| Event timeline | `SystemLog.tsx` | Inspect | Source-backed |
| Event card | `EventLogCard` | Detail/metadata/resolve | Source-backed; mutation state incomplete |
| Cycle card | `CycleEventCard` | Open event detail | Source-backed; loaded-data scope |
| Event detail | `EventDetailDrawer` | Open/close | Source-backed |
| Analytics health | `Analytics.tsx` | Refresh | Source-backed |
| Session health trend | `useHealthHistory` | Inspect | Source-backed; session-local |
| Weekly report | `Analytics.tsx` | Toggle | Source-backed; lifecycle incomplete |
| Grafana | `Analytics.tsx` | Open/refresh | Source-backed configured source; fallback gap |

Any new clickable element requires an explicit outcome contract before implementation.

---

## 7. Event Data Contract

Backend `SystemEventRecord` contains these fields: `id`, `device_id`, `level`, `category`, `title`, `message`, `reason`, `metadata`, `timestamp`, `source`, `primary_reason_code`, `resolved_at`.

### 7.1 Field semantics

| Field | Meaning |
|---|---|
| `id` | Database event identifier |
| `device_id` | Device owning record |
| `level` | Event severity/classification |
| `category` | Event domain/category |
| `title` | Human-readable event title |
| `message` | Event description/context |
| `reason` | Optional reason/error information |
| `metadata` | Event-specific structured technical context |
| `timestamp` | Event-record epoch-millisecond timestamp |
| `source` | Producer classification |
| `primary_reason_code` | Optional primary supervisor reason code |
| `resolved_at` | Event-record resolution timestamp |

Frontend `SystemEvent` currently omits `source` and `primary_reason_code`. This is a type/traceability gap, not permission to invent fields.

### 7.2 Severity

Source-backed values: `info`, `success`, `warning`, `critical`.

`success` is not physical completion. `critical` is not automatic proof of physical danger.

### 7.3 Category

Source-backed shared categories: `system`, `dosing`, `water`, `calibration`, `sensor`, `alert`, `user_action`, `device`, `automation`.

Visible filter list is not an exhaustive backend taxonomy.

### 7.4 Source

Current backend constraint: `rule`, `watchdog`, `ai_supervisor`.

Source identifies producer class, not correctness, physical completion, or user identity.

### 7.5 Reason codes

Current source-backed values include `ec_out_of_range`, `ec_degrading`, `ph_out_of_range`, `ph_degrading`, `water_level_out_of_range`, `water_level_degrading`, `temp_out_of_range`, `temp_degrading`, `leak_suspected`, `sensor_fault_suspected`, `dosing_ineffective`, `unexplained_anomaly`, and `topic_stale_controller_status`.

`suspected` and `anomaly` values must not be presented as confirmed diagnoses.

---

## 8. Data Integrity Rules

The following distinctions are mandatory: Event Record is not Current State; Event Success is not Physical Success; `resolved_at` is not Fault Cleared; `timestamp` is not Telemetry Freshness; `last_seen` is not Event Freshness; No Event is not System Healthy.

Do not infer current sensor health from an empty sensor-event filter, current actuator state from event text, physical dose delivery from event volume, actor identity without actor source, or complete history from loaded row count.

---

## 9. Event Timeline Contract

Current frontend query uses `GET /api/devices/{deviceId}/events?limit=200`, with optional `category` and `before_timestamp`. Backend also supports `after_timestamp` and `level`; current UI does not expose those controls.

Without `after_timestamp`, backend returns newest first by `timestamp`.

Current page size is `200`.

Pagination uses the oldest loaded timestamp as `before_timestamp`, then appends older records. Existing records remain visible while older records load. No duplicate records.

Pagination does not imply unlimited retention or complete-history count.

The repository has `system_events` retention/index migrations. No exact retention period is invented here.

---

## 10. Filter Contract

Current filters: `all`, `unresolved`, `alert`, `dosing`, `water`, `device`, `sensor`, `automation`, `user_action`, `system`.

`unresolved` is a local filter over loaded events: `resolved_at` absent/null.

Category filters query the backend. `user_action` currently maps to `user_action,alert` in the frontend.

Filters never mutate event records or selected device context.

---

## 11. Search Contract

Current local search fields are `title`, `message`, `category`, and `reason`. Matching is case-insensitive substring matching.

Search scope is currently loaded events only. It is not backend-wide historical search.

Backend-wide search is `Deferred`/`Implementation-required` only if later required.

Search empty must be distinct from true empty history.

---

## 12. Important vs All Technical

Important mode may merge consecutive `info` events sharing category and title. Mergeable technical categories are `system`, `sensor`, and `calibration`.

Warning, critical, and success are not merged by this technical-noise rule.

All Technical keeps technical events individually represented after cycle grouping.

Important mode is presentation transformation. It does not delete records or change severity. All Technical does not mean every backend log exists in `system_events`.

---

## 13. Cycle Grouping Contract

Cycle ID is extracted from `metadata.cycle_id` or `metadata.dosing_data.cycle_id`.

Loaded events sharing a cycle ID may be represented by one Cycle card in both modes. Individual events remain inspectable.

Backend exposes `GET /api/devices/{device_id}/events/cycle/{cycle_id}`, but current Cycle card does not call it. Current grouping is therefore limited to loaded events.

Complete-cycle retrieval is `Implementation-required` if product requires guaranteed full-cycle detail.

Within a Cycle card, events are sorted ascending by timestamp for chronology.

---

## 14. Event Card Contract

Card may display severity/category treatment, title, resolved badge from `resolved_at`, timestamp, supported FSM badge, message, reason, metadata disclosure, event detail, and acknowledge/reopen when permitted.

Card body is not a mutation target. Explicit controls are required.

Current FSM display mappings include `WaterRefilling`, `WaterDraining`, `MimoDosing`, `ActiveMixing`, `Monitoring`, and `EmergencyStop`. These are display mappings, not a generic FSM API.

---

## 15. Event Detail Contract

`EventDetailDrawer` is the event inspection surface.

Opening detail selects the target loaded event and performs no mutation. Content may include title, timestamp, message, reason, rendered metadata, and raw JSON.

Current raw JSON serializes the frontend event object. Because frontend type omits backend `source` and `primary_reason_code`, it is not guaranteed to represent every backend row field.

Closing returns to the same Journal context without mutation.

Current detail uses loaded event data; remote-detail loading/error is not applicable unless a remote detail query is introduced.

---

## 16. Metadata Inspection Contract

Metadata is collapsed by default when present. `Xem thông số kỹ thuật` expands it. `JSON thô` is technical inspection.

Metadata inspection never executes a command, changes resolution state, or creates a new capability.

---

## 17. Acknowledge / Reopen Contract

Backend function is `resolve_event`; UI actions are `Đánh dấu đã xử lý` and `Mở lại`.

Meaning is **event-record resolution state**, not physical problem resolution.

Request uses `PUT /api/devices/{deviceId}/events/{eventId}/acknowledge` with body `{ resolved: true | false }` and requires `events:write`.

Required lifecycle: user intent -> context/permission guard -> pending -> backend result -> refetch -> reconcile authoritative `resolved_at`.

Current implementation sends the PUT and refetches on `res.ok`, but has no explicit per-event pending/unknown state. This is `Implementation-required`.

`resolve_system_event` does not expose rows-affected/not-found information. Deterministic not-found semantics are `Implementation-required` if required.

---

## 18. Confirmation / Reversibility Contract

| Action | Confirmation | Reversibility |
|---|---|---|
| Open detail | No | N/A |
| Expand metadata | No | N/A |
| Search/filter | No | N/A |
| Export | No | N/A |
| Mark resolved | No generic confirmation | `resolved=false` |
| Reopen | No generic confirmation | `resolved=true` |

No delete-event action is specified because no source-backed delete endpoint was identified.

---

## 19. Export Contract

Current CSV is generated locally from `systemEvents`, before presentation grouping.

Export scope is currently loaded event records only. It is not guaranteed to include all retained backend history.

Current columns are `ID`, `Thời Gian`, `Mã Thiết Bị`, `Cấp Độ`, `Danh Mục`, `Tiêu Đề`, `Nội Dung Message`.

Current filename is `nhat-ky-{deviceId}.csv`.

The current export does not automatically add `source`, `primary_reason_code`, `reason`, `metadata`, or `resolved_at`.

---

## 20. Health Summary Contract

Current endpoint is `GET /api/devices/{deviceId}/health-summary`.

It returns a fixed one-hour event-derived summary: `window_seconds`, `ec_dosing_count`, `ph_dosing_count`, `water_operation_count`, `warning_count`, `critical_count`, and `latest_ph_dosing_at`.

These are event-derived counts, not live telemetry measurements.

Current frontend uses nullish `0` fallback. Backend returns HTTP `500` on database failure, so error can masquerade as zero.

`Implementation-required`: distinguish real zero from unavailable/error.

---

## 21. Analytics Contract

### 21.1 Device health

`GET /api/devices/{deviceId}/analytics/health` returns `free_heap_bytes`, `wifi_rssi_dbm`, `uptime_seconds`, `backend_process_cpu_percent`, and `last_updated_at`.

Backend currently sets `backend_process_cpu_percent` to `None`. Null renders unavailable, not `0%`.

### 21.2 Session health history

`useHealthHistory` retains up to 40 samples in frontend memory. At the current 15-second polling interval this is approximately ten minutes when samples arrive normally.

It resets on reload, is not backend-persisted, and is not canonical long-term telemetry history.

### 21.3 Weekly report

Current preference mutation is `PUT /api/admin/me/preferences` with `weekly_report` in user preferences.

This is an account preference, not a Journal event mutation. Success does not prove report generation, email delivery, receipt, or content.

### 21.4 Grafana

When configured, `grafana_url` is embedded and linked externally. Grafana is external observability, not the canonical `system_events` source.

Standalone `SystemLog` currently contains a hard-coded `http://localhost:3000` fallback. This must not be treated as production configuration.

`Implementation-required`: use configured Grafana source or explicit unavailable state.

---

## 22. Page State Model

Relevant states: `Initial`, `Missing Context`, `Loading`, `Ready`, `Empty`, `Filtered Empty`, `Partial`, `Stale`, `Offline / Unavailable`, `Network Error`, `Backend Error`, `Unknown`, `Recovery`.

Rules:

- `Empty` means successful query with no matching records.
- `Filtered Empty` means loaded records exist but current filter/search matches none.
- Backend/network error is not empty.
- Unknown means final mutation state cannot be established.
- Missing context means no device-scoped event query.

---

## 23. Component State Contract

### Event timeline

`idle | loading | ready | empty | filtered-empty | error | stale`

### Older pagination

`available | loading-older | end | error`

Existing records remain visible during pagination.

### Event card

`normal | resolved | metadata-expanded | acknowledge-pending | reopen-pending | mutation-error | mutation-unknown`.

Pending/unknown states are currently incomplete and are `Implementation-required`.

### Event detail

`closed | ready` in current local-data implementation.

### Health summary

`loading | ready | unavailable | error`.

### Analytics health

`loading | ready | partial | unavailable | error`.

### Weekly report

`ready | pending | success | rejected/error | unknown`.

---

## 24. Interaction / Outcome Contract

Every interactive element follows the canonical form: User does X -> UI does Y -> State becomes Z -> Data/action result is A -> Available next actions are B.

- Tab: switch active tab; preserve device; no mutation.
- Search: update local query; filter loaded events; no mutation.
- Category filter: update query/filter; fetch device-scoped data; reconcile timeline.
- View mode: regroup loaded data; no mutation.
- Older events: pagination pending; request timestamp cursor; append older records.
- Event detail: select event; open detail; inspect only.
- Metadata: expand disclosure; no mutation.
- Acknowledge/reopen: pending -> backend result -> refetch -> authoritative `resolved_at`.
- CSV: validate loaded data -> generate file -> success/error.
- Analytics refresh: refetch health -> updated/error state.
- Grafana: open configured external source -> no Journal mutation.

---

## 25. Action / Command & Mutation Lifecycle Contract

Journal's current consequential event mutation is acknowledge/reopen.

User intent -> context validation -> permission guard -> request dispatch -> pending -> backend acceptance/rejection -> persistence result -> refetch -> authoritative reconciliation -> final event-record state.

No Journal mutation changes physical device state.

### 25.1 Safety & Control Authority Boundary

Journal is not a runtime control surface. It must not issue actuator, pump, emergency-stop, device-control, or Automation commands from an event record.

When an event indicates an operational problem:

- Journal may expose the historical record and available technical context.
- Operations owns current runtime control and safety actions.
- Automation owns automatic orchestration.
- Settings owns persistent configuration.
- Safety/control authority remains with the owning runtime/control contract.

An event describing `EmergencyStop`, a pump action, dosing, water operation, or another runtime state does not create a Journal command.

Journal must not use a generic confirmation dialog to imitate an emergency action. If a user needs an immediate safety action, hand off to the existing Operations/safety surface and its canonical safety contract.

---

## 26. Permission Contract

Read access is governed by authenticated API/device authorization. Known `deviceId` does not itself grant access.

Acknowledge/reopen requires `events:write`.

Without write permission, inspection remains available when read-authorized; acknowledge/reopen is unavailable/disabled. Backend remains authoritative.

---

## 27. Navigation Contract

Verified routes:

- `/journal`
- `/logs` -> `/journal`
- `/analytics` -> `/journal`

Detail close returns to the same Journal context while mounted.

Navigation does not imply acknowledge/reopen success. Returning to Journal must reconcile authoritative state when needed.

Device-scoped navigation preserves selected `deviceId`; no implicit device substitution.

---

## 28. Cross-Page Handoff Contract

Journal origin -> deviceId / inspection intent -> destination -> destination restores context -> destination-owned task -> return -> reconciliation where required.

Operations owns runtime control. Settings owns persistent configuration. Automation owns workflow authoring/execution. Dashboard owns current monitoring/triage. Cultivation owns cultivation lifecycle. Journal owns historical trace.

Navigation must not silently change device context or imply mutation success.

---

## 29. Ownership & Responsibility Matrix

| Capability/data | Data owner | UI owner | Mutation owner | Journal role |
|---|---|---|---|---|
| Retained event records | `system_events` backend | Journal | Event producers | Inspect |
| Event resolution state | `system_events` backend | Journal | Event API | Mutate record state |
| Runtime actuator state | Device/runtime | Operations | Operations | Historical inspection |
| Persistent configuration | Config backend | Settings | Settings | Historical inspection |
| Automation workflow | Automation backend | Automation | Automation | Historical inspection |
| Cultivation lifecycle | Cultivation backend | Cultivation | Cultivation | Historical inspection |
| Device context | Shared device store | Shell/pages | Context owner | Consume/preserve |
| Hardware health | Device metrics/backend | Journal Analytics | Runtime/device source | Inspect |
| Grafana | External source | Journal Analytics | External system | Link/embed |
| Weekly report preference | User preference backend | Journal Analytics | Account preference API | Present preference |

Reading shared data does not grant mutation ownership.

---

## 30. Concurrency / Conflict Contract

Event creation and event resolution can occur concurrently.

### Acknowledge/reopen race

User A may resolve while User B reopens. Journal does not invent merge or priority behavior. Backend is authoritative; refetch reconciles displayed state.

### Concurrent event creation

New records may appear after the current query. Loaded list is not a frozen complete history.

### Context isolation

Pending mutation is scoped to `deviceId + eventId`. A Device A response must never update Device B.

### Revision

No event revision/ETag contract was identified. Do not claim optimistic locking, conflict prevention, merge, or last-write-wins safety.

---

## 31. Error / Unknown / Recovery Contract

### Event query error

Show explicit failure and Retry. Do not show true empty.

Current `SystemLog.tsx` returns `[]` on HTTP error. This is a blocking implementation gap.

### Pagination error

Keep loaded records. Retry only the older-page request.

### Acknowledge error

Do not change displayed resolution state optimistically.

### Acknowledge unknown

Do not infer failure from missing response. Refetch/reconcile.

### Sensor empty

Current wording claims system stability from zero sensor events. This is invalid evidence. Required neutral semantics: `Không có sự kiện cảm biến trong dữ liệu đang tải.`

### Missing context

No device-scoped query. Explain that a device must be selected.

---

## 32. Stale / Freshness Contract

`timestamp` is event time. `last_seen` is device contact. `last_updated_at` is health-snapshot time.

None is a universal telemetry-freshness guarantee. No freshness threshold is invented.

If old events remain after refresh failure, label them as last-known/stale rather than current.

---

## 33. Forms / Validation

Journal has no general persistent form.

Search is free text/local filtering. Filters use source-backed values. Acknowledge/reopen validates selected device/event context and blocks duplicate pending requests; backend validates authorization/persistence. Export uses loaded records.

---

## 34. History & Audit Contract

Journal owns presentation of retained `system_events`, not creation of every event.

Inspected producer paths include system logs, water cycles, dosing cycles, FSM transitions, device status, automation/script execution, webhook automation alerts, configuration changes, and supervisor/watchdog/AI-supervisor paths.

The generic event record has no `actor_id`. Do not invent actor identity.

Journal must not claim every event has actor, source, reason, full parameters, physical result, or complete causal chain.

---

## 35. Observability Contract

Journal should distinguish event recorded, event resolved/reopened, query loading/error, mutation pending/accepted/rejected/unknown, and health/analytics availability.

Event existence, `success`, `resolved_at`, HTTP `200`, MQTT attempt, or absence of later error is not physical `Applied` evidence.

---

## 36. Desktop & Mobile Contract

### Desktop

Timeline is primary. Event detail is a secondary side pane when space permits. Tabs, summary, search, view mode, filters, and export remain reachable.

### Mobile

Timeline is linear. Event detail may use full-screen/sheet presentation. No hover dependency. Raw JSON must scroll. Acknowledge/reopen remains distinct from detail opening.

Required visual states: loading, true empty, filtered empty, query error, pagination error, mutation pending/error/unknown, and analytics unavailable/partial.

---

## 37. Accessibility Contract

- Search has accessible label.
- Clear-search control has accessible name.
- Filter selected state is programmatically exposed.
- View switch exposes current state.
- Detail close has accessible name.
- Metadata disclosure exposes expanded state.
- Acknowledge/reopen exposes action meaning.
- Loading/error/success/pending states are not color-only.
- Severity is not conveyed only by color.
- Raw JSON remains readable and scrollable.
- Mobile controls meet adequate touch-target sizing.

---

## 38. Deep Links

Verified routes are `/journal`, `/logs -> /journal`, and `/analytics -> /journal`.

No verified event/cycle URL query contract exists. New deep-link forms are `Implementation-required` if desired.

Device context must come from shared selected-device state rather than implicit substitution.

---

## 39. Implementation Mapping

### Source-backed

- Journal route and two tabs.
- Device-scoped event query.
- Category filters.
- 200-event pagination with `before_timestamp`.
- Local search and unresolved filter.
- Important/All Technical modes.
- Cycle grouping from loaded metadata.
- Event detail/metadata inspection.
- Acknowledge/reopen endpoint and `events:write` backend gate.
- Loaded-event CSV export.
- One-hour event-derived health summary.
- Device health endpoint.
- Session-local health history.
- Weekly-report preference.
- Configured Grafana embed/link.
- Legacy redirects.

### Partially source-backed

- Backend `source`/`primary_reason_code` exist; frontend type omits them.
- Cycle endpoint exists; current cycle card does not call it.
- Acknowledge/reopen has refetch but lacks explicit pending/unknown UI.
- Health-summary backend errors exist; UI can render zero fallback.
- Shared `deviceId` exists; Journal has no page-local selector.
- Analytics metrics can be null.
- Weekly-report preference uses optimistic local state.

### Implementation-required

1. Propagate event query errors instead of returning `[]`.
2. Add explicit event loading/error/unknown semantics.
3. Add acknowledge/reopen pending state and duplicate guard.
4. Preserve/reconcile unknown acknowledge outcomes.
5. Distinguish health-summary error/unavailable from real zero.
6. Replace sensor-empty health claim with neutral wording.
7. Isolate Device A async results from Device B UI.
8. Align frontend event type with backend source/reason fields if displayed.
9. Integrate cycle endpoint if complete-cycle inspection is required.
10. Remove hard-coded localhost Grafana fallback.
11. Add explicit weekly-report unknown state if required.
12. Add page selector only if shell does not provide one.

### Deferred

- Backend-wide full-text search.
- Arbitrary date-range event explorer.
- Server-side full-history CSV.
- Event delete.
- Generic actor model.
- Universal causal-chain visualization.
- New Journal telemetry-history backend.
- New anomaly/root-cause engine.
- New event taxonomy without source support.

---

## 40. Source-to-UI Traceability

| UI behavior | Frontend | Backend/source | Status |
|---|---|---|---|
| Journal route | `App.tsx` | `/journal` | Source-backed |
| Selected device | `useDeviceStore.deviceId` | Device-scoped API | Source-backed |
| Event timeline | `SystemLog.tsx` | `GET /events` | Source-backed |
| Pagination | `SystemLog.tsx` | `before_timestamp` | Source-backed |
| Category filter | `SystemLog.tsx` | `category` | Source-backed |
| Unresolved | `SystemLog.tsx` | `resolved_at` | Source-backed, local |
| Search | `eventGrouping.ts` | Loaded data | Source-backed, local |
| Important grouping | `eventGrouping.ts` | Loaded data | Source-backed |
| Cycle grouping | `eventGrouping.ts` | Metadata cycle ID | Source-backed, loaded scope |
| Full cycle | `CycleEventCard` | `/events/cycle/{cycle_id}` | Partially source-backed |
| Event detail | `EventDetailDrawer.tsx` | Loaded event | Source-backed |
| Metadata | `MetadataRenderer` | JSONB | Source-backed |
| Acknowledge | `SystemLog.tsx` | `PUT /events/{id}/acknowledge` | Source-backed, lifecycle incomplete |
| Health summary | `HealthSummaryBar` | `/health-summary` | Source-backed, error UI incomplete |
| CSV | `handleExportCSV` | Loaded data | Source-backed, limited |
| Device health | `Analytics.tsx` | `/analytics/health` | Source-backed |
| Session history | `useHealthHistory` | No backend history | Source-backed, local |
| Weekly report | `Analytics.tsx` | `/admin/me/preferences` | Source-backed |
| Grafana | `Analytics.tsx` | `grafana_url` | Source-backed configured; fallback gap |
| Source/reason code | Frontend type omits fields | `SystemEventRecord` | Partially source-backed |
| Query error | `queryFn` returns `[]` | HTTP `500` | Implementation-required |
| Mutation pending/unknown | No explicit state | Existing endpoint | Implementation-required |

---

## 41. Acceptance Criteria

### AC-01 — Device context

**Given** device A is selected  
**When** Journal loads  
**Then** all device-scoped queries target A  
**And** no other device's event is rendered as current.

### AC-02 — Missing context

**Given** no `deviceId` exists  
**When** Journal renders  
**Then** no device-scoped event query is issued  
**And** missing context is shown instead of empty history.

### AC-03 — Event load

**Given** the event endpoint returns records  
**When** Journal loads  
**Then** records appear newest first  
**And** each record belongs to the selected device.

### AC-04 — Query error

**Given** the event endpoint returns HTTP `500`  
**When** Journal loads  
**Then** an explicit error state is shown  
**And** the page does not show true empty history.

### AC-05 — Pagination

**Given** 200 events are loaded and older records exist  
**When** the user loads older events  
**Then** the oldest loaded timestamp is used as `before_timestamp`  
**And** older records append without replacing current records.

### AC-06 — Category/filter

**Given** multiple event categories exist  
**When** a category or unresolved filter is selected  
**Then** matching selected-device records are shown  
**And** event records are not mutated.

### AC-07 — Search scope

**Given** events are loaded  
**When** the user searches title/message/category/reason  
**Then** matching loaded events are shown  
**And** unloaded backend history is not claimed to be searched.

### AC-08 — Filtered empty

**Given** loaded events exist  
**When** search/filter matches none  
**Then** filtered-empty state is shown  
**And** resetting the filter/search restores applicable records.

### AC-09 — Important grouping

**Given** consecutive technical `info` events share category/title  
**When** Important mode is active  
**Then** they may be merged  
**And** warning/critical/success are not merged by this rule.

### AC-10 — All Technical

**Given** technical events are loaded  
**When** All Technical is selected  
**Then** technical events remain individually represented after cycle grouping.

### AC-11 — Cycle completeness

**Given** only some cycle events are loaded  
**When** a cycle card is shown  
**Then** the UI does not claim complete backend cycle history.

### AC-12 — Detail

**Given** an event is visible  
**When** detail opens  
**Then** correct event data is shown  
**And** opening detail performs no mutation.

### AC-13 — Permission

**Given** the user lacks `events:write`  
**When** acknowledge/reopen is attempted  
**Then** backend rejects it or UI blocks unsupported execution  
**And** no optimistic resolution state remains.

### AC-14 — Acknowledge success

**Given** the user has `events:write` and the event exists  
**When** acknowledge succeeds  
**Then** Journal refetches  
**And** displayed `resolved_at` reflects authoritative data.

### AC-15 — Acknowledge failure

**Given** acknowledge/reopen fails  
**When** failure returns  
**Then** prior authoritative state remains  
**And** actionable error is shown.

### AC-16 — Unknown mutation

**Given** acknowledge/reopen was sent but transport outcome is unknown  
**When** final state cannot be established  
**Then** Journal does not optimistically change the event  
**And** reconciliation is available.

### AC-17 — No physical-resolution claim

**Given** an event record is resolved  
**When** Journal displays it  
**Then** it describes the event record as resolved  
**And** does not claim physical fault clearance.

### AC-18 — CSV scope

**Given** events are loaded  
**When** CSV export succeeds  
**Then** exported records correspond to loaded event records  
**And** full backend history is not claimed.

### AC-19 — Health summary error

**Given** health-summary returns an error  
**When** summary renders  
**Then** error/unavailable is distinguishable from real zero counts.

### AC-20 — Sensor empty

**Given** sensor filter returns zero events  
**When** empty state renders  
**Then** it says no matching sensor events are present in loaded data  
**And** does not claim sensor health.

### AC-21 — Nullable metric

**Given** CPU metric is null  
**When** Analytics renders  
**Then** CPU is shown as unavailable  
**And** not `0%`.

### AC-22 — Session history

**Given** health samples exist in the current session  
**When** the sparkline renders  
**Then** it represents local retained samples  
**And** is not labeled persistent backend history.

### AC-23 — Weekly preference failure

**Given** weekly-report update is rejected  
**When** failure returns  
**Then** rejected local state is not left authoritative  
**And** prior state is restored/refetched.

### AC-24 — Device switch during mutation

**Given** event mutation for device A is pending  
**When** selected device changes to B  
**Then** A's result cannot update B's visible event state.

### AC-25 — Cross-page handoff

**Given** device A is selected  
**When** Journal opens a device-scoped destination  
**Then** destination resolves A context  
**And** navigation does not imply mutation success.

### AC-26 — Grafana source

**Given** no configured Grafana URL exists  
**When** Analytics renders  
**Then** an unavailable/configuration state is shown  
**And** no hard-coded localhost URL is treated as production configuration.

### AC-27 — Ownership

**Given** the user needs runtime control, persistent configuration, or Automation authoring  
**When** they are in Journal  
**Then** they are directed to Operations, Settings, or Automation  
**And** Journal does not duplicate those mutation surfaces.

---

## 42. Wireframe Contract

Required visual states:

1. Desktop overview.
2. Mobile overview.
3. Event detail.
4. Timeline loading.
5. True empty.
6. Filtered empty.
7. Query error.
8. Pagination loading/error.
9. Acknowledge/reopen pending/error/unknown.
10. Analytics partial/unavailable.

Wireframes must not introduce arbitrary search APIs, full-history export guarantees, event delete, new diagnostics, physical confirmation, unsupported filters, or new telemetry sources.

### Desktop contract

Timeline is primary. Event detail is a secondary side pane when space permits. Summary, search, view mode, filters, and export remain reachable.

### Mobile contract

Timeline is linear. Event detail may use full-screen/sheet presentation. No hover dependency. Raw JSON must scroll. Acknowledge/reopen remains distinct from detail opening.

---

## 43. Anti-Pattern Review

Never:

- treat event history as current state;
- treat event `success` as physical success;
- treat `resolved_at` as physical recovery;
- claim no event means healthy;
- claim loaded history is complete backend history;
- claim local search is backend-wide search;
- claim loaded CSV is full-history export;
- convert query errors into empty state;
- convert unavailable metrics into zero;
- use `last_seen` as event freshness;
- infer capabilities from arbitrary metadata;
- allow Device A async results to update Device B;
- assume last-write-wins or merge behavior;
- create Journal-owned actuator/configuration/Automation controls;
- invent actor identity;
- use hard-coded localhost Grafana in production;
- present suspected/anomaly reason codes as confirmed diagnoses;
- add delete without a source-backed endpoint;
- claim complete cycle history from partial loaded-event grouping.

---

## 44. Definition of Done

Journal is implementation-complete only when:

- [ ] `/journal` owns merged event/analytics surface.
- [ ] Legacy `/logs` and `/analytics` resolve to Journal.
- [ ] Selected device context is explicit and cannot leak across devices.
- [ ] Event loading, empty, filtered-empty, error, and recovery are distinct.
- [ ] Pagination uses timestamp cursor correctly.
- [ ] Search scope is honest.
- [ ] Important/Technical semantics are preserved.
- [ ] Cycle grouping does not overclaim completeness.
- [ ] Event detail open/close is explicit.
- [ ] Metadata/raw JSON remain inspection-only.
- [ ] Acknowledge/reopen uses `events:write` and explicit lifecycle.
- [ ] Final resolution state is reconciled.
- [ ] Unknown mutation outcomes remain unknown until reconciled.
- [ ] CSV scope is explicit.
- [ ] Health-summary errors do not masquerade as zero.
- [ ] Sensor-empty state does not claim health.
- [ ] Nullable health metrics remain unavailable.
- [ ] Session trends are labeled local/session scope.
- [ ] Weekly preference does not imply delivery success.
- [ ] Grafana uses configured source or unavailable state.
- [ ] Backend authorization remains authoritative.
- [ ] Cross-page context is preserved.
- [ ] Journal does not duplicate other domains' ownership.
- [ ] Accessibility requirements are implemented.
- [ ] AC-01 through AC-27 pass.

---

## 45. Current Implementation Audit

### 45.1 Checklist status

This document is checklist-complete at documentation-contract level:

- Page identity/responsibility: **Covered**
- Source of truth: **Covered**
- Selected context: **Covered with implementation boundary**
- Information architecture: **Covered**
- Component inventory: **Covered**
- Interaction/outcome: **Covered**
- Component state: **Covered with explicit gaps**
- Data contract: **Covered**
- State model: **Covered**
- Mutation lifecycle: **Covered**
- Confirmation/reversibility: **Covered**
- Safety/control authority: **Not applicable as Journal control owner; physical-control boundary explicit**
- Permission: **Covered**
- Navigation/cross-page handoff: **Covered**
- Desktop/mobile: **Covered**
- Forms/validation: **Covered / limited because no general Journal form**
- History/audit: **Covered**
- Empty/loading/error/unknown: **Covered with implementation gaps**
- Accessibility: **Covered**
- Observability: **Covered**
- Wireframe: **Covered**
- Acceptance: **Covered**
- Implementation mapping: **Covered**
- Source-to-UI traceability: **Covered**
- Ownership matrix: **Covered**
- Concurrency/conflict: **Covered with source limitation**
- Anti-pattern: **Covered**
- Definition of Done: **Covered**

### 45.2 Blocking implementation gaps

1. Event query HTTP errors are currently collapsed into `[]`.
2. Acknowledge/reopen lacks explicit per-event pending state.
3. Acknowledge/reopen lacks explicit unknown-outcome handling.
4. Health-summary errors can render as zero counts.
5. Sensor-empty wording overclaims system stability.
6. Frontend event type omits backend `source` and `primary_reason_code`.
7. Cycle detail does not fetch the complete cycle endpoint.
8. Standalone SystemLog contains hard-coded `http://localhost:3000` Grafana fallback.
9. Journal has no page-local device selector; shell integration must be verified.
10. Weekly-report preference has optimistic state without explicit unknown semantics.

### 45.3 Resolution gates

Conformance requires:

- query errors remain distinct from empty;
- acknowledge/reopen is scoped to `(deviceId, eventId)` and exposes pending/result/unknown;
- final resolution state is authoritative/refetched;
- summary error is distinct from zero;
- event absence never becomes health proof;
- backend event fields are shown only after frontend type/semantic alignment;
- cycle completeness claims match actual query path;
- Grafana uses configured source or unavailable state;
- async results cannot cross device context;
- no unsupported capability is added merely to satisfy this spec.

### 45.4 Normative implementation sequence

1. **Context foundation:** selected `deviceId`, switching isolation, mutation scoping.
2. **Event query foundation:** loading/error/empty/partial semantics.
3. **Timeline foundation:** pagination, filters, search, grouping, cycles.
4. **Detail foundation:** detail and metadata inspection.
5. **Mutation foundation:** acknowledge/reopen lifecycle and reconciliation.
6. **Health foundation:** summary error/null semantics.
7. **Analytics foundation:** session history, preference lifecycle, Grafana source.
8. **Cross-page acceptance:** context and ownership.
9. **Acceptance audit:** AC-01 through AC-27.

This sequence resolves existing lifecycle/truthfulness gaps; it does not create unsupported backend capabilities.

---

## 46. Locked Journal Architecture

1. Journal is the canonical historical event trace.
2. Historical records do not become current physical state.
3. No event does not prove health.
4. Event resolution is record state, not physical recovery.
5. Search and CSV are loaded-data scoped.
6. Cycle grouping does not imply complete cycle retrieval.
7. Metadata is technical context, not unrestricted capability source.
8. `events:write` is required for acknowledge/reopen.
9. Backend event state is authoritative after mutation.
10. Unknown mutation outcomes remain unknown until reconciliation.
11. Health summary is event-derived, not live telemetry.
12. Session health trends are local presentation history.
13. Grafana is external observability, not event source of truth.
14. Operations owns runtime control.
15. Automation owns workflow orchestration.
16. Settings owns persistent configuration.
17. Cultivation owns cultivation lifecycle.
18. Journal does not become a second mutation owner for those domains.

---

## 47. Canonical Journal Formula

**Selected Device, Historical Event Source, Timeline, Filter/Search Scope, Event Detail, Resolution Record State, Honest Unknown/Error, Analytics Context, Cross-Page Handoff, Historical Ownership.**

Journal must always answer:

**Tôi đang xem lịch sử của thiết bị nào?**

**Sự kiện này được ghi nhận lúc nào?**

**Sự kiện thuộc loại/cấp độ nào?**

**Nguồn và technical context nào thực sự có sẵn?**

**Event record đã được đánh dấu xử lý hay chưa?**

**Nếu không có bằng chứng mạnh hơn, trạng thái nào vẫn là Unknown/Not confirmed?**

**Công việc tiếp theo thuộc Operations, Automation, Settings, Dashboard, hay Cultivation?**

---

## 48. PAGE-SPEC-CHECKLIST Compliance Contract

This section explicitly audits `docs/design-system/PAGE-SPEC-CHECKLIST.md` and does not create product capability.

### 48.1 Page Identity & Responsibility

- Page: Journal.
- Route: `/journal`.
- Purpose: historical event inspection and supported analytics for selected device.
- Roles: Admin, Operator, Viewer subject to backend authorization.
- Owned: event history presentation, loaded-data search/filter, cycle grouping, detail, event-record resolution, loaded-data export, supported analytics.
- Not owned: runtime control, Automation authoring, persistent configuration, cultivation lifecycle, pairing, roles, physical confirmation.
- Entry/exit: Section 1.

### 48.2 Source of Truth

Frontend page/components, hooks, store, router, backend alert/analytics APIs, PostgreSQL event model, migrations, and event producers are mapped in Sections 3 and 40.

### 48.3 Selected Context

`useDeviceStore.deviceId` is canonical. Device-scoped reads/mutations use it. Missing context and context-switch isolation are explicit. No richer station model is invented.

### 48.4 Component / Outcome / State

Component inventory: Section 6. Interaction/outcome: Section 24. Component state: Section 23. Detail/open behavior: Section 15.

### 48.5 Data Contract

Event fields, severity, categories, source, reason codes, metadata, health metrics, and semantic boundaries are defined in Sections 7, 8, 20, and 21.

### 48.6 Action / Mutation / Reversibility

Acknowledge/reopen is the current consequential Journal mutation. Sections 17, 18, and 25 define permission, lifecycle, reconciliation, confirmation, and reversibility.

### 48.7 Permission

Backend authorization is authoritative; `events:write` is required for event resolution mutation.

### 48.8 Navigation / Cross-Page Handoff

Sections 27 and 28 define entry, exit, context preservation, pending mutation behavior, and ownership handoff.

### 48.9 Desktop / Mobile

Section 36 defines desktop timeline/detail and mobile linear/detail behavior.

### 48.10 Forms / Validation

Section 33 defines the limited forms/validation applicable to Journal.

### 48.11 History / Audit

Section 34 defines event producers, event ownership, actor/source limitations, and Journal's historical boundary.

### 48.12 Error / Unknown

Sections 22 and 31 distinguish missing context, empty, filtered-empty, stale, backend/network errors, and unknown mutation outcomes.

### 48.13 Observability

Section 35 defines user-visible event/mutation/health status without inventing physical confirmation.

### 48.14 Wireframe

Section 42 defines required desktop/mobile and critical states without adding unsupported capability.

### 48.15 Acceptance

AC-01 through AC-27 are the implementation acceptance gate.

### 48.16 Source-to-UI Traceability

Section 40 maps important UI behavior to frontend and backend source.

### 48.17 Ownership Matrix

Section 29 separates data, UI, mutation, and Journal read ownership.

### 48.18 Concurrency / Conflict

Section 30 defines concurrent event mutation without inventing revision/merge semantics.

### 48.19 Anti-Pattern

Section 43 prohibits fabricated health, completeness, physical confirmation, search/export scope, and ownership.

### 48.20 Definition of Done

Section 44 defines implementation completion; Section 45 separates documentation completeness from current implementation gaps.

---

## 49. Final Review Question

> **Can another engineer implement Journal, its event interactions, state transitions, resolution lifecycle, selected-device behavior, analytics boundaries, cross-page ownership, and error semantics from this specification without inventing missing product behavior?**

If the answer is no, the Journal specification is not complete.
