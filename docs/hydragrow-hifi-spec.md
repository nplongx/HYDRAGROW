# HydraGrow — Hi-Fi Spec (Frames 09–20, Lo-Fi v1 → Hi-Fi)

Source: Figma `UvfamFTHSrneof4eKOxozm`, canvas `Wireframes — Full App (Lo-Fi v1)` #190:2.
Dashboard #191:2 spec already delivered in chat — repeated below as §3 for completeness.

## 0. Global tokens + components (apply once, reuse everywhere)

Tokens (Figma Variables):
```
Brand/Green-700 #2E7D32 | Green-50 #E8F5E9 | Green-100 #EAF6EC
Warning/700 #B8590A | Warning-50 #FFF3E0
Critical/700 #C62828 | Critical-50 #FDECEA
Ink/900 #111111 | Ink-700 #2B2B2B | Ink-500 #6B6B6B | Ink-400 #9A9A9A
Line #EEEEEE | Bg #FFFFFF | Bg-muted #F7F7F7 | Bg-green #F2F7F2
Radius: card 12, row 8-10, pill 20, button 8, FAB 24, sheet 16 top
Type Inter: H1 18 Bold, Title 16 Bold, Body 12 Regular, Label 10 SemiBold UPPER, Caption 11 Regular, Tiny 9
Shadow: card 0 1 3 rgba(0,0,0,.08), FAB-red 0 4 12 rgba(198,40,40,.35)
```

Shared components (build once):
- `Banner/Offline`: Ink-700 fill, white 12 SemiBold centered, padding 10/12: `⚠ Mất kết nối — dữ liệu cũ từ {time}`. Visible/hidden variant. P0 consistent banner.
- `Banner/Alert`: variants info/warning/critical (Warning-50/Warning-700, Critical-50/Critical-700). Icon 18 in 9-radius box + text 11 SemiBold + action `[Xem]`.
- `Pill/Status`: `Đang gửi…` (Warning-50/Warning-700), `✓ Xác nhận` (Green-50/Green-700), `⚠ Lỗi` (Critical-50/Critical-700). 9px SemiBold, padding 4/8, radius 20.
- `Button/Primary`: Ink-700 fill, white 13 SemiBold, padding 12/16, radius 8, disabled 40% + spinner variant (P1 loading).
- `Button/Danger`: Critical-700 fill white text. `Button/Ghost`: transparent, Ink-700 text.
- `FAB/Emergency`: Critical-700, white 12 SemiBold `🛑 Dừng khẩn cấp`, padding 12/16, radius 24. Fixed bottom-right above BottomNav. Tap → confirm sheet with 2-step `[Xác nhận dừng] [Huỷ]`.
- `BottomNav`: 5 items, 20px icon + 9px label, active Green-700 / inactive Ink-400.
- `Field/Input`: stroke #D9D9D9, radius 8, padding 10/12, error variant (Critical stroke + 10px Critical helper text).

Optimistic UI rule (P1 system-wide): every device toggle shows `Pill/Status` transition sending → confirmed / error-timeout. No silent toggles.

---

## 1. Frame 09 Login #190:3
- Layout 390w column padding 80/24/40 gap 28.
- Logo 64 + `HydraGrow` 20 Bold center + `Khí canh thông minh` 12 Ink-500.
- Fields Email / Mật khẩu using Field/Input. Error texts P0 specific: `Sai mật khẩu`, `Tài khoản bị khoá — liên hệ admin`, `Không có mạng — kiểm tra kết nối`. Not generic.
- `Quên mật khẩu?` 11px right-aligned → sheet: nhập email → `Đã gửi mã OTP` → nhập OTP + mật khẩu mới (P0 forgot flow).
- Login button: loading variant spinner + disabled anti-double-tap (P1).
- P1 biometric row after first login: `[Đăng nhập bằng Face ID / Vân tay]` + PIN 4–6 digits option on trusted device.
- Hide Google button if no OAuth (P2): variant `oauth=false → hidden`, show only email login.
- P2 demo link: `Dùng thử với dữ liệu mẫu →` ghost button.

## 2. Frame 10 Pairing #190:26
- Header `Thiết Bị Của Tôi` H1 18 + sub 12.
- Section label `THIẾT BỊ ĐÃ LIÊN KẾT` 10 UPPER Ink-400.
- Rows: name 13 SemiBold (`Trạm Vườn Sau` custom name/emoji P1, e.g. `🥬 Vườn Sau`) + sub 11 `HG-0231 · Online · 2 phút trước` / offline reason P1: `HG-0198 · Offline · Mất Wi-Fi 3 giờ trước` (not just Offline). Sort alerts-first P2.
- Primary `+ Liên kết thiết bị mới` → sheet with 2 tabs: `[Quét QR] [Nhập mã tay]`.
- QR preview dashed box (existing #190:46) → hi-fi: camera viewport 180h, corner guides Green-700, hint 11 `Đặt mã QR mặt sau thiết bị vào khung`.
- P0 secure pairing: after scan show `Mã trên thiết bị: 4821 — khớp? [Xác nhận] [Huỷ]`.
- P1 OTA row per device: `Firmware v1.4.2 [Cập nhật]`; P2 Share: `[Chia sẻ]` → User Management roles.

## 3. Frame 11 Dashboard #191:2 (already delivered)
Offline banner + Push opt-in (`Nhận cảnh báo khi mực nước thấp? [Bật][Để sau]`, dismissible, Green-50 card) + clickable NextActionCard (Bg-green, whole-card tap → Operations/Control highlight) + Warning/Critical banners with `[Xem]` + metric cards with normal/warning/critical strokes + 80×24 sparklines + `vs hôm qua` caption + running-device deep-links + Emergency FAB fixed with confirm sheet. See chat message for exact node mapping (#191:10, #191:23, #205:7, #191:29/33/37/41, #205:14).

## 4. Frame 12 Control #191:74
- Header `Vận hành` + Pill `Tự động`. Tabs `[Điều khiển active dark][Tự động hóa]`.
- `ChangeTag Đã cập nhật P0` keep as dev note, hide in hi-fi export.
- DeviceCard: Top row (icon 28 + name 13 SemiBold + Toggle 36×20 + Pill/Status) + Slider row (track 6px #EAEAEA, value 11 + `≈ X ml/phút` 10 Ink-500 P1 conversion).
- P0 safety: pH Up ON → pH Down card disabled grey + lock note 10 Warning-700 `🔒 Đã khoá vì Bơm pH Up đang chạy`. Same for Fert A/B ratio sync warning.
- P0 states: `Đang gửi… / ✓ Xác nhận / ⚠ Lỗi phản hồi` pills per card.
- P1 max-runtime: subtitle under toggle `Tự tắt sau 15 phút [Đổi]` when ON.
- P2 water level inline in Tank group header: `CẤP & XẢ — Mực 18% Thấp`.
- EmergencyBar bottom Critical-700 `🛑 Dừng khẩn cấp toàn bộ thiết bị` full-width above BottomNav.
- P1 history link: `Lịch sử thao tác tay gần đây ›` → bottom sheet list. P1 one-shot timer: long-press toggle → `Chạy 10 phút rồi tắt`.

## 5. Frame 13 Automation #191:181
- ConflictBanner Warning-50: `Xung đột: "Tưới buổi sáng" và "Xả bồn" cùng Van cấp nước lúc 06:00 [Xem chi tiết]`.
- Rows: name 13 SemiBold + schedule 11 + `Chạy lần cuối: … · Thành công/Thất bại` 9px Ink-400 (P1 last-run). Error-paused rows get Critical stroke + `Tạm dừng do lỗi cảm biến` distinct from user-disabled grey toggle.
- Secondary `📚 Thư viện mẫu` + Primary `+ Tạo luồng mới`. Note card about canvas editor 02–06 keep as info.
- P0 dry-run: row swipe / overflow `… → [Chạy thử]` without activating devices. P1 per-flow log link. P2 export/import as file.

## 6. Frame 14 Seasons
- Current card: crop, stage chip, `Ngày 18/35` progress tied to stage milestones (not linear) with milestone dots.
- P0 deviation alert Critical/Warning banner: `Chậm 3 ngày so với công thức`.
- P0 photo journal strip: horizontal 64px thumbs timeline `Ngày 1/7/14… [+]` → viewer.
- P1 harvest estimate `Dự kiến thu hoạch ~12 ngày nữa (theo EC/pH thực tế)`; milestone notes `+ Ghi chú`; history yield cards `2.1kg vs 1.8kg vụ trước`.
- P2 compare 2 seasons overlay chart entry.

## 7. Frame 15 Recipes
- Active recipe card: EC/pH targets + stage stepper tappable → stage params sheet (Germination vs Flowering EC/pH).
- P1 Apply guard: `[Áp dụng]` → confirm modal with impact preview `pH 6.0→6.4 ảnh hưởng 3 luồng` + `[Xác nhận][Huỷ]`.
- Library rows with `Phù hợp: Xà lách · Khí canh` tags for filter.
- P0 `Lưu vụ thành công thành công thức` button from Season; P1 out-of-range warning; P2 community share.

## 8. Frame 16 Dosing History
- Header total `148 ml hôm nay` + `vs TB 7 ngày 132 ml +12%` reference (P1).
- Filters: `[Hôm nay][7 ngày][30 ngày]` chips.
- Real bar chart by hour (not placeholder): 24 bars, anomaly bar Critical + tooltip. Group flat log by session/batch expandable (P1 grouping).
- P0 abnormal alert banner when >X% over Y hours + `Ước còn N ngày` remaining estimate from consumption rate.
- P2 export monthly report `[Xuất PDF/CSV]`.

## 9. Frame 17 Journal Events
- Filter chips `[Tất cả][Cảnh báo][Hệ thống][Thiết bị]` + device dropdown (P1 multi-station) + search field (P1 keyword).
- Rows: severity dot (green/orange/red) + time + text + inline actions P0: `[Xem cảm biến] [Đánh dấu đã xử lý]` on alert rows.
- Collapse repeats: `Mất/kết nối lại 5 lần trong 10 phút — gộp [Mở rộng]`.
- P0 export `[Xuất CSV/PDF theo ngày]`; P2 note/label per event.

## 10. Frame 18 Analytics
- Mobile summary layer (not shrunken Grafana): 3 cards `Cảm biến / Dự đoán hệ thống (đổi tên từ MIMO/Kalman) / Phần cứng` each with preview number + mini chart + `[Mở đầy đủ → Grafana]`.
- P1 scheduled reports `[Nhận báo cáo tuần qua email]` toggle. P2 set threshold from chart (long-press chart → threshold sheet).

## 11. Frame 19 Settings
- P1 mobile: replace 5 cramped tabs with vertical list or merge Sensors into Thresholds & Water. Sections: Tổng quan / Ngưỡng & Nước / Châm phân / Cảm biến / Kết nối + 2 reconnected (P0): `Sao lưu & Khôi phục cấu hình (ConfigBackup.tsx)` + `Người dùng & Quyền (UserManagement.tsx)`.
- P0 Save scope: per-tab `[Lưu tab này]` + dirty dot; or explicit `Lưu tất cả 5 tab`. Never ambiguous.
- P1 Calibrate: `Công suất bơm ml/s [Hiệu chuẩn thực tế]` measured flow action.
- P0 roles Admin/Operator/Viewer matrix by function; P1 change history `Ai đổi pH 6.0→6.4 lúc nào`; P2 export/import config file to clone stations.

## 12. Frame 20 Fleet View (new)
- Decide positioning P1: header toggle `[Tổng hợp cảnh báo | Chọn trạm]` — aggregate vs selector.
- Cards: name, status dot, key metric (EC/pH), open-alert red badge count, `stations with alerts first` sort/filter P0.
- P1 compare same-crop metrics; P2 map view.

## P0 checklist (do first)
Push notifications · consistent offline banner · 3-state command pills · manual safety locks (pH Up/Down, A/B) · always-visible emergency stop + confirm · flow conflict alerts · season deviation alerts · photo journal · real dosing chart + abnormal alerts · journal row actions + export · reconnect Backup + UserMgmt + roles · forgot-password flow · full QR/manual pairing + verification.

## Open questions (from proposal)
FCM/APNs ready? Role boundaries Operator vs Viewer? Fleet = 1-station household vs multi-station commercial priority? Grafana mobile API vs full embed?
