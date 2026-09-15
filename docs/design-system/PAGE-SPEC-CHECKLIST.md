# HYDRAGROW Page Specification Checklist

## 0. Purpose

This checklist is the mandatory quality gate for every page-level functional specification in HYDRAGROW.

It defines the minimum contract required before a page spec is considered implementation-ready. A page spec must describe not only what is visible, but also ownership, source of truth, interaction, state, safety, navigation, responsive behavior, data integrity, and acceptance behavior.

The checklist is a review tool. It does not create product capabilities by itself. Any capability marked as implemented must be supported by the repository/backend source of truth.

---

## 1. Page Identity & Responsibility

- [ ] Page name is defined.
- [ ] Route is defined.
- [ ] Primary purpose is defined.
- [ ] User problem is defined.
- [ ] Primary user roles are defined.
- [ ] Secondary roles are defined where applicable.
- [ ] Page-owned responsibilities are explicit.
- [ ] Responsibilities belonging to other pages/domains are explicit.
- [ ] Entry points are defined.
- [ ] Exit/navigation points are defined.

### Ownership rule

Every important capability must have one clear owning domain. Shared data does not imply shared mutation ownership.

---

## 2. Source of Truth

- [ ] Relevant frontend page/component files identified.
- [ ] Relevant hooks identified.
- [ ] Relevant stores/context identified.
- [ ] Relevant API endpoints identified.
- [ ] Relevant backend handlers/services identified.
- [ ] Relevant models/types/schemas identified.
- [ ] Existing behavior documented.
- [ ] Existing limitations documented.
- [ ] Source-backed capabilities separated from proposed capabilities.
- [ ] Implementation-required capabilities explicitly marked.
- [ ] No unsupported capability is presented as implemented.

### Source status vocabulary

Use explicit labels such as:

- `Source-backed`
- `Partially source-backed`
- `Implementation-required`
- `Deferred`

---

## 3. Selected Context

For station/device-scoped pages:

- [ ] Selected Station Context is defined.
- [ ] Current source of selected station/device identity is defined.
- [ ] Context persistence is defined.
- [ ] Context is preserved across related page navigation.
- [ ] Station switching behavior is defined.
- [ ] Missing/invalid context behavior is defined.
- [ ] Loading context is defined.
- [ ] Old station data cannot appear under a newly selected station.

If the repository currently has only a device identity rather than a richer station context object, the spec must not invent a richer runtime contract without implementation work.

---

## 4. Information Architecture

- [ ] Page hierarchy is defined.
- [ ] Header is defined.
- [ ] Primary navigation is defined.
- [ ] Secondary navigation is defined where applicable.
- [ ] Sections are defined.
- [ ] Primary content is defined.
- [ ] Secondary/detail surfaces are defined.
- [ ] Drawer/modal/bottom-sheet behavior is defined where applicable.
- [ ] Mobile full-screen surfaces are defined where applicable.
- [ ] Deep links are defined where applicable.
- [ ] Journal integration is defined where applicable.
- [ ] Settings integration is defined where applicable.

---

## 5. Component Inventory

Every meaningful component must answer:

- [ ] What is it?
- [ ] What data does it display?
- [ ] What is its source?
- [ ] Is it interactive?
- [ ] What happens when it is clicked/tapped?
- [ ] What happens when it is toggled/selected/submitted?
- [ ] What menu actions exist?
- [ ] What is the disabled state?
- [ ] What is the loading state?
- [ ] What is the empty state?
- [ ] What is the error state?
- [ ] What is the unknown/stale state?

No visual element should look interactive if its resulting behavior is undefined.

---

## 6. Interaction Contract

This section is mandatory for every interactive component.

For each interaction:

- [ ] Trigger is defined.
- [ ] Immediate UI response is defined.
- [ ] Pending state is defined when applicable.
- [ ] Backend mutation/request is defined when applicable.
- [ ] Success state is defined.
- [ ] Rejection state is defined.
- [ ] Network/error state is defined.
- [ ] Timeout behavior is defined when applicable.
- [ ] Unknown outcome is defined when applicable.
- [ ] Recovery action is defined.
- [ ] Navigation result is defined.
- [ ] Duplicate/conflicting interaction behavior is defined.

### 6.1 Component Open / Detail Contract

Every clickable, tappable, selectable, expandable, or otherwise detail-opening component must define its outcome explicitly. Do not infer a drawer, modal, popover, route, or inline editor from the visual design alone.

For each component that opens or reveals another surface/state:

- [ ] Component/element is identified.
- [ ] Trigger and hit target are defined.
- [ ] Opened surface type is defined: inline, drawer, modal, popover, bottom sheet, full-screen, or route.
- [ ] Context/input passed to the opened surface is defined.
- [ ] Data/content shown is defined.
- [ ] Available actions are defined.
- [ ] Action permissions are defined.
- [ ] Loading behavior is defined.
- [ ] Empty behavior is defined.
- [ ] Error behavior is defined.
- [ ] Unknown/stale behavior is defined where applicable.
- [ ] Close/back behavior is defined.
- [ ] Unsaved-change behavior is defined.
- [ ] Pending-mutation behavior is defined.
- [ ] Refresh/reconciliation behavior after changes is defined.
- [ ] Mobile presentation is defined where different.
- [ ] Keyboard/focus behavior is defined where applicable.

### Outcome Contract rule

Every interactive element must answer:

```text
User does X
-> UI does Y
-> State becomes Z
-> Data/action result is A
-> Available next actions are B
```

If the resulting surface or behavior is not currently supported by source, mark it `Implementation-required` rather than inventing the UI behavior.

### 6.2 Component State Contract

Every interactive component with meaningful asynchronous, permission, safety, or data-dependent behavior must define its component-level state contract.

For each consequential component:

- [ ] Relevant states are named explicitly.
- [ ] State-to-UI presentation is defined.
- [ ] Allowed actions in each state are defined.
- [ ] Disabled/read-only behavior is defined.
- [ ] State transition trigger is defined.
- [ ] Success/rejection/error/unknown transitions are defined where applicable.
- [ ] Recovery transition is defined where applicable.
- [ ] Stale/offline/permission state is defined where applicable.
- [ ] State is not inferred from a value whose semantics are different (for example, `last_seen` is not telemetry freshness).

Canonical form:

```text
Component State
-> Visible UI
-> Allowed Actions
-> Transition Trigger
-> Next State
```

Page-level State Model and component-level state must not contradict each other. A page spec may classify a state as not applicable, but must not leave a consequential component state implicit.

### Canonical interaction model

```text
User Interaction
      |
      v
Immediate UI State
      |
      v
Pending / Local Validation
      |
      v
Backend / Runtime Operation
      |
      +---- Success
      +---- Rejection
      +---- Error
      +---- Timeout / Unknown
      |
      v
Authoritative State Reconciliation
```

---

## 7. Data Contract

For every important displayed or mutated field:

- [ ] Field name is known.
- [ ] Type is known.
- [ ] Unit is known where applicable.
- [ ] Source is known.
- [ ] Null/missing behavior is defined.
- [ ] Stale behavior is defined.
- [ ] Formatting is defined.
- [ ] Precision is defined where relevant.
- [ ] Valid range is defined where relevant.
- [ ] Derived values are identified.
- [ ] Estimated values are identified.
- [ ] Measured values are identified.
- [ ] Commanded values are identified.
- [ ] Confirmed values are identified.

### Data integrity vocabulary

```text
Measured != Estimated
Estimated != Commanded
Commanded != Confirmed
```

The spec must never collapse these concepts into one UI status.

---

## 8. State Model

The page must define all relevant states:

- [ ] Initial
- [ ] Loading
- [ ] Ready
- [ ] Empty
- [ ] Partial data
- [ ] Stale
- [ ] Offline
- [ ] Fault
- [ ] Unknown
- [ ] Permission denied
- [ ] Network error
- [ ] Backend error
- [ ] Mutation pending
- [ ] Mutation success
- [ ] Mutation rejected
- [ ] Timeout
- [ ] Recovery

Only states relevant to the page need to be implemented, but omitted states must be intentionally classified as not applicable rather than forgotten.

---

## 9. Action / Command Contract

For every command or consequential mutation:

- [ ] Action name is defined.
- [ ] Target is defined.
- [ ] Parameters are defined.
- [ ] Units are defined.
- [ ] Allowed range is defined.
- [ ] Preconditions are defined.
- [ ] Interlocks are defined.
- [ ] Permission is defined.
- [ ] Rate limit is defined where applicable.
- [ ] Cooldown is defined where applicable.
- [ ] Ownership is defined.
- [ ] Backend acceptance is defined.
- [ ] Runtime execution semantics are defined.
- [ ] Physical confirmation semantics are defined where applicable.
- [ ] Failure behavior is defined.
- [ ] Unknown outcome is defined.

Backend acceptance must never be described as physical completion unless the source explicitly provides that confirmation.

### 9.1 Mutation Lifecycle Contract

Every consequential mutation must define its complete lifecycle:

- [ ] User action is identified.
- [ ] Client-side validation is defined where applicable.
- [ ] Confirmation requirement is defined.
- [ ] Request/command dispatch is defined.
- [ ] Pending UI is defined.
- [ ] Backend acceptance/rejection is defined.
- [ ] Persistence result is defined where applicable.
- [ ] Runtime execution semantics are defined where applicable.
- [ ] Physical observation/confirmation semantics are defined where applicable.
- [ ] Success UI is defined.
- [ ] Error/rejection UI is defined.
- [ ] Timeout/unknown outcome is defined where applicable.
- [ ] Query invalidation/state refresh/reconciliation is defined.
- [ ] Duplicate submission behavior is defined.

The lifecycle must preserve these distinctions:

```text
Commanded
!= Backend Accepted
!= Persisted
!= Observed
!= Physically Confirmed
```

### 9.2 Confirmation / Reversibility Contract

Every consequential action must explicitly classify whether confirmation is required and whether the resulting operation can be reversed or cancelled.

For each consequential action:

- [ ] Confirmation policy is defined: immediate, confirm-before-execute, or not applicable.
- [ ] Confirmation trigger and wording are defined where confirmation is required.
- [ ] Reversibility is defined: reversible, cancellable, irreversible, or unknown.
- [ ] Cancel behavior is defined where cancellation is supported.
- [ ] Rollback/reversal behavior is defined where supported.
- [ ] Post-confirmation pending behavior is defined.
- [ ] Destructive consequences are disclosed before confirmation where applicable.
- [ ] Safety-critical immediate actions are not delayed by a generic confirmation pattern when the safety contract requires immediate execution.

Confirmation must not be used as a substitute for truthful mutation state. A confirmed user intent is not the same as backend acceptance, persistence, observation, or physical confirmation.

---

## 10. Safety & Control Authority

Required for pages that can affect physical operation or safety:

- [ ] Safety boundary is defined.
- [ ] E-STOP behavior is defined where applicable.
- [ ] Interlocks are defined.
- [ ] Manual ownership is defined.
- [ ] Auto Controller ownership is defined.
- [ ] Automation ownership is defined.
- [ ] Control Authority / Arbitration boundary is defined where multiple writers exist.
- [ ] Safety Gate behavior is defined.
- [ ] Dangerous actions have explicit confirmation semantics where appropriate.
- [ ] Offline behavior is defined.
- [ ] Fault behavior is defined.
- [ ] Recovery behavior is defined.
- [ ] Unsafe optimistic UI is prohibited.

### Ownership rule

No two independent control surfaces may silently compete for the same controlled resource.

---

## 11. Permission Contract

For each relevant role:

- [ ] View permission is defined.
- [ ] Edit permission is defined.
- [ ] Execute permission is defined.
- [ ] Delete permission is defined.
- [ ] Configuration permission is defined.
- [ ] Backend authorization is identified as authoritative.

Use repository-backed roles rather than inventing new roles in the UI spec.

---

## 12. Navigation Contract

- [ ] Page entry is defined.
- [ ] Page exit is defined.
- [ ] Back behavior is defined.
- [ ] Deep-link behavior is defined where applicable.
- [ ] Detail navigation is defined.
- [ ] Drawer/modal navigation is defined.
- [ ] Journal navigation is defined where applicable.
- [ ] Settings navigation is defined where applicable.
- [ ] Station/device context is preserved.
- [ ] Pending operation behavior during navigation is defined.
- [ ] Unsaved-change behavior is defined.

### 12.1 Cross-Page Handoff Contract

When an interaction moves the user between pages or owned surfaces, the handoff must preserve the context and intent required by the destination.

For each consequential cross-page handoff:

- [ ] Origin surface is identified.
- [ ] Destination surface is identified.
- [ ] Selected station/device context is defined where applicable.
- [ ] Context payload or shared source is defined.
- [ ] Destination behavior when context is missing/invalid is defined.
- [ ] Pending mutation behavior during navigation is defined.
- [ ] Unsaved changes behavior is defined.
- [ ] Return behavior is defined.
- [ ] Mutation/result reconciliation on return is defined where applicable.
- [ ] Ownership handoff is explicit when the destination becomes the mutation/control owner.
- [ ] No hidden context switch is introduced by navigation.

Canonical form:

```text
Origin
-> Context / Intent
-> Destination
-> Destination State
-> Action / Inspection
-> Return / Reconciliation
```

Navigation must not silently change the selected station/device or imply that the destination owns a capability that belongs to another domain.

---

## 13. Desktop & Mobile Contract

### Desktop

- [ ] Layout hierarchy is defined.
- [ ] Primary/secondary panes are defined.
- [ ] Drawer behavior is defined where applicable.
- [ ] Hover behavior is defined where applicable.
- [ ] Keyboard interaction is defined where applicable.
- [ ] Information density is intentional.

### Mobile

- [ ] Responsive hierarchy is defined.
- [ ] Touch interactions are defined.
- [ ] Bottom sheet/full-screen behavior is defined where applicable.
- [ ] Hover is not required.
- [ ] Touch target requirements are respected.
- [ ] Graph/canvas is not mandatory unless explicitly justified.
- [ ] Mobile navigation is defined.
- [ ] Sticky actions are defined where necessary.

---

## 14. Forms & Validation

- [ ] Required fields are defined.
- [ ] Optional fields are defined.
- [ ] Defaults are defined where applicable.
- [ ] Allowed values are defined.
- [ ] Range validation is defined.
- [ ] Cross-field validation is defined.
- [ ] Backend validation is acknowledged.
- [ ] Inline error behavior is defined.
- [ ] Submit behavior is defined.
- [ ] Unsaved draft behavior is defined.
- [ ] Cancel/reset behavior is defined.

---

## 15. History & Audit

- [ ] Recent activity behavior is defined.
- [ ] Full history owner is defined.
- [ ] Journal boundary is defined.
- [ ] Audit events are defined for consequential actions.
- [ ] Actor is captured where available.
- [ ] Station/device is captured where available.
- [ ] Target is captured where available.
- [ ] Timestamp is captured where available.
- [ ] Requested parameters are captured where appropriate.
- [ ] Result/rejection/error is captured where available.

Do not duplicate the full Journal inside a page-specific detail surface unless explicitly required by product design.

---

## 16. Empty / Loading / Error / Unknown

- [ ] First-time empty state.
- [ ] Filtered empty state where filtering exists.
- [ ] Loading state.
- [ ] Partial-loading state where applicable.
- [ ] Network error.
- [ ] Backend error.
- [ ] Offline state.
- [ ] Unknown state.
- [ ] Retry action.
- [ ] Stale-data presentation.

The UI must distinguish missing information from a confirmed negative state.

---

## 17. Cross-Page Contract

- [ ] Shared data is identified.
- [ ] Shared context is identified.
- [ ] Mutation ownership is identified.
- [ ] Cross-page navigation is identified.
- [ ] Journal integration is identified.
- [ ] Settings integration is identified.
- [ ] Safety/ownership relationships are identified.
- [ ] Conflicting responsibilities are explicitly resolved.

Example:

```text
Operations
  = runtime control

Auto Mode
  = closed-loop regulation

Automation
  = event/workflow orchestration

Settings
  = persistent configuration

Journal
  = canonical historical trace
```

These are examples of ownership boundaries, not a universal requirement that every page contain every domain.

---

## 18. Accessibility

- [ ] Keyboard navigation where applicable.
- [ ] Visible focus state.
- [ ] Accessible labels.
- [ ] Status is not communicated by color alone.
- [ ] Errors are discoverable/announced appropriately.
- [ ] Confirmation wording is explicit.
- [ ] Touch targets are adequate.

---

## 19. Observability

- [ ] User-visible runtime status is defined.
- [ ] Execution status is defined where applicable.
- [ ] Error code is shown where available and useful.
- [ ] Timestamp is shown where relevant.
- [ ] Audit event is defined where required.
- [ ] Runtime log/deep link is defined where available.
- [ ] Unknown outcomes are observable rather than silently discarded.

---

## 20. Wireframe Contract

Every page spec should define the visual contract needed to implement it:

- [ ] Desktop overview.
- [ ] Mobile overview.
- [ ] Primary detail surface.
- [ ] Critical interaction.
- [ ] Critical error state.
- [ ] Empty state.
- [ ] Loading state.
- [ ] Dangerous-action state where applicable.

Wireframes must not introduce capabilities absent from the functional specification.

---

## 21. Acceptance Criteria

Every important requirement must be convertible into an acceptance test.

Preferred format:

```text
Given <initial state>
When <user/system action>
Then <observable result>
And <additional invariant>
```

Checklist:

- [ ] Happy path.
- [ ] Validation failure.
- [ ] Permission failure.
- [ ] Safety/interlock failure where applicable.
- [ ] Network failure.
- [ ] Timeout/unknown outcome where applicable.
- [ ] Station/context switch where applicable.
- [ ] Recovery path.

---

## 22. Implementation Mapping

- [ ] Frontend page file identified.
- [ ] Components identified.
- [ ] Hooks identified.
- [ ] Stores/context identified.
- [ ] API endpoints identified.
- [ ] Backend handlers/services identified.
- [ ] Models/types identified.
- [ ] Existing implementation mapped.
- [ ] Required implementation changes listed.
- [ ] Deferred work listed.

The spec must distinguish documentation/design changes from code changes.

### 22.1 Source-to-UI Traceability

For each important component, displayed data, mutation, or runtime-affecting action:

- [ ] UI component/page is identified.
- [ ] Hook/state/store is identified where applicable.
- [ ] API endpoint is identified where applicable.
- [ ] Backend handler/service is identified where applicable.
- [ ] Model/type/schema is identified where applicable.
- [ ] Runtime/device integration is identified where applicable.
- [ ] Source status is recorded for the complete path.

The trace must be sufficient to verify whether a behavior is source-backed, partially source-backed, or implementation-required.

---

## 22.2 Ownership & Responsibility Matrix

For shared capabilities or data, the spec must distinguish ownership dimensions:

- [ ] Data owner is defined.
- [ ] UI owner is defined.
- [ ] Mutation/write owner is defined.
- [ ] Runtime/control owner is defined where applicable.
- [ ] Read consumers are identified where useful.
- [ ] Conflicting writers are identified.
- [ ] Cross-page handoff is defined where ownership changes.

Reading shared data does not imply permission to mutate or control it.

---

## 22.3 Concurrency / Conflict Contract

When data can change from multiple users, pages, automations, devices, or backend processes:

- [ ] Concurrent writers are identified.
- [ ] Version/revision mechanism is identified where available.
- [ ] Stale-edit behavior is defined.
- [ ] Conflict detection behavior is defined where applicable.
- [ ] Conflict resolution strategy is defined where applicable.
- [ ] Reload/refresh behavior is defined.
- [ ] Overwrite behavior is explicit if allowed.
- [ ] Merge behavior is explicit if supported.
- [ ] User notification is defined where a conflict can affect intent.

Do not assume last-write-wins, merge, or automatic reconciliation unless the source or product contract supports it.

---

## 23. Anti-Pattern Review

- [ ] No fabricated telemetry.
- [ ] No speculative hardware capability.
- [ ] No unsupported controls presented as functional.
- [ ] No optimistic physical confirmation.
- [ ] No stale data presented as current.
- [ ] No missing value converted to a meaningful zero/Off state without source justification.
- [ ] No duplicate ownership between pages.
- [ ] No hidden station-context switch.
- [ ] No destructive action without appropriate confirmation.
- [ ] No generic confirmation added to an action whose safety contract requires immediate execution.
- [ ] No clickable component has an undefined outcome surface or state transition.
- [ ] No consequential component has an undefined component-level state contract.
- [ ] No mutation treats backend acceptance as persistence, observation, or physical confirmation without source support.
- [ ] No confirmation pattern obscures reversibility/cancellation semantics.
- [ ] No navigation silently changes selected station/device context.
- [ ] No source-to-UI behavior is claimed without traceable implementation evidence.
- [ ] No shared data ownership is confused with mutation/control ownership.
- [ ] No concurrency behavior is invented when multiple writers can change the same resource.
- [ ] No full persistent configuration duplicated inside runtime surfaces.
- [ ] No full historical journal duplicated inside local detail surfaces.

---

## 24. Definition of Done

A page spec is implementation-ready only when:

- [ ] Page responsibility is clear.
- [ ] Source of truth has been verified.
- [ ] Ownership boundaries are clear.
- [ ] Selected context is clear where applicable.
- [ ] Information architecture is complete.
- [ ] Component inventory is complete.
- [ ] Every interactive component has an Interaction Contract.
- [ ] Every interactive component has an explicit Outcome Contract where it opens/reveals another surface or state.
- [ ] Component states are defined for consequential interactions.
- [ ] Component State Contract is complete for consequential interactions.
- [ ] Data Contract is complete for important data.
- [ ] State Model covers relevant non-happy paths.
- [ ] Command Contract exists for consequential actions.
- [ ] Mutation Lifecycle Contract exists for consequential mutations.
- [ ] Confirmation / Reversibility Contract exists for consequential actions.
- [ ] Safety and Control Authority are defined where required.
- [ ] Permission is defined.
- [ ] Navigation is defined.
- [ ] Cross-Page Handoff Contract is defined for consequential cross-page flows.
- [ ] Desktop and mobile behavior are defined.
- [ ] Validation is defined.
- [ ] History/audit ownership is defined.
- [ ] Error/offline/unknown behavior is defined.
- [ ] Cross-page contracts are defined.
- [ ] Wireframe contract is complete.
- [ ] Acceptance criteria are testable.
- [ ] Implementation mapping is complete.
- [ ] Source-to-UI traceability is complete for important behavior.
- [ ] Ownership dimensions are explicit for shared capabilities.
- [ ] Concurrency/conflict behavior is defined where multiple writers exist.
- [ ] No unsupported capability is presented as implemented.

---

## 25. Canonical Page-Spec Formula

```text
PAGE SPEC
=
Responsibility
+ Source of Truth
+ Ownership
+ Selected Context
+ Information Architecture
+ Component Contract
+ Outcome Contract
+ Interaction Contract
+ Component State Contract
+ Data Contract
+ State Model
+ Action / Command Contract
+ Mutation Lifecycle Contract
+ Confirmation / Reversibility Contract
+ Safety / Control Authority
+ Permission
+ Navigation
+ Cross-Page Handoff Contract
+ Desktop / Mobile
+ Validation
+ History / Audit
+ Error / Unknown Handling
+ Cross-Page Contract
+ Wireframe Contract
+ Acceptance Criteria
+ Implementation Mapping
+ Source-to-UI Traceability
+ Ownership Matrix
+ Concurrency / Conflict Contract
```

### Final review question

> **Can another engineer implement the page, its interactions, its state transitions, its safety boundaries, and its cross-page behavior from this spec without inventing missing product behavior?**

If the answer is no, the page spec is not complete.

---

## 26. Current Spec Audit Order

When applying this checklist to the existing HYDRAGROW documentation, use this order:

1. Dashboard
2. Automation
3. Operations
4. Settings
5. Journal
6. Other pages

For each page, resolve source and ownership questions before visual refinement.
