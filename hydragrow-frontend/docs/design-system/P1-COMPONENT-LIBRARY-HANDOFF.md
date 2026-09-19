# HYDRAGROW P1 Component Library — OpenPencil Handoff

Source: local OpenPencil web editor, document `HYDRAGROW-P1-Component-Library`.

## Designed component families

### Cultivation
- `CropIllustration/Lettuce`
- `CropIllustration/Basil`
- `CropIllustration/Strawberry`
- `CropIllustration/Tomato`
- `CropIllustration/Other`
- `SeasonStageChecklist`
- `SeasonCompletionSummary`

### Fleet / station
- `StationTower/Compact`
- `StationTower/Card`
- `StationTower/Empty`

Base station artwork stays neutral. Online/offline/warning/unavailable remain UI state overlays.

### Automation
- `AutomationNode/Trigger`
- `AutomationNode/Schedule`
- `AutomationNode/Condition`
- `AutomationNode/Dose`
- `AutomationNode/Water`
- `AutomationNode/Actuator`
- `AutomationNode/Alert`
- `AutomationNode/Stage`
- `AutomationNode/EndSeason`
- `AutomationNode/Chain`
- `AutomationNode/Config`
- `AutomationNode/Safety`
- `FlowOverviewCard`
- `NodeEditorPanel`
- `TestPanel`
- `AutomationConflictBanner`

### Operational / observability primitives
- `HealthScore`
- `SensorBentoCard`
- `DosingSummaryCard`
- `FaultExplanation`

Total reusable OpenPencil components: **30**.

## Visual rules

- Inter typography.
- HYDRAGROW semantic green/water/warning/critical palette.
- Flat vector botanical/station motifs; no status encoded only by artwork.
- Completion artwork restrained; metrics remain authoritative UI data.
- Automation `Config` uses indigo exception defined by the design spec.
- Safety node remains critical and distinct from ordinary automation actions.
- Pending/test simulation never represents physical confirmation.
- Mobile composition stacks data before artwork; artwork may shrink, but operational meaning remains text/icon based.

## QA

- OpenPencil overlap analysis: **0 overlaps**, 288 nodes analyzed.
- Typography: Inter only, 14 styles, 122 text nodes.
- Spacing grid: 8px foundation; compact component internals use smaller rhythm values where required by icon/control geometry.
- OpenPencil component inventory: **30 COMPONENT nodes**.

## Artifacts

- `HYDRAGROW-P1-Component-Library.svg` — gallery export.
- `HYDRAGROW-P1-Component-Library.jsx` — Design JSX source used to author the gallery.
- Editable component library remains open in OpenPencil web as `HYDRAGROW-P1-Component-Library`.

The OpenPencil web editor's browser file-save path is separate from the repository filesystem; therefore the repository handoff uses the exported SVG/JSX while the editable `.fig` remains in the OpenPencil document.

## Source mapping

Visual intent is derived from `FRONTEND-ASSET-DESIGN-SPEC.md`, `COMPONENT-VISUAL-DESIGN-SPEC.md`, `DESIGN-TOKENS-VISUAL-SPEC.md`, plus current React implementations under `hydragrow-frontend/src/components`.
