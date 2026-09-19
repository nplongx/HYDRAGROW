+# HYDRAGROW — Frontend Asset Design Specification

## 0. Purpose

Spec này xác định những visual asset cần thiết kế riêng cho `hydragrow-frontend`.

Mục tiêu:
- bổ sung visual identity cho frontend mà không biến app vận hành thành giao diện trang trí;
- dùng asset riêng cho những nơi icon/component chuẩn không đủ;
- giữ asset nhất quán với design system hiện tại;
- ưu tiên SVG/vector cho UI;
- không thay thế telemetry, status, safety signal bằng illustration;
- asset generation phải tạo ra file production-ready, không dùng preview làm asset.

## 1. Sources and scope

Nguồn chính:
- `docs/design-system/DESIGN-TOKENS-VISUAL-SPEC.md`
- `docs/design-system/UX-RULES.md`
- `docs/design-system/COMPONENT-CONTRACT.md`
- `docs/design-system/PAGE-CONTRACT.md`
- `docs/design-system/PATTERN-LIBRARY.md`
- `docs/design-system/DASHBOARD-FUNCTIONAL-SPEC.md`
- `docs/design-system/AUTOMATION-FUNCTIONAL-SPEC.md`
- `docs/design-system/CULTIVATION-FUNCTIONAL-SPEC.md`
- `docs/design-system/JOURNAL-FUNCTIONAL-SPEC.md`
- `docs/design-system/OPERATIONS-FUNCTIONAL-SPEC.md`
- `docs/design-system/SETTINGS-FUNCTIONAL-SPEC.md`
- `docs/design-system/UX-FLOW-TASK-FLOW-SPEC.md`
- `docs/design-system/STATE-TRANSITION-SPEC.md`
- `docs/design-system/layer3/FLEET-VIEW-SPEC.md`
- `docs/design-system/layer3/ONBOARDING-SPEC.md`
- `hydragrow-frontend/CHUAN-GIAO-DIEN-FRONTEND.md`
- `docs/FRONTEND_FUNCTIONAL_SPEC.md`
- current `hydragrow-frontend/src`.

## 2. Asset strategy

### 2.1 Existing icon library first

UI thông thường không cần custom asset: navigation, search, filter, edit/delete, play/pause, refresh, expand/collapse, success/warning/error/info, lock, device, sensor, pump, water, calendar, user/role.

Không tạo SVG riêng nếu icon chuẩn đã đủ nghĩa.

### 2.2 Custom asset only where it adds product meaning

Custom illustration/vector chỉ dùng cho:
1. empty state;
2. onboarding;
3. cultivation identity;
4. station/fleet visual identity;
5. major success/completion moment;
6. brand/product mark;
7. advanced automation canvas nếu icon library không đủ.

### 2.3 Visual character

Asset phải flat vector, clean, operational, calm, geometric, ít chi tiết, dễ nhận diện ở kích thước nhỏ, transparent background, không có text trong artwork, không gradient phức tạp, không photorealistic, không 3D.

Palette ưu tiên semantic tokens:
- `primary-deep` `#14532D`
- `primary` `#15803D`
- `status` `#047857`
- `pill` `#D1FAE5`
- `water` `#0284C7`
- `warning` `#D97706`
- `error` `#DC2626`.

## 3. Priority backlog

| Priority | Asset family | Mục đích |
|---|---|---|
| P0 | HYDRAGROW product mark | app shell, login, onboarding |
| P0 | Unpaired station illustration | dashboard/pairing empty state |
| P0 | Onboarding hero illustration | first-run flow |
| P0 | Generic no-data / no-history | empty states |
| P0 | Sprout + water-drop | recurring product visual |
| P1 | Season/cultivation set | Cultivation |
| P1 | Fleet/station illustration | Fleet |
| P1 | Completion illustration | season completion |
| P1 | Automation node icon set | advanced canvas |
| P2 | Auth illustration | auth screens |
| P2 | Backup/restore illustration | Settings |
| P2 | Permission/team illustration | Roles |
| P2 | Offline/unavailable illustration | unavailable state |

## 4. P0 asset specifications

### A01 — HYDRAGROW product mark

Use: login, onboarding, app shell, desktop application identity.

Deliverables:
- `hydragrow-mark.svg`
- `hydragrow-lockup.svg`
- monochrome variant.

Symbol hoạt động độc lập. Không bake text vào symbol; lockup text nên render bằng UI typography khi localization cần.

### A02 — Unpaired station illustration

Use: Dashboard/Fleet khi chưa có station.

Visual: hydroponic/aeroponic tower nhỏ + sensor/controller + vài sprout; trạng thái chưa kết nối biểu diễn nhẹ bằng connection marker.

Không biểu diễn unpaired như lỗi/offline. CTA phải còn rõ.

Deliverables: `empty-unpaired-station.svg`, viewBox khoảng 320×240; compact 160×120.

### A03 — Onboarding hero

Use: Welcome, Connect station, Receive first telemetry, Start cultivation.

Visual: aeroponic tower, green sprouts, water loop/drop, sensor/controller, subtle data signal. Một master composition để crop/reveal theo step.

Deliverables: `onboarding-master.svg` + crop/variant nếu frontend cần.

Aha moment là telemetry thật đầu tiên. Illustration không được giả dữ liệu.

### A04 — Hydroponic sprout + water-drop

Canonical recurring motif cho empty state, cultivation, onboarding và product accent.

Deliverables: `sprout-water.svg` + compact 24/32px-compatible variant + large variant.

### A05 — Empty-state illustration family

Deliverables:
- `empty-no-data.svg`
- `empty-no-history.svg`
- `empty-filter.svg`.

Visual: minimal log/chart abstraction + sprout/water motif. Filter-empty phải khác true empty state.

## 5. P1 cultivation assets

### A06 — Crop illustration set

Flat botanical vectors. Initial set:
- `crop-lettuce.svg`
- `crop-basil.svg`
- `crop-strawberry.svg`
- `crop-tomato.svg`
- `crop-other.svg`.

Nếu backend xác nhận crop taxonomy khác, asset follow taxonomy đó; không tự mở rộng danh sách.

### A07 — Season stage illustrations

Generic stage family cho `SeasonStageChecklist`: preparation, establishment, growth, flowering/fruiting nếu recipe có, harvest/completion.

Không hard-code biology stage nếu recipe có stage names động. Mapping thuộc frontend.

Deliverable: 5–6 SVG.

### A08 — Season completion

Use: `SeasonCompletionSummary`.

Visual: mature healthy plant + subtle water motif + restrained completion feeling. Không confetti quá mạnh. Artwork không phải status proof.

Deliverable: `season-complete.svg` + compact variant.

## 6. P1 fleet/station assets

### A09 — Station/tower illustration

Use: Fleet grouping, station card fallback, station detail empty/unavailable, station onboarding.

One base hydroponic tower silhouette, three densities: compact 48px, card 96–128px, empty state 240–320px.

Không cần vẽ artwork riêng cho online/offline/warning/unavailable. Dùng base asset + UI status overlay. Status vẫn do text/icon/token đảm nhiệm.

Deliverable: `station-tower.svg`.

## 7. P1 automation assets

### A10 — Automation node icon set

Families:
- trigger/sensor;
- schedule;
- condition;
- dose;
- water;
- actuator;
- alert;
- stage;
- end-season;
- chain;
- config;
- safety/E-STOP.

Không vẽ icon riêng chỉ vì khác label. Reuse base glyph + semantic color.

Automation canvas có thể dùng `config` / `config-soft` indigo theo exception của design system.

Deliverable: SVG icon set, grid-aligned, cùng stroke/weight/geometry.

## 8. P2 assets

### A11 — Authentication illustration
Login/register/forgot-password. Một composition dùng chung: hydroponic tower + water/controller motif.

### A12 — Backup/restore illustration
Configuration/document + secure storage/restore metaphor. Không imply security guarantee vượt semantics thật.

### A13 — Team/permissions illustration
Users + shield/role abstraction. Permission meaning vẫn phải dùng text + Check/X.

### A14 — Offline/unavailable illustration
Station/controller + disconnected signal. Không dùng cho normal disabled controls.

## 9. State rules

Mỗi reusable illustration cần định nghĩa: Default, Compact, Empty, Unavailable, Offline, Success, Error khi thực sự cần.

Không tạo asset cho Pending nếu spinner/status component đã đủ.

Critical/fault state phải dùng text + semantic signal; artwork không được làm loãng severity.

## 10. Output contract

Primary format: SVG, transparent background, optimized paths, no embedded bitmap, no external font dependency, stable `viewBox`, no baked text.

Suggested canonical root:
`hydragrow-frontend/public/assets/{brand,illustrations,crops,automation}`.

Nếu build/import convention hiện tại yêu cầu `src/assets`, chọn một root duy nhất; không tạo hai asset roots.

Naming không gắn với temporary screen/version name. Ví dụ: `empty-unpaired-station.svg`, không phải `dashboard-final-v2.svg`.

## 11. Generation contract

Recraft asset chỉ được coi là hoàn tất khi:
1. actual generation tool call;
2. output là generated asset, không phải preview;
3. SVG/vector được ưu tiên;
4. download asset từ URL/path do tool trả về;
5. validate file tồn tại và non-zero;
6. SVG có root `<svg`;
7. không có embedded raster/external font;
8. inspect rendered result;
9. optimize nếu cần nhưng không đổi visual content;
10. lưu evidence gồm prompt/model/asset URL.

Vector ưu tiên `recraftv4_1_vector` khi Recraft MCP hỗ trợ.

Không tự viết SVG để che generation failure.

## 12. Recraft prompt standard

Base prompt:

> Production-ready flat vector SVG for HYDRAGROW, a hydroponic/aeroponic monitoring and control dashboard. Calm operational visual language, clean geometric shapes, semantic green palette based on #14532D and #15803D with restrained #0284C7 water accent, transparent background, no text, no gradients, no photorealism, no 3D, minimal detail, crisp paths, scalable at small UI sizes.

Append asset-specific composition after base prompt.

Example unpaired station:

> Create ONE production-ready SVG illustration of a compact hydroponic tower with a few healthy green sprouts, a small controller/sensor module, and a subtle disconnected-state cue. Calm empty-state composition, centered, generous negative space, transparent background, no text.

Example onboarding:

> Create ONE production-ready SVG master illustration for HYDRAGROW onboarding: compact aeroponic tower, green sprouts, water circulation/drop motif, small sensor/controller, subtle data signal motif. Designed so four onboarding steps can crop or reveal different regions. Calm operational product illustration, transparent background, no text.

## 13. What NOT to design

Do not commission custom artwork for telemetry cards, status pills, buttons, navigation items, normal alerts, E-STOP, loading spinner, generic success/error icons, charts, tables, or standard form controls.

Do not create decorative plant backgrounds behind operational data.
Do not use artwork to make a fault state look pleasant.

## 14. Page-to-asset mapping

| Frontend surface | Asset |
|---|---|
| Login/Register | A11 + A01 |
| Dashboard, no station | A02 |
| Dashboard, no telemetry | A05 |
| Dashboard station identity | A09 / existing icons |
| Fleet empty | A02/A09 |
| Fleet station card | A09 |
| Operations empty/unavailable | A09/A14 |
| Cultivation | A06 + A07 |
| Season completion | A08 |
| Dosing history empty | A05 |
| Journal empty | A05 |
| Automation canvas | A10 |
| Automation empty | A04/A05 |
| Pairing | A02 |
| Settings backup | A12 |
| Roles empty | A13 |
| Global unavailable | A14 |
| App shell/auth brand | A01 |

## 15. Acceptance criteria

### Visual
- [ ] Custom asset follows semantic palette.
- [ ] No essential meaning is baked as artwork text.
- [ ] No unnecessary duplication of standard icons.
- [ ] Status remains understandable without artwork.
- [ ] Artwork remains legible at compact size.
- [ ] SVG has stable `viewBox`.
- [ ] No embedded raster image.
- [ ] No external font dependency.
- [ ] Transparent background has no accidental rectangle.

### Product
- [ ] Empty state explains what is absent and next action remains visible.
- [ ] Offline/unavailable is not confused with empty.
- [ ] Success artwork appears only for confirmed completion.
- [ ] Critical/fault state remains visually dominant.
- [ ] Mobile does not depend on large artwork.

### Engineering
- [ ] One canonical frontend asset root.
- [ ] Existing build/import convention is used.
- [ ] No unused asset added only because it was generated.
- [ ] Recraft-generated asset has evidence of actual generation.
- [ ] Vector artifact downloaded and validated before merge.

## 16. Initial design order

1. `hydragrow-mark.svg`
2. `sprout-water.svg`
3. `empty-unpaired-station.svg`
4. `empty-no-data.svg`
5. `empty-no-history.svg`
6. `onboarding-master.svg`
7. `season-complete.svg`
8. crop illustration set
9. `station-tower.svg`
10. automation icon set
11. auth illustration
12. backup/restore illustration
13. roles illustration
14. offline/unavailable illustration.

The first nine establish the visual asset language. P2 follows only after real UI gaps are confirmed.

## 17. Definition of Done

```text
Design-system semantics
  ↓
Asset specification
  ↓
Recraft/vector generation
  ↓
Downloaded SVG
  ↓
File/SVG validation
  ↓
Frontend integration
  ↓
Responsive visual QA
  ↓
Accessibility/state QA
```

A generated image not integrated into the intended frontend state is not finished asset work.
