# HydraGrow Frontend — DESIGN.md

> This document describes the **software architecture** of `hydragrow-frontend`: its layers, data flows, technical decisions, and the reasons behind them — intended for new contributors or coding agents who need to understand "why the code is organized this way" before making changes.
>
> This document **does not repeat**:
>
> - Design tokens, colors, shared classes, and the UI checklist → xem [`CHUAN-GIAO-DIEN-FRONTEND.md`](https://claude.ai/chat/CHUAN-GIAO-DIEN-FRONTEND.md).
> - Rules for writing display copy/labels → xem [`docs/ui-writing-guideline.md`](https://claude.ai/chat/docs/ui-writing-guideline.md).
> - Mandatory rules when opening a PR touching this module → xem [`module-rules/frontend.md`](https://claude.ai/docs/superpowers/specs/module-rules/frontend.md) (authoritative source; this document only summarizes and provides additional context).
>
> Reflects the code at commit `bdf37b3` (2026-09-13). If this document differs from the current code — **trust the code, not the document**, and update the mismatched section.

---

## 1. Where the Frontend Fits in the System

`hydragrow-frontend` is the only client application that end users (station owners, operators) interact with. It **never** communicates directly with MQTT or firmware — everything goes through `hydragrow-backend` over HTTP (REST) and WebSocket.

```
                [hydragrow-backend]
            Actix-web · REST API + WebSocket
                        │
            HTTPS (REST) │ WSS (realtime)
                        │
                [hydragrow-frontend]
              React 19 + TypeScript + Vite
                        │
            ┌───────────┴───────────┐
        build:web                build:tauri
            │                         │
            ▼                         ▼
      [Web / PWA]              [Desktop · Tauri v2]
       → Vercel                 → Win / macOS / Linux

```

One React codebase, two build targets. See section 4 for how the "where it runs" concerns are separated from logic/UI.

The overall system architecture (firmware, MQTT topic, DB) nằm ở [`../README.md`](https://claude.ai/README.md) — not repeated here.

---

## 2. Technology Stack

| Lớp Lựa chọn Ghi chú  |                                                    |                                                                                          |
| --------------------- | -------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| UI framework          | React 19 + TypeScript                              | `strict` via `tsconfig.json`; rule bắt buộc "do not use `any` to bypass backend type errors" |
| Build tool            | Vite 7                                             | 2 output khác nhau tùy target (`dist/` vs `dist-tauri/`), xem `vite.config.ts`           |
| Styling               | Tailwind CSS v4 (`@tailwindcss/vite`)              | Token định nghĩa ở `src/App.css`, chuẩn hoá trong CHUAN-GIAO-DIEN-FRONTEND.md            |
| Routing               | react-router-dom v7                                | `BrowserRouter`, lazy-load cho `Settings`                                                |
| Server state / cache  | TanStack Query v5                                  | Dữ liệu dạng request-response (mùa vụ, công thức, automation script...)                  |
| Client/runtime state  | Zustand v5                                         | Dữ liệu đẩy real-time via WebSocket (xem mục 5)                                          |
| Schema & validate     | Zod v3                                             | Đặc biệt vian trọng cho IR của automation (mục 9)                                        |
| Desktop shell         | Tauri v2 (Rust)                                    | `src-tauri/`, dùng plugin `http`, `fs`, `dialog`, `notification`, `store`, `opener`      |
| Domain logic thuần    | Gleam → biên dịch JS                               | `gleam_core/`, xem mục 10                                                                |
| Authentication & push           | Firebase (Authentication + FCM)                              | `lib/firebaseAuthentication.ts`, `hooks/useFCM.ts`, `public/firebase-messaging-sw.js`              |
| Automation canvas     | `@xyflow/react` (React Flow)                       | Trình soạn thảo flow trực vian duy nhất (Blockly đã bị loại bỏ)                          |
| Icon                  | `lucide-react`                                     |                                                                                          |
| Toast                 | `react-hot-toast`                                  | Chuẩn duy nhất cho thông báo ngắn, không thêm lib toast khác                             |
| QR                    | `html5-qrcode` (scan) + `react-qr-code` (generate) | Ghép thiết bị (`DevicePairing`)                                                          |
| Test                  | Vitest + Testing Library (jsdom)                   | Co-located với file nguồn                                                                |
| Lint                  | ESLint + typescript-eslint                         | `npx eslint .` phải clean before merge                                                 |

---

## 3. Directory Structure

```
hydragrow-frontend/
├── src/
│   ├── pages/            # 1 file = 1 route (Dashboard, Operations, Cultivation, Journal, Settings...)
│   ├── components/
│   │   ├── ui/             # Design-system nội bộ: Button, Badge, StatusPill, ControlCard, TabShell...
│   │   ├── layout/         # MainLayout — khung nav + outlet
│   │   ├── auth/           # Login/Register/ForgotPassword screens
│   │   ├── automation/     # Flow editor (React Flow) + panel liên quan
│   │   └── control/, dosing/, logs/, pairing/, recipes/, safety/, seasons/
│   │                       #   → component đặc thù theo domain nghiệp vụ
│   ├── hooks/            # useDeviceSync, useDeviceControl, useFleetStatus, useAutomationBuilder...
│   ├── store/            # useDeviceStore.ts (Zustand — state real-time)
│   ├── contexts/         # AuthContext.tsx (phiên đăng nhập)
│   ├── lib/              # apiClient, firebase*, automation/* (compiler pipeline), dosing/, logs/, seasons/
│   ├── platform/         # Lớp trừu tượng Web ↔ Tauri: http.ts, storage.ts, settings.ts, file.ts
│   └── types/            # Type TS phải khớp 1-1 với struct Rust trong hydragrow-shared
├── gleam_core/           # Logic thuần biên dịch sang JS (mục 10)
├── src-tauri/            # Vỏ desktop: Rust commands, capabilities, valve_guard, secret_store
├── docs/                 # ui-writing-guideline.md, route-content-audit.md
└── CHUAN-GIAO-DIEN-FRONTEND.md   # Chuẩn UI/UX (nguồn sự thật cho token & IA)

```

Organization rule: components are grouped by **business domain** (`automation/`, `dosing/`, `safety/`, `seasons/`...), not by technical type — `ui/` is the only exception, reserved for shared UI blocks not tied to any business domain.

---

## 4. Two Runtimes, One Codebase: Web and Tauri Desktop

`hydragrow-frontend` builds to two different targets from the same React tree: bản Web (PWA-ready, deploy Vercel) và bản Desktop đóng gói via Tauri. Instead of scattering `if (isTauri)` branches across components, all runtime differences are isolated in `src/platform/`:

| Năng lực Web Tauri Desktop  |                                           |                                                      |
| --------------------------- | ----------------------------------------- | ---------------------------------------------------- |
| HTTP request                | `window.fetch` (có wrapper timeout 12s)   | `@tauri-apps/plugin-http`                            |
| Lưu cấu hình/cache          | `localStorage`                            | `@tauri-apps/plugin-store` (`device-state.json`)     |
| File / dialog               | —                                         | `@tauri-apps/plugin-fs`, `@tauri-apps/plugin-dialog` |
| Secret nhạy cảm             | biến môi trường lúc build/deploy          | `src-tauri/src/secret_store.rs` (native)             |
| Thông báo đẩy               | Firebase Cloud Messaging (service worker) | `@tauri-apps/plugin-notification` + FCM              |

`src/platform/http.ts` checks `isTauriRuntime()` tại call-site and dispatches accordingly — components call `httpFetch(...)` as usual, without needing to know where they are running. This is the correct extension point when adding new platform capabilities: thêm vào `platform/`, do not add runtime conditions to business components.

Vite cũng phân biệt output theo target (`vite.config.ts`): `outDir` là `dist-tauri` khi build cho Tauri, `dist` khi build web; dev server Tauri cố định cổng `1420` (HMR via `1421`) vì Tauri hard-code cổng này lúc `tauri dev`.

**Why both:** nông dân/người vận hành dùng điện thoại hoặc máy tính không cần cài gì (web) là kênh chính; bản desktop phục vụ nơi cần chạy ổn định lâu dài trên một máy trạm cố định, tận dụng tích hợp OS (notification native, lưu secret ngoài trình duyệt, filesystem) mà web không có.

---

## 5. State Management: Three Layers Separated by Data Lifecycle

| Loại state Nơi lưu Cập nhật từ Ví dụ  |                               |                                             |                                                                                                           |
| ------------------------------------- | ----------------------------- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Telemetry/real-time                   | Zustand (`useDeviceStore`)    | WebSocket (via `useDeviceSync`)             | `sensorData`, `deviceStatus`, `fsmState`, `controllerHealth`, `systemEvents`, `tankAlert`, `ownedDevices` |
| Server/resource state                 | TanStack Query                | REST request-response, cache theo query key | mùa vụ, công thức, lịch sử châm, danh sách automation script, roles                                       |
| Phiên đăng nhập                       | React Context (`AuthenticationContext`) | Firebase Authentication listener                      | `status` (`loading`/`authenticated`/`unauthenticated`), `user`                                            |

Reason for the separation: each type has a different lifecycle and update frequency. Mixing them together (vd nhét cache REST vào Zustand, hoặc để mỗi component tự `useState` cho dữ liệu real-time) sẽ makes change origins harder to trace and tests harder to write.

TanStack Query cấu hình default tại `App.tsx`: `staleTime` 60s, `gcTime` 5 phút, `retry` 2 lần, không tự refetch khi cửa sổ được focus lại.

Theo rule bắt buộc ở `module-rules/frontend.md`: mọi cập nhật Zustand từ WebSocket phải đi via **action sets tên rõ ràng** (không set state trực tiếp rải rác trong callback socket) — điều này giữ cho state dễ test và dễ trace khi có bug "tại sao giá trị này đổi".

---

## 6. Real-Time Data Synchronization

`useDeviceSync()` is called **exactly once** ở `MainLayout` — the entire session after login shares one connection, child pages only read from Zustand through selectors and do not open their own connections.

Flow: WebSocket tới backend → raw payload (JSON phản ánh serialize phía Rust, ví dụ enum FSM dạng `{"Fault": "lý do"}`, tên field như `PUMP_A`) → normalization layer trong `useDeviceSync.ts` chuyển thành stable shape for the UI (`SystemFault:lý do`, `pump_a: boolean`...) → ghi vào `useDeviceStore`. Isolating "sự khó chịu" của định dạng phía backend/firmware from components is intentional — components only see normalized data.

Every request (REST lẫn thiết lập WebSocket) automatically attaches `Authenticationorization: Bearer <Firebase ID token>` via `platform/http.ts`, components do not add the header themselves. Có cổng mock auth (`X-User-Id`, bật bằng query param `mock_auth=true` hoặc `localStorage`) for test/dev when avoiding dependency on real Firebase — **do not use outside dev/test environments**.

Mandatory rule: do not call `fetch`/WebSocket directly in components — always go through the centralized client/service layer (`platform/http.ts`, `lib/apiClient.ts`) để xử lý auth header, retry, lỗi mạng ở một chỗ.

---

## 7. Navigation & Page Architecture

The previous flat structure (5 tab + menu "Thêm" chứa 5 mục khác) has been consolidated into **5 work groups** theo quyết định trong CHUAN-GIAO-DIEN-FRONTEND.md §4, and has been implemented in code (route cũ redirect sang route mới trong `App.tsx`):

| Current Route Page Group  |                          |                                                  |
| --------------------------- | ------------------------ | ------------------------------------------------ |
| `/dashboard`                | `Dashboard`              | Overview                                        |
| `/operations`               | `Operations`             | Operations (combines Manual Control + Automation) |
| `/cultivation`              | `Cultivation`            | Cultivation (combines Seasons + Recipes + Dosing History) |
| `/journal`                  | `Journal`                | Journal (combines System Events + Analytics)       |
| `/settings`                 | `Settings` (lazy-loaded) | Settings                                          |

Utility routes are not separate tabs; they are accessed from `/settings`: `/pairing` (device pairing), `/fleet` (multi-device fleet view), `/config-backup`, `/user-management` và `/roles` (both render the `Roles` component).

Legacy routes (`/control`, `/automation`, `/crop-seasons`, `/recipes`, `/dosing-history`, `/logs`, `/analytics`) are retained as `<Navigate replace>` to the new group routes — deep link cũ (bookmark, thông báo cũ) do not break.

**Rule when adding a new route:** first ask which of the five groups the route belongs to before adding it — avoid recreating the "Thêm" chứa mọi thứ không biết xếp đâu (bài học đã ghi lại trong CHUAN-GIAO-DIEN-FRONTEND.md).

`MainLayout` xử lý điều hướng responsive bằng CSS breakpoint, using the same component tree for both web and Tauri desktop (không tách "bản desktop" riêng):

- Below `lg`: mobile header + floating pill-style bottom nav.
- From `lg` and above (kể cả cửa sổ Tauri, vì cửa sổ desktop default đủ rộng): fixed left sidebar.

`MainLayout` also blocks the entire layout and redirects to `/settings` if `isMissingConfig` — preventing the app from rendering in a "silently broken" state when `backend_url`/`api_key` is valid.

---

## 8. Authenticationentication

Firebase Authentication (email/password + Google) via `lib/firebaseAuthentication.ts`. `AuthenticationContext` wraps the entire app, expose `status: 'loading' | 'authenticated' | 'unauthenticated'` along with actions (`login`, `register`, `googleLogin`, `resetPassword`, `logout`).

`AuthenticationGate` (trong `App.tsx`) decides whether to render `LoginScreen` / `RegisterScreen` / `ForgotPasswordScreen` or the main app based on `status`. The ID token obtained from Firebase is stored via `setIdToken` and automatically attached to every request ở tầng `platform/http.ts` — components do not manage authentication headers themselves.

---

## 9. Automation: From Drag-and-Drop Canvas to Scripts Running on the Controller

This is the most complex part of the frontend — essentially a small compiler running in the browser.

Previously, there were two parallel editors (Blockly kéo-thả khối và React Flow dạng canvas), which fragmented both the UI and IR transformation logic. The decision to consolidate on React Flow (CHUAN-GIAO-DIEN-FRONTEND.md §6) has been implemented — thư mục `blockly/` no longer exists trong `src/components/automation/`.

Current pipeline:

```
[Canvas React Flow]  →  [buildIr.ts]  →      [IR — ir.ts]      →  [compileToRhai.ts]  →  [Script Rhai]
 node/edge người        dựng IR từ      Zod schema: Condition,        sinh mã Rhai         controller-core /
 dùng kéo-thả           canvas          conditionTree,                từ IR                backend thực thi
 (reactflow/*.tsx)                      contextVariables,
                                         scheduleConflicts...

```

The most important thing to preserve when modifying this: `Condition`/`ScriptSensorInput`/`ScriptFsmInput` trong `lib/automation/ir.ts` must match field-for-field với struct tương ứng ở `hydragrow-backend/src/models/script.rs` (ghi rõ trong comment code nguồn) — changing one side requires changing the other side in the **same PR**, following the general principle "do not allow silent schema drift between the two sides" của module-rules.

`TestPanel.tsx` allows conditions/flows to be tested directly in the UI before saving, reducing the risk of pushing an incorrect automation to a real device.

---

## 10. `gleam_core`: Pure Business Logic Compiled to JavaScript

Part of the domain logic (not all of it) is written in [Gleam](https://gleam.run/) — a functional language with static typing and compile-time exhaustiveness — instead of TypeScript:

- `analytics.gleam`, `dashboard.gleam` — display aggregation calculations
- `faults.gleam` — fault/warning classification
- `fsm.gleam` — FSM state interpretation logic for display
- `settings/{calibration,cron,payload,validation}.gleam` — configuration payload validation & construction

`gleam.toml` sets `target = "javascript"`; builds to `gleam_core/build/dev/javascript/gleam_core`, imported in TS through the alias `@gleam` declared in `vite.config.ts` (`resolve.alias`) — do not import the long build path directly.

**Why separate it from TypeScript:** calculations where a small error has large consequences (phân loại lỗi cảm biến, chuyển trạng thái FSM, tổng hợp dashboard) benefit from exhaustive pattern matching and Gleam's absence of implicit `null`; test bằng `gleeunit`, independent of the React component tree.

**Operational note:** after modifying `.gleam` files, run `gleam build` before `vite build`/`vite dev` picks up the changes — this step is not currently wired into the npm script (xem mục 15).

---

## 11. Device Safety: Defense in Depth

Because the application controls physical pumps/valves, none of the three layers below is designed to stand alone:

1. **UI (mọi runtime):** `EmergencyStopButton` + `EmergencyStopConfirmDialog` requires confirmation before sending an emergency stop command; `AutomationConflictBanner` warns about schedule conflicts before saving a flow.
2. **Native, only bản Tauri desktop:** `src-tauri/src/valve_guard.rs` keeps the latest received pump/valve state, blocks at the client các lệnh thủ công rõ ràng xung đột (ví dụ mở van nước vào khi van xả đang mở) **before** gói tin rời máy. Comment trong code nói rõ vai trò: "the controller remains the final authoritative safety layer; this guard only blocks an avoidable conflicting request."
3. **Final authority, outside the frontend scope:** FSM và safety guard trong `hydragrow-controller-core`/firmware is where the actual decision is made on whether a command may be executed.

Implication for code changes: do not remove the confirmation step ở lớp 1/2 to "make operations faster" — they exist because layer 3 has network latency, và the frontend must never be written under the assumption that "UI blocking is sufficient."

---

## 12. Security

- **CSP** declared in `src-tauri/tauri.conf.json` (applies to the desktop webview): only `'self'` + backend domain (`hydragrow.onrender.com`, both `https` and `wss`) + Firebase/Google domains required for Authentication/FCM; `object-src 'none'`, `frame-ancestors 'none'`.
- **Tauri capability** according to the principle of least privilege: `capabilities/default.json` only allows `http:default` call the exact backend origin (no wildcard). Mandatory rule when adding a new Tauri command: khai báo đúng quyền cần trong `capabilities/`, do not grant more than necessary, with Rust tests (`cargo test` trong `src-tauri/`).
- **Secrets must not be committed in real form:** `public/config.json` contains only placeholders (real values are overridden at deployment time); `.env.local` (git-ignored) cho key Firebase phía web; the desktop build uses `src-tauri/src/secret_store.rs` cho more sensitive data thay vì `localStorage`.
- **Authentication:** ID token Firebase gắn theo từng request (`Authenticationorization: Bearer`), passwords are not stored on the client.

---

## 13. Testing Strategy

Vitest + Testing Library, môi trường `jsdom` (`vitest.config.ts`). Tests are placed next to source files (`Foo.tsx` + `Foo.test.tsx`) — currently about 85 file test trên 133 file nguồn phi-test.

Mandatory checklist when adding new code (theo `module-rules/frontend.md`):

- [ ] New component has at least 1 render test + 1 primary interaction test.
- [ ] New Zustand action has a unit test (input event → state kỳ vọng).
- [ ] New Tauri command has a Rust test trong `src-tauri/`.
- [ ] `npx tsc --noEmit` clean, do not use `any` to bypass backend type errors.

Run locally:

```bash
cd hydragrow-frontend
npx tsc --noEmit
npx eslint .
npx vitest run
cd src-tauri && cargo check && cargo clippy -- -D warnings

```

CI (`frontend-ci`, triggered when a PR touches `hydragrow-frontend/`) runs the same command set above.

---

## 14. Build & Deployment

| Target Command Output Runs on  |                                           |                                  |                                                                                    |
| ---------------------------- | ----------------------------------------- | -------------------------------- | ---------------------------------------------------------------------------------- |
| Web                          | `npm run build:web` (`tsc && vite build`) | `dist/`                          | Vercel — rewrites every route to `index.html` for React Router (`vercel.json`)        |
| Desktop                      | `npm run build:tauri`                     | `dist-tauri/` + bundle native    | Packaged for Windows/macOS/Linux (`bundle.targets: "all"` trong `tauri.conf.json`) |
| Web dev                      | `npm run dev:web`                         | Vite dev server                  | —                                                                                  |
| Desktop dev                  | `npm run dev:tauri`                       | Fixed port `1420` (HMR `1421`) | Tauri hard-code cổng này, do not change arbitrarily                                       |

Backend referenced in configuration: `hydragrow.onrender.com` (Render.com). `devUrl` trong `tauri.conf.json` points to the currently deployed Vercel build (`hydragrow-ashy.vercel.app`) so the desktop dev build can point to an already-running web build when needed.

---

## 15. Technical Debt & Points to Note When Reading Further

Recorded so future readers (human or agent) do not have to rediscover them:

- **`vite-plugin-pwa`** **has not been enabled.** Present in `devDependencies` và có comment TODO ("Thêm VitePWA vào bên trong mảng plugins") trong `vite.config.ts`, but the plugin is not actually present in the `plugins` — the web build currently has no PWA (installation/offline support) even though the dependency is ready.
- **`docs/route-content-audit.md`** **is outdated.** Bảng route trong đó (`/control`, `/blockchain`, `/analytics`...) does not match `App.tsx` current (has been consolidated into `/operations`, `/cultivation`, `/journal`). The table should be updated or clearly labeled "historical document" to prevent future readers from confusing it with current routes.
- **`/user-management`** **và** **`/roles`** **point to the same component** (`Roles`). If there is no business reason to keep both paths, they should be reduced to one official route.
- **`docs/hydragrow-hifi-spec.md`** **uses a different color palette** (`Green-700 #2E7D32`...) than the color palette actually being implemented trong `App.css`/CHUAN-GIAO-DIEN-FRONTEND.md (`primary-deep #14532D`...). This is a Lo-Fi→Hi-Fi spec from an earlier phase, not the current color source — it should be marked "historical" directly in that file.
- **Build** **`gleam_core`** **is a manual step**, not yet wired into `npm run build:web`/`dev:web` — modifying `.gleam` while forgetting `gleam build` will cause the changes not to appear in the build..
- **`SensorBentoCard.tsx`, `ConfigBackup.tsx`, `RecipeBuilder.tsx`** vẫn dùng màu Tailwind hardcode ngoài token — khoanh vùng có chủ đích trong Layer 2 rollout (2026-09-13), không phải bị bỏ sót; xem `colorAllowlist.json` mục `_comment_trackedDebt` và `docs/design-system/PR-QA-CHECKLIST.md` mục 9.

---

## 16. Related Documentation

| Tài liệu Content                                                                                                         |                                                                                  |
| ------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| [`CHUAN-GIAO-DIEN-FRONTEND.md`](https://claude.ai/chat/CHUAN-GIAO-DIEN-FRONTEND.md)                                       | Design tokens, shared classes, IA, UI checklist — source of truth for the *visual* layer |
| [`docs/ui-writing-guideline.md`](https://claude.ai/chat/docs/ui-writing-guideline.md)                                     | Rules for writing copy/labels/notifications                                                |
| [`docs/route-content-audit.md`](https://claude.ai/chat/docs/route-content-audit.md)                                       | Copy audit history (see section 15 — routes are outdated)                            |
| [`../docs/superpowers/specs/module-rules/frontend.md`](https://claude.ai/docs/superpowers/specs/module-rules/frontend.md) | Mandatory rules when opening a PR touching this module (authoritative source)                    |
| [`../README.md`](https://claude.ai/README.md)                                                                             | Overall system architecture, MQTT topics, database                                    |
| [`../hydragrow-shared/README.md`](https://claude.ai/hydragrow-shared/README.md)                                           | Types/structs shared between backend and frontend                                  |
| [`../CONTRIBUTING.md`](https://claude.ai/CONTRIBUTING.md)                                                                 | General PR process for the entire repository                                                     |