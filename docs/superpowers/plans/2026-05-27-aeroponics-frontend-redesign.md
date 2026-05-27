# Aeroponics Frontend Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the HydraGrow React frontend into a clear aeroponics operations interface for farmers while preserving technical telemetry and controls.

**Architecture:** Keep existing routes, context, hooks, and API contracts. Introduce a shared light biophilic visual system first, then update layout and shared components, then page-level information hierarchy. Use existing Lucide, Recharts, React, and Tailwind v4 CSS utilities.

**Tech Stack:** React 19, TypeScript, Vite, Tailwind CSS v4, Recharts, Lucide React, Tauri-compatible frontend.

---

## File Structure

- Modify `hydragrow-frontend/src/App.css`: global farm theme tokens, reusable utility classes, accessibility states, reduced-motion handling.
- Modify `hydragrow-frontend/src/App.tsx`: toast visual styling to match the light operations UI.
- Modify `hydragrow-frontend/src/components/layout/MainLayout.tsx`: app shell, header, navigation, online/offline indicator, bottom menu.
- Modify `hydragrow-frontend/src/components/ui/PageHeader.tsx`: consistent page header with optional subtitle/action area.
- Modify `hydragrow-frontend/src/components/ui/SensorBentoCard.tsx`: condition cards with status/range/description support.
- Modify `hydragrow-frontend/src/components/ui/LoadingState.tsx`, `StateView.tsx`, `InputGroup.tsx`, `SubCard.tsx`, `AccordionSection.tsx`, `Switch.tsx`: shared light-theme primitives.
- Modify `hydragrow-frontend/src/pages/Dashboard.tsx`: current state, sensor cards, equipment strip, next-action alerts, advanced diagnostics.
- Modify `hydragrow-frontend/src/pages/ControlPanel.tsx`: farm-function equipment grouping and safer disabled/locked explanations.
- Modify `hydragrow-frontend/src/pages/Analytics.tsx`: filters and trend cards in the shared visual system.
- Modify `hydragrow-frontend/src/pages/Settings.tsx`: make common controls farmer-readable and advanced controls visually separated.
- Modify `hydragrow-frontend/src/pages/DosingHistory.tsx`, `SystemLog.tsx`, `CropSeasons.tsx`: operation timeline styling and shared filters/actions.

## Task 1: Build Baseline

**Files:**
- Read: `hydragrow-frontend/package.json`

- [ ] **Step 1: Run the existing frontend build**

Run:

```bash
cd hydragrow-frontend
npm run build
```

Expected: Either PASS, or an existing build failure unrelated to redesign. If it fails before code changes, record the failure and continue only if the failure is unrelated to the files in this plan.

## Task 2: Shared Farm Theme

**Files:**
- Modify: `hydragrow-frontend/src/App.css`
- Modify: `hydragrow-frontend/src/App.tsx`

- [ ] **Step 1: Replace dark global tokens with farm operations tokens**

Update `App.css` so the theme exposes:

```css
@theme {
  --color-primary: #15803d;
  --color-accent: #ca8a04;
  --color-success: #16a34a;
  --color-warning: #d97706;
  --color-error: #dc2626;
  --color-water: #0284c7;
  --color-surface: #ffffff;
  --color-surface-muted: #ecfdf5;
  --color-border: #bbf7d0;
  --color-text: #14532d;
  --color-text-muted: #4b6354;
}
```

Also update `:root` and `body` to use a light green page background, dark green text, and the existing system font stack.

- [ ] **Step 2: Add reusable component classes**

Define or update classes in `@layer components`:

```css
.app-page
.page-header
.page-header-main
.page-header-icon
.page-header-title
.page-header-subtitle
.ui-card
.ui-form-row
.ui-form-label
.ui-helper-text
.ui-state
.ui-state-title
.ui-state-desc
.ui-input
.ui-btn-md
.ui-loading
.ui-loading-fullscreen
.ui-loading-card
.ui-loading-spinner
.ui-loading-message
.farm-status-pill
.farm-section-title
.farm-muted-panel
```

Each class must use light surfaces, visible borders, high text contrast, and focus rings. Disabled controls must use `disabled:opacity-50 disabled:cursor-not-allowed`.

- [ ] **Step 3: Add reduced-motion protection**

Add:

```css
@media (prefers-reduced-motion: reduce) {
  *, ::before, ::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    scroll-behavior: auto !important;
    transition-duration: 0.01ms !important;
  }
}
```

- [ ] **Step 4: Update toast styling**

In `App.tsx`, change `Toaster` defaults from slate/dark to white/green:

```tsx
style: {
  background: '#ffffff',
  color: '#14532d',
  borderRadius: '14px',
  border: '1px solid #bbf7d0',
  boxShadow: '0 16px 40px rgba(20, 83, 45, 0.12)',
}
```

- [ ] **Step 5: Verify**

Run:

```bash
cd hydragrow-frontend
npm run build
```

Expected: Build exits 0, or only the previously recorded baseline failure remains.

## Task 3: App Shell and Shared Components

**Files:**
- Modify: `hydragrow-frontend/src/components/layout/MainLayout.tsx`
- Modify: `hydragrow-frontend/src/components/ui/PageHeader.tsx`
- Modify: `hydragrow-frontend/src/components/ui/SensorBentoCard.tsx`
- Modify: `hydragrow-frontend/src/components/ui/LoadingState.tsx`
- Modify: `hydragrow-frontend/src/components/ui/StateView.tsx`
- Modify: `hydragrow-frontend/src/components/ui/InputGroup.tsx`
- Modify: `hydragrow-frontend/src/components/ui/SubCard.tsx`
- Modify: `hydragrow-frontend/src/components/ui/AccordionSection.tsx`
- Modify: `hydragrow-frontend/src/components/ui/Switch.tsx`

- [ ] **Step 1: Update MainLayout**

Change the shell to a light farm app frame:

- Root background: `bg-emerald-50 text-emerald-950`.
- Header: white surface, emerald border, app label `HydraGrow Khí Canh`.
- Status pill: online text `Đang kết nối` and offline text `Mất tín hiệu`.
- Bottom nav: white surface, emerald active states, muted inactive states.
- More menu: white menu with high-contrast text and active emerald highlight.

- [ ] **Step 2: Update PageHeader**

Ensure `PageHeader` supports title, optional subtitle, icon, and optional right content. Use the shared `.page-header` classes and no dark slate colors.

- [ ] **Step 3: Extend SensorBentoCard props**

Update `SensorBentoCardProps` to accept:

```tsx
statusLabel?: string;
statusTone?: 'good' | 'warn' | 'danger' | 'info';
rangeLabel?: string;
description?: string;
```

Render value, unit, status, range, and description without requiring callers to provide all fields.

- [ ] **Step 4: Update shared primitives**

Update `LoadingState`, `StateView`, `InputGroup`, `SubCard`, `AccordionSection`, and `Switch` so the default style is light, readable, and accessible. Keep their public props backward-compatible.

- [ ] **Step 5: Verify**

Run:

```bash
cd hydragrow-frontend
npm run build
```

Expected: Build exits 0, or only the previously recorded baseline failure remains.

## Task 4: Dashboard Redesign

**Files:**
- Modify: `hydragrow-frontend/src/pages/Dashboard.tsx`

- [ ] **Step 1: Add local display helpers**

Inside `Dashboard.tsx`, add helper functions for:

- Sensor tone from fault and numeric value.
- Pump labels and tones.
- Budget percentage formatting.
- Short next-action message from offline, sensor offline, fault, or notification permission.

Helpers must be local to the file unless reused by another task.

- [ ] **Step 2: Rebuild top status panel**

Replace the dark hero with a light operations panel showing:

- `friendlyState.label`
- `friendlyState.description`
- online/offline indicator
- station health score
- device ID
- current mode from `settings?.control_mode`

- [ ] **Step 3: Rebuild sensor cards**

Use `SensorBentoCard` for EC, pH, water temperature, and tank water level with:

- value and unit
- status label
- fault-specific maintenance text
- target/range label from settings where available

- [ ] **Step 4: Rebuild active equipment and diagnostics**

Render active pump/valve/mist/mixing chips in a white panel. Keep advanced diagnostics expandable and convert all dark slate panels to farm theme classes.

- [ ] **Step 5: Verify**

Run:

```bash
cd hydragrow-frontend
npm run build
```

Expected: Build exits 0, or only the previously recorded baseline failure remains.

## Task 5: Control Panel Redesign

**Files:**
- Modify: `hydragrow-frontend/src/pages/ControlPanel.tsx`

- [ ] **Step 1: Update AdvancedDeviceControl**

Change equipment cards to light panels with:

- Clear running/stopped/locked text.
- Disabled reason text for auto mode, emergency lock, offline state, and pending command.
- Primary switch area with stable layout.
- Advanced settings in a bordered light panel.

- [ ] **Step 2: Reorganize page sections**

Keep the same equipment list but use farm-function section headings:

- Phun sương và khí hậu.
- Cấp/xả nước.
- Châm dinh dưỡng.
- Cân pH.
- Trộn tuần hoàn.

- [ ] **Step 3: Update alerts and reset action**

Convert offline and emergency alerts to light danger/warning panels. Keep `resetFault` behavior unchanged.

- [ ] **Step 4: Verify**

Run:

```bash
cd hydragrow-frontend
npm run build
```

Expected: Build exits 0, or only the previously recorded baseline failure remains.

## Task 6: Analytics Redesign

**Files:**
- Modify: `hydragrow-frontend/src/pages/Analytics.tsx`

- [ ] **Step 1: Update chart theme constants**

Use readable line colors on white cards:

```tsx
cyan: '#0284c7'
fuchsia: '#c026d3'
orange: '#ea580c'
blue: '#2563eb'
```

Keep distinct text labels so color is not the only status cue.

- [ ] **Step 2: Update FlatChartCard**

Render white chart cards with:

- title and icon
- current/average/min/max stats
- readable tooltip
- light grid and axis colors
- subtle area fills

- [ ] **Step 3: Update filters and states**

Convert filters, loading, error, and empty states to shared light classes. Keep the current season/range/interval behavior unchanged.

- [ ] **Step 4: Verify**

Run:

```bash
cd hydragrow-frontend
npm run build
```

Expected: Build exits 0, or only the previously recorded baseline failure remains.

## Task 7: Settings Redesign

**Files:**
- Modify: `hydragrow-frontend/src/pages/Settings.tsx`

- [ ] **Step 1: Update page header and advanced toggle**

Make the header farmer-readable:

- Title: `Cài đặt vườn khí canh`
- Subtitle: explain common settings vs technical settings.
- Advanced toggle: light warning panel, label `Chế độ kỹ thuật`.

- [ ] **Step 2: Convert common sections**

Convert network, general, growth, water, dosing, and sensor sections to the shared light component classes. Keep all form fields and existing save payload unchanged.

- [ ] **Step 3: Preserve advanced gating**

Keep current `isAdvancedMode` gates for PWM, physical coefficients, safety, communication, sensor enable switches, and EC calibration. Make hidden/visible states visually clear.

- [ ] **Step 4: Verify**

Run:

```bash
cd hydragrow-frontend
npm run build
```

Expected: Build exits 0, or only the previously recorded baseline failure remains.

## Task 8: Timeline Pages

**Files:**
- Modify: `hydragrow-frontend/src/pages/DosingHistory.tsx`
- Modify: `hydragrow-frontend/src/pages/SystemLog.tsx`
- Modify: `hydragrow-frontend/src/pages/CropSeasons.tsx`

- [ ] **Step 1: Update DosingHistory**

Convert cards, chips, expanded diagnostic panels, filters, export button, loading, error, and empty states to the farm light theme. Keep all fetch and CSV export behavior unchanged.

- [ ] **Step 2: Update SystemLog**

Convert timeline cards and metadata panels to light cards. Critical and warning events must have explicit text labels and visible danger/warning borders.

- [ ] **Step 3: Update CropSeasons**

Convert season cards and actions to the shared visual system. Preserve existing season lifecycle behavior.

- [ ] **Step 4: Verify**

Run:

```bash
cd hydragrow-frontend
npm run build
```

Expected: Build exits 0, or only the previously recorded baseline failure remains.

## Task 9: Final Visual and Build Verification

**Files:**
- Verify all modified frontend files.

- [ ] **Step 1: Search for leftover dark slate-heavy styling**

Run:

```bash
rg -n "bg-slate-950|bg-slate-900|text-slate-100|border-slate-800|from-slate-900|to-slate-950" hydragrow-frontend/src hydragrow-frontend/src/App.css
```

Expected: Any remaining results are intentional technical detail panels or code paths that were not in scope; otherwise update them.

- [ ] **Step 2: Run final build**

Run:

```bash
cd hydragrow-frontend
npm run build
```

Expected: Build exits 0.

- [ ] **Step 3: Start dev server**

Run:

```bash
cd hydragrow-frontend
npm run dev -- --host 0.0.0.0
```

Expected: Vite starts and prints a local URL. Keep the server running only long enough to report the URL.

- [ ] **Step 4: Final diff review**

Run:

```bash
git diff -- hydragrow-frontend/src hydragrow-frontend/src/App.css hydragrow-frontend/src/App.tsx docs/superpowers/plans/2026-05-27-aeroponics-frontend-redesign.md
```

Expected: Diff only contains planned frontend redesign and plan file changes.
