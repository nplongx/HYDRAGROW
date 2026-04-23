# Code Standards (HydraGrow)

> **Single Source of Truth (SSOT):** Tài liệu này là **nguồn sự thật duy nhất** cho quy ước code trong toàn bộ repo HydraGrow. Mọi PR/review phải tuân theo tài liệu này. Nếu có mâu thuẫn với tài liệu cũ hoặc thói quen cá nhân, **ưu tiên tài liệu này**.

## 1) Phạm vi & nguyên tắc chung

- Áp dụng cho các phần chính của repo:
  - `hydragrow-backend/` (Rust)
  - `hydragrow-frontend/` (TypeScript + React)
  - `hydragrow-frontend/src-tauri/` (Rust)
  - `ESP32-C3-SENSOR-NODE/` (C++)
  - SQL migrations trong `hydragrow-backend/migrations/`
- Ưu tiên:
  1. Rõ nghĩa theo domain (nông nghiệp thuỷ canh, sensor, dosing, crop season).
  2. Nhất quán > sở thích cá nhân.
  3. Dễ tìm kiếm, dễ refactor, dễ onboarding.

## 2) Quy ước đặt tên

### 2.1 Class/Struct/Type/Enum

- Dùng `PascalCase`.
- Tên phải là danh từ theo domain, ví dụ: `SensorReading`, `CropSeason`, `DosingPlan`.
- Tránh hậu tố mơ hồ như `Data`, `Info`, `Manager` nếu không thêm ngữ nghĩa.

### 2.2 Function/Method

- Dùng `snake_case` cho Rust/C++ backend-firmware; `camelCase` cho TypeScript.
- Dùng động từ + đối tượng rõ nghĩa:
  - Tốt: `calculateNutrientDose`, `publish_sensor_event`, `apply_ph_calibration`
  - Tránh: `handleData`, `doThing`, `process`
- Hàm boolean bắt đầu bằng `is/has/can/should` (TS) hoặc tương đương rõ nghĩa trong Rust/C++ (`is_online`, `has_alarm`).

### 2.3 Biến

- Biến thông thường: `camelCase` (TS), `snake_case` (Rust/C++).
- Biến phạm vi nhỏ vẫn phải có nghĩa theo domain; tránh `tmp`, `x`, `data` (trừ vòng lặp cực ngắn).

### 2.4 Hằng số

- Dùng `UPPER_SNAKE_CASE` cho hằng compile-time hoặc giá trị cố định theo domain.
- Hạn chế hardcode số trực tiếp trong logic nghiệp vụ; đưa thành hằng có tên rõ nghĩa.

### 2.5 File & thư mục

- Rust module/file: `snake_case.rs`.
- TypeScript React component file: `PascalCase.tsx` cho component, `camelCase.ts/ts` cho hook/util theo chuẩn hiện tại.
- Thư mục: `kebab-case` hoặc theo convention sẵn có trong module; không trộn trong cùng cấp.
- Tên file phản ánh một trách nhiệm chính.

### 2.6 Config key & Environment Variable

- Key config nội bộ (JSON/TOML/YAML): `snake_case`.
- Env var: `UPPER_SNAKE_CASE`, có prefix theo ngữ cảnh hệ thống (ví dụ `HYDRAGROW_`, `TAURI_`, `VITE_`).
- Không đặt tên env var mơ hồ (`TOKEN`, `KEY`); cần cụ thể (`HYDRAGROW_FCM_SERVER_KEY`).

## 3) Comment, Docstring, Logging

### 3.1 Comment/Docstring

- Chỉ viết comment khi cần giải thích **vì sao**, không lặp lại **điều gì** code đã thể hiện.
- Public API/hàm phức tạp phải có doc ngắn gọn: input, output, side effects, lỗi có thể trả về.
- Cập nhật comment cùng lúc với code; comment lỗi thời được xem như bug chất lượng.

### 3.2 Logging

- Log có cấu trúc, đủ context để truy vết: `device_id`, `crop_season_id`, `request_id`, `command_id`.
- Mức log thống nhất:
  - `ERROR`: lỗi làm fail flow chính.
  - `WARN`: bất thường có thể tự phục hồi.
  - `INFO`: mốc vận hành quan trọng.
  - `DEBUG/TRACE`: phục vụ debug, không gây nhiễu production.
- Không log secrets, token, private key, mật khẩu, thông tin nhạy cảm.

## 4) Error handling

- Không nuốt lỗi im lặng (`catch {}` rỗng, bỏ qua `Result` mà không xử lý).
- Mọi lỗi phải:
  1. Có ngữ cảnh domain rõ ràng.
  2. Được phân loại (validation, network, DB, external service...).
  3. Được propagate hoặc map sang response phù hợp.
- Tránh panic/unwrap trong runtime path production (Rust); chỉ chấp nhận tại test/prototype có lý do rõ ràng.
- Frontend: hiển thị thông báo người dùng ở mức phù hợp, không lộ chi tiết nội bộ hệ thống.

## 5) Import order

- Thứ tự import (mọi ngôn ngữ):
  1. Standard library
  2. Third-party
  3. Internal modules (cùng repo)
- Mỗi nhóm cách nhau 1 dòng trống.
- Không dùng wildcard import trừ khi framework bắt buộc.
- Loại bỏ import không dùng.

## 6) Cấu trúc module

- Mỗi module chỉ nên có một trách nhiệm chính.
- API/public surface tối giản; ẩn implementation details.
- Tránh phụ thuộc vòng tròn giữa modules.
- Tách rõ lớp: transport/API ↔ service/business ↔ data access/model.

## 7) Quy ước theo ngôn ngữ

## 7.1 Rust (backend + tauri)

- Naming: type `PascalCase`, function/variable/module `snake_case`, constant `UPPER_SNAKE_CASE`.
- Ưu tiên `Result<T, E>` + custom error type theo domain.
- Dùng `clippy` và `rustfmt` làm baseline formatting/lint.
- Tránh `unwrap/expect` trong production path.

## 7.2 TypeScript/React (frontend)

- Component: `PascalCase`; hooks: `useXxx`; function/variable: `camelCase`.
- Type/interface đặt tên theo domain (`SensorSnapshot`, `ControlCommandPayload`), tránh `any`.
- Ưu tiên type narrowing và explicit return type cho hàm public.
- Side effects (fetch/websocket/subscription) phải cleanup rõ ràng.

## 7.3 C++ (firmware ESP32)

- Type/class: `PascalCase`; function/variable: `snake_case` (theo chuẩn C++ embedded trong repo này).
- Hạn chế cấp phát động không kiểm soát trong vòng lặp thời gian thực.
- Mọi hằng phần cứng (pin, timing, threshold) phải đặt tên rõ nghĩa.
- Error/status từ sensor/communication phải có retry/backoff hoặc chiến lược fail-safe.

## 7.4 SQL migration

- Tên migration có timestamp + mô tả ngắn rõ nghĩa.
- Không viết migration “đa mục đích” khó rollback.
- Tên bảng/cột theo `snake_case`, phản ánh domain.

## 8) Anti-pattern bắt buộc loại bỏ

- Viết tắt khó hiểu, không theo domain (`cfg`, `mgr`, `val2`, `tmpData`).
- Magic number trong business logic (đặc biệt ngưỡng pH/EC, lịch dosing, thời gian retry).
- Một khái niệm nhưng đặt nhiều tên khác nhau giữa backend/frontend (`cropSeason` vs `seasonCrop` vs `cs`).
- Hàm quá dài, làm nhiều việc không liên quan.
- Catch/log lỗi chung chung không có context (`"error occurred"`).
- Comment kể lại code thay vì nêu quyết định kỹ thuật.
- Dùng kiểu dữ liệu “mơ hồ” (`any`, `serde_json::Value` không schema, `void*`) khi có thể định kiểu rõ.

## 9) Governance

- Bất kỳ thay đổi chuẩn nào phải cập nhật trực tiếp tài liệu này trong cùng PR.
- Reviewer có quyền yêu cầu rename/refactor để đạt chuẩn trước khi merge.
- Khi xung đột convention giữa module cũ và chuẩn mới, ưu tiên chuẩn mới cho code mới/chạm vào.
