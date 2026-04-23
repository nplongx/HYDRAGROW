# Quality Baseline (toàn repo)

Ngày rà soát: 2026-04-23  
Phạm vi: `hydragrow-backend`, `hydragrow-frontend`, `hydragrow-frontend/src-tauri`, `ESP32-C3-CONTROLLER-NODE`, `ESP32-C3-SENSOR-NODE`.

## Cách thu thập
- Static review theo module/package + đối chiếu naming giữa BE/FE/Firmware.
- Chạy check định dạng/build khả dụng trong môi trường hiện tại.

## 1) hydragrow-backend (Rust API)

### 1.1 Naming violations

| Priority | Nhóm vi phạm | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P1 | **Tên pump cùng 1 khái niệm nhưng nhiều alias** (`A`, `PUMP_A`, `OSAKA`, `OSAKA_PUMP`, `MIST`, `MIST_VALVE`, `PUMP_IN`, `WATER_PUMP_IN`,...) | `valid_pumps` cho phép song song nhiều tên cho cùng actuator trong API control. | Tăng độ phức tạp map dữ liệu, khó validate thống nhất FE/BE/Firmware, dễ phát sinh bug khi thêm pump mới. | 1-2 ngày (định nghĩa enum canonical + compatibility layer deprecate alias). |
| P1 | **Temporal field naming bất nhất** (`time`, `timestamp`, `last_seen`, `timestamp_ms`) trong response/event | Các endpoint và websocket payload đang dùng nhiều biến thời gian khác nhau. | FE phải viết adapter đặc biệt theo từng luồng dữ liệu, khó chuẩn hóa telemetry schema. | 1 ngày (thiết kế contract thời gian chuẩn + adapter backward-compatible). |
| P2 | **Trạng thái online không thống nhất key** (`online` trong MQTT status retained vs `is_online` trong WS payload/API) | Firmware/controller publish `online`, backend WS transform sang `is_online`. | Tăng chi phí maintain pipeline trạng thái kết nối. | 0.5 ngày. |

### 1.2 Formatting/Lint violations

| Priority | Nhóm vi phạm | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P2 | **rustfmt check fail** (khoảng trắng thừa) | `cargo fmt --check` fail tại `src/db/mod.rs` (diff whitespace). | CI/pipeline formatting không sạch, gây nhiễu review. | <0.5 ngày. |
| P1 | **Thiếu policy lint nhất quán (clippy/rustdoc) trong baseline** | Chưa thấy script lint chuẩn hoá mức workspace cho backend. | Lỗi style/code-smell khó phát hiện sớm. | 1 ngày (thêm script + CI gate). |

### 1.3 Bất nhất kiến trúc

| Priority | Nhóm bất nhất | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P1 | **Response envelope không đồng nhất** (`{"status":"success","data":...}` vs trả object trực tiếp vs `{"error":...}`) | Nhiều handler API trả hình dạng khác nhau giữa module config/sensor/control/solana. | FE phải xử lý nhiều nhánh parse response, khó reusable API client. | 2-3 ngày (chuẩn hóa ApiResponse + error contract + migrate dần endpoint). |
| P1 | **Data-access pattern không đồng nhất** (vừa query trực tiếp trong handler, vừa qua db/service layer) | Một số route dùng `sqlx::query` trực tiếp ở `api/config`, một số route gọi `db::postgres`/`services`. | Khó test unit/integration và khó tách business logic khỏi transport layer. | 2-4 ngày (đưa về service/repository layer thống nhất). |

---

## 2) hydragrow-frontend (React + Tauri plugin HTTP)

### 2.1 Naming violations

| Priority | Nhóm vi phạm | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P1 | **Cùng 1 khái niệm pump/state nhưng mixed naming** (UPPER_SNAKE từ MQTT vs snake_case trong UI state) | `normalizePumpStatus` phải map nhiều biến thể (`PUMP_A` -> `pump_a`, `MIST` -> `mist_valve`,...). | Tăng logic chuyển đổi và rủi ro thiếu map khi protocol mở rộng. | 1-2 ngày (shared contract types + canonical naming). |
| P2 | **Device identity mixed style** (`deviceId` trong React state vs `device_id` payload/model) | Hooks/context dùng camelCase nội bộ nhưng payload snake_case. | Chấp nhận được ở boundary, nhưng thiếu guideline rõ ràng. | 0.5 ngày (ban hành convention + utility mapper). |

### 2.2 Formatting/Lint violations

| Priority | Nhóm vi phạm | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P1 | **Thiếu lint script chính thức** (`eslint`/`biome`/`prettier --check`) | `package.json` chỉ có `dev/build/preview/tauri`, không có `lint`/`format:check`. | Không có quality gate cho style/unsafe pattern React hooks. | 1 ngày (thiết lập lint + check formatting trong CI). |
| P2 | **Build warning chunk lớn (>500kB)** | `npm run build` cảnh báo bundle chính lớn. | Ảnh hưởng hiệu năng load app/webview và maintainability module boundary. | 1-2 ngày (split routes/lazy load/manualChunks). |

### 2.3 Bất nhất kiến trúc

| Priority | Nhóm bất nhất | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P1 | **Luồng data-fetch phân tán** (context tự fetch + hook tự fetch + WebSocket parse inline) | `DeviceContext` chứa nhiều concern (bootstrap config, fetch initial data, WS lifecycle, cache store), trong khi hooks khác cũng gọi HTTP trực tiếp. | Khó test, khó tái sử dụng, dễ side-effect chồng chéo. | 3-5 ngày (tách service layer + state machine/query layer như TanStack Query). |
| P2 | **Error handling pattern không đồng nhất** (toast inline, console.error, silent catch) | Nhiều `catch` rỗng hoặc xử lý thủ công khác nhau giữa hooks/context. | Giảm khả năng quan sát lỗi và nhất quán UX thông báo. | 1-2 ngày (error policy + central notifier/logger). |

---

## 3) hydragrow-frontend/src-tauri (Rust desktop bridge)

### 3.1 Naming violations

| Priority | Nhóm vi phạm | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P2 | **Boundary naming chưa chuẩn hóa rõ ràng với FE/BE** | Có nguy cơ lặp lại map field ở nhiều lớp khi command/WS model tăng. | Nợ kỹ thuật schema khi mở rộng command mới. | 1 ngày (định nghĩa shared DTO/conversion convention). |

### 3.2 Formatting/Lint violations

| Priority | Nhóm vi phạm | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P2 | **Chưa có baseline lint riêng module Tauri** | Chưa thấy command check riêng (clippy/fmt/check). | Chất lượng code phụ thuộc thói quen cá nhân. | 0.5-1 ngày. |

### 3.3 Bất nhất kiến trúc

| Priority | Nhóm bất nhất | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P2 | **Phân lớp command/client/model cần chuẩn hóa theo feature** | Cấu trúc file đã có (`commands`, `http_client`, `ws_client`, `models`) nhưng chưa có rule module boundary rõ ràng cho future feature. | Dễ trôi kiến trúc khi thêm nhanh tính năng. | 1-2 ngày (viết ADR + module template). |

---

## 4) ESP32-C3-CONTROLLER-NODE (Rust firmware)

### 4.1 Naming violations

| Priority | Nhóm vi phạm | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P1 | **Tên field/pump/action mixed theo nhiều domain** (business + transport + hardware alias) | `MqttCommandPayload` + mapping trong FSM/control cần xử lý nhiều biến thể command/pump. | Tăng coupling giữa protocol MQTT và FSM internal naming. | 2 ngày (protocol enum + translate layer tại ingress). |
| P2 | **Topic naming cùng loại status nhưng tách nhiều dạng** (`status`, `controller/status`, `sensor/status`) | Phải theo dõi nhiều topic status song song. | Tăng complexity cho backend subscriber/alert pipeline. | 1 ngày (chuẩn topic contract v2 + alias migration). |

### 4.2 Formatting/Lint violations

| Priority | Nhóm vi phạm | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P1 | **Không chạy được rustfmt trong môi trường hiện tại** (missing component) | `cargo fmt --check` báo thiếu `cargo-fmt` ở nightly toolchain. | Không xác nhận được baseline format tự động cho firmware Rust. | 0.5 ngày (pin toolchain + thêm rustfmt component trong setup/CI). |

### 4.3 Bất nhất kiến trúc

| Priority | Nhóm bất nhất | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P1 | **Main loop ôm nhiều trách nhiệm** (WiFi state, MQTT state, health publish, FSM relay) | `main.rs` điều phối gần như toàn bộ orchestration runtime. | Khó unit test, khó mở rộng retry/backoff logic tinh vi. | 3-4 ngày (tách supervisor/service objects theo concern). |
| P2 | **Business/FSM và transport gắn chặt qua payload string JSON** | Nhiều kênh `mpsc<String>` cho fsm/report/sensor command. | Khó tiến hóa schema có versioning/validation compile-time. | 2 ngày (typed message bus nội bộ + serialize tại biên MQTT). |

---

## 5) ESP32-C3-SENSOR-NODE (Arduino C++)

### 5.1 Naming violations

| Priority | Nhóm vi phạm | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P1 | **Tên biến/config mixed style** (`snake_case`, `camelCase`, macro `UPPER_CASE`) không theo guideline rõ ràng | `main.cpp` dùng đồng thời nhiều style cho cùng nhóm cấu hình/cảm biến. | Khó đọc/duy trì khi mở rộng nhiều cảm biến mới. | 1 ngày (ban hành style + rename cơ học). |
| P2 | **Topic command/config/status naming chưa đồng bộ hoàn toàn với controller/backend semantics** | Sensor có `.../sensor/status`, controller dùng thêm `.../status`/`.../controller/status`. | Cần adapter đa hướng ở backend. | 1 ngày. |

### 5.2 Formatting/Lint violations

| Priority | Nhóm vi phạm | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P1 | **Chưa có check formatting/lint tự động cho Arduino code** | Không thấy `clang-format`/`cpplint`/PlatformIO lint gate trong baseline. | Chất lượng style và warning phụ thuộc review thủ công. | 1-2 ngày (thêm clang-format + compile warnings -Wall/-Wextra). |

### 5.3 Bất nhất kiến trúc

| Priority | Nhóm bất nhất | Bằng chứng | Tác động | Effort ước lượng |
|---|---|---|---|---|
| P1 | **Blocking IO pattern** (`delay`, vòng lặp sync) khác mạnh so với controller event-driven Rust | `main.cpp` dùng nhiều thao tác blocking (đọc ADC, delay, WiFi reconnect loop). | Dễ tăng jitter publish và khó phối hợp khi thêm command thời gian thực. | 3-5 ngày (refactor non-blocking scheduler/state-driven loop). |
| P2 | **Parsing JSON & command xử lý inline trong callback lớn** | `mqttCallback` xử lý cả command + config trực tiếp. | Khó test từng rule parse/validation. | 1-2 ngày (tách parser + handler theo message type). |

---

## 6) Ưu tiên xử lý liên module (đề xuất roadmap)

1. **P0 (nên làm ngay trong sprint kế tiếp)**
   - Thiết kế **canonical naming contract** cho `pump`, `state`, `time fields` giữa FE/BE/Firmware.
   - Chuẩn hóa **API/WS response envelope** để FE parse 1 kiểu duy nhất.

2. **P1**
   - Thiết lập **quality gates**: rustfmt + clippy + eslint/prettier + clang-format.
   - Tách **service/repository layer** ở backend và giảm phân tán data-fetch ở frontend context.

3. **P2**
   - Chuẩn hóa kiến trúc module boundary (Tauri + firmware internal message typing).
   - Tối ưu bundle frontend, gom chuẩn logging/error policy.

## 7) Tổng effort ước lượng (macro)

- **P0:** 4-6 ngày công.
- **P1:** 8-14 ngày công.
- **P2:** 6-10 ngày công.

> Gợi ý thực thi: triển khai theo từng vertical slice (protocol -> backend adapter -> frontend adapter -> firmware) để luôn giữ backward compatibility trong quá trình migration.
