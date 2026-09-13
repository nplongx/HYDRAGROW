# Pattern Library — Shared UI Components (`hydragrow-frontend`)

Single source of truth for the four shared UI patterns covered by the
design-system consistency audit. Every signature below is copied verbatim from
the `interface` definition in the cited file; every example is a real call site
copied from the live codebase — not invented.

Standard: `hydragrow-frontend/CHUAN-GIAO-DIEN-FRONTEND.md`.
Shared classes live in `hydragrow-frontend/src/App.css`.
Guards: `hydragrow-frontend/src/lib/design-lint/`.

---

## 1. `DeviceStatePill` — semantic status pill

**File:** `hydragrow-frontend/src/components/ui/DeviceStatePill.tsx`

**Props (verbatim):**

```tsx
export type DeviceState = 'online' | 'offline' | 'warning' | 'dosing' | 'auto' | 'manual';

interface DeviceStatePillProps {
  state: DeviceState | string;
  label?: string;
  className?: string;
}
```

State → token mapping (from `META` in the same file, also verbatim):

| `state` | label (default) | classes |
|---|---|---|
| `online` | `Trực tuyến` | `bg-success-bg text-status`, dot `bg-status` |
| `offline` | `Ngoại tuyến` | `bg-surface-muted text-faint`, dot `bg-faint` |
| `warning` | `Cảnh báo` | `bg-warning-bg text-warn-deep`, dot `bg-warn-deep` |
| `dosing` | `Đang châm` | `bg-info-bg text-info-fg`, dot `bg-info-fg` |
| `auto` | `Tự động` | `bg-pill text-primary`, dot `bg-primary` |
| `manual` | `Thủ công` | `bg-surface-muted text-text-muted`, dot `bg-text-muted` |

Unknown strings fall back to `manual`.

**Real example** (`hydragrow-frontend/src/pages/Roles.tsx:371-374`, added by the
Task 1 fix — previously a hand-rolled `bg-rose-50 text-rose-700` /
`bg-emerald-50 text-emerald-700` pill):

```tsx
<DeviceStatePill
  state={user.is_active ? 'online' : 'offline'}
  label={user.is_active ? 'Hoạt động' : 'Tạm dừng'}
/>
```

Other call site: `hydragrow-frontend/src/pages/Dashboard.tsx:141-144`
(`state={isOnline ? 'online' : 'offline'}` with `Trạm Online` / `Trạm Offline` labels).

**Don't:** hand-roll a status pill with raw Tailwind colors
(`bg-rose-50`, `bg-emerald-50`, `bg-red-100`, …). **Use `DeviceStatePill`
instead** — raw pills duplicate the `--color-status` / `--color-success-bg` /
`--color-error` tokens and are flagged by
`src/lib/design-lint/hardcodedColors.test.ts`. See the Roles.tsx before/after
(Task 1 commit `feat(design-lint): add hardcoded-color drift guard`) as the
reference example.

---

## 2. `Switch` — toggle control

**File:** `hydragrow-frontend/src/components/ui/Switch.tsx`

**Props (verbatim):**

```tsx
interface SwitchProps {
  checked?: boolean;
  isOn?: boolean;
  onChange?: (checked: boolean) => void;
  onClick?: (checked: boolean) => void;
  disabled?: boolean;
  label?: string;
  size?: 'sm' | 'md';
  colorClass?: string;
}
```

Notes: `checked` wins over `isOn` when both are given; both `onChange` and
`onClick` receive the *next* boolean. Renders a native
`<button role="switch" aria-checked={…}>` (default track `bg-primary` when on,
`bg-toggleoff` when off; override via `colorClass`).

**Real example** (`hydragrow-frontend/src/components/automation/FlowEditorHeader.tsx:44-48`,
added by the Task 2 fix — previously a raw `<input type="checkbox"
className="ui-switch">`, where `ui-switch` was never defined in `App.css`):

```tsx
<Switch
  checked={enabled}
  onChange={(next) => onChange({ enabled: next })}
  label="Kích hoạt"
/>
```

Other call sites: `src/pages/settings/ThresholdsSection.tsx:248-301` (sensor/auto-refill
toggles via `isOn` + `onClick`), `src/pages/settings/GeneralSection.tsx:140`
with `colorClass="bg-amber-600"`.

Tests assert on `getByRole('switch')` — see `src/components/ui/Switch.test.tsx`.

**Don't:** write a raw `<input type="checkbox">` styled by hand for a toggle.
**Use `<Switch>` instead** — the raw checkbox bypasses the
`--color-primary` / `--color-toggleoff` tokens, has no `role="switch"`
semantics, and orphan-class lint (`orphanClasses.test.ts`) will flag
one-off classes like `ui-switch`.

---

## 3. `StateView` — loading / empty / error placeholder

**File:** `hydragrow-frontend/src/components/ui/StateView.tsx`

**Props (verbatim):**

```tsx
interface StateViewProps {
  icon: LucideIcon | React.ElementType;
  title: string;
  description?: string;
  action?: React.ReactNode;
  className?: string;
}
```

Renders `.ui-state` > `.ui-state-icon` (icon at `size={32}`) > `.ui-state-title`
> optional `.ui-state-desc` > optional `action`. Matches
`CHUAN-GIAO-DIEN-FRONTEND.md` §7 ("Rỗng / chưa cấu hình: `.ui-state` +
`.ui-state-title` + `.ui-state-desc`").

**Real example** (`hydragrow-frontend/src/pages/SystemLog.tsx:223-227`):

```tsx
<StateView
  icon={Zap}
  title="Dòng thời gian trống"
  description="Chưa ghi nhận khoảnh khắc nào khớp bộ lọc/tìm kiếm hiện tại."
/>
```

Other call sites: `src/pages/DosingHistory.tsx:129-133` (error state with
`icon={AlertTriangle}`), `src/pages/DosingHistory.tsx:146-150` (empty state
with `icon={Box}` + `description`), `src/components/seasons/SeasonHistoryList.tsx:32`
(`icon={History}`, `title="Chưa có lịch sử mùa vụ"`).

**Don't:** build a bespoke empty/error `div` (custom icon circle + heading +
caption) per page. **Use `StateView` instead** — bespoke blocks drift into
orphan classes (the `.ui-state-icon` gap fixed in Task 2 is exactly what
happens when the pattern is copied instead of reused).

---

## 4. `EmergencyStopButton` — safety FAB / bottom bar

**File:** `hydragrow-frontend/src/components/safety/EmergencyStopButton.tsx`

**Props (verbatim):**

```tsx
interface EmergencyStopButtonProps {
  deviceId: string | null;
  variant: 'floating' | 'bar';
}
```

- `variant="floating"`: 56px circle FAB (`w-14 h-14`, `rounded-full`,
  `bg-red-600 hover:bg-red-700`), fixed bottom-right
  (`bottom-[76px] right-4 lg:bottom-6 lg:right-6`), `aria-label="Dừng khẩn cấp"`.
- `variant="bar"`: full-width bottom bar (`rounded-2xl`, same red), fixed
  bottom (`left-4 right-4 lg:left-[17rem] lg:right-6 lg:bottom-6`).
- Both open `EmergencyStopConfirmDialog`; confirm calls `emergencyStop()` from
  `useDeviceControl` and toasts `Đã gửi lệnh dừng khẩn cấp.` on success.

**Real examples** (both variants are live — one per page):

```tsx
// hydragrow-frontend/src/pages/Dashboard.tsx:315
<EmergencyStopButton deviceId={deviceId} variant="floating" />
```

```tsx
// hydragrow-frontend/src/pages/Operations.tsx:34
<EmergencyStopButton deviceId={deviceId} variant="bar" />
```

**Don't:** add a second red emergency-styled control on Dashboard or
Operations, and don't resize the FAB below `w-14 h-14` (56px clears both the
iOS HIG 44pt and Material 48dp minimums). Both properties are locked by
`src/components/safety/EmergencyStopButton.guard.test.tsx` (Task 4): a
touch-target assertion plus a source scan asserting no other
`bg-red-*/bg-danger/bg-error` element exists on either page (Von Restorff
uniqueness). Note for tests: the component takes `deviceId` + `variant` —
there is no `onConfirm` prop; render as
`<EmergencyStopButton deviceId={null} variant="floating" />`.

---

## 5. `pumpVisualTheme` + `PumpControlStatePill` — định danh & trạng thái bơm dosing

**Files:** `hydragrow-frontend/src/lib/dosing/pumpVisualTheme.ts`,
`hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.ts`,
`hydragrow-frontend/src/components/ui/PumpControlStatePill.tsx`.

`pumpVisualTheme.ts` là nguồn sự thật DUY NHẤT cho màu định danh bơm/van
(`pumpThemeFor(pumpId)` → `'nutrient' | 'phUp' | 'phDown' | 'aqua'`). Trước
khi có module này, `AdvancedDeviceControl.tsx` và `DosingReportCard.tsx` tự
vẽ 2 bảng màu khác nhau cho cùng khái niệm "bơm pH Up" — một dùng
`fuchsia`, một dùng `purple` — và `ControlPanel.tsx` từng truyền
`colorTheme="purple"` không khớp key nào, âm thầm rơi về theme mặc định
(bug thật, sửa tại `LAYER2-STATEMACHINE-003`, 2026-09-13).

**Don't:** định nghĩa lại một `Record<string, {activeIcon, glow, ...}>`
cục bộ trong component mới cho bơm/van. **Dùng `PUMP_VISUAL_THEME[pumpThemeFor(pumpId)]`
thay vào đó** — kể cả khi component đó không nằm trong
`src/components/control/` (ví dụ: chart legend, badge lịch sử châm phân).

`pumpControlStateMachine.ts`'s `derivePumpControlState()` tính trạng thái
3 chế độ (`idle` / `running` / `locked`, kèm `reason` khi `locked`) từ 4
cờ nguyên thuỷ (`currentStatus`, `isAutoMode`, `isEmergency`,
`lockedByPumpId`) — dùng hàm này thay vì tự viết lại biểu thức boolean
`isAutoMode || (isEmergency && !currentStatus) || Boolean(lockedByPumpId)`
ở nơi khác. `PumpControlStatePill` hiển thị kết quả đó theo đúng khuôn
mẫu chấm tròn + nhãn + token đã dùng ở `DeviceStatePill` (mục 1).

---

## 6. `DosingHourlyChart` / `Sparkline` — data-visualization dùng chung

**Files:** `hydragrow-frontend/src/components/dosing/DosingHourlyChart.tsx`,
`hydragrow-frontend/src/components/ui/Sparkline.tsx`.

`DosingHourlyChart` vẽ cột nhóm 24 giờ × 4 bơm, màu lấy từ
`pumpVisualTheme.ts` (mục 5), luôn kèm bảng `sr-only` làm text alternative
và `aria-label` theo từng cột giờ. `Sparkline` là đường xu hướng SVG tối
giản cho 1 chuỗi số — dùng cho chỉ số không có endpoint lịch sử ở backend
(xem `useHealthHistory.ts`: ring-buffer trong bộ nhớ phiên, không phải
lịch sử vĩnh viễn).

**Don't:** tự vẽ lại 1 dãy `<div style={{height}}>` mới cho biểu đồ cột —
đó chính là cách `DosingTotalCard.tsx` từng làm trước khi có
`DosingHourlyChart` (không nhãn trục, không chú giải, không text
alternative). **Dùng `DosingHourlyChart` cho dữ liệu nhiều-chuỗi-theo-giờ,
`Sparkline` cho 1 chuỗi xu hướng đơn giản.**

---

## 7. `pumpControlStateMachine` — trạng thái điều khiển thiết bị 3 chế độ

**File:** `hydragrow-frontend/src/lib/dosing/pumpControlStateMachine.ts`

`derivePumpControlState({currentStatus, isAutoMode, isEmergency,
lockedByPumpId})` → `{state: 'idle'|'running'|'locked', reason:
'auto_mode'|'emergency'|'interlock'|null}`. `running` luôn thắng mọi lý do
khoá (bơm đang thực sự chạy thì không có ý nghĩa hiển thị "đã khoá" cho
lệnh BẬT tiếp theo). Hiển thị qua `PumpControlStatePill` (mục 5); lý do
khoá `interlock` còn được hiển thị đầy đủ qua `Banner` (xem
`AdvancedDeviceControl.tsx`).

**Don't:** viết lại biểu thức `isAutoMode || (isEmergency && !currentStatus)
|| Boolean(lockedByPumpId)` ở component khác. **Import
`derivePumpControlState` thay vào đó** — logic ưu tiên giữa 3 lý do khoá
(`auto_mode > emergency > interlock`) chỉ nên tồn tại ở một nơi.

---

## 8. `SeasonStageChecklist` + `SeasonCompletionSummary` — động lực mùa vụ (Zeigarnik & Peak-End)

**Files:** `hydragrow-frontend/src/components/seasons/SeasonStageChecklist.tsx`,
`hydragrow-frontend/src/components/seasons/SeasonCompletionSummary.tsx`,
`hydragrow-frontend/src/lib/seasons/seasonProgress.ts`.

`SeasonStageChecklist` (Zeigarnik) trực quan hoá các giai đoạn mùa vụ (đã xong,
hiện tại, sắp tới) cùng số ngày còn lại (mở vòng lặp tâm lý giúp người dùng chủ động theo dõi).
`SeasonCompletionSummary` (Peak-End) là dialog chúc mừng và tổng kết các chỉ số khi kết thúc
mùa vụ (lưu lại ấn tượng tích cực ở thời điểm hoàn thành hành trình).

**Don't:** kết thúc mùa vụ đột ngột bằng màn hình tạo mùa mới trống trơn hoặc chỉ hiển thị thanh tiến độ
phần trăm tĩnh. **Dùng `SeasonStageChecklist` để duy trì sự chú ý trong suốt mùa vụ, và
`SeasonCompletionSummary` để tạo điểm nhấn hoàn thành có ý nghĩa.**

---

## 9. `validateDeviceId` — xác thực mã thiết bị ghép nối (form-design)

**Files:** `hydragrow-frontend/src/lib/pairing/deviceIdValidation.ts`,
`hydragrow-frontend/src/pages/DevicePairing.tsx`.

Hàm xác thực thuần `validateDeviceId(raw)` kiểm tra mã thiết bị theo quy tắc:
không rỗng, chỉ gồm ký tự chữ, số, dấu gạch dưới `_`, dấu gạch ngang `-`, và độ dài 3-64 ký tự.
Được gọi `onBlur` để kích hoạt validation nội tuyến (inline error) kèm thuộc tính `aria-invalid` và `aria-describedby`,
đồng thời chặn submit khi mã không hợp lệ.

**Don't:** chỉ kiểm tra mã thiết bị khi bấm nút "Ghép nối" hoặc để backend trả lỗi về mới báo người dùng.
**Dùng `validateDeviceId` khi blur và trước khi submit** để người dùng phát hiện lỗi ngay tại trường nhập.

---

## 10. `EVENT_CATEGORY_THEME` & `splitByMatch` — tìm kiếm và phân loại nhật ký (search-ux)

**Files:** `hydragrow-frontend/src/lib/logs/eventCategoryTheme.ts`,
`hydragrow-frontend/src/lib/logs/highlightMatch.ts`,
`hydragrow-frontend/src/components/logs/HealthSummaryBar.tsx`,
`hydragrow-frontend/src/components/logs/EventLogCard.tsx`.

`EVENT_CATEGORY_THEME` chuẩn hoá bảng màu tokenized cho từng nhóm sự kiện hệ thống (`ecDosing`, `phDosing`, `water`, `warning`, `device`),
thay thế các mã màu Tailwind hardcode (`text-indigo-*`, `text-purple-*`, `text-amber-*`).
`splitByMatch(text, query)` phân đoạn chuỗi văn bản thành mảng các đoạn khớp / không khớp để hiển thị highlight từ khoá tìm kiếm an toàn (tránh XSS, không dùng `dangerouslySetInnerHTML`).

**Don't:** dùng `dangerouslySetInnerHTML` với thẻ `<mark>` hoặc hardcode màu riêng lẻ trong từng card nhật ký.
**Dùng `splitByMatch` cho highlight tìm kiếm và `EVENT_CATEGORY_THEME` cho nhãn danh mục sự kiện.**

