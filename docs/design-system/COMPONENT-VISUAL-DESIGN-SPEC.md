# HYDRAGROW — Component Visual Design Specification

## 0. Purpose

`COMPONENT-CONTRACT.md` defines behavior. `DESIGN-TOKENS-VISUAL-SPEC.md` defines visual primitives. This document bridges both with concrete visual compositions for the canonical reusable components.

It is the input contract for Penpot design work and React implementation. It does not replace either source document.

## 1. Design principles

- Operational scanning first; decoration second.
- One dominant visual hierarchy per component.
- Semantic state must remain understandable without color.
- Component geometry is stable across pages; composition changes at page level.
- Use existing semantic tokens. No one-off colors, radii, shadows, or typography.
- Dense desktop monitoring must remain readable on tablet/mobile.
- Safety-critical actions remain visually unique.

## 2. Canonical component set

P0 components to design in Penpot first:

1. `AppShell`
2. `PageHeader`
3. `Panel`
4. `Button`
5. `StatusPill` / `DeviceStatePill`
6. `TelemetryCard`
7. `TelemetryGroup`
8. `ActuatorCard`
9. `QuickActionBar`
10. `Banner`
11. `StateView`
12. `Switch`
13. `InputGroup`
14. `TabShell`
15. `DataTable`
16. `Modal` / `ConfirmDialog`
17. `EmergencyStopButton`

Existing domain-specific components (`PumpControlStatePill`, `SensorBentoCard`, dosing cards, fleet cards, season cards, etc.) inherit these primitives and should be designed after P0.

## 3. Shared geometry

### Desktop

- App content max width: `1440px`.
- Main content horizontal padding: `24px` at desktop, `16px` below `768px`.
- Standard component gap: `12px` / `16px`.
- Panel/card radius: `16px`.
- Compact control radius: `10px`.
- Pill radius: full.
- Standard control height: `40px`.
- Compact control height: `32px`.
- Primary touch target: minimum `44px` where interactive.

### Mobile

- Sidebar collapses to navigation trigger.
- Two-column telemetry becomes one column or horizontally scrollable grouped strip according to page priority.
- Panel padding reduces from `20–24px` to `16px`.
- Header actions wrap instead of shrinking labels below readability.

## 4. Component visual contracts

### 4.1 AppShell

**Composition:** fixed/anchored navigation rail + top utility region + scrollable content canvas.

**Hierarchy:** brand > primary navigation > page content > global safety action.

**States:** normal, compact/mobile navigation, permission-limited navigation.

**Rules:** navigation never competes visually with page title or safety state; emergency stop is not hidden inside navigation.

### 4.2 PageHeader

**Anatomy:** eyebrow/context, page title, optional description, right-side actions.

**Visual:** title is strongest text; description is muted; actions use compact controls.

**Variants:** default, with status, with tabs, with primary action.

### 4.3 Panel

**Anatomy:** optional header + content + optional footer/action row.

**Visual:** `surface-default`, subtle border, `16px` radius, low elevation only when separation requires it.

**Variants:** standard, emphasized, critical.

### 4.4 Button

**Variants:** primary, secondary/outline, ghost, destructive, icon-only.

**States:** default, hover, focus, active, disabled, pending.

**Hierarchy:** one primary action per local action group unless the task explicitly requires multiple equivalent actions.

### 4.5 StatusPill / DeviceStatePill

**Anatomy:** status dot/icon + short canonical label.

**Visual:** compact, high legibility, semantic background/text token pair.

**States:** online, offline, warning, dosing, auto, manual; extend only when a new domain state is formally added.

### 4.6 TelemetryCard

**Anatomy:** label + current value + unit + state/freshness + optional trend.

**Visual hierarchy:** value > unit > label > freshness metadata.

**Variants:** standard, compact, trend, stale, unavailable, fault.

**Rule:** stale/unavailable state must be explicit. Never show stale value with normal styling.

### 4.7 TelemetryGroup

**Composition:** 2–4 related `TelemetryCard`s with one group heading/context.

**Rule:** group by operational meaning, not by arbitrary data source.

### 4.8 ActuatorCard

**Anatomy:** actuator identity + current physical state + control + feedback/freshness.

**Visual:** identity and physical state dominate; control is secondary. Pending command must visibly differ from confirmed physical state.

### 4.9 QuickActionBar

**Composition:** horizontally ordered high-frequency actions.

**Rule:** only reversible, common actions belong here. Destructive/safety actions use dedicated treatment.

### 4.10 Banner

**Variants:** info, warning, fault, success.

**Anatomy:** icon + title/message + optional action/dismiss.

**Rule:** severity is communicated by text/icon plus semantic color, never color alone.

### 4.11 StateView

**Use:** empty, unavailable, error, loading placeholders.

**Anatomy:** icon + title + optional description + optional action.

**Visual:** centered, low-noise, generous whitespace. Action is optional and directly related to recovery.

### 4.12 Switch

**Anatomy:** label + track/thumb.

**States:** on, off, disabled, focus, pending where asynchronous.

**Rule:** label explains the behavior being enabled, not an implementation detail.

### 4.13 InputGroup

**Anatomy:** label + control + unit/help/error.

**States:** default, focus, filled, disabled, invalid, warning.

**Rule:** validation message stays adjacent to the field and does not rely on color alone.

### 4.14 TabShell

**Visual:** compact horizontal tabs with one clear active state.

**Responsive:** horizontal overflow is permitted; do not wrap into ambiguous multi-row navigation.

### 4.15 DataTable

**Hierarchy:** column label > primary cell content > supporting metadata.

**States:** default row, hover, selected, disabled, loading, empty.

**Responsive:** preserve primary columns; secondary columns may collapse into row detail.

### 4.16 Modal / ConfirmDialog

**Anatomy:** title + explanation + content + action row.

**Visual:** overlay + elevated surface; destructive confirmation gets explicit consequence text.

**Rule:** primary action label names the actual consequence.

### 4.17 EmergencyStopButton

**Visual:** unique critical red treatment, minimum `56px` floating target where floating variant is used.

**Variants:** floating, bottom bar.

**Rule:** never visually compete with ordinary destructive actions; confirmation is mandatory.

## 5. Penpot file structure

Create one local Penpot file:

```text
HYDRAGROW — Component Library
├── 00 Foundations
├── 01 Navigation
├── 02 Controls
├── 03 Feedback
├── 04 Data Display
├── 05 Safety
└── 06 Component Gallery
```

`06 Component Gallery` contains large, inspectable examples of every P0 component and its meaningful states. Each component gets a canonical base frame plus state/variant frames where useful.

## 6. Penpot naming

Use slash-delimited component names:

```text
Button/Primary
Button/Secondary
Button/Danger
StatusPill/Online
StatusPill/Warning
TelemetryCard/Default
TelemetryCard/Stale
TelemetryCard/Fault
StateView/Empty
StateView/Error
Switch/On
Switch/Off
```

Layer names must describe semantic roles, not coordinates or visual colors.

## 7. Design-to-code rule

Penpot captures visual geometry, variants, tokens, and composition. React remains runtime source of truth for behavior, state, accessibility, responsive behavior, and integration.

After Penpot design is established, map each canonical component to the existing React component where one already exists. Do not create a second implementation of an existing primitive merely to match the Penpot file.

## 8. Definition of done

A P0 component is complete only when:

- base visual composition exists in Penpot;
- meaningful states/variants are represented;
- token usage is inspectable;
- responsive behavior is documented;
- corresponding React component exists or is explicitly tracked;
- browser rendering matches the Penpot intent closely enough for visual QA;
- no raw one-off color/spacing/radius decisions were introduced.

## 10. Penpot implementation status

Penpot MCP is enabled and connected to the local self-hosted file. Current verified library state is intentionally smaller than the historical inventory below.

Current gallery page:

```text
06 Component Gallery
```

Verified in the current file:

- `Button` variant group
- variants: `Style=Primary`, `Style=Danger`
- two component instances placed in `P0 Gallery`
- library readback confirms one variant group and both gallery instances

The Penpot token catalog is currently empty, so this pass does not claim token binding.

This is still visual exploration, not final production handoff. The expanded inventory in §11 remains the design backlog. Next: add the remaining P0/P1 families in small MCP batches, bind the HYDRAGROW token set after the visual structure is stable, add meaningful interaction states, then map each designed component to its existing React implementation.

## 11. Expanded component inventory

The initial P0 set is not the whole frontend component surface. The following inventory is derived from the current `hydragrow-frontend/src/components` tree. It is split by reuse/operational importance so the Penpot library grows without turning every one-off page fragment into a design-system primitive.

### 10.1 Shared UI primitives — P1

| Component | Source | Visual contract | Key states/variants |
|---|---|---|---|
| `AccordionSection` | `src/components/ui/AccordionSection.tsx` | header row + optional icon/badge + chevron + bordered content region | open, closed, controlled, hidden |
| `Badge` | `src/components/ui/Badge.tsx` | compact semantic label, optional dot | success, warning, danger, info, neutral; dot/no-dot |
| `LoadingState` | `src/components/ui/LoadingState.tsx` | low-noise loading placeholder with progress/spinner affordance | default, compact |
| `SubCard` | `src/components/ui/SubCard.tsx` | nested surface inside a parent panel | default, dense |
| `Slider` | `src/components/ui/Slider.tsx` | labelled range control with value/readout | default, focus, disabled |
| `HealthScore` | `src/components/ui/HealthScore.tsx` | numeric score + segmented health meter + semantic label | great, attention, critical, offline |
| `Sparkline` | `src/components/ui/Sparkline.tsx` | minimal single-series trend line | positive/neutral/negative trend, empty |
| `SensorBentoCard` | `src/components/ui/SensorBentoCard.tsx` | sensor identity + value/unit + semantic status + optional range/trend | normal, compact, warning, danger, unavailable |
| `DosingSummaryCard` | `src/components/ui/DosingSummaryCard.tsx` | dosing total + last-dose metadata | populated, no-dose |
| `FaultExplanation` | `src/components/ui/FaultExplanation.tsx` | fault title/code + explanation + recovery guidance | known fault, unknown fault, dismissible |
| `FsmStatusBadge` | `src/components/ui/FsmStatusBadge.tsx` | FSM state/fault semantic badge | normal state, fault, unknown |
| `PumpControlStatePill` | `src/components/ui/PumpControlStatePill.tsx` | pump control state + optional lock reason | idle, running, locked/auto, locked/emergency, locked/interlock |
| `DosingHourlyChart` | `src/components/dosing/DosingHourlyChart.tsx` | 24-hour grouped pump visualization + legend/text alternative | normal, sparse, zero-data |

### 10.2 Layout/navigation — P1

| Component | Source | Visual contract | Key states/variants |
|---|---|---|---|
| `MainLayout` | `src/components/layout/MainLayout.tsx` | page canvas + navigation/content frame | desktop, tablet, mobile |
| `DesktopSidebar` | `src/components/layout/AppShell.tsx` | navigation rail with active route and utility area | active, collapsed, permission-limited |
| `MobileHeader` | `src/components/layout/AppShell.tsx` | compact mobile title/navigation trigger | default, contextual |
| `MobileBottomNav` | `src/components/layout/AppShell.tsx` | primary mobile navigation strip | active, permission-limited |
| `ConnectionStatus` | `src/components/layout/AppShell.tsx` | global connectivity/transport status | connected, reconnecting, offline |

### 10.3 Automation builder — P1

These are not generic UI primitives. They are a domain component family and should share the same surface/control tokens while preserving their React Flow semantics.

| Component | Source | Visual contract | Key states/variants |
|---|---|---|---|
| `AutomationPageHeader` | `src/components/automation/AutomationPageHeader.tsx` | automation page title + creation/config actions | default, action-heavy |
| `AutomationConflictBanner` | `src/components/automation/AutomationConflictBanner.tsx` | conflict severity + affected automation + detail action | no conflict, conflict |
| `AutomationMetricsBanner` | `src/components/automation/AutomationMetricsBanner.tsx` | compact automation health/metric summary | healthy, degraded, empty |
| `AutomationMultiDeviceTemplatePanel` | `src/components/automation/AutomationMultiDeviceTemplatePanel.tsx` | template selection + target device summary | empty target, selected, applying |
| `FlowOverviewCard` | `src/components/automation/FlowOverviewCard.tsx` | automation identity + kind + enabled state + trigger summary + last run | enabled, disabled, cron, webhook, config |
| `FlowEditorHeader` | `src/components/automation/FlowEditorHeader.tsx` | editable flow name/type + enable switch | new, editing, enabled, disabled |
| `FlowEditorFooter` | `src/components/automation/FlowEditorFooter.tsx` | test/delete/save action row | new, dirty, pending |
| `FlowDetailDrawer` | `src/components/automation/FlowDetailDrawer.tsx` | side drawer with flow details | open, closed, loading |
| `NextFlowSelector` | `src/components/automation/NextFlowSelector.tsx` | successor-flow selection control | none, selected, invalid |
| `ScriptListPanel` | `src/components/automation/ScriptListPanel.tsx` | automation list + load/select actions | empty, populated, selected |
| `ConfigExplorerWidget` | `src/components/automation/ConfigExplorerWidget.tsx` | compact config override summary + drill-in | no overrides, active overrides |
| `ConfigExplorerView` | `src/components/automation/ConfigExplorerView.tsx` | full config override/audit view | active, reverted, empty |
| `WebhookAndChainPanel` | `src/components/automation/WebhookAndChainPanel.tsx` | webhook/chain configuration | empty, configured, invalid |
| `NodePalette` | `src/components/automation/reactflow/NodePalette.tsx` | draggable node-type palette | idle, hover, drag |
| `NodeEditorPanel` | `src/components/automation/reactflow/NodeEditorPanel.tsx` | inspector/editor for selected node | none selected, selected, invalid, pending |
| `ConditionGroupEditor` | `src/components/automation/reactflow/ConditionGroupEditor.tsx` | boolean condition group editor | AND, OR, nested, invalid |
| `VariableCombobox` | `src/components/automation/reactflow/VariableCombobox.tsx` | searchable variable/token selector | closed, open, selected, no match |
| `WebhookFieldMappingEditor` | `src/components/automation/reactflow/WebhookFieldMappingEditor.tsx` | external payload → internal field mapping | empty, mapped, invalid |
| `TestPanel` | `src/components/automation/reactflow/TestPanel.tsx` | automation test input/output surface | idle, running, success, failure |
| `ActionNode` / `TriggerNode` / `ConfigNode` / `FlowSummaryNode` | `src/components/automation/reactflow/nodeTypes.tsx`, `FlowSummaryNode.tsx` | React Flow node cards with type identity, ports and summary | selected, unselected, invalid, disabled |

### 10.4 Dosing / control / fleet — P1

| Component | Source | Visual contract | Key states/variants |
|---|---|---|---|
| `AdvancedDeviceControl` | `src/components/control/AdvancedDeviceControl.tsx` | actuator identity + safety/interlock state + command control | idle, running, locked, pending, fault |
| `DosingAnomalyBanner` | `src/components/dosing/DosingAnomalyBanner.tsx` | dosing anomaly severity + explanation/action | warning, critical, empty |
| `DosingReportCard` | `src/components/dosing/DosingReportCard.tsx` | timeline event + pump volume chips + timestamp | nutrient, pH, water, mixed |
| `DosingTotalCard` | `src/components/dosing/DosingTotalCard.tsx` | total today + trend chart + 7-day comparison | normal, rising, falling, zero |
| `FleetStationCard` | `src/components/fleet/FleetStationCard.tsx` | station identity + online state + EC/pH snapshot + warning count | online, offline, warning, no-data |
| `QuickActionBar` | `src/components/ui/QuickActionBar.tsx` | ordered high-frequency action group | normal, disabled, pending |

### 10.5 Logs / observability — P1

| Component | Source | Visual contract | Key states/variants |
|---|---|---|---|
| `HealthSummaryBar` | `src/components/logs/HealthSummaryBar.tsx` | health counters + view mode + search | important/all, healthy/degraded |
| `EventLogCard` | `src/components/logs/EventLogCard.tsx` | event severity/category + timestamp + summary | info, warning, error, success |
| `CycleEventCard` | `src/components/logs/CycleEventCard.tsx` | grouped cycle timeline event | collapsed, expanded, actionable |
| `EventDetailDrawer` | `src/components/logs/EventDetailDrawer.tsx` | technical event detail drawer | open, loading, error |
| `MetadataRenderer` | `src/components/logs/MetadataRenderers.tsx` | structured technical metadata | dosing, generic, empty |

### 10.6 Cultivation / season — P1

| Component | Source | Visual contract | Key states/variants |
|---|---|---|---|
| `ActiveSeasonCard` | `src/components/seasons/ActiveSeasonCard.tsx` | active season identity + progress + recipe + edit/end actions | active, editing, delayed, loading |
| `CreateSeasonForm` | `src/components/seasons/CreateSeasonForm.tsx` | season creation form | pristine, invalid, submitting |
| `SeasonHistoryList` | `src/components/seasons/SeasonHistoryList.tsx` | historical seasons list | empty, populated, selected |
| `SeasonPhotoJournal` | `src/components/seasons/SeasonPhotoJournal.tsx` | chronological photo journal | empty, populated, uploading |
| `SeasonStageChecklist` | `src/components/seasons/SeasonStageChecklist.tsx` | stage progression checklist + remaining days | completed/current/upcoming |
| `SeasonCompletionSummary` | `src/components/seasons/SeasonCompletionSummary.tsx` | completion summary dialog | open, closed |
| `ActiveRecipeStatus` | `src/components/recipes/ActiveRecipeStatus.tsx` | active recipe/stage summary | active, missing |

### 10.7 Roles / auth / onboarding / pairing — P1

| Component | Source | Visual contract | Key states/variants |
|---|---|---|---|
| `RoleBadge` | `src/components/roles/RoleBadge.tsx` | role identity badge | admin, operator, viewer |
| `PermissionMatrix` | `src/components/roles/PermissionMatrix.tsx` | capability × role matrix | editable, read-only |
| `InviteForm` | `src/components/roles/InviteForm.tsx` | invite identity + role/scope fields | pristine, invalid, submitting, success |
| `AuthCard` | `src/components/auth/AuthCard.tsx` | centered auth surface with footer | login, register, recovery |
| `AuthTextField` | `src/components/auth/AuthCard.tsx` | auth-specific labelled field | default, focus, invalid, disabled |
| `AuthSubmitButton` | `src/components/auth/AuthCard.tsx` | auth primary submit control | default, pending, disabled |
| `OnboardingStep` | `src/components/onboarding/OnboardingStep.tsx` | step identity + content + action | current, complete, locked |
| `OnboardingWizard` | `src/components/onboarding/OnboardingWizard.tsx` | multi-step setup shell | first, middle, last, submitting |
| `ScanConfirmOverlay` | `src/components/pairing/ScanConfirmOverlay.tsx` | device identity + confirmation code + confirm/cancel | detected, confirming, invalid |

### 10.8 Internal automation form primitives — P2

These exist inside automation editor files rather than as standalone shared components. Design them as a coherent sub-library before extracting them into `src/components/ui`.

`FieldGroup`, `InputWithSuffix`, `InputWithButton`, `PillsSelector`, `Segmented`, `ChipsRow`, `Chip`, `ToggleRow`, `ConfigCard`, `InspectorShell`, and `SafeNote` live primarily in `src/components/automation/reactflow/NodeEditorPanel.tsx` and `ConfigPanelUI.tsx`.

Visual rule: all of these inherit `InputGroup`, `Button`, `Switch`, `Badge`, `Panel/SubCard`, and spacing tokens. Do not promote them to global primitives until there are at least two independent non-automation consumers.

## 12. Penpot coverage target

The library target is **17 P0 contract entries + 50+ P1/P2 component surfaces**, not 17 total components. Current verified coverage is **1 variant group / 2 gallery instances**; the remaining inventory is backlog until readback confirms implementation.

Penpot implementation order:

1. Finish shared P1 UI primitives: `Badge`, `AccordionSection`, `LoadingState`, `SubCard`, `Slider`, `HealthScore`, `Sparkline`, `SensorBentoCard`, `DosingSummaryCard`, `FaultExplanation`, `FsmStatusBadge`, `PumpControlStatePill`.
2. Build the automation family as one coherent visual cluster: cards → editor header/footer → drawer → palette → nodes → inspector/form primitives → test panel.
3. Build operational data clusters: dosing, fleet, logs.
4. Build cultivation and lifecycle clusters: seasons, recipes, onboarding, pairing.
5. Build role/auth surfaces last because their visual grammar is mostly inherited from the shared primitives.

For each cluster, the gallery should show **base + meaningful state variants**, not a single static screenshot. Reusable Penpot components should be created for repeated primitives and stable domain cards; complex page-local compositions remain boards/examples.

## 13. Component coverage matrix

Coverage is tracked at three levels:

- `P0`: reusable foundation required across pages; must become a real Penpot component.
- `P1`: existing reusable/domain component with enough reuse or operational importance to receive a dedicated visual design.
- `P2`: internal/page-specific surface; document visual grammar and design it when the parent feature is refined, but do not force it into the global library prematurely.

Current verified status:

```text
P0: 1/17 component groups currently verified in Penpot
P1: visual inventory defined; implementation pending
P2: inventory complete; visual design follows parent feature
```

P2 automation form visual exploration is now represented in a dedicated page:

```text
07 Automation — P2
└── P2 / Automation Internal Form Grammar
    ├── FieldGroup
    ├── InputWithSuffix
    ├── InputWithButton
    ├── PillsSelector
    ├── Segmented
    ├── ChipsRow + Chip
    ├── ToggleRow
    ├── ConfigCard
    ├── InspectorShell
    └── SafeNote
```

These remain page-local automation surfaces rather than global Penpot library components. The visual treatment inherits the existing P0/P1 `InputGroup`, `Button`, `Switch`, `Badge`, `Panel/SubCard`, spacing, radius, and semantic-state grammar. Promotion to the global library still requires independent reuse outside automation.

This prevents the design file from being falsely treated as complete just because the first 17 primitives exist.
