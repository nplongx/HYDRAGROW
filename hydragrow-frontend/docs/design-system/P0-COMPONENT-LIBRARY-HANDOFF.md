# HYDRAGROW P0 Component Library — OpenPencil Handoff

Source design: OpenPencil web editor, document `HYDRAGROW-P0-Component-Library`.

## Component inventory

- `AppShell`
- `PageHeader`
- `Panel`
- `Button`
- `StatusPill`
- `TelemetryCard`
- `TelemetryGroup`
- `ActuatorCard`
- `QuickActionBar`
- `Banner`
- `StateView`
- `Switch`
- `InputGroup`
- `TabShell`
- `DataTable`
- `Modal`
- `EmergencyStopButton`

## Visual rules encoded

- Inter typography; regular / medium / semibold / bold only.
- 8pt spacing foundation; 16px panel radius; 12/16px common gaps.
- Semantic green / warning / critical / unavailable treatments from `src/design-system/tokens.css`.
- Status meaning uses text plus visual signal; never color alone.
- Pending is distinct from confirmed success.
- Unavailable is distinct from disabled.
- Emergency stop uses unique critical treatment and explicit confirmation pattern.
- Responsive reference covers desktop and mobile composition without changing semantic component roles.

## Exported artifacts

- `HYDRAGROW-P0-Component-Library.svg` — vector gallery export.
- `HYDRAGROW-P0-Component-Library.jsx` — OpenPencil Design JSX round-trip source.
- `P0-component-gallery.png` — visual QA preview.

The editable component library remains open in the local OpenPencil web editor tab used for MCP authoring.

## QA

- OpenPencil top-level overlap analysis: 0 findings after cleanup.
- Typography analysis: Inter only; 21 concrete text styles in the gallery.
- Spacing analysis: 8px grid used for primary layout values; several compact 2–10px values remain intentionally for control internals and typography rhythm.
