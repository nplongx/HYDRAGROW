# Figma Unimplemented Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every gap between the Figma Hi-Fi spec (file `UvfamFTHSrneof4eKOxozm`, node `213-2`, screens D1–D17) and the current hydragrow-frontend + hydragrow-backend, so that all Figma features are fully implemented against real backend endpoints.

**Architecture:** Work in 4 phases: (A) wire up orphaned pages + routes; (B) frontend-only Figma-fidelity gaps; (C) backend-mediated features (new routes/migrations); (D) previously decision-gated features (auth self-register/Google/forgot-password, Roles page, QR pairing + confirm screen, Analytics grafana-gate + report toggle). Keep the existing design-token system (`src/App.css` `@theme` + `.ui-*` classes), the nav anatomy (dashboard/operations/cultivation/journal/settings), and the class bans from `hydragrow-frontend/CHUAN-GIAO-DIEN-FRONTEND.md`. Test-first for every feature.

**Tech Stack:** Frontend React 19 + TypeScript + Vite + TanStack Query + Tailwind v4 + lucide-react + `@xyflow/react` + `react-qr-code` (+ NEW `html5-qrcode` for camera scan). Backend Rust/Actix/sqlx/pg + `hydragrow-shared` for shared enum/types.

---

## Verified gap inventory (source of truth)

Fleet of explore-agent audits (2026-09-10) established each item below as MISSING/PARTIAL against real source. Items already implemented correctly (nav 5 tabs, DosingHistory, automation canvas editor + conflict detection, Settings 5 tabs + email/advanced-mode/logout, DevicePairing manual claim + QR-after-claim, emergency-stop `emergency_stop` action, photo journal, recipe stage editor) are NOT re-worked — note any follow-up in the relevant task.

Key corrections discovered by audit (do not assume otherwise):
- No `PUT /events/{id}/acknowledge` / `resolve` backend route exists.
- Log category enum has no `device`/`automation`; DB column is free-text `TEXT` (no CHECK), so adding enum values needs no migration.
- No `DELETE /seasons/{id}` or photo delete route; config backup exists as `GET/POST /api/devices/{device_id}/admin/backup|restore`.
- Pairing is 1-step claim (`POST /api/devices/claim` with `device_id`/`label`/`hardware_id`); no 2-step confirm code. Decide: derive confirm code deterministically from `device_id` on the client (no firmware/backend change).
- Fleet: `GET /api/health/topics` (all devices) exists; NO fleet sensor aggregate; NO fleet warning-count aggregate (per-device `/health-summary` rolling 1 h only).
- Analytics: no `analytics/metrics` REST; mean/min/max via `GET /devices/{id}/sensors/range-stats?field=ec|ph|temp|water_level`; stability %/prediction/CPU-RAM only as Prometheus gauges on `GET /metrics` (controller: `agitech_controller_free_heap_bytes`, `_wifi_rssi_dbm`, `_uptime_seconds`).
- Users: `users` table has `scopes TEXT[]`, NO role column; only `POST /api/admin/users` (root API key). No list/whoami/update endpoints.
- `crop_recipe_stages` has NO `light_hours` column.
- Automation scripts: `GET /devices/{id}/scripts`; NO persisted last-run timestamp.

---

## Locked decisions (user delegated)

- **Auth:** self-registration via Firebase `createUserWithEmailAndPassword`; backend auto-upserts `users` row with read-only default (`read:telemetry`) on first verified token; admin elevates via Roles page. Forgot-password uses Firebase `sendPasswordResetEmail` (email reset link; no OTP/SMTP). No fake attempt counter — keep existing Firebase error text, add per-field red-error borders.
- **Roles:** add `users.role` column (`admin`|`operator`|`viewer`), list/update endpoints behind root API key, map role→scope bundles; implement D17 page (member list, role pills, invite form, permission matrix).
- **Analytics:** embed Grafana in a dark frame gated by a configurable `grafana_url` (Settings → Kết nối); when unset, keep current styled placeholder card. Metric cards fed from real `range-stats` + new `analytics/health` endpoint; CPU%/prediction render as "—" (no source). Weekly-report toggle persists a preference on the user row (documented preference-only).
- **Pairing:** add `html5-qrcode` camera scan parsing `hydragrow://claim/{device_id}`; D15 confirm screen shows code derived deterministically from `device_id`; "Mã khớp" → existing `POST /devices/claim`; "Mã không khớp" → cancel. Keep manual entry. Paired rows gain online/offline + last-seen from `/devices/{id}/status`.

---

## Phase A — Wire up orphaned pages and routes

### Task A1: Route registration + navigation entries

**Files:**
- Modify: `hydragrow-frontend/src/App.tsx`
- Modify: `hydragrow-frontend/src/components/layout/MainLayout.tsx` (only if a nav/sidebar entry is agreed)
- Modify: `hydragrow-frontend/src/pages/Settings.tsx` (GeneralSection)

- [x] **Step 1 (test):** Assert routes mount the orphaned pages. Add `src/pages/..navigation.test.tsx` or extend an existing app-shell test asserting that navigating to `/fleet`, `/config-backup`, `/user-management` renders the respective page headings.
- [x] **Step 2:** Register `<Route path="fleet" element={<FleetView />} />`, `config-backup`, `user-management` inside the `MainLayout` block.
- [x] **Step 3:** Settings GeneralSection: relabel "Ghép nối thiết bị" → "Quản lý thiết bị liên kết"; navigation to `/pairing` unchanged; add "Backup & Restore Cấu Hình" row → `/config-backup`; add "Vai trò của bạn" row (data via whoami in Phase C; while missing, render from `useAuth` scopes or "Quản trị viên" fallback guarded by scope).
- [x] **Step 4:** Dashboard: add a fleet-entry affordance (e.g. device-id pill or header link) → `/fleet`.
- [x] **Step 5:** Verify: `npm run test` focused + `npx tsc --noEmit` + manual route smoke.

## Phase B — Frontend-only Figma-fidelity gaps

### Task B1: Dashboard greeting, critical banner, action links

**Files:**
- Modify: `hydragrow-frontend/src/pages/Dashboard.tsx`
- Modify: `hydragrow-frontend/src/components/ui/QuickActionBar.tsx`
- Modify: `hydragrow-frontend/src/components/ui/QuickActionBar.test.tsx` (existing)

- [x] **Step 1 (test):** QuickActionBar now renders 4 actions including "Tưới ngay"; Dashboard header shows "Xin chào" when a display name is available; a critical sensor event renders a red KHẨN CẤP banner.
- [x] **Step 2:** Greeting: `useAuth` user `displayName ?? emailPrefix`; preserve FSM state info as the subtitle. Camera/red banner: reuse existing "next action" computation; when state is critical (sensor fault / emergency), render a `bg-red-*` banner distinct from the amber default.
- [x] **Step 3:** Add "Tưới ngay" as 4th quick action, calling the water-pump control action via existing `useDeviceControl` (confirm pump id for water in `src/lib/pumpLabels.ts`); keep other 3 actions' navigation.
- [x] **Step 4:** Verify + focused tests green; keep `Dashboard.test.tsx:20` no-indigo assertion passing.

### Task B2: SystemLog date grouping

**Files:**
- Modify: `hydragrow-frontend/src/pages/SystemLog.tsx`
- Modify: `hydragrow-frontend/src/pages/SystemLog.test.tsx`

- [x] **Step 1 (test):** Rendered list shows "HÔM NAY" / "HÔM QUA" section headers and events grouped under them.
- [x] **Step 2:** Group sorted events by calendar day of `timestamp`; render `HÔM NAY`, `HÔM QUA` (yesterday), else `toLocaleDateString('vi-VN')` + count. Preserve existing filters, CSV export, and expand behavior.

### Task B3: CropSeasons history duration + chevron

**Files:**
- Modify: `hydragrow-frontend/src/components/seasons/SeasonHistoryList.tsx`
- Test: extend existing seasons test

- [x] **Step 1 (test):** History row shows `N ngày` and a chevron icon.
- [x] **Step 2:** Compute day count between `start_time`/`end_time` (`Math.round`), render "· N ngày" next to dates; add `<ChevronRight />`.

### Task B4: Emergency stop dialog fidelity

**Files:**
- Modify: `hydragrow-frontend/src/components/safety/EmergencyStopConfirmDialog.tsx`
- Modify: callers feeding `runningPumps` in `EmergencyStopButton.tsx` / Operations

- [x] **Step 1 (test):** Header text "SẼ DỪNG NGAY"; running device rows include PWM% when a duty value is available.
- [x] **Step 2:** Change header to "SẼ DỪNG NGAY toàn hệ thống" (keep red styling); pass PWM% from `useDeviceControl` pump status through to `runningPumps` (`Record<string, { on: boolean; pwm?: number }>` or add pwm map); render `· {pwm}%` when present.

### Task B5: FleetView shell + filters + link

**Files:**
- Modify: `hydragrow-frontend/src/pages/FleetView.tsx`
- Modify: `hydragrow-frontend/src/hooks/useFleetStatus.ts` (if ship-worthy fleet summary is not ready in Phase C)

- [x] **Step 1 (test):** Header shows back button + subtitle; filter pills Tất cả / Có cảnh báo trước; "+ Liên kết thiết bị mới" navigates to `/pairing`.
- [x] **Step 2:** Add `useNavigate().back()` affordance, subtitle text, pill filters (client-side: "Có cảnh báo trước" filters devices with warning badge when added in Phase C), and the link.
- [x] **Step 3:** Phase C hooks EC/pH + warning badge into the device cards.

### Task B6: Automation flow-card schedule text

**Files:**
- Modify: `hydragrow-frontend/src/components/automation/FlowOverviewCard.tsx`
- Test: extend automation tests

- [x] **Step 1 (test):** Cron triggers render human-readable "06:00 hằng ngày" (and weekday variants where supported).
- [x] **Step 2:** Add cron→Vietnamese text helper (minute/hour/day-of-week fields); fall back to current generic text for unsupported expressions. Last-run ✓ waits for Phase C `last_run_at`.

### Task B7: Auth screens

**Files:**
- Modify: `hydragrow-frontend/src/components/auth/LoginScreen.tsx`
- Create: `hydragrow-frontend/src/components/auth/RegisterScreen.tsx`
- Create: `hydragrow-frontend/src/components/auth/ForgotPasswordScreen.tsx`
- Modify: `hydragrow-frontend/src/lib/firebaseAuth.ts` (add Google provider + forgot-password + register helpers)
- Modify: `hydragrow-frontend/src/components/auth/AuthGate.tsx` (route login vs register vs forgot)

- [x] **Step 1 (tests):** LoginScreen renders Google button, "Đăng ký" link, "Quên mật khẩu?" link, and red-border error state when error is set; RegisterScreen + ForgotPasswordScreen render and submit.
- [x] **Step 2:** Implement Firebase helpers; wire Google `signInWithPopup` (auto-upsert path in Phase C); registration (no scopes → default read-only on backend); forgot-password success state ("Đã gửi email đặt lại mật khẩu").

## Phase C — Backend-mediated features

### Task C1: Acknowledge/resolve events

- [x] Backend `hydragrow-backend/src/api/alert.rs` + `hydragrow-backend/migrations/<date>_add_resolved_at_to_system_events.sql` (`resolved_at TIMESTAMPTZ`): `PUT /api/devices/{device_id}/events/{event_id}/acknowledge` (scope `read:telemetry`).
- [x] Frontend `SystemLog.tsx` + `EventLogCard.tsx`: "Đánh dấu đã xử lý" button on alert rows; resolved state styling; filter "Chưa xử lý".

### Task C2: Log categories device/automation

- [x] Shared `hydragrow-shared/src/log.rs`: add `Device`, `Automation` variants (serialized `device`, `automation`).
- [x] Backend `alert.rs:normalize_categories` passthrough (TEXT col — no migration). Ensure `script_alert`-reason events surface under `automation` at write time (map in `mqtt/handlers/script_eval.rs`).
- [x] Frontend `SystemLog.tsx` FILTERS: add `device` ("Thiết bị") and `automation` ("Tự động hóa") pills.

### Task C3: Delete season + delete photo

- [x] Backend `crop_season.rs`: `DELETE /api/devices/{device_id}/seasons/{season_id}`; `crop_season_photo.rs`: `DELETE /api/devices/{device_id}/seasons/{season_id}/photos/{photo_id}`. Scope `write:config` (or `device:admin`).
- [x] Frontend `CropSeasons.tsx` (history rows) + `SeasonPhotoJournal.tsx`: delete buttons + confirm + optimistic removal.

### Task C4: Recipe light-hours

- [x] Migration `<date>_add_light_hours_to_crop_recipe_stages.sql`: `light_hours INT`.
- [x] Shared `hydragrow-shared/src/recipe.rs` `CropStage` + backend recipe create/update.
- [x] Frontend `RecipeBuilder.tsx` stage editor: "Ánh sáng (giờ/ngày)" number field, persisted; growth-stage pills preset (Ươm mầm 14h / Sinh trưởng 16h / Ra hoa 12h / Thu hoạch 12h) that prefill duration_days + light_hours.

### Task C5: Fleet summary endpoint

- [x] Backend `api/fleet.rs` (or extend `main.rs` mount): `GET /api/fleet/summary` → per device `{ device_id, label, is_online, last_seen, ec_latest, ph_latest, warning_count(1h) }` (fan out to `sensors/latest` + `health-summary`).
- [x] Frontend `useFleetStatus.ts`/`FleetView.tsx`: show EC/pH on cards + ⚠n badge; wire Phase B5 filter.

### Task C6: Scripts last-run

- [x] Migration `<date>_add_last_run_at_to_scripts.sql` (`last_run_at TIMESTAMPTZ`); set after successful script execution (`mqtt/handlers/script_eval.rs`); expose via `GET /devices/{id}/scripts`.
- [x] Frontend `FlowOverviewCard.tsx`: "lần cuối: hôm nay ✓" from `last_run_at` (relative date), hide if never run.

### Task C7: Users role + admin API

- [x] Migration `<date>_add_role_to_users.sql`: `role TEXT NOT NULL DEFAULT 'viewer'` (CHECK in ('admin','operator','viewer')), backfill from scopes.
- [x] Backend `api/admin_users.rs`: `GET /api/admin/users`, `GET /api/admin/whoami` (from Bearer token), `PATCH /api/admin/users/{id}` (role/scopes/is_active). Root API key or `device:admin`/`*` scope.
- [x] Frontend: Settings "Vai trò" row from whoami; gate Settings/Roles admin entries on `role`/scopes.

### Task C8: Analytics health endpoint

- [x] Backend `api/analytics.rs`: `GET /api/devices/{device_id}/analytics/health` → `{ free_heap_bytes, wifi_rssi_dbm, uptime_seconds, backend_process_cpu_percent, last_updated_at }` from in-memory gauges (`metrics.rs`).
- Frontend `Analytics.tsx`: metric cards fed by `range-stats` (EC/pH/temp/water mean) + health values (RAM/RSSI); unmeasurable (CPU% self-probe, prediction) as "—".

## Phase D — Decision-gated features (locked above)

### Task D1: Roles page (D17)

**Files:**
- Create: `hydragrow-frontend/src/pages/Roles.tsx` (or repurpose `UserManagement.tsx`)
- Modify: `App.tsx` route `/roles`

- [x] **Step 1 (test):** Member list rows with role pills; role picker on invite form; permission matrix (4 capabilities × 3 roles) renders from role→scope bundles.
- [x] **Step 2:** Wire to Phase C7 endpoints: list `/admin/users`, invite = `POST /admin/users`, change role = `PATCH /admin/users/{id}`.
- [x] **Step 3:** Route `/roles` + admin nav entry; replace/link current `UserManagement` scope form.

### Task D2: Pairing QR scan + confirm screen (D14–D15)

**Files:**
- Add dep: `html5-qrcode`
- Modify: `hydragrow-frontend/src/pages/DevicePairing.tsx`
- Create: `hydragrow-frontend/src/components/pairing/ScanConfirmOverlay.tsx` (or reuse inline)
- Modify: package.json

- [x] **Step 1 (test):** Confirm overlay shows derived code (e.g. "84 • KX") + "Mã khớp" / "Mã không khớp" buttons; Mã khớp triggers claim with the parsed device id.
- [x] **Step 2:** Add scan button → Html5Qrcode camera view; on decode extract device id from `hydragrow://claim/{id}` or raw id; show confirm overlay; on match call existing claim; rows get online/offline + last-seen from `/status`.

### Task D3: Analytics grafana gate + weekly report toggle

**Files:**
- Modify: `hydragrow-frontend/src/pages/Settings.tsx` (Kết nối / integrations tab: `grafana_url` + persisted)
- Modify: `hydragrow-frontend/src/pages/Analytics.tsx`
- Backend: user preference column + getter/setter (fold into Task C7 or `api/user_prefs.rs`)

- [x] **Step 1 (test):** Analytics renders dark iframe frame when `grafana_url` set; styled placeholder otherwise; report toggle persists.
- [x] **Step 2:** Settings Kết nối stores `grafana_url` (existing config write path); Analytics gate + weekly-report `Switch` with preference persisted to backend user row; UI copy notes preference-only ("sẽ gửi về email của bạn").

---

## Verification (run after every task)

```bash
# Frontend
cd hydragrow-frontend
npx tsc --noEmit
npm run lint        # 0 errors (pre-existing warnings tolerated)
npm run test        # full suite; run flaky SystemLog test isolated if it times out
npm run build

# Backend (from .agent/jules.yml commands; cargo)
cargo fmt --check
cargo clippy -- -D warnings   # or per subsystem lint_cmd
cargo test                     # per subsystem test_cmd
```

- Keep token/class bans in `hydragrow-frontend/CHUAN-GIAO-DIEN-FRONTEND.md`; `Dashboard.test.tsx:20` no-indigo assertion must stay green.
- Acceptance/evidence contracts + `docs/project-state/CURRENT-STATUS.md` / `TRACEABILITY.md` updated at completion (see AGENTS.md governance).
- Full-suite run at end of each phase; `git diff | wc -c` under 75 KB.

## Out of scope
- Firmware (ESP32) changes; actual SMTP email delivery; OTP-only forgot-password; real per-field attempt counters; Grafana instance provisioning (config-gated URL only).