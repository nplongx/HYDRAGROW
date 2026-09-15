# HYDRAGROW — E. DESIGN TOKENS & VISUAL SPECIFICATION

## E0. Purpose

This document defines the visual foundation of HYDRAGROW.

It translates the visual language referenced by the UX Rules and Component Contract into a stable token and specification layer covering color, typography, spacing, surfaces, borders, radius, elevation, controls, status treatment, responsive behavior, and accessibility.

The specification is implementation-agnostic. Framework-specific values should consume these semantic tokens rather than redefining visual decisions locally.

## E1. Visual Principles

HYDRAGROW should feel:

- Operational rather than decorative.
- Calm under normal conditions.
- Highly legible during abnormal conditions.
- Dense enough for monitoring without becoming visually noisy.
- Consistent across station, fleet, cultivation, configuration, and audit contexts.

Visual emphasis must communicate operational priority, not decoration.

## E2. Token Architecture

Tokens should be organized into three layers:

### Foundation tokens

Raw visual primitives such as:

- Base colors
- Font families
- Font sizes
- Font weights
- Spacing units
- Border widths
- Radii
- Shadows

### Semantic tokens

Meaningful product roles such as:

- `surface-default`
- `text-primary`
- `status-warning`
- `status-fault`
- `control-primary`

### Component tokens

Component-specific mappings such as:

- Button height
- Panel padding
- Telemetry value size
- Table row height
- Modal width

Components should consume semantic tokens whenever possible.

## E3. Color System

The color system should distinguish visual hierarchy from semantic state.

Primary groups:

- Background
- Surface
- Elevated surface
- Border
- Primary text
- Secondary text
- Muted text
- Accent / interactive
- Success / normal
- Warning
- Fault / critical
- Unavailable
- Focus

### Color rules

1. Semantic colors represent meaning.
2. Accent colors represent interaction or product identity.
3. Neutral colors establish hierarchy.
4. Critical states receive stronger visual emphasis than normal states.
5. Color must never be the sole carrier of operational meaning.

## E4. Semantic Status Colors

Canonical semantic roles should include:

| Token role | Meaning |
|---|---|
| `status-normal` | Normal / healthy operational state |
| `status-success` | Confirmed successful operation |
| `status-warning` | Condition requiring attention |
| `status-fault` | Fault / critical abnormal condition |
| `status-unavailable` | Required resource currently unavailable |
| `status-pending` | Operation awaiting confirmation |
| `status-locked` | Action/resource intentionally locked |
| `status-info` | Informational condition |

Do not introduce a new semantic color for a condition that already maps to one of these roles.

## E5. Status Color Behavior

Status treatment should normally combine:

- Color
- Text
- Icon or shape where useful
- Position/hierarchy

Critical status should not depend on a small color indicator that can be missed during scanning.

Pending should be visually distinct from success.

Unavailable should be visually distinct from normal disabled controls.

## E6. Surface System

Recommended semantic surface roles:

- `surface-app`
- `surface-default`
- `surface-subtle`
- `surface-elevated`
- `surface-overlay`
- `surface-disabled`
- `surface-critical`

### Rules

- App background provides the broadest visual field.
- Default surfaces contain primary content.
- Elevated surfaces indicate temporary or higher-priority layers.
- Overlay surfaces belong to dialogs and transient UI.
- Critical surfaces are reserved for meaningful operational conditions.

Do not use elevation solely for decoration.

## E7. Border System

Borders establish grouping, separation, and state.

Semantic roles should include:

- `border-default`
- `border-subtle`
- `border-strong`
- `border-focus`
- `border-warning`
- `border-fault`

Avoid excessive borders around every nested element.

Use spacing and surface changes before adding additional visual separators.

## E8. Border Width

Use a small controlled set of border widths.

Recommended semantic levels:

- Default
- Strong
- Focus

Component contracts should reference semantic width roles rather than arbitrary pixel values.

## E9. Radius System

Radius should reinforce component hierarchy.

Recommended semantic levels:

- `radius-sm` for compact controls
- `radius-md` for standard controls and cards
- `radius-lg` for larger containers
- `radius-full` for pills and circular indicators

Avoid mixing many visually unrelated radii.

## E10. Elevation System

Elevation should communicate layering.

Recommended semantic levels:

- `elevation-none`
- `elevation-low`
- `elevation-medium`
- `elevation-high`

Typical usage:

- Low: cards and panels where separation is useful.
- Medium: menus, popovers, floating controls.
- High: dialogs and blocking overlays.

Do not use shadows to indicate status severity.

## E11. Typography System

Typography must optimize for operational scanning.

Semantic roles should include:

- Display
- Page title
- Section title
- Card title
- Body
- Body strong
- Label
- Caption
- Telemetry value
- Telemetry unit
- Table header
- Table value
- Status text

Typography should be defined by semantic role rather than individual page preference.

## E12. Typography Hierarchy

Recommended hierarchy:

1. Page title
2. Major section title
3. Component title
4. Primary operational value
5. Body content
6. Supporting metadata
7. Caption

Do not use large typography for secondary information merely to create visual interest.

## E13. Font Weight

Use a restrained weight scale.

Typical semantic roles:

- Regular for body content.
- Medium for labels and controls.
- Semibold for headings and important values.

Bold should be reserved for strong emphasis where it materially improves scanning.

## E14. Line Height

Line height should support rapid scanning and prevent dense operational content from becoming visually merged.

Use tighter line height for compact labels and values, and more generous line height for explanatory text.

Do not reduce line height merely to fit more content into a viewport.

## E15. Numeric Typography

Telemetry and operational numbers require special treatment.

Rules:

- Keep value and unit visually associated.
- Use consistent decimal precision for the same metric.
- Align comparable numeric values consistently in tables.
- Preserve meaningful negative signs and thresholds.
- Avoid unnecessary leading/trailing zeros.

Where supported, tabular numeric glyphs are preferred for rapidly changing aligned values.

## E16. Spacing System

HYDRAGROW uses an **8pt spacing foundation**.

The base spacing scale should be derived from multiples and useful subdivisions of 8 where necessary.

Semantic roles should include:

- `space-xs`
- `space-sm`
- `space-md`
- `space-lg`
- `space-xl`
- `space-2xl`

Exact values should be centralized in the implementation token source.

## E17. Spacing Rules

Use spacing to communicate hierarchy.

General relationship:

- Related elements use smaller gaps.
- Distinct groups use larger gaps.
- Major sections use the largest consistent gaps.

Do not use arbitrary margins to compensate for inconsistent component structure.

## E18. Layout Grid

Pages should use a consistent layout grid.

The grid should define:

- Content max width
- Page gutters
- Column behavior
- Gap scale
- Panel alignment

Related pages should align major content edges where practical.

## E19. Page Density

HYDRAGROW should support information-dense monitoring without sacrificing readability.

Density should be controlled through:

- Spacing
- Typography
- Grouping
- Information priority
- Responsive reflow

Do not achieve density by shrinking all text and controls.

## E20. Iconography

Icons should reinforce meaning rather than replace essential text.

Rules:

- Use a consistent icon family.
- Maintain consistent visual weight.
- Use semantic icons consistently across the product.
- Do not use unfamiliar icons as the sole label for critical actions.
- Decorative icons should not compete with operational information.

## E21. Icon Semantic Roles

Where applicable, establish canonical icons for:

- Success
- Warning
- Fault
- Unavailable
- Information
- Lock
- Settings
- Navigation
- Refresh
- External/open details
- Expand/collapse
- Command state

Once established, do not substitute arbitrary icons per page.

## E22. Control Dimensions

Interactive controls must use a small, consistent dimension scale.

Semantic control sizes should include:

- Compact
- Standard
- Large

Standard controls should be the default for primary application interaction.

Compact controls should be used only where density is necessary and usability remains acceptable.

## E23. Button Visual Specification

Button hierarchy should include:

- Primary
- Secondary
- Destructive
- Quiet / tertiary where needed

Every button must define:

- Default
- Hover
- Focus
- Active
- Disabled
- Pending

The visual difference between primary and destructive actions must remain clear.

## E24. Input Visual Specification

Inputs should visually communicate:

- Editable
- Focused
- Valid
- Invalid
- Disabled
- Read-only where applicable

Validation state should not depend only on border color.

Error text should appear close to the affected control.

## E25. Focus Treatment

Focus must be visible and consistent across interactive components.

Semantic token:

`focus-ring`

Focus treatment should have sufficient contrast against surrounding surfaces and should not be confused with warning or fault status.

## E26. Disabled vs Unavailable

These states must remain visually distinct.

### Disabled

The control exists but cannot currently be interacted with under its local conditions.

### Unavailable

The underlying resource or capability cannot currently be provided.

The visual system must not use the same treatment for both without additional explanation.

## E27. Pending Visual Specification

Pending represents an unresolved operation.

Pending UI should:

- Show that an action was accepted or initiated.
- Prevent duplicate interaction where necessary.
- Avoid presenting the desired end state as confirmed.
- Remain visually distinct from success.

Use motion sparingly and never rely on animation alone to communicate pending state.

## E28. Fault Visual Specification

Fault is a high-priority operational state.

Fault treatment should use multiple visual signals:

- Semantic color
- Explicit text
- Appropriate icon or marker
- Stronger hierarchy

Fault presentation should remain visible in dense layouts.

## E29. Warning Visual Specification

Warning indicates attention is required but does not necessarily indicate an active fault.

Warning should be visually stronger than normal state and weaker than critical fault.

Avoid using warning treatment for every non-ideal condition, otherwise the warning hierarchy loses meaning.

## E30. Unavailable Visual Specification

Unavailable indicates that the required resource or data cannot currently be accessed.

The UI should expose:

- Unavailable status
- Last known information where useful
- Freshness of that information
- Recovery path when available

Unavailable must not look like healthy stale content.

## E31. Data Freshness

Freshness is a visual property of operational data.

Where freshness matters, provide enough information to distinguish:

- Current
- Recently updated
- Stale
- Unknown freshness

Do not make users infer freshness from a static value alone.

## E32. Tables Visual Specification

Tables should prioritize:

1. Record identity
2. Operational state
3. Important values
4. Timestamp/context
5. Actions

Use consistent row height, column alignment, header treatment, and status components.

Avoid excessive zebra striping or decorative row treatments that compete with operational state.

## E33. Modal and Overlay Visual Specification

Modal layers should clearly separate focused content from the underlying application.

Rules:

- Overlay should reduce background competition.
- Dialog surface should have sufficient contrast from the application surface.
- Destructive confirmations require stronger semantic emphasis.
- Blocking operations should remain clearly blocking.

## E34. Alert Visual Specification

Alerts should have a predictable hierarchy:

1. Critical fault
2. Warning
3. Informational
4. Success feedback

The same semantic level should look consistent whether displayed as a banner, row, card, or pill.

## E35. Motion

Motion is functional, not decorative.

Use animation for:

- Transition feedback
- Loading
- Pending activity
- Expand/collapse
- Layer entry/exit

Avoid:

- Continuous decorative animation
- Excessive bouncing
- Motion that distracts from fault state
- Animation required to understand status

Reduced-motion preferences must be respected.

## E36. Responsive Breakpoints

The implementation should define a small number of semantic layout ranges rather than page-specific breakpoints.

Recommended conceptual ranges:

- Compact/mobile
- Standard/tablet
- Wide/desktop

Exact breakpoint values belong in the implementation token layer.

## E37. Responsive Visual Rules

As width decreases:

1. Preserve critical status.
2. Preserve identity and context.
3. Preserve primary actions.
4. Reflow columns.
5. Reduce secondary metadata.
6. Convert dense tables when necessary.

Do not simply scale the desktop composition down proportionally.

## E38. Accessibility Color Rules

The visual system must support users who cannot reliably distinguish colors.

Therefore:

- Status includes text or another visual signal.
- Error includes explanatory text.
- Focus is independently visible.
- Disabled state is not communicated only by reduced color.
- Charts and telemetry trends should use more than hue when the distinction matters.

## E39. Contrast Rules

Text, controls, focus indicators, and status treatments must maintain sufficient contrast for their intended use.

Contrast should be checked against the actual background token used by the component, not against an assumed default background.

Critical operational information should receive especially strong legibility treatment.

## E40. Visual Hierarchy Rules

Visual hierarchy should follow operational priority.

Strong visual weight may come from:

- Size
- Weight
- Contrast
- Surface
- Position
- Spacing
- Semantic color

Use the smallest combination necessary to establish priority clearly.

Do not make every important item visually loud.

## E41. Empty and Loading Visuals

Empty states should feel intentional rather than broken.

Loading states should preserve structure where practical.

Skeletons may be used when they meaningfully communicate the expected structure, but should not become a substitute for clear state messaging.

## E42. Data Visualization

Telemetry visualizations should prioritize:

- Current value
- Thresholds
- Trend
- Time context
- Data freshness

Charts must not use decorative visual complexity that makes operational interpretation harder.

Thresholds and abnormal ranges should use the same semantic status system as the rest of the product.

## E43. Token Naming

Token names should describe semantic purpose.

Preferred:

```text
color.text.primary
color.surface.default
color.status.warning
spacing.md
radius.md
elevation.low
control.height.standard
```

Avoid names based on visual implementation:

```text
blue-500
gray-200
big-radius
green-button
```

Foundation tokens may use primitive names internally, but product-facing component code should prefer semantic tokens.

## E44. Token Override Rules

Overrides are allowed only when:

- A component has a documented semantic requirement.
- The override is reusable.
- The override does not contradict the global hierarchy.

Page-level arbitrary token overrides should be avoided.

## E45. Theme Consistency

If multiple themes are supported, semantic token names remain stable while their values change.

Components should not contain theme-specific visual logic when semantic tokens can express the difference.

## E46. Visual Consistency Across States

A component should retain its structural identity across states.

For example, a telemetry card should not become an unrelated visual object merely because its value is stale or faulted.

State changes should modify semantic emphasis while preserving recognition of the component.

## E47. Visual Consistency Across Pages

The same semantic object must look consistent across contexts.

Examples:

- An actuator status should use the same state vocabulary and treatment in Overview and Operations.
- A station status should remain recognizable in Fleet and station detail.
- A fault should use the same semantic treatment wherever it appears.

Context may change density or composition, but not the meaning of the visual language.

## E48. Implementation Boundary

This document defines visual intent and semantic token structure.

Implementation-specific concerns belong elsewhere, including:

- CSS architecture
- Framework APIs
- Component library configuration
- Build tooling
- Browser-specific workarounds
- Exact generated class names

Implementation must preserve the semantics defined here.

## E49. Visual QA Checklist

Before accepting a visual implementation, verify:

- Token usage is consistent.
- Typography hierarchy is clear.
- Spacing follows the shared scale.
- Surfaces and borders are coherent.
- Status semantics are consistent.
- Critical states are visually prominent.
- Pending is distinct from success.
- Unavailable is distinct from disabled.
- Focus is visible.
- Color is not the sole status signal.
- Responsive layouts preserve critical information.
- Dense operational content remains readable.
- Motion does not obscure state.

## E50. Visual Definition of Done

The visual system is ready when:

- Foundation tokens are defined.
- Semantic tokens are defined.
- Component tokens have clear ownership.
- Color semantics are stable.
- Typography hierarchy is stable.
- Spacing scale is stable.
- Surface/border/radius/elevation roles are stable.
- Control dimensions are defined.
- Status treatments are defined.
- Responsive rules are defined.
- Accessibility requirements are defined.
- Token naming conventions are defined.
- Visual QA criteria are testable.

## E51. Canonical Visual Rule

When a visual decision is repeated across the product, it should become a semantic token or canonical component rule rather than a page-specific value.

The design system should become more consistent as the product grows, not more fragmented.

## E52. Canonical Visual Formula

Every visual decision should support:

**Meaning, Hierarchy, Consistency, Legibility, State**

If a visual treatment does not improve one of these dimensions, it should be questioned before being added to the system.
