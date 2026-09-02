# HydraGrow — Chuẩn giao diện Frontend (UI/UX Standard)

> Tài liệu này chốt lại **một nguồn sự thật duy nhất** cho màu sắc, chữ, spacing, component và cấu trúc trang của `hydragrow-frontend`. Mọi PR chạm tới UI nên đối chiếu với tài liệu này trước khi merge.
>
> Bối cảnh: audit codebase hiện tại cho thấy đã có token trong `App.css` nhưng không được tuân thủ đều — ví dụ `bg-blue-*` xuất hiện 38 lần dù không nằm trong token nào, trang `Automation.tsx` bỏ qua toàn bộ hệ thống class dùng chung, và class `ui-btn-primary` được gọi ở `MainLayout.tsx` nhưng chưa từng được định nghĩa. Tài liệu này vừa là chuẩn để đi tới, vừa là checklist để dọn nợ cũ.

---

## 1. Design tokens

### 1.1 Màu sắc

Định nghĩa tại `src/App.css` (`@theme` + biến CSS). **Không tự chọn mã hex hay lớp màu Tailwind ngoài bảng này.**

| Vai trò | Token / biến | Lớp Tailwind tương ứng | Dùng khi nào |
|---|---|---|---|
| Primary | `--color-primary` `#15803d` | `emerald-700` | Hành động chính, trạng thái active, thương hiệu |
| Success | `--color-success` `#16a34a` | `emerald-600` | Online, đạt ngưỡng, hoàn tất |
| Warning | `--color-warning` `#d97706` | `amber-600` | Cần chú ý, gần ngưỡng, cảnh báo nhẹ |
| Danger | `--color-error` `#dc2626` | `red-600` | Lỗi, mất kết nối, vượt ngưỡng nguy hiểm |
| Info / Nước | `--color-water` `#0284c7` | `sky-600` | Dữ liệu liên quan tới nước/thông tin trung tính |
| Surface | `--color-surface` `#ffffff` | `white` | Nền card |
| Surface muted | `--color-surface-muted` `#ecfdf5` | `emerald-50` | Nền phụ, panel mờ |
| Border | `--color-border` `#bbf7d0` | `emerald-100/200` | Viền card, chia khối |
| Text | `--color-text` `#14532d` | `emerald-950` | Văn bản chính |
| Text muted | `--color-text-muted` `#4b6354` | `emerald-800/75` | Mô tả phụ, caption |
| Neutral | `--color-neutral` `#64748b` | `.log-neutral-badge` / `.log-neutral-dot` | Sự kiện kỹ thuật đã gộp dòng (Nhật ký, chế độ Quan trọng) — không dùng `slate-*` trực tiếp, luôn qua 2 class này |

**Quy tắc cứng:**
- **Cấm** `bg-blue-*`, `text-blue-*`, `border-blue-*`, `indigo-*`, `slate-*` cho UI chính — đây là màu "lạc token" đang tồn tại rải rác trong `Settings.tsx`, `Dashboard.tsx`, `DevicePairing.tsx`, `RecipeBuilder.tsx`, v.v. Chỗ nào cần một màu "thông tin/trung tính khác primary" → dùng **Info/Nước (`sky`)**, không phải `blue`.
- Màu định danh loại bơm/thiết bị (ví dụ bảng `pumpColors` trong `Dashboard.tsx`) có thể dùng thêm `orange`, `fuchsia`, `rose`, `cyan`, `indigo` **nhưng chỉ cho mục đích phân biệt nhãn**, không dùng cho nút hành động hay trạng thái hệ thống.

### 1.2 Typography

Font: `Inter` (fallback `system-ui`).

| Cấp | Size / line-height | Weight | Dùng cho |
|---|---|---|---|
| H1 | 32px | Extra Bold | Tiêu đề trang / hero |
| H2 | 22px | Bold | Tiêu đề section |
| H3 | 18px | Semi Bold | Tiêu đề card |
| Body | 14px | Regular | Nội dung chính |
| Caption | 11px, uppercase, tracking rộng | Bold | Nhãn nhóm (`farm-section-title`, `ui-form-label`) |

### 1.3 Spacing & bo góc

- Bo góc card: `rounded-2xl` (16px). Bo góc pill/nút nhỏ: `rounded-full` hoặc `rounded-xl`.
- Khoảng cách giữa các section trong 1 trang: `space-y-6` (class `.app-page` đã set sẵn).
- Padding card mặc định: `p-4 md:p-5` (theo `.ui-card`).

### 1.4 Shadow & hiệu ứng

- Card: `shadow-sm shadow-emerald-950/5`.
- Popup/menu nổi: `shadow-xl shadow-emerald-950/10`.
- Luôn tôn trọng `prefers-reduced-motion` (đã có sẵn ở cuối `App.css` — không xoá).

---

## 2. Class dùng chung bắt buộc

Trước khi viết class Tailwind tay, **kiểm tra xem `App.css` đã có class tương ứng chưa** — không tạo phong cách mới trùng chức năng.

| Class | Dùng cho |
|---|---|
| `.app-page` | Wrapper bắt buộc cho **mọi trang** (padding, max-width, spacing dọc) |
| `.page-header`, `.page-header-title`, `.page-header-subtitle` | Tiêu đề đầu trang |
| `.ui-card` | Khối nội dung dạng thẻ |
| `.ui-state` | Trạng thái rỗng / placeholder |
| `.ui-input` | Input văn bản |
| `.ui-btn-md` | Nút kích thước chuẩn — kết hợp với màu nền/viền theo bảng màu ở trên |
| `.farm-status-pill` | Pill trạng thái (online/offline, chế độ...) |
| `.farm-section-title` | Nhãn nhóm caption in hoa |
| `.farm-muted-panel` | Panel nền mờ phụ |

**Nợ kỹ thuật cần xử lý ngay:**
- `ui-btn-primary` được gọi trong `MainLayout.tsx` nhưng **chưa được định nghĩa** trong `App.css` → thêm định nghĩa (nền `--color-primary`, chữ trắng, hover đậm hơn) hoặc đổi nút đó sang `ui-btn-md` + class màu tường minh.
- Không tạo thêm `ui-btn-*` mới mà không thêm vào `@layer components` của `App.css`.

---

## 3. Cấu trúc một trang (page anatomy)

```
<div className="app-page">
  <PageHeader />              -- tiêu đề + mô tả ngắn, dùng .page-header*
  <section className="ui-card">...</section>   -- các khối nội dung
  <section className="ui-card">...</section>
</div>
```

Mọi trang mới **phải** bọc trong `.app-page` và dùng `.ui-card` cho các khối — kể cả trang có canvas/editor (xem mục 6 về Automation).

---

## 4. Điều hướng (Information Architecture)

Thay cấu trúc phẳng 10 điểm đến (5 tab + 5 mục trong menu "Thêm") bằng **5 tab theo nhóm việc**:

1. **Tổng quan** — Dashboard
2. **Vận hành** — Điều khiển thủ công + Tự động hoá (gộp vì cùng là thao tác bơm/van)
3. **Canh tác** — Mùa vụ + Công thức + Lịch sử châm (cùng vòng đời một vụ trồng)
4. **Nhật ký** — Sự kiện hệ thống + Grafana metrics
5. **Cài đặt** — Ghép thiết bị, người dùng, sao lưu cấu hình, cấu hình trạm (mọi việc "quản trị")

Quy tắc: khi thêm trang mới, **luôn hỏi trang đó thuộc nhóm nào trong 5 nhóm trên** trước khi thêm route/tab mới. Tránh tái lập kiểu menu "Thêm" chứa mọi thứ không biết xếp đâu.

---

## 5. Responsive / Desktop (quan trọng vì có bản Tauri desktop)

- Dưới `lg`: giữ bottom tab bar hiện tại (đúng cho mobile/tablet dọc).
- Từ `lg` trở lên (bao gồm app desktop qua Tauri): **chuyển sang sidebar điều hướng cố định bên trái**, nội dung dùng phần còn lại của màn hình thay vì bị giới hạn `max-w-6xl` căn giữa với khoảng trắng hai bên.
- Trang có canvas (automation flow) nên dùng bố cục 2 cột (danh sách + canvas) chỉ ở desktop; ở mobile hiển thị danh sách dạng thẻ, không nhúng canvas kéo-thả.

---

## 6. Automation — hợp nhất trình soạn thảo

Hiện có 2 hệ thống song song: Blockly (kéo-thả khối) và React Flow (canvas node), gây phân mảnh cả UI lẫn code.

**Quyết định chuẩn:**
- Mobile/tablet: danh sách flow dạng `.ui-card`, mỗi thẻ gồm tên, mô tả điều kiện, pill bật/tắt — không nhúng canvas.
- Desktop (`lg:` trở lên): giữ **một** trình soạn thảo trực quan — khuyến nghị giữ React Flow (đã hiện đại hơn, dễ style theo token), loại bỏ Blockly để không phải bảo trì song song hai bộ logic chuyển đổi IR (`blockly/extractIr.ts` và `reactflow/buildIr.ts`).
- Trang `Automation.tsx` phải được viết lại để dùng `.app-page` / `.ui-card` như mọi trang khác, không dùng `border-b`, `text-gray-400` mặc định của Tailwind.

---

## 7. Trạng thái & phản hồi

- Loading: `<LoadingState />` (đã có, dùng class `.ui-loading*`).
- Rỗng / chưa cấu hình: `.ui-state` + `.ui-state-title` + `.ui-state-desc`.
- Lỗi/cảnh báo inline: khối `rounded-2xl border` với nền theo mức độ nghiêm trọng — `amber-50` (cảnh báo), `red-50` (lỗi) — **không** phối `blue`.
- Toast: dùng `react-hot-toast` đã có sẵn, không thêm thư viện toast khác.

---

## 8. Checklist trước khi merge PR có thay đổi UI

- [ ] Không có `bg-blue-*` / `text-blue-*` / `border-blue-*` mới (trừ khi đã đổi chuẩn ở mục 1.1)
- [ ] Trang mới bọc trong `.app-page`, dùng `.ui-card` cho các khối nội dung
- [ ] Không tạo class `ui-btn-*` / `farm-*` mới mà không khai báo trong `App.css`
- [ ] Route mới đã được xếp vào đúng 1 trong 5 nhóm điều hướng (mục 4)
- [ ] Đã kiểm tra hiển thị ở cả breakpoint mobile và `lg:` (desktop/Tauri)
- [ ] Trạng thái loading/rỗng/lỗi dùng component chung, không viết `div` tùy biến mới

---

## 9. Tham khảo

Đề xuất trực quan (design tokens, IA mới, mockup Dashboard & Automation) đã được dựng trong Figma:
`https://www.figma.com/design/jIkpIARzCtbvJ66UosPXoy`
