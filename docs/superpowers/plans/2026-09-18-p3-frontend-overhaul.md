# P3 Frontend Overhaul Plan

## 1. Audit verdict

### Readiness

* **UI foundation:** READY
* **Design-system adoption:** READY, nhưng còn drift
* **Routing / page ownership:** READY
* **Data authority:** READY
* **Safety / command UX boundary:** READY
* **Responsive shell:** PARTIAL
* **IA:** PARTIAL
* **Persona model:** PARTIAL / chưa đủ evidence cho commercial
* **Component architecture:** PARTIAL
* **Regression safety:** HIGH, nhưng có test flake cần xử lý trước baseline P3

P3 có thể bắt đầu ngay.

Boundary:

> **Thay đổi cách frontend trình bày và điều hướng domain hiện có; không thay đổi domain truth.**

---

# 2. Những gì audit thực tế phát hiện

## 2.1 App shell đã có nền tốt

`App.tsx` hiện đã có:

* React Router canonical routes
* legacy-route redirect
* `StationContext`
* React Query
* auth gate
* capability gate
* device-scope gate
* route-level lazy loading
* recovery state cho `NoSelection`, `InvalidSelection`, `Unavailable`, `PermissionDenied`

Đây là nền đúng cho overhaul.

`MainLayout.tsx` cũng đã có:

* desktop sidebar
* mobile bottom navigation
* semantic `<nav>`
* `aria-current`
* safe-area handling
* scroll container riêng
* active-route handling
* connection/sensor/event context

### Không làm trong P3

Không thay:

* `StationContext` bằng store mới
* React Query bằng state framework khác
* route authority bằng page-local navigation
* thêm global frontend FSM

---

# 3. Route / IA audit

Canonical IA hiện tại:

```text
Dashboard
Operations
  ├── Control
  └── Automation
Cultivation
  ├── Seasons
  ├── Recipes
  └── Dosing History
Journal
  ├── Events
  └── Analytics
Settings
```

Utility:

```text
Pairing
Fleet
Config Backup
Roles
User Management
```

Đây là cấu trúc phù hợp với Page Contract hiện tại.

### Vấn đề

`routes.ts` đang đồng thời mang:

* canonical navigation
* legacy migration
* route scope
* capability
* semantic migration rules

Nó vẫn hoạt động, nhưng P3 nên biến route manifest thành **navigation/page contract boundary**, không để UI tự suy diễn IA từ pathname.

### P3 target

```text
route manifest
    ↓
page contract
    ↓
navigation model
    ↓
layout
    ↓
page
```

Không:

```text
page → tự biết route → tự quyết navigation → tự quyết scope
```

---

# 4. Design-system audit

Có component foundation thực sự:

```text
ui/
  Button
  Badge
  Banner
  StatusPill
  DeviceStatePill
  PumpControlStatePill
  FsmStatusBadge
  InputGroup
  LoadingState
  StateView
  PageHeader
  TabShell
  AccordionSection
  SubCard
  QuickActionBar
  Sparkline
  HealthScore
```

Và đã có automated design lint:

```text
lib/design-lint/
  contrast
  hardcodedColors
  orphanClasses
```

Đây là điểm mạnh.

### Nhưng UI implementation vẫn có drift

Audit class inventory cho thấy rất nhiều utility class trực tiếp:

* `bg-white`
* `border-*`
* `text-*`
* `rounded-*`
* spacing
* font sizing
* state colors

Trong khi design system đã có semantic primitives như:

```text
ui-card
ui-btn-primary
ui-input
ui-tab
ui-form-label
farm-section-title
bg-page-bg
text-primary-deep
text-text-muted
bg-surface-muted
```

=> P3 không nên tiếp tục tăng số lượng raw Tailwind composition.

### Rule P3

Ưu tiên:

```tsx
<Card>
<PageHeader>
<StatusPill>
<Section>
<EmptyState>
<StateView>
<ActionBar>
```

thay vì mỗi page tự compose:

```tsx
<div className="bg-white border rounded-xl p-4 ...">
```

Không cần wrapper cho mọi `<div>`. Chỉ chuẩn hóa những semantic pattern lặp lại.

---

# 5. Page complexity là bottleneck lớn nhất

Các file lớn nhất hiện tại:

```text
NodeEditorPanel.tsx       ~36.8 KB
ThresholdsSection.tsx     ~32.6 KB
Settings.tsx              ~31.3 KB
RecipeBuilder.tsx         ~27.1 KB
Dashboard.tsx             ~21.4 KB
DevicePairing.tsx         ~19.4 KB
FleetView.tsx             ~17.7 KB
SystemLog.tsx             ~16.1 KB
Analytics.tsx             ~13.7 KB
Automation.tsx            ~13.5 KB
```

Đây là bằng chứng rõ nhất rằng P3 nên là **boundary extraction**, không phải rewrite.

---

# 6. Settings là hotspot số 1

`Settings.tsx` đang gánh quá nhiều:

* application settings
* device config
* OTA
* WiFi
* calibration
* dosing validation
* persistence
* config payload construction
* save lifecycle
* dangerous actions
* section navigation

Đặc biệt đoạn save đang trực tiếp build payload cực lớn với hàng chục config field.

### P3 target

Tách theo semantic responsibility:

```text
Settings
├── SettingsHeader
├── SettingsNavigation
├── GeneralSettings
├── OperatingThresholds
├── DosingCalibration
├── ConnectivitySettings
├── FirmwareSettings
└── DangerZone
```

Logic:

```text
Settings page
    ↓
useDeviceConfig / existing hooks
    ↓
domain-specific section
    ↓
existing API/config authority
```

Không tạo settings store mới.

---

# 7. Automation là hotspot số 2

Automation hiện đã có architecture khá tốt về domain:

```text
Automation
├── overview
├── config explorer
├── flow editor
├── drawer
├── ReactFlow
├── node palette
├── node editor
├── condition editor
├── config inspector
├── test panel
└── multi-device template
```

Nhưng UI/editor complexity đang tập trung mạnh vào `NodeEditorPanel.tsx`.

### P3 target

Tách editor theo semantic node family:

```text
NodeEditorPanel
├── TriggerEditor
│   ├── SensorTrigger
│   ├── FSMTrigger
│   ├── CronTrigger
│   └── WebhookTrigger
├── ConditionEditor
├── ActionEditor
├── ConfigOverrideEditor
└── EditorFooter
```

Giữ nguyên:

* IR
* compiler
* cycle detection
* backend contract
* flow semantics

Không tạo automation FSM mới.

---

# 8. Dashboard

Dashboard hiện đã sử dụng đúng các domain primitive quan trọng:

* telemetry
* device status
* health
* pump state
* fault explanation
* dosing summary
* quick actions

Nhưng implementation vẫn chứa khá nhiều domain-to-display mapping trực tiếp:

```text
formatNumber
getTdsSetting
sensorStatus
fault mapping
pump extraction
config extraction
```

### P3 target

Tạo presentation selectors:

```text
dashboard/
  selectors.ts
  view-model.ts
```

Ví dụ:

```text
Telemetry
   ↓
DashboardViewModel
   ↓
SensorBentoCard
StatusSummary
QuickActionBar
DosingSummaryCard
```

Chỉ là **presentation transformation**.

Không cache domain data lần hai.

---

# 9. Fleet

Fleet đã được xây đúng hướng P2:

* global scope
* grid/map
* comparison
* selected devices
* backend compare
* không biến fleet thành default navigation

### P3

Chỉ làm:

* visual hierarchy
* scanability
* comparison UX
* responsive behavior

Không đưa Fleet lên primary nav mặc định khi discovery chưa chứng minh commercial-first.

---

# 10. Journal / Analytics / SystemLog

Đây là nơi P3 có cơ hội gom mental model.

Hiện có:

```text
Journal
SystemLog
Analytics
DosingHistory
```

và legacy routes đã map về Journal/Cultivation.

### P3 target

Journal trở thành semantic container:

```text
Journal
├── Timeline / Events
├── Operational history
└── Analytics
```

Nhưng không flatten mọi thứ thành một giant page.

Analytics vẫn phải phân biệt:

```text
observed telemetry
derived statistics
external Grafana
```

Không gọi derived metrics là "AI insight" hoặc agronomic recommendation khi chưa có evidence/model authority.

---

# 11. Data authority audit

Hiện tại boundary nhìn chung đúng:

```text
PostgreSQL/config/API
        ↓
React Query
        ↓
page/components

WebSocket
        ↓
realtimeRouter
        ↓
invalidate/refetch
        ↓
React Query
```

`StationContext` giữ selected station/device context.

Đây là architecture cần bảo vệ.

### Một số legacy/local state còn tồn tại

Ví dụ:

* `useFleetStatus`
* `useOwnedDevices`
* `Settings`
* `DevicePairing`
* `Analytics`
* `useDeviceSync`

Không nên xóa đồng loạt.

P3 chỉ migrate khi local state đang đóng vai trò **server-state cache**.

Local UI state vẫn hợp lệ:

```text
modal open
selected tab
draft input
wizard step
filter
expanded section
temporary form state
```

---

# 12. Authority violation cần tránh

Một số code hiện tại đáng chú ý:

### `SeasonPhotoJournal`

Có direct upload:

```text
fetch → Cloudinary
```

Đây là external integration, không phải domain-state authority.

P3 nên giữ integration boundary isolatable.

### `Settings`

Có:

```text
window.dispatchEvent('hydragrow:settings-updated')
```

Đây là dấu hiệu legacy event coupling.

Không cần rewrite ngay.

P3 nên dần thay bằng:

```text
mutation
  ↓
query invalidation
  ↓
React Query
```

chỉ khi contract hiện tại cho phép.

Không tạo event bus mới.

---

# 13. Type safety audit

Có một lượng `any` đáng kể.

Các hotspot:

```text
Settings
Dashboard
MetadataRenderers
Automation
ReactFlow nodes
Roles
DosingReportCard
OwnedDevices/Fleet
```

Đáng chú ý:

```text
Settings.tsx
ThresholdsSection.tsx
NodeEditorPanel.tsx
ConfigNodeInspector.tsx
MetadataRenderers.tsx
```

### P3 policy

Không làm "replace all any".

Ưu tiên theo boundary:

```text
API payload
↓
schema / typed model
↓
view model
↓
component
```

`any` trong test fixtures có thể để lại nếu không tạo risk.

---

# 14. Accessibility

Có nhiều tín hiệu tốt:

* semantic nav
* `aria-current`
* labeled controls
* role grid cho PermissionMatrix
* keyboard navigation
* screen-reader text cho chart
* emergency stop guard
* touch target tests
* form labels

Nhưng cần biến thành P3 acceptance gate, không chỉ component tests.

### P3 required

Test các viewport:

```text
375
768
1024
1440
```

và:

* keyboard-only
* focus visibility
* modal/drawer focus
* reduced motion
* no horizontal overflow
* safe-area mobile navigation

---

# 15. Responsive shell

Current shell:

```text
desktop:
fixed 256px sidebar
main margin-left 256px

mobile:
top header
bottom navigation
safe-area inset
```

Đây là viable.

Nhưng page-level responsive behavior chưa được centralize.

Có nhiều pattern tự quyết:

```text
p-4 sm:p-6 lg:p-8
grid-cols-1
sm:grid-cols-2
lg:grid-cols-...
sticky
overflow-x-auto
```

### P3

Định nghĩa:

```text
AppShell
PageContainer
PageHeader
SectionGrid
ResponsiveToolbar
BottomActionBar
```

với breakpoint contract chung.

Không bắt mọi page phải giống nhau; chỉ thống nhất structural rules.

---

# 16. Discovery impact

Discovery vẫn để mở câu hỏi:

```text
single-station household
vs
multi-station commercial
```

Do đó P3 IA phải là:

> **single-station-first nhưng fleet-capable**

chứ không:

> hobbyist-only

và cũng không:

> commercial operations console

### Practical consequence

Primary nav giữ:

```text
Tổng quan
Vận hành
Canh tác
Nhật ký
Cài đặt
```

Fleet là secondary/global utility.

Multi-station capabilities có thể xuất hiện contextually:

```text
Pairing
Fleet
device selector
bulk/template actions
```

nhưng không chiếm toàn bộ IA.

---

# 17. P3 workstreams

## P3.0 — Contract freeze

Trước khi UI rewrite:

1. Freeze semantic vocabulary.
2. Freeze canonical route ownership.
3. Freeze page scopes.
4. Freeze primary navigation.
5. Freeze state hierarchy:

   * loading
   * unavailable
   * stale
   * degraded
   * fault
   * command pending
   * command applied
6. Freeze responsive shell.

Output:

```text
P3 UI contract
P3 navigation contract
P3 component inventory
```

---

## P3.1 — App Shell

Target:

```text
MainLayout
├── DesktopSidebar
├── MobileHeader
├── MobileNav
├── PageContainer
└── GlobalStatusLayer
```

Acceptance:

* existing routes unchanged
* existing legacy redirects unchanged
* station scope unchanged
* capability gates unchanged
* mobile safe area preserved
* no content hidden under fixed nav

---

## P3.2 — Design primitives

Consolidate only repeated semantic patterns:

```text
PageHeader
Section
Card
StateView
EmptyState
ErrorState
StatusPill
ActionBar
TabShell
FilterBar
DataTable
Drawer
ConfirmDialog
```

Every primitive needs:

* keyboard behavior
* focus behavior
* responsive behavior
* semantic state
* tests where interaction is non-trivial

---

## P3.3 — Dashboard — DONE

Goal:

> one-screen answer to "trạm đang thế nào và tôi cần làm gì?"

Structure:

```text
Dashboard
├── StationStatus
├── CriticalState / Fault
├── CoreTelemetry
├── ActuatorState
├── ActiveOperation
├── QuickActions
└── RecentEvents
```

Do not add new analytics.

Do not add new recommendations.

---

## P3.4 — Operations

Structure:

```text
Operations
├── Control
│   ├── safe manual controls
│   ├── actuator state
│   └── command lifecycle
└── Automation
    ├── flows
    ├── filters
    ├── editor
    └── execution/test state
```

Critical rule:

```text
command requested
≠
device observed state
```

UI must preserve this distinction.

---

## P3.5 — Cultivation

Structure:

```text
Cultivation
├── Active Season
├── Season History
├── Recipes
└── Dosing History
```

Keep cultivation semantics separate from machine operations.

---

## P3.6 — Journal

Structure:

```text
Journal
├── Important
├── Technical
├── Timeline
└── Analytics
```

Reuse existing event grouping/filtering/metadata infrastructure.

Do not introduce second event model.

---

## P3.7 — Settings

Largest refactor.

Target:

```text
Settings
├── General
├── Operating limits
├── Dosing
├── Calibration
├── Connectivity
├── Firmware
└── Danger Zone
```

Advanced configuration should use progressive disclosure.

Do not expose the entire controller configuration as one giant form by default.

---

## P3.8 — Layer 3

Only after primary experience stabilizes:

```text
Fleet
Roles
User Management
Pairing
Config Backup
```

These remain utility/admin surfaces.

No commercial IA commitment until discovery validates it.

---

# 18. P3 implementation order

Recommended sequence:

```text
P3.0 Contract freeze
   ↓
P3.1 App Shell
   ↓
P3.2 UI primitives
   ↓
P3.3 Dashboard
   ↓
P3.4 Operations
   ↓
P3.5 Cultivation
   ↓
P3.6 Journal
   ↓
P3.7 Settings
   ↓
P3.8 Fleet / Roles / Admin
```

Do not start with Settings despite it being the largest hotspot.

First establish primitives and shell.

---

# 19. Refactor rules

## Allowed

* component extraction
* presentation view-model extraction
* route/navigation cleanup
* CSS/token consolidation
* responsive redesign
* accessibility improvements
* page composition changes
* progressive disclosure
* drawer/modal redesign
* query invalidation cleanup
* typed boundary improvements

## Not allowed

* new global state architecture
* replacing React Query
* replacing StationContext
* new frontend cache
* `/twin`
* new domain FSM
* controller logic in React
* duplicate safety logic
* new backend API solely to make UI easier
* invented threshold values
* invented commercial permissions
* AI/cloud Digital Twin
* making Fleet primary without discovery evidence

---

# 20. Definition of Done

Every P3 page must satisfy:

### Contract

* page has explicit scope
* page has explicit owner
* page has explicit loading state
* page has explicit error state
* page has explicit unavailable state
* page distinguishes stale/degraded where relevant

### Design

* uses design tokens/primitives
* no arbitrary new visual language
* consistent typography
* consistent spacing
* consistent status semantics

### Responsive

Verified at:

```text
375px
768px
1024px
1440px
```

### Accessibility

* keyboard navigation
* visible focus
* labels
* semantic controls
* color not sole state signal
* reduced-motion support
* touch targets

### Data

* React Query remains server-state authority
* StationContext remains station selection authority
* no duplicated server cache
* realtime remains best-effort invalidation/refetch

### Regression

Required:

```text
npm run build
npx vitest run
npx eslint .
npx tsc --noEmit
```

plus page-specific tests.

---

# 21. Current baseline evidence

Direct audit confirmed:

```text
Vitest:
127 test files
612 tests
PASS
```

There are React `act(...)` warnings in `useDeviceSync` tests.

A subsequent full verification run exposed one flaky/failing `SystemLog` test:

```text
SystemLog date grouping
"ẩn nút Tải thêm khi trang cuối trả về ít hơn PAGE_SIZE sự kiện"
```

while the rest of the run continued successfully.

Therefore:

> **P3 baseline regression gate = NOT YET CLEAN.**

Do not weaken the test. Fix the determinism/async contract before using this as the permanent P3 baseline.

The audit also confirmed a successful production build in the same verification sequence:

```text
vite
2225 modules transformed
build completed
```

The generated bundle shows useful current split points, but two large chunks remain notable:

```text
Automation     ~363 KB
DevicePairing  ~376 KB
index          ~461 KB
```

These should be treated as optimization targets, not reasons to rewrite architecture.

---

# 22. First P3 PR

The first implementation PR should be deliberately small:

```text
P3.1 App Shell
+
P3.2 core UI primitives
+
route/page contract enforcement
```

Not Dashboard + Settings + Automation simultaneously.

Acceptance:

```text
existing functionality unchanged
existing routes unchanged
existing authority unchanged
visual system becomes consistent
responsive shell becomes stable
```

Then Dashboard becomes the first full page migration.

---

# 23. Final assessment

Frontend is **architecturally ready for overhaul**.

The main problem is no longer missing infrastructure.

The problem is:

```text
too much responsibility inside pages
+
visual patterns still partially hand-composed
+
some legacy state/event coupling
+
large automation/settings surfaces
+
IA still intentionally Q1-neutral
```

Therefore P3 should be treated as:

> **progressive frontend decomposition around existing domain contracts**

not:

> frontend rewrite.

The safest high-leverage path is:

```text
Shell
→ primitives
→ Dashboard
→ Operations
→ Cultivation
→ Journal
→ Settings
→ Layer 3
```

with the existing domain/data/safety architecture kept intact.
