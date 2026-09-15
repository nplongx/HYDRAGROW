# HYDRAGROW Cultivation — Functional Specification

> Status: `Source-backed` baseline with explicit implementation gaps.
>
> Canonical quality gate: `/hydragrow/docs/design-system/PAGE-SPEC-CHECKLIST.md`.

## 0. Purpose

Cultivation is the merged `/cultivation` workspace for crop-season lifecycle, reusable recipe templates, recipe application to owned device(s), and cultivation-specific dosing history.

It is not the owner of runtime actuator control (`Operations`), workflow orchestration (`Automation`), persistent device configuration (`Settings`), or the full historical trace (`Journal`).

Current tabs:

- `Mùa vụ`: active season, season creation, progress, season history, photos, completion summary.
- `Công thức`: recipe template library/editor, stages, apply, bulk apply.
- `Lịch sử châm`: dosing range, aggregates, anomalies, recent records, CSV export.

---

## 1. Page Identity & Responsibility

### Identity

- Name: `Cultivation` / `Canh tác`.
- Route: `/cultivation`.
- Default tab: `Mùa vụ`.
- Legacy `/seasons`, `/crop-seasons`, `/recipes`, `/dosing-history` redirect to `/cultivation`.

### User problem

The operator needs to manage a crop season and its recipe lifecycle without confusing cultivation configuration with runtime control or historical monitoring.

### Roles

Repository-backed roles include `admin`, `operator`, and `viewer`. Recipe writes use `recipe:write`; recipe application additionally requires a linked user and device ownership. Do not invent UI roles.

### Cultivation owns

- season metadata and lifecycle;
- recipe template lifecycle;
- recipe application to owned device(s);
- cultivation-specific dosing history/export;
- season photo journal;
- progress derived from active season + active recipe.

### Other owners

- `Operations`: direct runtime pump/valve control.
- `Automation`: event/workflow orchestration.
- `Settings`: persistent unified device configuration and calibration.
- `Journal`: canonical full historical event trace.
- `Dashboard` / `Fleet`: fleet monitoring and triage.
- `Device Pairing`: device ownership/claim.

### Entry / exit

Entry: `/cultivation` or legacy cultivation routes.

Exit: primary navigation or another application route. Tab changes remain inside Cultivation.

---

## 2. Source of Truth

### Frontend

| Concern | Source |
|---|---|
| Page shell | `src/pages/Cultivation.tsx` |
| Seasons | `src/pages/CropSeasons.tsx` |
| Season state/API | `src/hooks/useCropSeason.ts` |
| Active season | `src/components/seasons/ActiveSeasonCard.tsx` |
| Create season | `src/components/seasons/CreateSeasonForm.tsx` |
| Season history | `src/components/seasons/SeasonHistoryList.tsx` |
| Season photos | `src/components/seasons/SeasonPhotoJournal.tsx` |
| Completion summary | `src/components/seasons/SeasonCompletionSummary.tsx` |
| Recipes | `src/pages/RecipeBuilder.tsx` |
| Active recipe | `src/hooks/useActiveRecipeStatus.ts` |
| Dosing history | `src/pages/DosingHistory.tsx` |
| Device context | `src/store/useDeviceStore.ts` |
| Device list | `src/hooks/useOwnedDevices.ts` |
| Models | `src/types/models.ts` |

### Backend

| Concern | Source |
|---|---|
| Season API | `src/api/crop_season.rs` |
| Recipe API | `src/api/recipe.rs` |
| Photo API | `src/api/crop_season_photo.rs` |
| Dosing analytics | `src/api/analytics.rs` |
| Device ownership | `src/api/device_pairing.rs` |
| Runtime control boundary | `src/api/control.rs` |
| MQTT signing | `src/api/mqtt_utils.rs` |
| Config ownership | `src/api/config.rs` |
| Cycle events | `src/api/alert.rs` |

### Status vocabulary

- `Source-backed`: verified in repository behavior.
- `Partially source-backed`: part of the contract exists; gaps are listed.
- `Implementation-required`: desired behavior not currently implemented.
- `Deferred`: intentionally not part of current implementation.

Known limitations:

- Cultivation has `deviceId`, not a richer station object.
- Historical season selection callback exists but is not wired by `CropSeasons`.
- Recipe status is reconstructed by backend and must not be treated as physical actuator confirmation.
- Photo flow depends on Cloudinary configuration.
- Some season/photo handlers shown do not have the same explicit scope checks as recipe handlers.
- Current season hook collapses several API failures into empty arrays.

---

## 3. Selected Context

The authoritative selected device is `useDeviceStore().deviceId`.

Device-scoped queries include `deviceId` in their query keys or API path. Recipe bulk apply is an explicit exception: the user selects additional owned device IDs.

When device context changes:

1. old device data must not remain presented as the new device;
2. device-scoped queries must reconcile using the new ID;
3. pending mutation results must not be attributed to the new device.

If `deviceId` or backend URL is missing, device-scoped queries are disabled. Missing context is not equivalent to an empty season/history.

`Implementation-required`: add a shared Cultivation-level missing-device state if child components do not make this condition sufficiently explicit.

---

## 4. Information Architecture

```text
/cultivation
  └─ TabShell: Canh tác
      ├─ Mùa vụ
      │   ├─ Active season OR Create season
      │   ├─ Season photo journal
      │   ├─ Season history
      │   └─ Completion summary
      ├─ Công thức
      │   ├─ Saved templates
      │   ├─ Recipe metadata
      │   ├─ Stage editor
      │   ├─ Timeline preview
      │   └─ Apply / bulk apply
      └─ Lịch sử châm
          ├─ Range selector
          ├─ Aggregate cards
          ├─ Anomaly banner
          └─ Recent records
```

Current implementation has no canonical season-detail route or recipe-detail route. Editing is inline.

Full history navigates to Journal when needed; persistent configuration navigates to Settings.

---

## 5. Component Inventory

### Shell

`TabShell`: title, subtitle, three tabs; no backend mutation and no device switching.

### Mùa vụ

`CreateSeasonForm`: recipe selection, recipe preview, season name, description, create submission.

`ActiveSeasonCard`: active metadata, progress, current stage, edit, end-season.

`SeasonPhotoJournal`: photo list, signed upload, Cloudinary upload, record, delete.

`SeasonHistoryList`: prior seasons and delete for non-active records.

`SeasonCompletionSummary`: transient post-completion summary.

### Công thức

`RecipeBuilder`: template list, editor, stage CRUD/reorder, timeline preview, create/update/delete, duplicate, apply, bulk apply.

### Lịch sử châm

`DosingHistory`: range selector, analytics cards, anomaly banner, recent dosing records, CSV export.

Every interactive element must have defined trigger, result, pending/error behavior, and permission where applicable.

---

## 6. Interaction Contract

### Create season

Trigger: submit form.

Local validation: non-empty name and selected recipe required.

Request: `POST /api/devices/{device_id}/seasons` with `name`, recipe `crop` as `plant_type`, description, and `recipe_id`.

Success: season and recipe-status queries invalidated; success feedback; form reset.

Failure: error feedback; no success state.

### Edit active season

Request: `PUT /api/devices/{device_id}/seasons/active`.

Required: non-empty name. Success closes editor and invalidates season queries.

Current cancel behavior discards local edits without dirty confirmation.

### End season

Trigger: `Kết thúc mùa vụ` -> browser confirmation.

Request: `PUT /api/devices/{device_id}/seasons/active/end`.

Success: season/recipe queries invalidated and completion summary shown.

The summary's grown days are locally derived; this is not a backend-measured production metric.

### Delete season

Trigger: delete historical non-active season -> confirmation.

Request: `DELETE /api/devices/{device_id}/seasons/{season_id}`.

`Implementation-required`: verify whether DB cascade actually deletes photos before the UI claims “season and all photos” are deleted.

### Add/delete photo

Add flow: sign request -> Cloudinary upload -> save photo metadata -> invalidate query.

Delete flow: confirmation -> `DELETE /seasons/{season_id}/photos/{photo_id}` -> invalidate query.

Any failure is surfaced as error feedback; failed upload/save is never shown as successful.

### Recipe editing

- Select template: copy template into local editor state.
- Add stage: append default stage.
- Move stage: local reorder.
- Delete stage: allowed only while at least one stage remains.
- Preset: changes selected stage fields locally.
- Duplicate: clears selected ID and changes local name; persistence occurs only on Save.

### Save recipe

Create: `POST /api/recipes`.

Update: `PUT /api/recipes/{recipe_id}`.

Backend requires non-empty `name`, `crop`, and `stages`.

Success invalidates recipe list. Failure leaves local editor state available and shows error.

### Delete recipe

Trigger: delete -> confirmation.

Request: `DELETE /api/recipes/{recipe_id}`.

Permission: `recipe:write`.

Success invalidates recipe list and resets selected editor if applicable.

### Apply recipe

Trigger: Play/apply -> confirmation.

Request: `POST /api/devices/{device_id}/recipe/apply`.

Backend requires `recipe:write`, linked user identity, and device ownership.

Backend signs and publishes an MQTT recipe payload, then persists `device_active_recipes`.

Success means backend operation/MQTT publish path succeeded. It is **not** proof that physical dosing/watering has completed.

### Bulk apply

Request: `POST /api/recipes/bulk-apply`.

Requires `recipe:write`, authenticated user, and ownership of all target devices.

Response contains `succeeded[]` and `failed[]`; partial success must not be displayed as full success.

`Implementation-required`: render failed device IDs/reasons instead of only a failure count.

### Dosing range/export

Range is one of `today`, `7d`, `30d`. Export is enabled only when records exist and creates a local CSV from returned records.

---

## 7. Data Contract

### CropSeason

```text
id: string
device_id: string
name: string
plant_type: string | null
description: string | null
start_time: string
end_time: string | null
status: 'active' | 'completed'
```

### RecipeTemplate

```text
id: string
name: string
crop: string
description?: string
stages: CropStage[]
created_at: string
```

### CropStage

```text
name: string
duration_sec: number
ec_target: number
ec_tolerance: number
ph_target: number
ph_tolerance: number
nutrient_a_ratio: number
nutrient_b_ratio: number
water_level_target: number
water_change_interval_days?: number
water_change_drain_cm?: number
auto_dilute_ec_trigger?: number
misting_on_duration_ms: number
misting_off_duration_ms: number
max_dose_per_cycle_ml?: number
light_hours?: number
```

UI edits duration in days and serializes to seconds.

### Active CropRecipe

```text
schema_version: number
recipe_id: string
season_id: string
device_id: string
revision: number
start_time_sec: number
current_stage_index: number
stages: CropStage[]
```

### Dosing history

Backend fields: `created_at`, `pump_a_ml`, `pump_b_ml`, `ph_up_ml`, `ph_down_ml`, `kalman`.

Displayed pump quantities are `ml`. Zero-valued pump fields are omitted from the recent-record list.

### Photo

```text
id: string
day_offset: number
cloudinary_url: string
```

### Integrity

```text
Measured != Estimated
Estimated != Commanded
Commanded != Confirmed
```

Recipe targets are configured values. Recipe-apply responses are backend/application results. Dosing records are historical reports. None may be relabeled as physical confirmation without source support.

`last_seen` is connection/contact information, not generic telemetry freshness.

---

## 8. State Model

### Page/data states

Relevant states: initial, loading, ready, empty, partial data, stale where applicable, offline/unavailable, fault, unknown, permission denied, network error, backend error, mutation pending, mutation success, mutation rejected, recovery.

The UI must not turn missing/unknown data into meaningful zero/Off state.

### Season

```text
No active season -> Create form
Create pending -> Active season
Active season -> Edit pending -> Active season
Active season -> End pending -> Completion summary -> No active season
```

Failure returns to the previous authoritative state.

### Recipe

```text
Draft -> Dirty -> Save pending -> Saved
Saved -> Dirty -> Save pending -> Saved
```

Draft is currently local React state and is lost on unmount/reload.

### Apply

```text
Idle -> Confirmation -> Pending -> Accepted -> Reconcile
Pending -> Rejected/Error -> Recovery
```

`Accepted` is not physical confirmation.

### Known current gap

`useCropSeason` returns empty arrays on non-OK season responses. This can collapse backend failure into an empty state.

`Implementation-required`: preserve query error state.

---

## 9. Action / Command Contract

### Data mutations

Season and recipe CRUD are persisted data mutations. Success is the corresponding authoritative HTTP response plus query reconciliation.

### Runtime-affecting recipe apply

| Property | Contract |
|---|---|
| Target | selected device or explicit bulk device IDs |
| Permission | `recipe:write` |
| Ownership | required |
| Backend | builds/signs/publishes recipe snapshot |
| Persisted state | `device_active_recipes` |
| Physical confirmation | not provided by current API response |
| Failure | explicit HTTP or per-device failure |
| Unknown | must not be rendered as physical completion |

Backend also exposes recipe clear, but Cultivation currently has no clear button. Status: `Source-backed backend`, `Implementation-required UI` if required by product.

Direct `/control` pump/valve commands remain owned by Operations.

---

## 10. Safety & Control Authority

```text
Cultivation = season + recipe lifecycle / recipe application
Operations  = runtime control
Automation  = workflow orchestration
Auto Mode   = control mode / ownership state
Settings    = persistent configuration
Journal     = canonical history
```

Cultivation must not introduce competing direct actuator controls.

Recipe apply may affect future runtime behavior, so its UI must describe application of recipe state, not physical completion.

Direct control interlocks currently include:

- `PH_UP` vs `PH_DOWN`;
- `WATER_PUMP_IN` vs `WATER_PUMP_OUT`.

E-STOP is not a Cultivation control and remains in the canonical runtime safety surface.

Offline/fault behavior: failed/unknown recipe application must not produce optimistic physical status.

---

## 11. Permission Contract

Backend authorization is authoritative.

| Capability | Repository requirement |
|---|---|
| Recipe create/update/delete | `recipe:write` |
| Recipe apply | `recipe:write` + linked user + ownership |
| Bulk apply | `recipe:write` + linked user + all-device ownership |
| Recipe clear | `recipe:write` + linked user + ownership |
| Device access | ownership backend |

Season/photo handlers shown in source do not consistently expose the same explicit scope checks. Do not claim a uniform role matrix until that is implemented.

---

## 12. Navigation Contract

- `/cultivation` is canonical.
- Legacy cultivation routes redirect to `/cultivation`.
- Tab changes do not navigate.
- Current season/recipe editing is inline.
- Journal is the destination for full historical event review.
- Settings is the destination for persistent configuration.
- Device context remains unchanged during tab/detail interaction.

No route guard currently guarantees mutation completion when navigating away.

No unsaved recipe draft persistence exists.

`Implementation-required`: add navigation/pending-mutation guards only if product requires them.

---

## 13. Desktop & Mobile Contract

### Desktop

- Season cards remain a primary vertical flow.
- Recipe library occupies a side column; editor/timeline occupies the main area.
- Dosing range/export controls share a compact toolbar.

### Mobile

- Tabs remain touch-accessible.
- Season metadata stacks.
- Recipe library/editor become one column.
- Stage controls wrap without requiring hover.
- Photo journal remains horizontally scrollable.
- Dosing toolbar may wrap.

`Implementation-required`: photo deletion currently uses hover visibility; make the action discoverable without hover on touch devices.

---

## 14. Forms & Validation

### Season creation

Required: name, recipe selection.

Optional: description.

Plant type is copied from selected recipe crop.

### Active season edit

Required: name.

Optional: plant type, description.

Current UI says plant type is “Khóa theo Recipe” but allows editing it. This is a contract contradiction.

`Implementation-required`: make it actually locked/derived or change the label.

### Recipe

Backend requires non-empty name, crop, and stages. Current frontend also disables Save for empty name/crop.

Current stage UI defines:

- duration minimum 1 day;
- light hours 0-24;
- nutrient A/B ratio minimum 0.1;
- other numeric fields with steps/defaults but no complete range contract.

`Implementation-required`: define domain ranges for all numeric stage fields if deterministic validation is required.

---

## 15. History & Audit

Cultivation-local:

- season history;
- dosing history;
- season photos.

Full event history remains Journal-owned.

Recipe apply and runtime control have backend audit/persistence paths, but the current recipe CRUD handlers do not establish complete explicit audit coverage for every recipe mutation.

`Implementation-required`: complete recipe/season mutation audit coverage if governance requires it.

---

## 16. Empty / Loading / Error / Unknown

### Seasons

- Loading: explicit loading state.
- Empty active season: create form.
- Empty history: explicit no-history state.
- Current gap: API failure can become empty due to hook behavior.

### Recipes

- Loading: template loading text.
- Empty: explicit no-template state.
- Mutation failure: toast.
- Bulk partial: failure count; detailed failure UI is missing.

### Dosing

- Loading spinner.
- Empty records state.
- API error StateView.
- Export disabled when empty.

### Photos

Upload/delete failures are surfaced, but query loading/error state is not explicit.

`Implementation-required`: add photo loading/error state.

---

## 17. Cross-Page Contract

| Page | Contract |
|---|---|
| Dashboard | may summarize active crop/stage; does not mutate season/recipe |
| Operations | owns direct runtime control |
| Automation | owns workflow orchestration |
| Settings | owns persistent device configuration/calibration |
| Journal | owns full historical trace |
| Fleet | provides multi-device monitoring/context |

Shared data does not imply shared mutation ownership.

---

## 18. Accessibility Contract

- Tabs expose selected state semantically.
- All form controls have labels.
- Icon-only actions have accessible names.
- Status is not conveyed by color alone.
- Errors and loading states have textual representation.
- Delete/apply confirmation identifies target.
- Keyboard users can reach tabs, form controls, stage actions, and recipe actions.
- Photo delete is not hover-only.
- Touch targets remain usable on mobile.

Current source has some `aria-label` coverage but does not establish a complete keyboard/focus contract.

---

## 18.1 Component State Contract

Every consequential Cultivation component must expose an explicit state model. A generic page-level loading state is not sufficient when an individual component can mutate, fail, or remain unknown independently.

### Active Season / Create Season

States: `idle`, `validating`, `pending`, `success`, `rejected`, `error`, `unknown`, `unavailable`.

For each state define visible presentation, allowed actions, transition trigger, recovery action, and semantic layer. `success` means the relevant persisted/application response succeeded; it does not mean a physical crop, pump, or dosing outcome was observed.

### Season History / Photo Journal

States: `loading`, `ready`, `empty`, `error`, `stale`, `mutation-pending`, `mutation-success`, `mutation-rejected`, `unknown`.

Upload/delete state remains attached to the affected season/photo rather than blocking unrelated content when the source supports independent operations.

### Recipe Library / Editor

States: `loading`, `ready`, `empty`, `draft`, `dirty`, `validating`, `saving`, `saved`, `rejected`, `error`, `unknown`.

The editor retains the unsaved draft after a failed save. `saved` requires the authoritative create/update result; local form completion is not persistence.

### Recipe Apply / Bulk Apply

States: `available`, `validating`, `confirming`, `pending`, `accepted`, `partial`, `rejected`, `error`, `timeout/unknown`, `reconciling`.

Bulk state is represented per target device when the backend returns per-device results. One successful target must not convert the whole operation to success.

### Dosing History / Export

States: `loading`, `ready`, `empty`, `error`, `exporting`, `export-success`, `export-error`.

No-data, missing-context, and query-error states remain distinguishable.

`enabled`, `saved`, `accepted`, `active`, `success`, and `online` must not be reused to imply persistence, runtime execution, observation, or physical confirmation unless the source explicitly establishes that meaning.

---

## 18.2 Mutation Lifecycle Contract

Every consequential mutation follows:

```text
User action
-> Client validation
-> Confirmation where required
-> Request dispatch
-> Pending UI
-> Backend acceptance/rejection
-> Persistence/application result where applicable
-> Query reconciliation
-> Observable result
```

### Season create/edit/end

- Identify the current device and target season.
- Validate required fields before dispatch.
- Pending state prevents duplicate submission.
- Backend response determines accepted/rejected/error state.
- Successful mutation reconciles season queries.
- Failed mutation preserves the previous authoritative state and recoverable input where applicable.
- Unknown outcome is never rendered as success.

### Recipe create/update/delete

- Identify the recipe/template and validate required metadata/stages.
- Delete requires explicit confirmation before dispatch.
- Save/delete enter pending state and prevent duplicate submission.
- Authoritative response controls persisted-state presentation.
- Failed save keeps the draft; failed delete keeps the recipe visible.
- Unknown delete outcome must not silently remove the authoritative item.

### Recipe apply / bulk apply

- Validate recipe identity, target device IDs, ownership, and `recipe:write` before dispatch.
- Use confirmation only when required by the owning product/safety contract.
- Distinguish backend acceptance from persisted active-recipe state.
- Preserve per-device success/failure/unknown results for bulk operations.
- Reconcile active-recipe queries after authoritative success.
- Never claim physical dosing completion because the current API provides no physical confirmation.

### Photo upload/delete

- Identify season/photo target before dispatch.
- Upload is successful only after signing/upload/record steps complete.
- Delete uses confirmation where classified as destructive.
- Pending/failure/unknown states remain visible at the affected photo.
- Reconcile after the authoritative record/delete result.

### Dosing export

- Validate range and device context.
- Enter `exporting` before generation.
- Success means the supported export artifact was generated; it does not imply new dosing.
- Export failure remains distinct from an empty dataset.

Duplicate submissions must be prevented or explicitly reconciled. Navigation away does not convert a pending mutation into success.

The semantic boundary remains:

```text
Commanded
!= Backend Accepted
!= Persisted
!= Observed
!= Physically Confirmed
```

---

## 18.3 Confirmation / Reversibility Contract

| Action | Confirmation | Reversibility / cancellation | Result semantics |
|---|---|---|---|
| Create season | No generic confirmation | Later edit/end; no implied undo | Persisted season result |
| Edit season | No generic confirmation | Later edit only; no implied undo | Persisted season result |
| End season | Confirm before execute | Not cancellable unless backend supports it | Ended-season result |
| Save recipe | No generic confirmation | Later edit | Persisted recipe result |
| Delete recipe | Confirm before execute | Irreversible unless recovery is source-backed | Authoritative deletion result |
| Apply recipe | Only if owning contract requires it | No implied undo | Application/active-recipe state |
| Bulk apply | Only if owning contract requires it | No implied rollback across targets | Per-device result set |
| Delete photo | Confirm before execute | Irreversible unless recovery is source-backed | Authoritative deletion result |
| Upload photo | No destructive confirmation | Cancel only if supported | Saved photo after record succeeds |
| Export dosing history | No confirmation | Cancel only if supported | Export artifact result |

Confirmation is confirmation of user intent, not backend acceptance or physical outcome. E-STOP is outside Cultivation and follows the canonical immediate-action safety contract.

---

## 18.4 Cross-Page Handoff Contract

Canonical sequence:

```text
Cultivation origin
-> device context + intent
-> destination
-> destination state
-> action / inspection
-> return
-> reconciliation
```

### Cultivation -> Journal

- Preserve `deviceId` and the originating history intent/range when supported.
- Journal owns the full historical trace.
- Missing/invalid context must not silently substitute another device.
- Return to Cultivation revalidates context and reconciles affected data.

### Cultivation -> Settings

- Preserve `deviceId`.
- Settings owns persistent configuration/calibration.
- Navigation success does not imply a Cultivation mutation succeeded.
- Reconcile on return from authoritative sources.

### Cultivation -> Operations

- Navigate only for a supported runtime action/inspection.
- Preserve the same `deviceId`.
- Operations becomes the runtime-control owner.
- Navigation itself does not imply actuator change.

### Cultivation -> Automation

- Preserve `deviceId` for a supported automation handoff.
- Automation owns workflow orchestration.
- Cultivation does not create a second scheduler or mutate Automation implicitly.

### Pending mutation / dirty draft

- Pending mutation remains pending/reconcilable; route transition is not success.
- Unsaved recipe draft is currently local-only. If navigation protection is absent, do not imply persistence.
- If a guard is later implemented, it must identify the unsaved target and provide explicit stay/leave semantics.

---

## 18.5 Concurrency / Conflict Contract

Potential writers include multiple users, recipe/season editors, recipe application, backend/runtime processes, and device context changes.

- Backend is authoritative for persisted recipe/season state.
- Do not assume last-write-wins, merge, or automatic conflict resolution without a verified contract.
- `recipe.revision` exists in source data, but no end-to-end optimistic concurrency mechanism is established; no client conflict algorithm is claimed.
- A stale editor must not silently overwrite another authoritative change unless the backend explicitly permits it.
- On rejected/stale save, preserve the user's draft and surface the authoritative failure.
- Refresh/reconciliation is scoped to the current `deviceId` and target entity.
- Pending results from device A must never be applied to device B after a context switch.
- Bulk apply reconciles each target independently when per-device outcomes exist.

`Implementation-required`: establish explicit revision/conflict handling if concurrent editing is a supported product requirement.

---

## 19. Observability Contract

User-visible:

- loading states;
- mutation success/failure;
- active recipe identity;
- season progress;
- dosing errors;
- bulk failure count.

Backend:

- recipe application persists active recipe state and publishes signed MQTT;
- runtime control and configuration changes have owning audit paths.

Unknown request outcomes must remain observable as unknown/error, not success.

Timestamps must retain their meaning: season timestamps, recipe `updated_at`, dosing `created_at`, etc.

---

## 20. Wireframe Contract

### Desktop

```text
Canh tác
[ Mùa vụ ] [ Công thức ] [ Lịch sử châm ]

Mùa vụ:
  Active Season / Create Season
  Progress + current stage
  Photo Journal
  Season History

Công thức:
  Saved Templates | Editor + Stages + Timeline

Lịch sử châm:
  [Hôm nay] [7 ngày] [30 ngày] [Xuất Excel]
  Aggregates + anomalies + recent records
```

### Critical visual states

- no active season;
- no recipes;
- no dosing records;
- recipe apply failure;
- bulk partial failure;
- photo upload unavailable;
- season mutation failure.

Wireframes must not introduce direct pump control, E-STOP, automation builder, or persistent global configuration.

---

## 21. Acceptance Criteria

### AC-01 Open page

Given `/cultivation` is opened
When the page mounts
Then `Mùa vụ` is selected
And selected device identity is unchanged.

### AC-02 Create season validation

Given no active season exists
When the user submits without a recipe
Then creation is rejected
And no successful creation state is shown.

### AC-03 Create season

Given valid name and recipe
When the user submits
Then the season API is called for the selected device
And successful response triggers query reconciliation.

### AC-04 End season

Given an active season exists
When the user confirms end-season
Then the end API succeeds before the completion summary is shown.

### AC-05 Recipe save

Given valid recipe metadata and at least one stage
When Save is submitted
Then the appropriate create/update endpoint is used
And the template list is reconciled after success.

### AC-06 Apply permission

Given the caller lacks `recipe:write`
When recipe apply is attempted
Then backend rejects it
And the UI does not claim the recipe was successfully applied.

### AC-07 Apply semantics

Given recipe apply returns success
When active recipe state is reconciled
Then the UI may show the recipe as active application state
But must not claim physical dosing completion without physical confirmation.

### AC-08 Bulk partial

Given multiple target devices
When bulk apply returns successes and failures
Then the UI distinguishes partial success
And failed devices/reasons are available.

### AC-09 Dosing error

Given the dosing API fails
When the query enters error state
Then an error state is shown
And the UI does not present empty history as a confirmed absence of dosing.

### AC-10 Device switch

Given device A is selected
When context changes to device B
Then device-scoped data is reconciled to B
And A's data is not displayed under B.

### AC-11 Photo failure

Given photo signing or upload fails
When the flow ends
Then the photo is not shown as successfully saved.

### AC-12 Recovery

Given a mutation fails
When the user retries after the underlying error is resolved
Then the retry uses current device/context
And only the authoritative successful response changes persisted-state presentation.

### AC-13 Component state semantics

Given a consequential Cultivation component is loading, mutating, rejected, failed, or unknown
When its state changes
Then the component shows the corresponding state and allowed recovery actions
And no generic success/empty state masks the actual result.

### AC-14 Confirmation and reversibility

Given a destructive action such as deleting a recipe/photo or ending a season
When the user initiates it
Then the documented confirmation policy is applied before dispatch
And the UI does not imply an undo path unless a source-backed reversal exists.

### AC-15 Pending navigation

Given a Cultivation mutation or unsaved recipe draft is active
When the user navigates to another owned surface
Then pending state/draft semantics remain truthful
And navigation success is not presented as mutation success.

### AC-16 Cross-page context

Given device A is selected
When the user opens Journal, Settings, Operations, or a supported Automation handoff
Then the destination receives or resolves the same device context
And returning to Cultivation reconciles affected state.

### AC-17 Concurrent edit

Given the same recipe/season can be changed by another writer
When the current editor submits stale data
Then the UI does not invent merge or last-write-wins behavior
And the authoritative rejection/failure is surfaced while preserving recoverable draft data.

### AC-18 Physical confirmation boundary

Given recipe apply returns an accepted/persisted result
When the UI renders the result
Then it may describe recipe application state
And must not describe physical dosing or actuator execution as confirmed without an explicit source-backed observation.

---

## 22. Implementation Mapping

| Requirement | Status |
|---|---|
| Merged `/cultivation` shell | Source-backed |
| Season CRUD | Source-backed |
| Active season progress | Source-backed |
| Season photo flow | Source-backed, Cloudinary-dependent |
| Completion summary | Source-backed |
| Recipe template CRUD | Source-backed |
| Recipe stage editor | Source-backed |
| Recipe apply | Source-backed |
| Bulk apply | Source-backed |
| Active recipe status | Source-backed |
| Dosing history | Source-backed |
| CSV export | Source-backed |
| Recipe clear UI | Implementation-required if needed |
| Bulk failure detail UI | Implementation-required |
| Season API error preservation | Implementation-required |
| Photo loading/error state | Implementation-required |
| Plant-type lock semantics | Implementation-required |
| Season/photo authorization harmonization | Implementation-required if required |
| Complete mutation audit coverage | Implementation-required if required |
| Rich station object | Deferred; do not invent |
| Draft persistence | Deferred unless required |
| Component State Contract | Source-backed contract; implementation verification required |
| Mutation Lifecycle Contract | Source-backed contract; implementation verification required |
| Confirmation / Reversibility Contract | Product contract; implementation verification required |
| Cross-Page Handoff Contract | Ownership/context contract; implementation verification required |
| Concurrency / Conflict Contract | Explicitly bounded; conflict mechanism implementation-required if supported |

---

## 23. Anti-Pattern Review

- No direct actuator control duplicated from Operations.
- No E-STOP duplicated.
- No recipe target presented as measured telemetry.
- No API acceptance presented as physical completion.
- No `last_seen` used as telemetry freshness.
- No full Journal duplicated.
- No full Settings configuration duplicated.
- No hidden device-context switch.
- No unsupported station object invented.
- Known gaps are explicitly marked rather than presented as implemented.
- Consequential component states are explicit and do not overload `success`, `accepted`, or `active` across semantic layers.
- Mutation lifecycle distinguishes validation, confirmation, dispatch, acceptance, persistence, reconciliation, and unknown outcomes.
- Confirmation and reversibility semantics are defined for destructive/consequential actions.
- Cross-page handoffs preserve device context and define return reconciliation.
- Concurrency behavior is explicit without inventing last-write-wins or merge semantics.

Open issues: season error/empty distinction, bulk failure details, photo state coverage, deletion semantics verification, plant-type contract, authorization consistency, audit completeness.

---

## 24. Definition of Done

Cultivation documentation is implementation-ready when:

- ownership is unambiguous;
- source of truth is mapped;
- selected device context is explicit;
- all consequential interactions have request/result/error semantics;
- recipe application is separated from physical confirmation;
- important data fields and units are defined;
- relevant states and recovery paths are defined;
- permissions use repository scopes/ownership;
- navigation and responsive behavior are defined;
- Journal/Settings/Operations/Automation boundaries are explicit;
- wireframes introduce no unsupported capability;
- acceptance criteria are testable;
- implementation gaps are explicitly separated from source-backed behavior.

---

## 25. Canonical Page-Spec Formula

```text
CULTIVATION PAGE SPEC
=
Responsibility
+ Source of Truth
+ Ownership
+ Selected Device Context
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

### Final review question

> Can another engineer implement Cultivation, its interactions, state transitions, permissions, recipe-application boundary, and cross-page behavior without inventing missing product behavior?

For the current repository, the documented source-backed scope is implementable; the gaps listed as `Implementation-required` must be resolved before those behaviors are claimed as fully implemented.

---

## 26. Checklist Compliance Audit

| Area | Status |
|---|---|
| 1 Page Identity | Covered |
| 2 Source of Truth | Covered |
| 3 Selected Context | Covered |
| 4 Information Architecture | Covered |
| 5 Component Inventory | Covered |
| 6 Interaction Contract | Covered |
| 7 Data Contract | Covered |
| 8 State Model | Covered with known source gaps |
| 9 Action / Command | Covered |
| 10 Safety / Control Authority | Covered |
| 11 Permission | Covered with authorization gap |
| 12 Navigation | Covered |
| 13 Desktop / Mobile | Covered |
| 14 Forms / Validation | Covered with range gaps |
| 15 History / Audit | Covered with audit gap |
| 16 Empty / Loading / Error / Unknown | Covered with source gaps |
| 17 Cross-Page | Covered |
| 18 Accessibility | Covered with verification gap |
| 19 Observability | Covered |
| 20 Wireframe | Covered |
| 21 Acceptance Criteria | Covered |
| 22 Implementation Mapping | Covered |
| 23 Anti-Pattern Review | Covered |
| 24 Definition of Done | Covered |
| 25 Canonical Formula | Covered |
| 26 Audit | Covered |

### Conclusion

**Cultivation functional spec is checklist-complete at the documentation-contract level.** This does not mean every listed improvement is already implemented in code.
