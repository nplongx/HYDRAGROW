# HYDRAGROW — A. UX RULES

## A0. Core Product Principle

HYDRAGROW is an operational control interface for a physical growing system.

The UI must always optimize for:

1. **State clarity** — users can immediately understand what the system is doing.
2. **Operational safety** — dangerous or irreversible actions are explicit and deliberate.
3. **Low cognitive load** — important information is visible without requiring interpretation.
4. **Consistency** — the same state, action, and concept must look and behave the same everywhere.
5. **Traceability** — important operational changes must be explainable after the fact.

The interface should feel like a calm industrial instrument, not a generic dashboard.

## A1. Global UX Principles

### A1.1 State before action

Always show the current system state before presenting controls that can change it.

### A1.2 Context before detail

The user must know which station, zone, crop, or device they are operating before acting on it.

### A1.3 One canonical meaning

A semantic status has one canonical meaning and one canonical visual treatment across the application.

### A1.4 Progressive disclosure

Show operationally important information first. Put diagnostics, historical data, and advanced configuration behind secondary surfaces when appropriate.

### A1.5 Physical reality wins

The UI must never imply that a physical action succeeded merely because a command was sent.

Command state and physical state are distinct.

## A2. Application Shell

The global shell should provide:

- persistent navigation
- current station/system context
- global connection/availability state
- user identity and role
- access to critical system controls where applicable

Navigation must remain stable across pages.

Do not move primary navigation based on page-specific needs.

## A3. Context Rules

Every operational screen must establish:

- current station
- current zone or relevant scope
- current operating mode when applicable
- connection/availability state

Context must not depend solely on color.

If a page can affect multiple stations, the selected target must be explicit before a command is issued.

## A4. Page Hierarchy

Every page should follow a predictable hierarchy:

1. Page identity
2. Current context
3. Primary operational state
4. Primary actions
5. Supporting information
6. Diagnostics/history/configuration

Do not place low-priority metrics above the primary system state.

## A5. Visual Hierarchy Rules

Use visual hierarchy consistently:

- **Primary:** current operational state and safety-critical information
- **Secondary:** actions required for normal operation
- **Tertiary:** supporting telemetry and context
- **Quaternary:** historical/diagnostic information

Visual emphasis must reflect operational importance, not data density.

## A6. Semantic Status Rules

The application uses canonical semantic statuses. The same status must retain the same meaning across:

- cards
- tables
- badges
- controls
- alerts
- detail pages

Do not create page-specific synonyms for canonical states.

## A7. Status Vocabulary Rules

Use short, concrete status labels.

Prefer:

- Online
- Offline
- Running
- Stopped
- Warning
- Fault
- Pending
- Disabled
- Locked

Avoid vague labels such as “Something happened”, “Issue”, or “Attention needed” when a more precise state is available.

## A8. Telemetry Rules

Telemetry must distinguish between:

- current value
- expected/target value
- unit
- timestamp/freshness
- quality/state

Never display a numeric telemetry value without its unit when the unit is not globally obvious.

Freshness must be visible when stale data could influence an operational decision.

## A9. Telemetry Safety Rules

Stale, unavailable, or invalid telemetry must not visually resemble valid live telemetry.

Use explicit states for:

- unavailable
- stale
- invalid
- disconnected

Do not silently replace missing values with zero.

## A10. Actuator / Physical Control Rules

Controls that affect physical equipment must make the target and resulting action clear.

For every physical action, the user should understand:

- what device is affected
- what action will occur
- whether the action is immediate or scheduled
- whether confirmation is required
- what state is expected afterward

Do not hide consequential actions behind ambiguous labels such as “Apply”.

## A11. Command Lifecycle

Commands must be represented as a lifecycle rather than an instantaneous UI event:

**Idle / Ready → Commanding → Confirmed / Failed / Timed out**

The UI must distinguish “command accepted” from “physical state confirmed”.

Pending commands must remain visibly pending until the system reports a meaningful result or timeout.

## A12. E-STOP Rules

Emergency stop is a safety mechanism, not a normal navigation action.

E-STOP must:

- be visually distinct
- be immediately understandable
- avoid accidental activation
- provide clear post-activation state
- remain consistent across the application

The UI must never imply that an E-STOP has been released simply because the user navigated away from the screen.

## A13. Critical State Rules

Critical states must interrupt normal visual hierarchy when necessary.

Examples include:

- emergency stop active
- critical equipment fault
- unsafe environmental condition
- loss of control/communication where operation cannot be trusted

Critical information must be visible without requiring the user to inspect secondary panels.

## A14. Warning / Alert Rules

Warnings should communicate:

1. What is wrong or abnormal
2. Why it matters
3. What the user can do next

Do not create alerts that provide no actionable information unless the alert is purely informational.

Alerts should not become visually louder than the actual operational risk.

## A15. Permission Rules

Permission is a product state, not merely a disabled button.

When an action is unavailable because of permissions:

- communicate that the action exists when appropriate
- communicate why it is unavailable when safe to disclose
- never imply a technical failure when the actual reason is authorization

Destructive or safety-sensitive operations should require the appropriate role.

## A16. Forms & Configuration

Configuration forms must:

- use clear labels
- show units
- show valid ranges where relevant
- preserve entered values when validation fails
- identify validation errors close to their source
- make save/apply behavior explicit

Separate configuration state from live operational state.

Changing a configuration value must not visually imply that the physical system has already adopted it unless confirmation exists.

## A17. Cultivation UX Rules

Cultivation views should prioritize the growing process over raw device data.

Show relationships between:

- crop/recipe
- growth stage
- environmental targets
- actual telemetry
- active interventions

Do not require users to reconstruct cultivation context from unrelated device screens.

## A18. Fleet UX Rules

Fleet views must optimize for comparison and exception detection.

Users should be able to identify:

- healthy stations
- unavailable stations
- stations requiring attention
- stations with active faults

Avoid displaying every metric at equal visual weight in fleet overview screens.

## A19. Journal / Audit Rules

Operational history should answer:

- what happened
- when it happened
- where it happened
- who initiated it, when applicable
- what the system reported afterward

Audit entries should be immutable from the normal operational UI.

## A20. Empty-State Rules

Empty states must explain why content is absent and what the user can do next.

Examples:

- No stations configured
- No active alerts
- No journal entries
- No telemetry available

Avoid empty screens that only say “No data”.

## A21. Offline / Unavailable Rules

Offline and unavailable states must be explicit.

The UI must not make unavailable data look current.

When communication is lost:

- preserve the last known value only if it is clearly marked as stale/last known
- expose connection state
- prevent unsafe assumptions about physical state

## A22. Loading / Pending Rules

Loading states should preserve layout stability.

Prefer skeletons or reserved space for predictable content rather than large layout shifts.

Pending operations must have a distinct visual state from loading unrelated page content.

## A23. Feedback Rules

Every meaningful user action should produce proportional feedback.

Examples:

- lightweight confirmation for safe local actions
- visible pending state for asynchronous commands
- persistent alert for unresolved critical failures

Do not use success feedback when only request submission has been confirmed.

## A24. Tables

Tables are for comparison and scanning.

Rules:

- keep column meanings stable
- align numeric values consistently
- avoid excessive decorative columns
- expose status clearly
- preserve row identity during interaction
- use sorting/filtering only when it improves operational discovery

Important state must not be hidden only in hover interactions.

## A25. Responsive Rules

The interface must remain operationally usable across supported viewport sizes.

Priority under constrained width:

1. Safety state
2. Current context
3. Primary action
4. Core telemetry
5. Secondary information

Do not simply compress every desktop component into a smaller version. Reflow or collapse secondary information when necessary.

## A26. Accessibility Rules

Accessibility is part of operational reliability.

Requirements:

- never communicate status through color alone
- maintain readable contrast
- support keyboard navigation where applicable
- provide visible focus states
- use meaningful labels for controls
- preserve logical reading/order flow
- ensure critical alerts are perceivable without relying on animation

## A27. Animation Rules

Animation should communicate state change, not decorate the interface.

Use motion for:

- transitions between states
- pending activity
- confirmation of meaningful changes

Avoid continuous motion unless it represents a live physical process or requires user attention.

## A28. Copy Rules

UI copy should be:

- concise
- concrete
- operational
- consistent

Prefer verbs that describe the actual action:

- Start
- Stop
- Pause
- Resume
- Save
- Apply
- Reset
- Acknowledge

Avoid ambiguous labels such as “Go”, “Do it”, or “Continue” when the consequence is not obvious.

## A29. Design Consistency Rules

The same concept must not acquire different UI patterns on different pages without a documented reason.

Consistency applies to:

- terminology
- status colors
- typography
- spacing
- component shape
- interaction behavior
- confirmation behavior
- error handling

When a new pattern is introduced, first determine whether an existing pattern can solve the problem.

## A30. Priority Rules for Conflicting Information

When multiple signals compete for attention, prioritize:

1. Immediate safety risk
2. Loss of control or trustworthy state
3. Active equipment fault
4. Operational intervention required
5. Warning conditions
6. Normal operating state
7. Historical and diagnostic detail

The most visually prominent information should follow this priority.

## A31. Anti-Patterns

Avoid:

- dashboard-as-decoration
- excessive cards with no hierarchy
- status communicated only by color
- fake real-time values
- optimistic physical-state updates
- ambiguous destructive actions
- hidden critical alerts
- inconsistent status vocabulary
- page-specific interaction conventions
- disabling controls without explaining why
- showing stale data as live
- confirmation dialogs for trivial actions
- confirmation dialogs that fail to explain consequential actions

## A32. Canonical Decision Rule

When a design decision is ambiguous, choose the solution that best answers these questions in order:

1. Is the physical system state clear?
2. Is the user's target/context clear?
3. Is the next action clear?
4. Is the action safe and appropriately deliberate?
5. Can the result be verified?
6. Can the event be understood later from the journal/audit trail?

If a design cannot answer these questions, it is not operationally complete.

## A33. Canonical UX Formula

HYDRAGROW UX should consistently follow:

**Context → State → Risk → Action → Result → Trace**

This sequence is the canonical UX grammar for operational flows.

