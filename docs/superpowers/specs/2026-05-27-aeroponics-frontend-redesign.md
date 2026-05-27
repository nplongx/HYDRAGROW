# HydraGrow Aeroponics Frontend Redesign

## Goal

Redesign the HydraGrow frontend into an aeroponics operations interface that is clear for farmers with or without technical experience, while preserving the detailed telemetry and controls needed by advanced operators.

The interface should answer these questions quickly:

- Is the growing system healthy right now?
- Which parameter needs attention?
- What equipment is running?
- What should the farmer do next?
- Where can a technician inspect detailed logs, calibration, and advanced configuration?

## Design Direction

Use an "operations farm" model across the full frontend:

- Light, high-contrast biophilic theme based on soft green backgrounds, white surfaces, emerald status accents, amber caution, red danger, and blue water states.
- Calm dashboard density: show the most important state first, then detailed diagnostics through expandable sections.
- Plain Vietnamese labels that describe farm outcomes before technical terms.
- Lucide icons only, no emoji icons.
- Stable, responsive layouts for phone, tablet, and desktop.
- Strong focus states, visible disabled states, and readable contrast.

The generated design-system recommendation from `ui-ux-pro-max` is Organic Biophilic with:

- Primary: `#15803D`
- Secondary: `#22C55E`
- CTA/accent: `#CA8A04`
- Background: `#F0FDF4`
- Text: `#14532D`

The implementation may adapt exact shades to the existing Tailwind setup, but the final UI should not remain a dark slate technical dashboard.

## Information Architecture

### Dashboard

The dashboard becomes the main "farm status" screen. It should include:

- A large current-status panel with farmer-readable state, online/offline status, device ID context, and station health score.
- Four primary condition cards: EC, pH, water temperature, and tank water level.
- Each condition card should show value, unit, friendly status, target/range where available, and fault/maintenance state.
- A running-equipment strip for pumps, valves, misting, dosing, and mixing.
- A clear "next action" or alert card when there is a fault, offline device, sensor issue, or notification permission request.
- Advanced diagnostics hidden behind an expandable section.

### Control Panel

The control panel should be grouped by real farm function:

- Mist and climate.
- Water in/out.
- Nutrient dosing A/B.
- pH correction.
- Mixing/circulation.

Each equipment card should show:

- Running/stopped/locked state.
- Why it is locked when auto mode or safety lock is active.
- Primary toggle with clear disabled styling.
- Advanced settings for PWM, duration, and force-run controls behind disclosure.

### Analytics

Analytics should show readable trend cards:

- EC, pH, temperature, and water level charts.
- Current, average, min, and max values.
- Clear empty, loading, and error states.
- Existing time range, season, and interval filters retained but visually simplified.

Charts should use line/area chart patterns with distinct colors and non-color labels. Animations should remain subtle and avoid flashing live effects.

### Settings

Settings should default to safe common controls:

- Device connection.
- Auto/manual operation.
- Main EC, pH, temperature, misting, and water targets.
- Sensor calibration workflow.

Risky or technical fields should remain available only when advanced mode is enabled:

- Pump capacity and PWM tuning.
- Safety thresholds.
- Sensor smoothing and publishing intervals.
- Physical coefficients and low-level calibration values.

### Dosing History, System Log, Crop Seasons

History and logs should read as an operation timeline:

- Use event cards with outcome-first titles.
- Keep technical metadata expandable.
- Improve filters and export controls with the shared visual system.
- Preserve all current data fields and API behavior.

## Shared Components

Update shared UI foundations so pages stay consistent:

- `App.css`: new theme tokens, body background, typography, cards, buttons, inputs, focus rings, status badges, and loading states.
- `MainLayout`: app shell, header, online/offline indicator, bottom navigation, and more menu.
- `PageHeader`: consistent page titles and optional subtitles/actions.
- `SensorBentoCard`: richer condition card with status/range support.
- `SubCard`, `InputGroup`, `AccordionSection`, `StateView`, `LoadingState`, `Switch`: align spacing, colors, focus, disabled states, and readability.

## Data Flow

Do not change backend contracts or API endpoints.

Use existing data from:

- `DeviceContext` for live sensor data, device status, controller health, FSM state, settings, system events, and pump state.
- Existing hooks such as `useDeviceControl`, `useCropSeason`, and `useFCM`.
- Existing page fetch logic for analytics, logs, dosing history, and settings.

UI changes should be presentational unless a small derived display helper is needed.

## Error Handling

All existing loading, missing configuration, offline, fetch error, and empty states should remain covered.

Improve wording so errors are actionable:

- Offline: explain that commands cannot be sent until the controller reconnects.
- Sensor offline: explain data may be stale.
- Fault: show the friendly fault guide when available.
- Disabled control: explain whether it is disabled by auto mode, safety lock, offline state, or pending command.

## Accessibility

Requirements:

- No color-only status indicators; pair colors with text and icons.
- Visible focus rings for keyboard navigation.
- Clickable controls use pointer cursor unless disabled.
- Disabled states use opacity plus `cursor-not-allowed`.
- Text contrast should meet WCAG AA on the light theme.
- Do not use hover scale effects that shift layout.
- Respect reduced-motion preferences for animations where practical.

## Implementation Scope

Frontend files in `hydragrow-frontend` are in scope, especially:

- `src/App.css`
- `src/App.tsx`
- `src/components/layout/MainLayout.tsx`
- `src/components/ui/*`
- `src/pages/Dashboard.tsx`
- `src/pages/ControlPanel.tsx`
- `src/pages/Analytics.tsx`
- `src/pages/Settings.tsx`
- `src/pages/DosingHistory.tsx`
- `src/pages/SystemLog.tsx`
- `src/pages/CropSeasons.tsx`

Out of scope:

- Backend API changes.
- Firmware changes.
- Database schema changes.
- Reverting unrelated existing worktree changes.

## Verification

Run frontend checks after implementation:

- `npm run build` in `hydragrow-frontend`.
- Start the Vite dev server and inspect the UI at desktop and mobile widths if the environment allows it.

If a visual browser check is not possible, report that limitation and rely on build/type verification.
