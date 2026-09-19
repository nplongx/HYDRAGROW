# P3 Frontend Migration Map

> Scope: remaining direct palette utility usage after the Automation/ReactFlow baseline migration.
>
> Scan excludes test files and counts utility occurrences in `src/**/*.{ts,tsx,css}`.

## Scan result

- **332 direct palette utility occurrences** remain.
- Worktree was clean before this scan.
- No source changes are required to complete this classification step.

## Classification

### A. Domain theme / semantic mapping — migrate deliberately, not by search/replace

These files encode domain meaning into reusable theme objects. They should be migrated by introducing semantic domain roles first, then changing consumers if needed.

| File | Count | Classification | Migration rule |
|---|---:|---|---|
| `src/lib/dosing/pumpVisualTheme.ts` | 32 | Domain theme | Preserve pump identity; introduce semantic pump/dosing roles instead of generic `primary`. |
| `src/lib/pumpLabels.ts` | 24 | Domain theme | Preserve actuator identity; map to semantic actuator categories. |
| `src/lib/logs/eventCategoryTheme.ts` | 16 | Domain theme | Preserve event category differentiation; map category → semantic info/status roles. |
| `src/hooks/useFlowCanvas.ts` | 8 | Domain theme/state presentation | Preserve node/action meaning; move color choices into semantic flow roles. |
| `src/components/ui/SensorBentoCard.tsx` | 20 | Domain theme component | Keep sensor category differentiation, but make the palette configurable through semantic sensor tones. |
| `src/components/automation/reactflow/FlowSummaryNode.tsx` | 2 | Domain theme | Preserve node type meaning; use semantic node/status roles. |

**Subtotal: 102 occurrences.**

### B. Safety / fault / destructive states — migrate to danger/fault tokens

These are already semantically clear and should not retain raw red palette utilities.

- `src/components/auth/ForgotPasswordScreen.tsx` — 1
- `src/components/auth/LoginScreen.tsx` — 4
- `src/components/auth/RegisterScreen.tsx` — 1
- `src/components/control/AdvancedDeviceControl.tsx` — 6
- `src/components/fleet/FleetStationCard.tsx` — 17
- `src/components/logs/EventDetailDrawer.tsx` — 1
- `src/components/logs/EventLogCard.tsx` — 12
- `src/components/safety/EmergencyStopButton.tsx` — 4
- `src/components/safety/EmergencyStopConfirmDialog.tsx` — 9
- `src/components/ui/FaultExplanation.tsx` — 7
- `src/components/ui/FsmStatusBadge.tsx` — 9
- `src/components/ui/InputGroup.tsx` — 6
- `src/pages/FleetView.tsx` — 8
- `src/pages/RecipeBuilder.tsx` — 14
- `src/pages/Roles.tsx` — 5
- `src/pages/settings/ConnectivitySection.tsx` — 12
- `src/pages/settings/DangerZoneSection.tsx` — 14
- `src/pages/settings/ThresholdsSection.tsx` — 4
- `src/pages/settings/SeasonHistoryList.tsx` — 2
- `src/pages/settings/SeasonPhotoJournal.tsx` — 2

**Rule:** error/fault/danger semantics first; no visual redesign hidden inside the migration.

### C. Automation / configuration surfaces — migrate to config/info/status semantics

- `src/components/automation/AutomationMetricsBanner.tsx` — 1
- `src/components/automation/ConfigExplorerWidget.tsx` — 15
- `src/components/automation/FlowEditorFooter.tsx` — 5
- `src/components/automation/FlowOverviewCard.tsx` — 12
- `src/components/automation/NextFlowSelector.tsx` — 6
- `src/components/automation/ScriptListPanel.tsx` — 6
- `src/components/automation/WebhookAndChainPanel.tsx` — 21

**Subtotal: 66 occurrences.**

These are component-level migration candidates. Prefer existing design primitives and semantic config/info/warning/error roles.

### D. General UI primitives — migrate first because they fan out

- `src/components/ui/AccordionSection.tsx` — 3
- `src/components/ui/InputGroup.tsx` — 6
- `src/components/ui/SubCard.tsx` — 1
- `src/components/ui/Switch.tsx` — 1
- `src/components/ui/FsmStatusBadge.tsx` — 9
- `src/components/ui/FaultExplanation.tsx` — 7
- `src/components/ui/SensorBentoCard.tsx` — 20

These should be migrated before broad page work so downstream pages inherit the P3 language automatically.

### E. Page-level visual migration

- `src/pages/Analytics.tsx` — 4
- `src/pages/ControlPanel.tsx` — 1
- `src/pages/DevicePairing.tsx` — 1
- `src/pages/FleetView.tsx` — 8
- `src/pages/RecipeBuilder.tsx` — 14
- `src/pages/Roles.tsx` — 5
- `src/pages/settings/ConnectivitySection.tsx` — 12
- `src/pages/settings/DangerZoneSection.tsx` — 14
- `src/pages/settings/GeneralSection.tsx` — 5
- `src/pages/settings/ThresholdsSection.tsx` — 4

Page migration should happen after shared primitives/domain themes are normalized.

### F. Layout / shell exceptions

- `src/components/layout/AppShell.tsx` — 2
- `src/App.css` — 1

These are small residuals. They should be cleaned during shell/component normalization, not treated as a page redesign.

## Priority order

1. **Shared UI primitives**: `FsmStatusBadge`, `FaultExplanation`, `InputGroup`, `AccordionSection`, `SubCard`, `Switch`, `SensorBentoCard`.
2. **Domain theme registries**: `pumpVisualTheme`, `pumpLabels`, `eventCategoryTheme`, `useFlowCanvas`.
3. **Safety/fault surfaces**: Emergency Stop, Danger Zone, fault/error states.
4. **Automation/config surfaces**: remaining automation cards/panels.
5. **Page-level migration**: Analytics, Fleet, RecipeBuilder, Roles, Settings, etc.
6. **Final sweep**: App.css/AppShell and a zero-direct-palette audit.

## Semantic rules

- `red/rose` → `error`, `fault`, or `danger` depending on behavior.
- `amber/orange/yellow` → `warning` only when the state is actually cautionary; otherwise define a domain role.
- `sky/blue/cyan/teal` → `info/water/config` only when that meaning is explicit.
- `indigo/purple/violet/fuchsia` → `config` or domain-specific role; never blindly map all to one generic accent.
- `emerald` → `primary`, `success`, `status`, or domain role depending on meaning.
- `slate` → text/surface/border semantics.
- Do not encode state architecture, business logic, or controller behavior into the design migration.

## Definition of done for Step 1

- [x] Remaining direct palette usage scanned.
- [x] Usage classified into domain themes, safety/fault, automation/config, shared UI, page-level, and shell exceptions.
- [x] Migration order established.
- [x] Domain-theme files explicitly protected from blind search/replace.
- [ ] Execute Step 2: shared primitives + domain theme normalization.
