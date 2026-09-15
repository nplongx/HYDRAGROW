# HYDRAGROW — C. COMPONENT CONTRACT

## C0. Purpose

This document defines the contract for reusable HYDRAGROW UI components.

Every component should have an explicit purpose, anatomy, data contract, states, interactions, actions, validation rules, accessibility behavior, responsive behavior, and anti-patterns.

The goal is to prevent page-by-page UI drift and ensure that components behave consistently across Overview, Operations, Cultivation, Journal, Fleet, Settings, Pairing, Backup, Roles, Zero Station, Permission Denied, and Station Unavailable flows.

## C1. Component Contract Principles

1. A component has one primary responsibility.
2. Visual appearance must follow the design tokens.
3. State must be explicit rather than inferred from styling.
4. Physical device state must never be inferred from a user interaction alone.
5. Safety-critical state has priority over convenience state.
6. Components must expose predictable interaction behavior.
7. Components must remain usable when data is loading, stale, unavailable, or in fault.
8. Components must support keyboard and assistive-technology interaction where applicable.
9. Responsive behavior is part of the component contract, not a page-level afterthought.
10. Components should compose without duplicating business logic.

## C2. Component Contract Structure

Each canonical component definition should specify:

- Purpose
- Anatomy
- Data inputs
- Derived state
- Visual states
- Interaction rules
- Events
- Action availability
- Validation
- Accessibility
- Responsive behavior
- Composition rules
- Variants
- Anti-patterns
- Definition of done

## C3. Component Naming

Use semantic names based on function rather than appearance.

Preferred:

- `TelemetryCard`
- `ActuatorCard`
- `CommandButton`
- `StationSelector`
- `AlertBanner`

Avoid:

- `BlueCard`
- `BigButton`
- `GreenStatus`
- `LeftPanel`

Names should remain valid if the visual design changes.

## C4. Component Anatomy

Components should have stable internal regions.

Typical regions include:

- Header
- Primary content
- Supporting metadata
- Status
- Primary action
- Secondary action
- Feedback

Do not introduce arbitrary regions solely to solve one page-specific layout problem.

## C5. Component States

Canonical component states are:

- Default
- Loading
- Pending
- Active
- Disabled
- Unavailable
- Warning
- Fault
- Locked
- Permission denied
- Confirmed

Not every component requires every state. A component should implement only states meaningful to its responsibility.

## C6. Component State Priority

When multiple states apply, render the highest-priority state:

1. Safety-critical
2. Fault
3. Unavailable
4. Permission denied / Locked
5. Pending
6. Warning
7. Active
8. Default

The component must not hide a higher-priority state because a lower-priority state is more visually convenient.

## C7. Panel

### Purpose

Group related information or controls into a stable visual region.

### Anatomy

- Optional title
- Optional description
- Content
- Optional actions

### Contract

- Must use surface, border, radius, spacing, and elevation tokens.
- Header hierarchy must be visually distinct from body content.
- Actions belong in the header only when they apply to the entire panel.
- Panel content must remain readable when actions are absent.

### Anti-patterns

- Nested panels without information-architecture justification.
- Using a panel only as decorative framing.
- Mixing unrelated domains in one panel.

## C8. Button

### Purpose

Trigger a non-persistent UI action.

### Contract

- Has a clear action label.
- Has default, hover, focus, active, disabled, and pending behavior where applicable.
- Pending state prevents accidental duplicate activation.
- Destructive actions use the appropriate semantic treatment.
- Button text describes the action, not the implementation.

### Accessibility

- Must have an accessible name.
- Focus state must remain visible.
- Disabled and pending states must be distinguishable.

## C9. Status Pill

### Purpose

Represent a concise semantic state.

### Contract

- Uses canonical status vocabulary.
- Uses semantic color tokens rather than arbitrary colors.
- Text remains understandable without color.
- One pill represents one primary state.

### Anti-patterns

- Using decorative labels as status.
- Creating synonyms for canonical statuses.
- Encoding severity only through color.

## C10. Telemetry Card

### Purpose

Display one important telemetry value and its state.

### Anatomy

- Metric label
- Current value
- Unit
- Optional trend or secondary value
- Freshness indicator
- State indicator

### Contract

- Current value must be visually dominant.
- Unit must remain adjacent to the value.
- Stale or unavailable telemetry must be explicit.
- Timestamp/freshness must be available where operationally relevant.
- Formatting must be consistent for the same metric type.

### Safety

Never present stale telemetry as current telemetry.

## C11. Telemetry Group

### Purpose

Organize related telemetry without turning the page into an undifferentiated metric grid.

### Contract

- Group metrics by operational meaning.
- Preserve consistent card hierarchy.
- Avoid excessive metric density.
- Critical metrics should remain visible at the appropriate page priority.

## C12. Actuator Card

### Purpose

Represent a controllable physical actuator together with its actual state.

### Anatomy

- Actuator name
- Physical state
- Command state
- Relevant telemetry
- Primary control
- Optional fault or interlock information

### Contract

- Physical state is authoritative for the device.
- Command state describes the lifecycle of a requested action.
- A pressed button does not mean the actuator physically changed state.
- Pending, fault, unavailable, and interlocked conditions must be explicit.

## C13. Command Button

### Purpose

Initiate a device command.

### Contract

- Action must be explicit.
- Current physical state must be visible nearby.
- Pending state must be represented after submission.
- Duplicate submission must be prevented.
- Success requires confirmation from the authoritative device state where applicable.
- Failure must expose a recoverable error state.

## C14. Confirmation Dialog

### Purpose

Confirm actions with meaningful risk, irreversibility, or physical consequence.

### Contract

- State what will happen.
- Identify the affected station/device when relevant.
- Explain meaningful consequences for destructive or safety-sensitive actions.
- Primary action must describe the actual action.
- Cancel must remain easy to find.

### Anti-patterns

- Generic “Are you sure?” dialogs.
- Confirmation for routine, low-risk actions.
- Hiding the actual consequence behind vague wording.

## C15. Alert Banner

### Purpose

Communicate an important condition requiring awareness or action.

### Contract

- Severity is explicit.
- Message explains the condition.
- Action is included when remediation is possible.
- Critical alerts must not be visually confused with informational feedback.
- Dismissibility follows alert semantics and safety requirements.

## C16. Alert List / Alert Row

### Purpose

Display multiple operational alerts in a scannable structure.

### Contract

Each row should expose, where available:

- Severity
- Station/device
- Condition
- Timestamp
- Current status
- Available action

Rows should remain sortable/filterable where the surrounding workflow requires it.

## C17. Table

### Purpose

Display structured collections of records for comparison, scanning, and operational review.

### Contract

- Column meaning must be clear.
- Headers remain associated with values.
- Numeric values align consistently.
- Status values use canonical status components.
- Empty, loading, unavailable, and error states are explicit.
- Row actions must not visually compete with primary record data.

### Responsive behavior

Tables may switch to a compact record representation when horizontal space is insufficient. Do not simply allow critical columns to become unreadable.

## C18. Modal

### Purpose

Temporarily focus the user on a bounded task or information set.

### Contract

- Focus moves into the modal when opened.
- Focus returns to the triggering element when closed where appropriate.
- Escape behavior follows the risk level of the interaction.
- Modal title describes its purpose.
- Background interaction is blocked when required.

## C19. Form Field

### Purpose

Collect one logically bounded input.

### Anatomy

- Label
- Control
- Optional description
- Validation message
- Optional unit or contextual metadata

### Contract

- Label must remain associated with the control.
- Validation must be specific and actionable.
- Do not rely on placeholder text as the label.
- Errors should identify what needs to change.

## C20. Numeric Input

### Purpose

Collect bounded numeric configuration values.

### Contract

- Unit is explicit.
- Valid range is defined.
- Step/increment is defined where applicable.
- Invalid values must not silently become valid values through implicit coercion.
- Boundary values require deliberate handling.

## C21. Select / Segmented Control

### Purpose

Allow selection from a finite set of mutually exclusive options.

### Contract

- Options use stable semantic labels.
- Current selection is explicit.
- Selection changes must not be confused with command execution unless the control is explicitly defined as a command.
- Use segmented controls only for small, high-frequency option sets.

## C22. Tabs

### Purpose

Switch between closely related views within the same context.

### Contract

- Tabs represent sibling views, not unrelated destinations.
- One tab is active.
- Active state is visible without relying only on color.
- Keyboard navigation follows the expected tab interaction model.

## C23. Navigation Item

### Purpose

Navigate between application-level destinations.

### Contract

- Label represents destination semantics.
- Current destination is explicit.
- Permission restrictions are reflected consistently.
- Navigation should not be disabled merely because destination data is temporarily unavailable.

## C24. Station Selector

### Purpose

Establish the active station context.

### Contract

- Current station is always visible when station context matters.
- Selection changes page data and controls consistently.
- Station identity must not depend only on color or position.
- Unavailable stations remain distinguishable from stations that simply have no alerts.

## C25. Station Card

### Purpose

Summarize a station for Fleet or overview contexts.

### Contract

Should expose the minimum useful operational summary:

- Station identity
- Overall state
- Key telemetry
- Alert summary
- Availability
- Relevant action

Do not duplicate the entire station detail page inside the card.

## C26. Empty State

### Purpose

Explain that valid content currently has nothing to display.

### Contract

- State what is empty.
- Explain why when useful.
- Provide a next action when one exists.
- Do not represent an error as an empty state.

## C27. Loading State

### Purpose

Communicate that content is being retrieved or prepared.

### Contract

- Preserve known context where possible.
- Avoid unnecessary full-page blocking.
- Do not display fabricated or stale-looking values as loading content.
- Loading indicators should match the scope of the operation.

## C28. Unavailable State

### Purpose

Communicate that required data or capability cannot currently be accessed.

### Contract

- Explain that the resource is unavailable.
- Distinguish unavailable from empty.
- Distinguish unavailable from fault when the cause is known.
- Provide recovery guidance when possible.

## C29. Permission-Denied State

### Purpose

Communicate that the user cannot perform or access a requested operation.

### Contract

- Explain the restriction without exposing sensitive authorization details.
- Do not present the action as available if the user cannot execute it.
- Provide an alternative path when appropriate.

## C30. Pending State

### Purpose

Represent an action that has been submitted but is not yet confirmed.

### Contract

- The pending action must be identifiable.
- Duplicate submission is prevented.
- The UI must not falsely represent the requested end state as confirmed.
- Timeout and failure behavior must be defined by the state-transition contract.

## C31. Success / Confirmation State

### Purpose

Communicate that an operation has completed successfully.

### Contract

- Success should correspond to authoritative confirmation where required.
- Feedback should identify the completed action.
- Temporary success feedback must not obscure more important operational state.

## C32. Error State

### Purpose

Communicate that an operation failed or that required data could not be processed.

### Contract

- Explain the failure in user-understandable language.
- Preserve useful context.
- Provide retry or recovery when possible.
- Do not silently discard failed commands.

## C33. Component Events

Components should expose semantic events rather than implementation details.

Preferred:

- `onSelectStation`
- `onSubmitCommand`
- `onRetry`
- `onDismiss`
- `onOpenDetails`

Avoid leaking low-level DOM or transport behavior into page-level consumers.

## C34. Controlled vs Derived State

### Controlled state

State that is owned by the page, application, or domain layer and passed into the component.

Examples:

- Current station
- Physical actuator state
- Permission state
- Telemetry freshness

### Derived state

State computed from authoritative inputs.

Examples:

- Whether a command can be submitted
- Display severity
- Whether a telemetry value is stale

Components should not create a second authoritative copy of domain state.

## C35. Component Data Ownership

The domain/application layer owns authoritative data.

The component owns presentation concerns such as:

- Layout
- Local focus state
- Local open/closed state for bounded UI
- Temporary visual interaction state

Do not place station/device truth inside presentation components.

## C36. Action Availability

Action availability is determined by:

- Current physical state
- Command lifecycle
- Safety interlocks
- Permission
- Availability
- Required configuration

The component must render the resulting action state consistently.

An action that cannot safely execute should not appear executable.

## C37. Destructive Actions

Destructive actions require:

- Explicit semantic labeling
- Appropriate visual severity
- Confirmation when the consequence warrants it
- Clear failure feedback
- Auditability where required by the operation

Do not use destructive styling for ordinary actions merely to attract attention.

## C38. Component Accessibility

Every interactive component must support:

- Keyboard access
- Visible focus
- Accessible name
- Logical reading order
- Sufficient text/background contrast
- Non-color status communication
- Appropriate semantic roles

Dynamic state changes should be announced when they materially affect the user's task.

## C39. Responsive Contract

Components must define behavior across available widths.

Preferred adaptation order:

1. Preserve critical information.
2. Reduce secondary metadata.
3. Reflow content.
4. Stack controls.
5. Replace dense representations when necessary.

Do not solve responsive layouts by arbitrarily shrinking critical controls or text below usable sizes.

## C40. Component Composition

Components should compose through explicit semantic boundaries.

Example:

`StationCard` may contain:

- `StatusPill`
- `TelemetryGroup`
- `AlertSummary`
- `CommandButton`

The parent owns composition and context. Child components own their local presentation contracts.

## C41. Component Variants

Variants should represent meaningful semantic differences.

Good variants:

- `Button: primary / secondary / destructive`
- `StatusPill: normal / warning / fault / unavailable`
- `TelemetryCard: compact / standard`

Avoid variants that exist only for one page's pixel-level layout.

## C42. Component Tokens

Components must consume shared tokens for:

- Color
- Typography
- Spacing
- Radius
- Border
- Elevation
- Control dimensions
- Focus treatment

Hard-coded visual values should be avoided when an equivalent design token exists.

## C43. Component State and Semantic Status

Component state and domain status are related but not identical.

For example:

- A `CommandButton` can be in `pending` state while the actuator remains physically `OFF`.
- A `TelemetryCard` can be in `warning` state because the measured value crosses a threshold.
- A `StationCard` can be `unavailable` while its last known telemetry remains available as historical context.

Components must preserve this distinction.

## C44. Error Boundaries

Failure in one component should not unnecessarily destroy unrelated operational context.

Where practical:

- Isolate component failures.
- Preserve unaffected telemetry and navigation.
- Provide localized recovery.
- Avoid replacing the entire page with a generic error when only one region failed.

## C45. Refresh Behavior

Refreshing data must preserve user context where possible.

- Preserve selected station.
- Preserve active tab.
- Preserve filters when the data model allows it.
- Indicate refresh/loading state without causing unnecessary layout shift.
- Never overwrite newer user input with an older response.

## C46. Transition Handling

Components that represent stateful operations must follow the canonical state-transition rules.

The component should:

1. Render the authoritative current state.
2. Render pending command state separately when applicable.
3. Prevent invalid actions.
4. Show feedback for accepted, rejected, failed, or timed-out operations.
5. Update from authoritative confirmation rather than assuming success.

## C47. Component Contract Example

For an actuator control:

```text
ActuatorCard
  identity: valve name
  physicalState: OFF | ON | FAULT | UNKNOWN
  commandState: IDLE | PENDING | SUCCESS | FAILED | TIMEOUT
  permission: allowed | denied
  availability: available | unavailable
  interlock: clear | blocked
  action: command ON/OFF
```

The UI must keep `physicalState` and `commandState` separate.

Example behavior:

- User requests ON.
- Command becomes PENDING.
- Physical state remains OFF until authoritative telemetry confirms ON.
- If confirmation arrives, command becomes SUCCESS and physical state becomes ON.
- If the command fails or times out, command becomes FAILED or TIMEOUT while physical state remains authoritative.

## C48. Component Testing Contract

Every canonical component should be tested for:

- Default rendering
- All implemented semantic states
- Disabled behavior
- Pending behavior
- Error behavior
- Empty/unavailable behavior where applicable
- Keyboard interaction
- Accessibility semantics
- Responsive behavior
- Long labels and values
- Missing optional data
- Repeated interaction / duplicate submission
- State transition correctness

For physical controls, test that UI intent cannot be mistaken for physical confirmation.

## C49. Component Anti-Patterns

Avoid:

- Page-specific copies of canonical components.
- Components with hidden business rules.
- Components that infer physical state from clicks.
- Components that encode status through color only.
- Generic error messages that provide no recovery path.
- Arbitrary one-off variants.
- Excessive nesting.
- Hidden disabled states with no explanation when explanation is needed.
- Loading states that erase useful context.
- Responsive behavior that hides critical operational information.

## C50. Component Definition of Done

A component is canonical only when:

- Purpose is documented.
- Anatomy is stable.
- Inputs are defined.
- States are defined.
- State priority is defined where relevant.
- Interactions are defined.
- Events are semantic.
- Accessibility is addressed.
- Responsive behavior is defined.
- Variants are justified.
- Anti-patterns are documented.
- Tests cover meaningful state transitions.

## C51. Canonical Component Rule

If a new page needs a component that already exists, reuse the canonical component before creating a new one.

If the existing component cannot express the required behavior, extend its contract only when the behavior is reusable across the product.

Do not fork a canonical component for a single screen.

## C52. Canonical Component Formula

Every interactive operational component should be designed around:

**Context, Data, State, Guard, Action, Feedback, Confirmation**

The component is complete only when these dimensions are coherent.
