# HYDRAGROW Frontend Redesign

## Status

Approved direction: shell-first redesign of the existing Vite + React frontend.

## Goal

Rebuild the frontend presentation around the repository design system without changing routes, domain behavior, telemetry contracts, or safety semantics. The result should feel like a calm industrial instrument: operationally legible, status-first, responsive, and consistent across every page.

## Scope

Target `hydragrow-frontend/**` and this design specification. Preserve existing business logic and data hooks unless a small composition change is required to expose an existing contract correctly. Do not invent telemetry, aggregate unavailable data, or weaken safety confirmation flows.

## Design principles

- Use the design tokens and component contracts in `docs/design-system/` as the source of truth.
- Make station context, connectivity, freshness, and role context visible where relevant.
- Prioritize the hierarchy: what is happening, what is abnormal, what needs attention, then detail and controls.
- Use semantic tokens and accessible text/labels; color must not be the sole status signal.
- Reserve high-salience danger treatment for E-STOP and genuinely dangerous actions.
- Keep motion restrained and purposeful; never animate a fault into looking healthy.
- Use mobile-first layouts. Desktop gets persistent navigation; mobile gets bottom navigation and stacked operational cards.

## Architecture

### Shared visual foundation

Normalize `App.css` and shared layout styles around the documented token system. Establish reusable page anatomy: `app-page`, page header, context/status row, panel/card surfaces, section labels, and state views. Remove ad-hoc color and spacing patterns from the primary shell and migrated pages.

### Application shell

Refine `MainLayout` and its navigation components so desktop and mobile share the same information architecture. The shell must expose current route, station context, connection state, and user/role context without competing with the page content. Maintain existing route behavior and navigation affordances.

### Status and state language

Reuse and standardize existing primitives such as `DeviceStatePill`, `StateView`, `Banner`, and `PageHeader`. Every data-driven surface must distinguish loading, empty, unavailable, stale, offline, warning, fault, and confirmed states using text plus visual treatment. Controls must communicate command accepted versus physical state confirmed where the existing contract provides that distinction.

### Dashboard and fleet

Recompose Dashboard around two levels:

1. Multi-station overview using the existing fleet status hooks/cards, with attention-first ordering and explicit stale/offline/fault presentation.
2. Selected station detail using only available telemetry, followed by device/control areas and recent events.

Do not add fabricated KPI totals or inferred health scores. Preserve E-STOP behavior and keep it visually unique.

### Domain pages

Apply the same page anatomy and hierarchy to Operations, Cultivation, Journal, Settings, Automation, authentication, onboarding, and error surfaces. Operations lead with state before controls. Cultivation groups season/recipe/dosing by lifecycle. Journal emphasizes fault/warning events and clear filtering. Settings groups administrative concerns. Automation uses React Flow on desktop and a readable flow-card representation on mobile.

## Responsive behavior

- Mobile-first CSS with the documented `lg` navigation transition.
- Desktop: persistent sidebar and wider two-dimensional panels where appropriate.
- Mobile: bottom navigation, stacked cards, readable flow cards instead of a cramped canvas, and controls sized for touch.
- Validate at the current preview viewport (900x603) and a narrow mobile viewport.

## Accessibility and safety

- Preserve semantic HTML, keyboard navigation, focus visibility, accessible names, and reduced-motion behavior.
- Pair status color with text, iconography, or structural cues.
- Keep destructive actions explicit and confirmable. E-STOP remains the only emergency action with exceptional visual emphasis.
- Do not change authorization, command, or confirmation semantics while restyling.

## Implementation sequence

1. Normalize tokens and shared visual primitives.
2. Rework the application shell and responsive navigation.
3. Migrate Dashboard and Fleet as the reference vertical slice.
4. Migrate Operations, Cultivation, Journal, Settings, and Automation.
5. Migrate auth/onboarding/error surfaces and perform responsive/accessibility cleanup.

## Validation

Use the commands declared in `.agent/verify.yml` for the frontend subsystem. Add or update focused tests only where existing behavior is affected by composition. Verify the running preview in a real browser for navigation, Dashboard station states, responsive shell behavior, and the E-STOP affordance. Self-review the final diff for invented data, broken route behavior, unsafe control changes, and token drift.

## Acceptance criteria

- All in-scope frontend pages use the documented visual language and shared page/status anatomy.
- Desktop and mobile navigation remain functional and expose route context.
- Dashboard clearly presents multi-station status and selected-station detail using existing data contracts.
- Loading, unavailable, stale, offline, warning, fault, and confirmed states are distinguishable without relying on color alone.
- E-STOP remains prominent, safe, and behaviorally unchanged.
- Verification commands pass with concrete output, and browser checks show no blocking layout or interaction regressions.
- No telemetry, health score, or aggregate is fabricated for presentation purposes.

## Non-goals

- No backend, database, authentication-provider, telemetry-schema, or routing migration.
- No new product workflows unrelated to visual hierarchy and frontend consistency.
- No replacement of the existing component library unless the current implementation cannot satisfy the documented contract.
