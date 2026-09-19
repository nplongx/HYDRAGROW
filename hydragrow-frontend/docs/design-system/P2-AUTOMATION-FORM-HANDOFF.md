# HYDRAGROW P2 Automation Internal Form Grammar — OpenPencil Handoff

Source: local OpenPencil document `HYDRAGROW-P1-Component-Library`, page `07 Automation — P2`.

## Page-local surfaces

Designed as one coherent automation sub-library; these are **not** global Penpot/OpenPencil components:

- `FieldGroup`
- `InputWithSuffix`
- `InputWithButton`
- `PillsSelector`
- `Segmented`
- `ChipsRow + Chip` (design node is `ChipsRow`, with `Chip` treatment shown inside)
- `ToggleRow`
- `ConfigCard`
- `InspectorShell`
- `SafeNote`

## Visual grammar

- Inter typography.
- Existing HYDRAGROW semantic green/water/warning/critical treatment only.
- Existing indigo config exception is used by `ConfigCard`.
- Standard panel/card radius: 16px; compact controls: 10px; pills: full radius.
- Control geometry stays around 40–44px; interactive targets preserve the 44px minimum intent.
- Validation is text-first and adjacent to the affected field.
- Pending/test state never represents physical confirmation.
- `SafeNote` is visually unique for safety/interlock meaning and uses icon + explicit text + semantic red.
- Responsive rule: below 768px, cards stack; inspector identity precedes fields; chip rows may scroll; labels remain readable.
- Promotion rule: keep these page-local until an independent non-automation consumer exists.

## React handoff

OpenPencil captures visual geometry and state examples. Existing React remains the runtime source of truth for behavior, accessibility, responsive behavior, validation, and API state. Do not create duplicate global primitives merely to match this page.

## QA evidence

- OpenPencil overlap analysis after geometry cleanup: **0 overlaps**, 166 nodes analyzed.
- Typography analysis: **Inter only**, 12 styles, 86 text nodes.
- Spacing follows the existing 8px foundation with compact control geometry where required.
- No new global component promotion introduced.

## Artifacts

- `P2-Automation-Internal-Form-Grammar.jsx` — authored OpenPencil JSX source.
- `P2-Automation-Internal-Form-Grammar.svg` — OpenPencil SVG export.
- `P2-automation-form-gallery.png` — gallery PNG for visual QA.

The editable OpenPencil document remains the design source. Repository artifacts are the review/handoff exports.
