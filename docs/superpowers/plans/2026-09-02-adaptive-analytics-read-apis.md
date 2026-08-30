# Phase 4 — Adaptive & Analytics Read APIs Implementation Plan

> **For agentic workers:** Phase này CHỈ xây API đọc (read-only), không có Blockly block mới —
> đọc kỹ mục "Quyết định phạm vi" trước khi tự ý thêm block, vì có một thay đổi kiến trúc lớn
> đang bị hoãn có chủ đích.

## Đính chính 1 câu sai ở Phase 3 (đọc trước khi tiếp tục)

Khi grounding Phase 4, phát hiện `api/crop_season.rs` đã có sẵn endpoint
`PUT /devices/{device_id}/seasons/active/end` gọi `end_active_crop_season` — nghĩa là câu khẳng
định ở Phase 3 "`end_active_crop_season`... chưa từng được gọi từ đâu cả" **sai**: nó đã có người
gọi, chỉ là gọi thủ công qua HTTP (con người bấm nút trên UI), không phải từ một automation script.
Việc này **không** làm sai Task 3 của Phase 3 (recipe_override giờ có thêm 1 người gọi mới — kịch
bản tự động — hoàn toàn hợp lệ và vẫn cần thiết), chỉ sai ở CÂU GIẢI THÍCH lý do. Ghi nhận công
khai để không ai đọc lại Phase 3 rồi hiểu nhầm hàm đó là dead code trước đó.

## Grounding (đọc kỹ — sửa 1 điểm sai trong roadmap, và 1 quyết định phạm vi lớn)

**Sửa roadmap:** Bảng mục 4 của roadmap ghi "⑦ Adaptive — Kalman gain viewer | ✅ dữ liệu đã có
(`KalmanLearningData` trong `DosingCycleEvent`, ghi InfluxDB)" — **sai vị trí lưu**. Đọc thẳng
`mqtt/handlers/dosing_cycle.rs` cho thấy `DosingCycleEvent` (kèm `kalman`) được lưu ở:
1. Prometheus gauges (`ADAPTIVE_GAIN_PER_ML`...) — chỉ phục vụ Grafana/Prometheus, KHÔNG query
   được từ backend.
2. **Bảng Postgres `dosing_reports`**, cột `payload` (JSONB — nguyên văn `DosingCycleEvent`,
   gồm cả `kalman`) — đây mới là nguồn đọc lại được. InfluxDB (`sensor_data` measurement, xem
   `db/influx.rs`) chỉ chứa `ec/ph/temp/water_level` thô từ cảm biến, KHÔNG chứa dữ liệu Kalman.

**Phát hiện tốt, giảm việc cần làm:** `api/sensor.rs::get_history`
(`GET /devices/{id}/sensors/history`) **đã** query InfluxDB theo khoảng thời gian tuỳ ý
(`start`/`end`/`range`), có cả chế độ `aggregateWindow(fn: mean)` khi truyền `resolution`. Việc
còn thiếu duy nhất: một con số TỔNG HỢP (mean/min/max/count) cho CẢ khoảng thời gian — endpoint
hiện tại trả về một CHUỖI điểm dữ liệu (để vẽ biểu đồ), không phải 1 số duy nhất để nhét vào điều
kiện. Task 1–2 lấp đúng khoảng trống này, tái dùng lại phần dựng Flux range clause đã chạy thật.

**Quyết định phạm vi lớn (đọc kỹ):** đề xuất gốc muốn "Query InfluxDB range... dùng làm input cho
condition block" — nghĩa là một block Blockly kiểu **value block** (có ngõ ra, cắm được vào ô của
block khác), khác hẳn mọi block đã xây từ Phase 0–3 (toàn bộ đều là `previousStatement`/
`nextStatement`, không có ngõ ra). Để cắm được, `hydragrow_sensor_condition` (block điều kiện DÙNG
CHUNG cho cả 3 kind: alert/recipe_override/action_command) phải đổi field `VALUE` từ
`FieldNumber` (số gõ tay) sang **value input** (ổ cắm) — một thay đổi kiến trúc core, ảnh hưởng
toàn bộ 3 kind cùng lúc, rủi ro cao hơn hẳn các task đã làm. Quyết định: Phase 4 chỉ xây phần DỮ
LIỆU (2 API đọc, dùng được ngay cho dashboard/phân tích), KHÔNG ép thay đổi core block này vào
cuối một chuỗi phase đã chạy ổn định. Việc "cắm value-block vào ô điều kiện" cần một plan riêng,
làm kỹ, review riêng cho đúng 1 thay đổi rủi ro cao thay vì bị vùi trong Phase 4.

**Vẫn ngoài scope (không đổi so với roadmap):** "MIMO solver step: gọi một bước tính toán" — không
thể gọi trực tiếp, vì `hydragrow-controller-core::adaptive` không được backend link tới (chạy trên
MCU). Diễn giải lại thành "đọc lại delta dosing đã tính từ 1 chu kỳ lịch sử" — chính là dữ liệu
Task 3–5 cung cấp (mỗi `dosing_reports` row đã có sẵn `delta_ec()`/`delta_ph()`/dose ml, là kết quả
MỘT bước solver đã chạy thật trên firmware). "Gain learner reset" — vẫn chưa có action MQTT nào
được xác nhận, để ngoài scope như Phase 1/3 đã quyết.

**Goal:** 2 API đọc mới: (1) mean/min/max/count của 1 field cảm biến trong khoảng thời gian
(InfluxDB), (2) lịch sử dosing kèm Kalman gain trong khoảng thời gian (Postgres).

**Tech Stack:** Rust (sqlx, influxdb2).

---

## Task 1: `compute_range_stats` — tổng hợp mean/min/max/count thuần

**Files:**
- Create: `hydragrow-backend/src/services/analytics.rs`
- Modify: `hydragrow-backend/src/services/mod.rs`

- [ ] **Step 1: Viết failing test**
- [ ] **Step 2: Chạy test để xác nhận fail**
- [ ] **Step 3: Viết implementation**
- [ ] **Step 4: Chạy test để xác nhận pass**
- [ ] **Step 5: Commit**

---

## Task 2: Endpoint `GET /sensors/range-stats` (InfluxDB)

**Files:**
- Modify: `hydragrow-backend/src/api/sensor.rs`

- [ ] **Step 1: Viết failing test**
- [ ] **Step 2: Chạy test để xác nhận fail**
- [ ] **Step 3: Viết implementation — tách `build_range_clause`, thêm endpoint mới**
- [ ] **Step 4: Chạy test để xác nhận pass**
- [ ] **Step 5: Commit**

---

## Task 3: `extract_kalman_from_payload` — đọc lại Kalman gain từ `dosing_reports.payload`

**Files:**
- Modify: `hydragrow-backend/src/services/analytics.rs`

- [ ] **Step 1: Viết failing test**
- [ ] **Step 2: Chạy test để xác nhận fail**
- [ ] **Step 3: Viết implementation**
- [ ] **Step 4: Chạy test để xác nhận pass**
- [ ] **Step 5: Commit**

---

## Task 4: `get_device_dosing_reports_in_range` — lọc theo khoảng thời gian tuỳ ý

**Files:**
- Modify: `hydragrow-backend/src/db/postgres.rs`
- Modify: `hydragrow-backend/src/db/tests/test_postgres.rs`

- [ ] **Step 1: Viết failing test**
- [ ] **Step 2: Chạy test để xác nhận fail**
- [ ] **Step 3: Viết implementation**
- [ ] **Step 4: Chạy test để xác nhận pass**
- [ ] **Step 5: Commit**

---

## Task 5: Endpoint `GET /analytics/dosing-history` (Postgres + Kalman)

**Files:**
- Create: `hydragrow-backend/src/api/analytics.rs`
- Modify: `hydragrow-backend/src/api/mod.rs`
- Modify: `hydragrow-backend/src/main.rs`

- [ ] **Step 1: Viết file mới**
- [ ] **Step 2: Đăng ký module**
- [ ] **Step 3: Kiểm tra biên dịch**
- [ ] **Step 4: Commit**
