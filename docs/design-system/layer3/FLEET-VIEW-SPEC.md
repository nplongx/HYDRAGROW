# Fleet View UX Redesign Specification (Track C)

**Document Status:** Approved / Spec  
**Target:** `hydragrow-frontend/src/pages/FleetView.tsx`, `hydragrow-frontend/src/components/fleet/FleetStationCard.tsx`  
**Applied Skills:** `information-architecture`, `responsive-design`, `hicks-law`, `data-visualization`

---

## 1. Current State & Audit

### 1.1 Findings & Pain Points
- **Flat List Layout:** Stations were rendered in a single-column flat list inside a fixed `max-w-3xl` container. As device counts grow, visual hierarchy breaks down and scanning becomes slow.
- **Orphaned Route:** Fleet view was previously only linked from Settings > General.
- **Cognitive Load (Hick's Law violation):** No categorization or grouping for stations. A grower managing 5-10 stations across different crops (e.g., Lettuce vs Strawberries) had to parse every item sequentially.
- **Suboptimal Touch Targets:** Cards lacked a clear minimum 48px tap area for agricultural field environments where touch screens / mobile devices are operated with one hand or gloves.
- **Data Visualization & Color Hierarchy:** EC and pH chips used uniform muted backgrounds regardless of whether values drifted out of safe thresholds.
- **Unprioritized Sort Order:** Stations in critical warning states could be buried below healthy stations.

---

## 2. Information Architecture & Navigation

### 2.1 Warning-First Dynamic Sorting
In aeroponics and hydroponics, rapid anomaly triage takes precedence over alphabetical or chronological order:
```ts
filteredDevices.sort((a, b) => {
  const warnB = summaries[b.device_id]?.warning_count ?? 0;
  const warnA = summaries[a.device_id]?.warning_count ?? 0;
  if (warnB !== warnA) return warnB - warnA;
  // Secondary sort: online stations before offline
  const onlineA = a.is_online ? 1 : 0;
  const onlineB = b.is_online ? 1 : 0;
  return onlineB - onlineA;
});
```

### 2.2 Crop-Based Grouping (Threshold: 4+ Devices)
- **Small Deployments (< 4 stations):** Render as a clean, cohesive responsive grid without artificial section headers.
- **Medium/Large Deployments (>= 4 stations):** Group stations by `crop` (e.g., "🌱 Xà lách", "🍓 Dâu tây", "🌿 Khác / Chưa thiết lập").
- Group headers clearly display the crop icon, localized crop name, and station count badge: e.g., `🌱 Xà lách (3 trạm)`.
- Reduces decision time (Hick's Law) by clustering related micro-climates and nutrient recipes.

---

## 3. Station Card Design (`FleetStationCard`)

### 3.1 Layout & Hierarchy
- **Container:** `<button>` element with minimum touch target (min-h-[48px], p-4, rounded-2xl), full keyboard navigation (`Tab` / `Enter` / `Space`), high contrast border with Tailwind v4 design tokens (`--color-line`, `--color-surface`).
- **Header:**
  - Online/Offline status indicator dot with pulse animation on online status.
  - Station name / label (text-base font-bold text-primary-deep truncate).
  - Hardware ID (subtle mono font, text-faint text-xs).
- **Badges:**
  - Crop stage pill (`bg-pill text-status text-xs font-semibold`).
  - Warning counter badge: when `warning_count > 0`, prominent amber/red badge (`bg-red-50 text-red-700 border-red-200 animate-pulse-subtle`).
- **Telemetry Chips (Data-Visualization):**
  - **EC (Electrical Conductivity):** Evaluated against standard safe hydroponic band (Normal: 1.0 - 2.5 mS/cm -> `--color-status` / `bg-pill`; Warning drift -> `--color-warning` / `bg-warning-bg`; Critical -> `--color-error` / `bg-danger-bg`).
  - **pH:** Evaluated against safe root absorption band (Normal: 5.5 - 6.5 -> `--color-status` / `bg-pill`; Drift -> `--color-warning` / `bg-warning-bg`; Critical -> `--color-error` / `bg-danger-bg`).
  - Fallback display: `EC — · pH —` when telemetry is pending or unavailable.
- **Footer:**
  - Relative time for `last_seen` (e.g., "Vừa xong", "5 phút trước", "2 giờ trước") formatted cleanly in Vietnamese.
  - Firmware version chip (`FW: v1.4.2`).
- **Accessibility:**
  - Full ARIA compliance: `aria-label="Trạm {label}: {status}, {warnings}, EC {ec}, pH {ph}"`.
  - Semantic interactive button semantics.

---

## 4. Responsive Multi-Column Grid

Adapts across screen viewports:
- **Mobile (< 640px):** Single column (`grid-cols-1`) with generous vertical tap spacing.
- **Tablet (640px - 1023px):** 2 columns (`sm:grid-cols-2 gap-3.5`).
- **Desktop (>= 1024px):** 3 columns (`lg:grid-cols-3 gap-4`).
- Container upgraded from restrictive `max-w-3xl` to fluid responsive `max-w-6xl mx-auto`.

---

## 5. Empty States & Error Handling

- **No Devices Linked:**
  - Friendly hydroponic tower illustration icon (`Cpu` / `Sprout`).
  - Title: "Chưa có trạm nào"
  - Description: "Tài khoản của bạn chưa liên kết trạm thủy canh nào. Hãy bắt đầu bằng cách ghép nối trạm đầu tiên."
  - Primary CTA button: "Liên kết thiết bị mới" -> navigates to `/pairing`.
- **Filtered No Warnings:**
  - Title: "Tất cả trạm hoạt động tối ưu"
  - Description: "Không có trạm nào ghi nhận cảnh báo bất thường về EC, pH hoặc kết nối."
  - Action: Quick button to reset filter to "Tất cả".

---

## 6. Q1 Extension Points (Household vs Commercial)

Designed defensively to accommodate commercial multi-station setups without breaking single-station household setups:
1. **P2 Map / Greenhouse Spatial Layout View:** Tab switcher toggle between "Dạng lưới" (Grid) and "Bản đồ nhà màng" (Greenhouse Map layout).
2. **Multi-Org / Zone Grouping:** Extension from single `crop` grouping to composite key `[organization_id, zone_id, crop]`.
3. **Batch Recipe Dispatch Button:** Header action enabling multi-select stations for bulk nutrient recipe updates (`/recipes/dispatch`).
4. **Station Side-by-Side Comparison Panel:** Select 2-4 stations to inspect synchronized sensor telemetry curves.
