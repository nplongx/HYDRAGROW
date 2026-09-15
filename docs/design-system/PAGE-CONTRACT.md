# HYDRAGROW — D. PAGE CONTRACT

## D0. Purpose

This document defines the contract for HYDRAGROW application pages.

The purpose is to ensure that every page has a predictable information hierarchy, operational context, data ownership, state representation, action model, responsive behavior, and navigation relationship.

A page is not only a visual layout. It is a defined operational context composed from canonical components and governed by the UX and state-transition rules.

## D1. Page Contract Principles

Every page must define:

- Purpose
- Primary user goal
- Required context
- Information hierarchy
- Primary data
- Secondary data
- Page-level states
- Primary action
- Secondary actions
- Permissions
- Safety constraints
- Navigation relationship
- Responsive behavior
- Loading behavior
- Empty behavior
- Error behavior
- Unavailable behavior
- Acceptance criteria

Pages must compose canonical components rather than implement independent versions of them.

## D2. Page Anatomy

A canonical page should follow this general structure:

```text
Application Shell
  Page Header
    Page Title
    Context
    Primary Action
  Primary Content
    Critical Information
    Operational Information
    Supporting Information
  Secondary Content
  Feedback / Alerts
```

Not every page requires every region.

The page should remain visually understandable when optional regions are absent.

## D3. Page Header Contract

The page header should communicate:

- Where the user is.
- What the page represents.
- Which context is active.
- What the most important available action is.

### Header rules

- One clear page title.
- Context appears close to the title when relevant.
- Primary action should be visually identifiable.
- Do not overload the header with secondary controls.
- Critical state should not be hidden below the fold.

## D4. Page Context Contract

Pages must explicitly establish the context required to interpret their content.

Possible context includes:

- Current station
- Current cultivation
- Current date/range
- Fleet scope
- User role
- Configuration scope

If a page depends on station context, the active station must be identifiable.

If no valid context exists, the page must render an appropriate empty, unavailable, or selection state rather than silently showing unrelated data.

## D5. Information Hierarchy

Page content follows this priority:

1. Safety-critical information
2. Active faults
3. Availability problems
4. Operational warnings
5. Primary operational state
6. Critical telemetry
7. Primary actions
8. Secondary telemetry
9. Historical/supporting information

Do not give secondary information stronger visual hierarchy than an active operational problem.

## D6. Page-Level States

Every page must explicitly account for applicable states:

- Loading
- Ready
- Empty
- Warning
- Fault
- Unavailable
- Permission denied
- Partial data
- Offline
- Error

A page may combine multiple component states.

Page state must not obscure more important component-level operational states.

## D7. Page Loading Contract

Loading behavior should preserve known context whenever possible.

Preferred behavior:

- Keep page identity visible.
- Keep station context visible.
- Preserve stable navigation.
- Load independent sections independently where practical.
- Avoid unnecessary full-page blocking.

Do not display fabricated values while data is loading.

## D8. Page Error Contract

A page-level error should communicate:

- What failed.
- What remains available.
- Whether retry is possible.
- What the user can do next.

A localized data failure should remain localized when possible.

For example, failure to load one telemetry group should not automatically erase navigation, station identity, or unrelated operational information.

## D9. Page Unavailable Contract

Unavailable means the page or required resource cannot currently be accessed.

The page must distinguish:

- No data
- Loading
- Unavailable
- Permission denied
- Fault

The distinction should be visible to the user.

## D10. Overview Page

### Purpose

Provide the fastest operational understanding of the current HYDRAGROW environment.

### Primary Goal

Answer:

- What is happening?
- Is anything wrong?
- Which station needs attention?
- What requires action now?

### Required Context

Depending on product configuration:

- Fleet or station scope
- Current operational period
- Current overall system status

### Primary Content

Recommended hierarchy:

1. Overall system status
2. Critical alerts
3. Station status summary
4. Critical telemetry
5. Active actuator conditions
6. Recent operational events

### Primary Actions

Actions should lead to the operational destination where remediation occurs.

Examples:

- Open affected station
- Open active alert
- Open Operations
- Open Cultivation

### State Rules

If critical faults exist, they must receive higher priority than normal telemetry.

If no alerts exist, the page should communicate a healthy/normal state rather than leaving the alert region visually ambiguous.

### Acceptance Criteria

- User can understand overall status quickly.
- Critical faults are immediately visible.
- Station context is clear.
- Primary remediation path is obvious.
- No critical state is represented only through color.

## D11. Operations Page

### Purpose

Provide direct operational visibility and control over physical station equipment.

### Primary Goal

Understand current physical state and safely execute required commands.

### Required Context

- Active station
- Device/actuator scope
- Current physical state
- Relevant telemetry
- Permission state
- Safety interlocks

### Primary Content

Recommended hierarchy:

1. Station identity and availability
2. Safety-critical conditions
3. Active faults/interlocks
4. Actuator state
5. Relevant telemetry
6. Controls
7. Supporting information

### Control Rules

Physical state must be visually distinct from command state.

A control interaction means that a command was requested.

It does not mean that the physical device has completed the requested transition.

### Command Lifecycle

The page must support:

- Available
- Submitted
- Pending
- Confirmed
- Failed
- Timed out
- Rejected
- Superseded where applicable

### Safety

Safety interlocks override ordinary command availability.

An unavailable or faulted actuator must not appear normally executable.

### Acceptance Criteria

- Physical state is authoritative.
- Command state is separate.
- Invalid commands cannot be submitted.
- Pending commands cannot be accidentally duplicated.
- Confirmation reflects authoritative device state.
- Fault and interlock states are obvious.

## D12. Cultivation Page

### Purpose

Provide context for cultivation operations and their relationship to station conditions.

### Primary Goal

Understand current cultivation status, relevant conditions, and required operational intervention.

### Required Context

- Cultivation context
- Relevant station(s)
- Current operational period
- Relevant telemetry
- Active alerts

### Primary Content

Recommended hierarchy:

1. Cultivation status
2. Critical conditions
3. Relevant telemetry
4. Active alerts
5. Operational controls where applicable
6. Historical/contextual information

### Rules

Cultivation information must remain connected to the station/device context that produces or affects it.

Avoid presenting cultivation targets as if they were actual telemetry.

Targets, configured values, and measured values should remain distinguishable.

### Acceptance Criteria

- Target and actual values are distinguishable.
- Relevant station context is visible.
- Critical cultivation conditions are prioritized.
- Operational intervention has a clear path.

## D13. Journal Page

### Purpose

Provide an auditable record of operational activity and relevant events.

### Primary Goal

Understand what happened, when it happened, and in what context.

### Primary Content

Records may include:

- Commands
- State changes
- Alerts
- Faults
- Configuration changes
- User actions
- System events

### Record Contract

Where applicable, each record should expose:

- Timestamp
- Event type
- Station/device
- Actor
- Result
- Relevant context

### Rules

Journal records are historical evidence.

Do not visually present historical state as current state.

Current status should be obtained from the current authoritative state.

### Acceptance Criteria

- Events are chronologically understandable.
- Actor and station context are visible where applicable.
- Historical records are not confused with current state.
- Filtering and search remain understandable.

## D14. Fleet Page

### Purpose

Provide a cross-station operational overview.

### Primary Goal

Identify which stations are healthy, degraded, unavailable, or require intervention.

### Primary Content

Recommended hierarchy:

1. Fleet-level summary
2. Critical stations
3. Warning stations
4. Normal stations
5. Station telemetry summary
6. Station actions

### Station Card Contract

Each station summary should expose enough information to decide whether to open it.

At minimum:

- Station identity
- Overall state
- Key telemetry
- Alert summary
- Availability

### Rules

Fleet view should optimize for comparison and triage.

Do not expose every detail available on the station detail page.

### Acceptance Criteria

- Problematic stations can be identified quickly.
- Station states are comparable.
- Unavailable stations are distinct from healthy stations.
- Opening a station preserves the correct station context.

## D15. Settings Page

### Purpose

Manage configuration that affects application or station behavior.

### Primary Goal

Change configuration safely and understand its resulting scope.

### Required Context

Depending on setting:

- User scope
- Station scope
- System scope
- Configuration category

### Form Rules

Every editable field should define:

- Current value
- Valid range/options
- Unit where applicable
- Description where necessary
- Validation
- Save behavior
- Failure behavior

### Save Behavior

Do not imply that a configuration change has succeeded until the authoritative system confirms it.

For settings with physical consequences, the page should make the consequence clear before submission.

### Acceptance Criteria

- Scope is clear.
- Invalid values are rejected clearly.
- Save/pending/success/failure states are distinct.
- Configuration changes are auditable where required.

## D16. Pairing Page

### Purpose

Connect or associate a station/device with HYDRAGROW.

### Primary Goal

Complete pairing while minimizing ambiguity and accidental association.

### Primary Content

Recommended sequence:

1. Pairing status
2. Device identity
3. Verification information
4. Pairing action
5. Result
6. Recovery guidance

### States

- Not paired
- Discovering
- Pairing
- Paired
- Failed
- Timeout
- Unavailable

### Rules

The page must never represent a device as paired merely because the user initiated the pairing process.

Authoritative confirmation is required.

## D17. Backup Page

### Purpose

Manage backup and restoration of supported system data.

### Primary Goal

Understand backup state and safely initiate backup/restore operations.

### Contract

The page should distinguish:

- Last successful backup
- Backup in progress
- Backup failed
- Backup unavailable
- Restore available
- Restore in progress
- Restore completed
- Restore failed

### Safety

Restore operations that can overwrite existing data require explicit confirmation.

Consequences must be understandable before execution.

## D18. Roles Page

### Purpose

Manage user roles and permissions.

### Primary Goal

Understand who can perform which operations.

### Contract

The page should clearly distinguish:

- User identity
- Assigned role
- Permission set
- Editable permissions
- Restricted permissions

### Rules

Permission state must be consistent with action availability elsewhere in the application.

A user should not see a normal executable control for an operation they are explicitly forbidden to perform.

## D19. Zero Station Page

### Purpose

Initialize or reset a station according to the supported HYDRAGROW workflow.

### Primary Goal

Safely perform station initialization/reset while making its consequences explicit.

### Safety

This page is potentially destructive and operationally sensitive.

Required behavior:

- Clear station identity
- Clear operation scope
- Explain consequences
- Require deliberate confirmation where applicable
- Show pending state
- Require authoritative completion
- Show failure/recovery state

Never use a generic confirmation message for this operation.

## D20. Permission Denied Page

### Purpose

Explain that the requested resource or operation cannot be accessed with the current permissions.

### Contract

The page should provide:

- Clear explanation
- Requested resource/context
- Safe navigation path
- Alternative action when available

Do not expose sensitive authorization internals.

## D21. Station Unavailable Page

### Purpose

Represent a station that cannot currently provide the required operational service or data.

### Contract

The page should communicate:

- Station identity
- Availability state
- Last known information when appropriate
- Freshness of that information
- Possible reason when known
- Recovery path

### Rules

Last-known data must never be presented as current data.

If historical information is displayed, it must be clearly identified as such.

## D22. Page Navigation Contract

Navigation should preserve operational context where appropriate.

Examples:

- Overview to station detail preserves station identity.
- Fleet to station detail preserves selected station.
- Alert to affected station opens the relevant operational context.
- Operations to Journal preserves station/device filtering when useful.

Navigation should not unexpectedly reset important user context.

## D23. Page Permission Contract

Page access and action access are separate concerns.

A user may:

- Access a page but not execute a command.
- View telemetry but not change configuration.
- View journal records but not modify them.

The page must represent these distinctions consistently.

## D24. Page Safety Contract

Safety-critical pages must prioritize:

1. Current physical condition
2. Fault/interlock state
3. Availability
4. Command state
5. User action
6. Confirmation

Do not place the primary action above critical information when doing so could cause the user to act without understanding the current physical condition.

## D25. Page Responsive Contract

At smaller widths:

1. Preserve critical state.
2. Preserve station/device identity.
3. Preserve primary actions.
4. Stack dense content.
5. Reduce secondary information.
6. Replace dense tables with readable records where required.

Never hide safety-critical status merely to preserve a desktop layout.

## D26. Page Accessibility Contract

Every page must provide:

- Logical heading hierarchy
- Keyboard navigation
- Visible focus
- Accessible names for controls
- Non-color status communication
- Logical reading order
- Sufficient contrast
- Meaningful error messages

Dynamic operational changes should be announced when they materially affect the user's task.

## D27. Page Data Ownership

Pages consume authoritative application/domain data.

Pages may derive:

- Display grouping
- Sorting
- Filtering
- Presentation severity
- Action availability

Pages must not independently redefine domain truth.

For example, a page must not invent an actuator status because a button was pressed.

## D28. Page Refresh Contract

Refreshing a page should preserve useful context.

Where appropriate:

- Preserve active station.
- Preserve filters.
- Preserve selected tab.
- Preserve scroll/context where practical.
- Prevent older responses from overwriting newer state.

Refreshing must not silently reset a pending operational command.

## D29. Page Composition Contract

Pages should be composed from canonical components.

Example:

```text
Operations Page
  PageHeader
  StationSelector
  AlertBanner
  TelemetryGroup
    TelemetryCard
    TelemetryCard
  ActuatorCard
    StatusPill
    CommandButton
  JournalPreview
```

The page owns composition and context.

Components own their defined presentation and interaction contracts.

## D30. Page Anti-Patterns

Avoid:

- Different versions of the same canonical component on different pages.
- Page-specific status vocabulary.
- Page-specific interpretations of physical state.
- Mixing current and historical data without distinction.
- Hiding unavailable state behind empty content.
- Treating pending commands as confirmed state.
- Overloading page headers with controls.
- Excessive dashboard density.
- Critical information hidden below secondary content.
- Responsive layouts that remove essential operational information.
- Generic error pages for localized failures.
- Page-specific business logic duplicated across components.

## D31. Page Definition of Done

A page is ready for implementation when:

- Purpose is documented.
- Primary user goal is clear.
- Required context is defined.
- Information hierarchy is defined.
- Primary and secondary data are defined.
- Page states are defined.
- Actions are defined.
- Permission behavior is defined.
- Safety constraints are defined.
- Loading behavior is defined.
- Error behavior is defined.
- Empty behavior is defined.
- Unavailable behavior is defined.
- Responsive behavior is defined.
- Accessibility requirements are defined.
- Canonical components are identified.
- Navigation relationships are defined.
- Acceptance criteria are testable.

## D32. Canonical Page Rule

A page should answer three questions immediately:

1. **Where am I?**
2. **What is happening?**
3. **What can I safely do next?**

If any of these questions cannot be answered from the visible page structure, the page contract is incomplete.

## D33. Canonical Page Formula

Every HYDRAGROW page should be designed around:

**Context, Information, State, Priority, Action, Feedback, Recovery**

The page is complete only when these dimensions remain coherent across normal, degraded, unavailable, and fault conditions.
