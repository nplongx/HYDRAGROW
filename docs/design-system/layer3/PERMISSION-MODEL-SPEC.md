# HydraGrow Permission Model & Roles Component Specification (Track A)

**Document Status:** Approved / Production Spec  
**Target Components:**
- `hydragrow-frontend/src/components/roles/PermissionMatrix.tsx`
- `hydragrow-frontend/src/components/roles/RoleBadge.tsx`
- `hydragrow-frontend/src/components/roles/RoleSelector.tsx`
- `hydragrow-frontend/src/components/roles/index.ts`
- Consumer files: `hydragrow-frontend/src/pages/Roles.tsx`, `hydragrow-frontend/src/components/roles/InviteForm.tsx`  
**Applied Skills:** `component-spec`, `hicks-law`, `accessibility-audit`  
**Review Target:** WCAG 2.2 AA Compliance, Tailwind v4 Design Token Conformance, Layer 3 Architecture

---

## 1. Executive Summary & Design Rationale

HydraGrow's access control bridges low-level IoT actuation (pumps, foggers, emergency stop, Wi-Fi provisioning, OTA firmware) with collaborative greenhouse crop management (nutrient recipes, EC/pH thresholds, member invitations).

### 1.1 Cognitive Load Reduction (Hick's Law)
At the backend database and API level (`hydragrow-backend/src/api/admin_users.rs`), authorization is governed by granular OAuth-style string scopes (`read:telemetry`, `write:config`, `control:pump`, `control:emergency`, `device:ota`, `device:network`, `script:write`, `recipe:write`, `user:invite`, `*`). 

Exposing 9+ raw checkboxes during user invitation or member management directly violates **Hick's Law** ($RT = a + b \cdot \log_2(n + 1)$), causing severe decision paralysis and dangerous operator error (such as accidentally granting emergency actuator control to a casual farm visitor).

**The Solution:**
1. **Three-Tier Pre-Packaged Archetypes:** Chunk permissions into 3 well-defined, distinct roles:
   - `admin` (Quản trị viên) — Full system governance & member provisioning (`*`).
   - `operator` (Vận hành viên) — Day-to-day cultivation, sensor monitoring, recipe adjustment, hardware OTA & network setup.
   - `viewer` (Người xem) — Read-only telemetry inspection for observers, interns, or off-site owners.
2. **Recommended Smart Default:** Default all invite flows to `operator` with clear visual recommendation, eliminating decision fatigue for standard team onboarding.
3. **Progressive Disclosure:** Expose high-level functional capabilities in a matrix view, keeping raw OAuth scopes tucked into tooltip previews or developer inspection views.

---

## 2. Component Specification: `PermissionMatrix`

### 2.1 Overview
The `PermissionMatrix` visualizes the capabilities permitted across system roles in a responsive grid. It builds grower and administrator confidence by clearly demonstrating what each role can and cannot execute on the physical aeroponics towers.
- **When to use:** On the `/roles` management page, inside role definition modals/drawers, or within security audit screens.
- **When NOT to use:** For inline single-member permission toggles (permissions in HydraGrow are role-bound, not arbitrary per-user checkboxes).

### 2.2 Anatomy
- **Container (`div.ui-card`):** Elevated card container featuring standard border line and subtle shadow tokens.
- **Header Section:**
  - Card Title: "Ma trận phân quyền (5 Năng lực × 3 Vai trò)" (`h2.farm-section-title`).
  - Subtitle: Explanatory subtitle text (`p.text-xs.text-text-muted`).
- **Table Grid (`table[role="grid"]`):**
  - **Header Row (`thead tr[role="row"]`):**
    - Capability Column Header (`th[role="columnheader"]`): "Năng lực hệ thống" (width: 50%).
    - Role Column Headers (`th[role="columnheader"]`): Center-aligned headers for Admin, Operator, Viewer with embedded `RoleBadge`.
  - **Data Rows (`tbody tr[role="row"]`):**
    - Capability Header Cell (`td[role="rowheader"]`): Title (`font-semibold text-primary-deep`) and description (`text-xs text-text-muted`).
    - Permission Cells (`td[role="gridcell"]`): Center-aligned status cell displaying check or cross indicator badge.
- **Mobile Stacked Cards (`div.space-y-3 sm:hidden`):**
  - Card per capability showing title, description, and 3 inline status chips for Admin, Operator, and Viewer.

### 2.3 Variants
- **Default Table Variant:** Full tabular grid with spacious cell padding (`py-3 px-4`), optimal for desktop and tablet screens ($\ge 640\text{px}$).
- **Compact Variant (`compact={true}`):** Reduced padding (`py-2 px-3`) and tighter typography for embedding within dialogs, flyout drawers, or multi-column layout panes.
- **Responsive Stacked Card Variant:** Automatically rendered on mobile screens ($< 640\text{px}$) via CSS media query container styling, transforming columns into stacked cards to eliminate horizontal scrolling.

### 2.4 Props/API

| Prop Name | Type | Default | Required | Description |
|---|---|---|---|---|
| `capabilities` | `Capability[]` | `CAPABILITIES` | No | List of capability items to display as rows |
| `roles` | `UserRole[]` | `['admin', 'operator', 'viewer']` | No | Array of role keys to display as columns |
| `compact` | `boolean` | `false` | No | Enables compact visual styling for tight containers |
| `className` | `string` | `''` | No | Additional CSS classes for the outer wrapper |
| `data-testid` | `string` | `'permission-matrix'` | No | Test identifier for Vitest and Testing Library |

### 2.5 States
- **Default (Read-only Display):** Standard high-contrast grid rendering capability rows and role status columns.
- **Interactive Focus State:** When a grid cell receives keyboard focus, a distinct ring appears (`ring-2 ring-primary ring-offset-1 rounded-md outline-none`).
- **Hover State:** Row hover background transition (`hover:bg-soft/40 transition-colors`).
- **Empty State:** If `capabilities` array is empty, renders a centered fallback message ("Chưa có năng lực nào được khai báo").

### 2.6 Behavior
- **Keyboard Traversal:** Grid cells support arrow key navigation (`ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `Home`, `End`), updating focused cell coordinates without scrolling the entire window.
- **Dynamic Role Columns:** When `roles` prop changes, the grid dynamically calculates column count and adjusts header and cell rendering accordingly.
- **Dynamic Capabilities:** Accepts custom capability arrays, enabling commercial tier extension without modifying component internals.

### 2.7 Accessibility (WCAG 2.2 AA)
- **Role Semantics:** Table element uses `role="grid"`, table rows use `role="row"`, header cells use `role="columnheader"`, row titles use `role="rowheader"`, and status cells use `role="gridcell"`.
- **Roving Focus / TabIndex:** The active cell has `tabIndex={0}`; all other cells have `tabIndex={-1}`.
- **Screen Reader Announcements:** Each `role="gridcell"` includes an explicit, localized `aria-label`:
  - Allowed: `"Vận hành viên: Có quyền OTA & Mạng thiết bị"`
  - Denied: `"Người xem: Không có quyền Điều khiển & Vận hành"`
- **Non-Color Perception (WCAG 1.4.1):** Permission status uses both icon shape (Check vs X) and semantic background colors (`bg-pill` vs `bg-surface-muted`), guaranteeing distinguishability for color-blind users.

### 2.8 Usage Guidelines
- **Do:** Place the `PermissionMatrix` directly below the member management table on `/roles` so administrators can reference capabilities before altering role assignments.
- **Do:** Keep capability titles clear, concise, and accompanied by concrete system actions in parentheses (e.g. "Pumps / E-Stop", "Recipes / Config").
- **Don't:** Add interactive edit toggles directly into this matrix unless building a custom role editor for commercial tiers.
- **Don't:** Hide descriptions on desktop viewports; clarity of physical impact is critical in agricultural automation.

---

## 3. Component Specification: `RoleBadge`

### 3.1 Overview
`RoleBadge` standardizes role presentation pills across the HydraGrow web application. It communicates user authorization levels at a glance in tables, member cards, activity feeds, and navigation bars.
- **When to use:** Next to user avatars, in member list tables, in invitation confirmation cards, and inside role selector chips.
- **When NOT to use:** To represent hardware connectivity or telemetry status (use `DeviceStatePill` or `farm-status-pill` instead).

### 3.2 Anatomy
- **Badge Container (`span`):** Pill-shaped container (`rounded-full`) with inline-flex alignment.
- **Icon (Optional):** Optional leading icon (`Shield` for admin, `Wrench` or `Cpu` for operator, `Eye` for viewer).
- **Label Text:** Localized Vietnamese role name (`Quản trị viên`, `Vận hành viên`, `Người xem`).

### 3.3 Variants
- **Size `sm`:** `px-2 py-0.5 text-[11px] font-semibold rounded-full gap-1` (used in dense lists and table headers).
- **Size `md` (Default):** `px-2.5 py-1 text-xs font-semibold rounded-full gap-1.5` (used in standard member cards and form summaries).
- **Style Roles:**
  - `admin`: `bg-amber-100 text-amber-800 border border-amber-200/60`
  - `operator`: `bg-pill text-status border border-emerald-200/60`
  - `viewer`: `bg-surface-muted text-text-muted border border-line`

### 3.4 Props/API

| Prop Name | Type | Default | Required | Description |
|---|---|---|---|---|
| `role` | `UserRole` | — | Yes | The system role to render |
| `size` | `'sm' \| 'md'` | `'md'` | No | Badge sizing variant |
| `label` | `string` | `undefined` | No | Optional override for display label |
| `className` | `string` | `''` | No | Additional CSS class overrides |

### 3.5 States
- **Normal:** Crisp text on tinted background adhering to WCAG 2.2 AA contrast ratios (> 4.5:1).
- **Hover:** Subtly dims background on interactive parents (`hover:brightness-95`).
- **Disabled / Inactive:** If associated with a deactivated account, parent styles can apply `opacity-60`.

### 3.6 Behavior
- Self-contained presentation component with no internal state or side effects.
- Gracefully falls back to `'viewer'` styling if an unrecognized role string is passed.

### 3.7 Accessibility (WCAG 2.2 AA)
- Rendered with semantic text inside `<span>`.
- Inherits adequate contrast: `text-amber-800` on `bg-amber-100` (contrast ratio ~ 5.8:1), `text-status` (`#047857`) on `bg-pill` (`#d1fae5`) (contrast ratio ~ 4.9:1), `text-text-muted` (`#4b6354`) on `bg-surface-muted` (`#f1f6f2`) (contrast ratio ~ 4.8:1).
- Screen readers read the localized text directly without extra ARIA markup.

### 3.8 Usage Guidelines
- **Do:** Use `RoleBadge` consistently across all pages rather than hardcoding Tailwind color classes inline.
- **Do:** Use `size="sm"` inside dense table headers and `size="md"` in user profiles or dialog summaries.
- **Don't:** Override role color mappings arbitrarily; visual consistency preserves mental models.

---

## 4. Component Specification: `RoleSelector`

### 4.1 Overview
`RoleSelector` is an accessible form control dropdown and contextual preview component that allows operators to assign roles while clearly previewing the granted system scopes.
- **When to use:** In member invitation modals (`InviteForm`), member role update cells in tables, and settings forms.
- **When NOT to use:** When displaying a static role assignment (use `RoleBadge`).

### 4.2 Anatomy
- **Form Label (`label`):** Standard label with optional required asterisk ("Vai trò phân bổ *").
- **Select Dropdown (`select.ui-input` / native select):** Styled dropdown listing role options with descriptive annotations.
- **Scope Preview Card (`div.farm-muted-panel`):** Progressive disclosure card rendered below the select when `showScopePreview` is enabled, explaining exact operational capabilities granted.
- **Helper / Error Text (`p.ui-helper-text` / `p.text-error`):** Contextual validation or instructional guidance.

### 4.3 Variants
- **Standard Select Variant:** Clean `<select>` dropdown suitable for fast edits in table rows.
- **Card-Preview Variant (`showScopePreview={true}`):** Displays an active scope summary card beneath the dropdown, explaining what commands the user can dispatch to aeroponics hardware.

### 4.4 Props/API

| Prop Name | Type | Default | Required | Description |
|---|---|---|---|---|
| `value` | `UserRole` | — | Yes | Currently selected role |
| `onChange` | `(role: UserRole) => void` | — | Yes | Selection change callback |
| `showScopePreview` | `boolean` | `false` | No | Whether to render the descriptive scope preview card |
| `disabled` | `boolean` | `false` | No | Disables input interactions |
| `id` | `string` | `undefined` | No | ID for form label association |
| `className` | `string` | `''` | No | Custom wrapper CSS classes |
| `error` | `string` | `undefined` | No | Error message to display |

### 4.5 States
- **Default:** Unfocused select with standard line border (`border-line bg-white`).
- **Focus:** Highlighted border with primary ring (`focus:border-primary focus:ring-2 focus:ring-primary/20`).
- **Disabled:** Muted background (`bg-surface-muted cursor-not-allowed opacity-60`).
- **Error:** Red border (`border-error`) with error message rendered below.

### 4.6 Behavior
- **Hick's Law Smart Default:** When uninitialized in invite forms, defaults to `operator` (the recommended role).
- **Scope Preview Synchronization:** Whenever the dropdown changes value, the preview card instantly transitions to display the corresponding capability summary.
- **Form Integration:** Triggers standard change events compatible with standard React state and form libraries.

### 4.7 Accessibility (WCAG 2.2 AA)
- Associated with `<label>` via `htmlFor` / `id`.
- Native `<select>` guarantees full keyboard navigation (`Up`/`Down`, `Space`, `Enter`) across desktop and mobile screen readers.
- `aria-invalid` set to `true` when `error` prop is present.
- `aria-describedby` links error text or scope preview description to the input.

### 4.8 Usage Guidelines
- **Do:** Enable `showScopePreview={true}` on invitation forms where the user is deciding how much privilege to grant.
- **Do:** Highlight `operator` as the recommended default for aeroponics team members.
- **Don't:** Present raw OAuth strings (`control:pump`, `device:ota`) as primary labels; always translate into clear functional descriptions.

---

## 5. Standard Capability Dataset (Canonical Source of Truth)

The 5 baseline system capabilities mapped against backend OAuth scopes:

| Capability ID | Title (VI) | Description (VI) | Admin | Operator | Viewer | Backend Scopes Mapped |
|---|---|---|:---:|:---:|:---:|---|
| `telemetry` | Giám sát & Số liệu (Telemetry) | Xem thông số cảm biến thời gian thực, biểu đồ lịch sử và nhật ký sự kiện | ✓ | ✓ | ✓ | `read:telemetry` |
| `control` | Điều khiển & Vận hành (Pumps / E-Stop) | Bật/tắt bơm dinh dưỡng, phun sương, xả tràn và kích hoạt dừng khẩn cấp E-stop | ✓ | ✓ | ✗ | `control:pump`, `control:emergency` |
| `recipes` | Cấu hình nông học & Kịch bản (Recipes / Config) | Tạo và chỉnh sửa công thức mùa vụ, ngưỡng pH/EC, lịch trình chiếu sáng | ✓ | ✓ | ✗ | `write:config`, `recipe:write`, `script:write` |
| `device` | OTA & Mạng thiết bị (OTA & Device Network) | Cập nhật firmware OTA, thiết lập mạng WiFi trạm | ✓ | ✓ | ✗ | `device:ota`, `device:network` |
| `permissions`| Phân quyền & Mời thành viên (Permissions & Member Invitation) | Mời và đổi vai trò thành viên | ✓ | ✗ | ✗ | `user:invite` |

*Note: In Layer 2 Discrepancy 3, `CAPABILITIES[device].operator` was aligned to `true` to match backend `ROLE_DEFAULT_SCOPES.operator` which contains `device:ota` and `device:network`.*

---

## 6. Q1 Extension Points (Commercial vs Household Tiering)

HydraGrow is engineered defensively to scale from a single-tower household setup to a multi-acre commercial greenhouse facility without breaking existing role schemas or database contracts.

```
+-------------------------------------------------------------------------+
|                  HYDRAGROW ROLE & PERMISSION ARCHITECTURE               |
+-------------------------------------------------------------------------+
|                                                                         |
|  [Tier 1: Today - Fixed 3-Role Model]                                   |
|  • Admin (*)                                                            |
|  • Operator (telemetry, control, recipes, device)                       |
|  • Viewer (telemetry)                                                   |
|                                                                         |
|        │                                                                |
|        ▼ (Defensive Extension Hooks)                                    |
|                                                                         |
|  [Tier 2: Commercial Multi-Role Extension (N-Roles)]                    |
|  • "Contract Technician" (Kỹ thuật viên bảo trì bên ngoài):             |
|     - Scopes: read:telemetry, device:ota, device:network, control:pump  |
|     - Excluded: recipe:write, script:write (IP protection for recipes)  |
|  • "Agronomist Consultant" (Chuyên gia nông học tư vấn):               |
|     - Scopes: read:telemetry, recipe:write, script:write                |
|     - Excluded: device:ota, device:network, control:emergency           |
|                                                                         |
|        │                                                                |
|        ▼                                                                |
|                                                                         |
|  [Tier 3: Spatial & Per-Station Scoped Authorization]                   |
|  • Resource Scoping: Scope format upgraded to `action:resource:id`      |
|    e.g. `control:pump:station-vn-hcm-01`                                |
|  • Zone-Level Roles: Operator in Zone A (Dâu tây) but Viewer in Zone B |
|                                                                         |
|        │                                                                |
|        ▼                                                                |
|                                                                         |
|  [Tier 4: Priva-Model Multi-Tenant Site Sharing]                        |
|  • Federated Tenant Delegation: Farm Owner grants temporary access to   |
|    equipment vendor (Argus/Priva-style OEM maintenance).                |
|  • Expiring Delegation Tokens with automated revocation.                |
|                                                                         |
+-------------------------------------------------------------------------+
```

### 6.1 Extension 1: Specialized Commercial Roles (N-Roles)
- **Problem & Rationale:** In commercial agriculture, third-party contractors and agronomy consultants need partial, segregated privileges. Farm owners strictly guard proprietary crop recipes (pH/EC curves and dosing formulas are trade secrets) from external maintenance technicians. Conversely, consulting agronomists should adjust dosing recipes without having authority to flash firmware or reset network Wi-Fi keys.
- **Architectural Implementation:**
  1. **Contract Technician (`technician`):**
     - Allowed: `read:telemetry`, `control:pump`, `device:ota`, `device:network`.
     - Denied: `recipe:write`, `script:write`, `user:invite`.
  2. **Agronomist Consultant (`agronomist`):**
     - Allowed: `read:telemetry`, `write:config`, `recipe:write`, `script:write`.
     - Denied: `control:emergency`, `device:ota`, `device:network`, `user:invite`.
- **Frontend Readiness:** `PermissionMatrix` accepts dynamic `roles: UserRole[]` and `capabilities: Capability[]` as props rather than hardcoded 3-column structures. New roles can be registered without rewriting UI logic.
- **Backend Schema Compatibility:** `users.scopes TEXT[]` in PostgreSQL already supports arbitrary scope lists; roles in `admin_users.rs` map to default scope sets, allowing future database-driven role definitions.

### 6.2 Extension 2: Per-Station & Zone Scoped Permissions
- **Problem & Rationale:** Commercial greenhouse installations operate multiple independent aeroponics zones (e.g. Nursery Tower Zone vs Flowering Strawberry Zone). An operator managing Zone A must not inadvertently trigger emergency drain valves in Zone B.
- **Architectural Implementation:**
  - Protocol upgrade from flat scopes (`control:pump`) to resource-qualified scopes:
    - Format: `action:resource:scope_id` (e.g. `control:pump@zone:strawberries`, `read:telemetry@station:tower-01`).
  - `UserItem` interface gains an optional `station_scopes: Record<string, string[]>` mapping.
- **UI Adaptation:** `RoleSelector` will support a station/zone selector dropdown ("Áp dụng cho: Tất cả trạm | Khu vực A | Trạm 01").

### 6.3 Extension 3: Priva-Inspired Multi-Tenant Site Sharing
- **Problem & Rationale:** Industrial climate computers (Priva Compass, Argus Controls, Hoogendoorn) utilize federated multi-organization site sharing. An external vendor or greenhouse engineering firm can be granted temporary, time-bounded maintenance access without creating full internal user credentials.
- **Architectural Implementation:**
  - Future `site_shares` table linking `firebase_uid`, `farm_id`, `role`, and `expires_at`.
  - Frontend `PermissionMatrix` will render a "Thời hạn truy cập" (Access Expiry) indicator for temporary contractor roles.
  - Revocation is automatic upon timestamp expiration.

### 6.4 Extension 4: Granular Custom Scope Override Matrix
- **Problem & Rationale:** Advanced commercial enterprise tier customers occasionally require fine-grained exception overrides (e.g. granting a specific operator access to run firmware diagnostics without granting full Admin role).
- **Architectural Implementation:**
  - An "Advanced Mode" toggle in `PermissionMatrix` enabling interactive checkbox overrides per user.
  - When custom overrides diverge from the archetype default, `RoleBadge` displays a `Custom` modifier tag (e.g. `Vận hành viên (Tùy biến)`).

---

## 7. Verification & Testing Checklist

- [x] **Spec Completeness:** Contains Overview, Anatomy, Variants, Props/API, States, Behavior, Accessibility, and Usage Guidelines for all three components (`PermissionMatrix`, `RoleBadge`, `RoleSelector`).
- [x] **Hick's Law Conformance:** Documents cognitive load reduction through 3 archetype roles, smart defaults (`operator`), and progressive disclosure of technical scopes.
- [x] **Accessibility Audit (WCAG 2.2 AA):** Covers `role="grid"`, `role="gridcell"`, `role="rowheader"`, `role="columnheader"`, `aria-label` per status cell with role and capability names, arrow key matrix traversal, and non-color redundant check/cross indicators.
- [x] **Design Token Conformance:** Strict usage of `--color-primary`, `--color-primary-deep`, `--color-pill`, `--color-status`, `--color-surface-muted`, and `--color-line`.
- [x] **Q1 Extension Points:** Documents 4 commercial-tier additions (Specialized Commercial Roles, Per-Station Resource Scoping, Priva Multi-Tenant Site Sharing, Granular Scope Override Matrix) with technical rationale and architectural flow.
