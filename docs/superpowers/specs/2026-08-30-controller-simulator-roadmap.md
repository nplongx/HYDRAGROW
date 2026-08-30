# ROADMAP — `hydragrow-simulator` (Digital-Twin / Controller-in-the-Loop)

Đọc kèm: [2026-08-30-controller-simulator-digital-twin-spec.md](../specs/2026-08-30-controller-simulator-digital-twin-spec.md) — mọi quyết định kiến trúc, mọi trích dẫn code thật đều nằm ở đó. File này chỉ chia việc theo Phase.

**Cách dùng file này (dành cho agent thực thi, VD Jules):**
1. Đọc `CONTRIBUTING.md` + `docs/superpowers/specs/module-rules/README.md` trước — bắt buộc theo quy ước repo.
2. Với MỖI Phase dưới đây: dùng skill `brainstorming` nếu có điểm chưa rõ, rồi dùng skill `writing-plans` để tự viết một plan chi tiết kiểu TDD (Task → Files → Step 1 viết test fail → Step 2 chạy xác nhận fail → Step 3 implement → Step 4 chạy xác nhận pass), đúng format đã thấy ở `docs/superpowers/plans/2026-08-30-action-blocks-dosing-water-estop.md`. Lưu plan đó vào `docs/superpowers/plans/<ngày>-simulator-phase-<n>-<slug>.md` trước khi code.
3. Một Phase = một hoặc vài PR nhánh `feat/simulator-phase-<n>-<slug>`, theo đúng quy trình PR ở `CONTRIBUTING.md`.
4. Nếu trong lúc đọc code phát hiện một giả định ở đây SAI (VD: field đã đổi tên, hàm không còn `pub`) — dừng lại, ghi "Correction" ở đầu plan riêng của Phase đó (đúng tinh thần mục "Correction trước khi bắt đầu" trong plan mẫu), không âm thầm đoán tiếp.
5. Nếu ≥2 Phase/Task không phụ thuộc lẫn nhau cần chạy song song, dùng skill `parallel-worktree-sessions` — mục "Parallel lanes" ở cuối mỗi Phase đã gợi ý ranh giới an toàn để tách lane.
6. Trước mọi PR, chạy đúng bộ lệnh kiểm tra trong `module-rules/README.md` cho subsystem đã sửa, cộng thêm bộ lệnh riêng của `hydragrow-simulator` (thêm ở Phase 0).

---

## Phase 0 — Scaffolding crate + hoà nhập quy ước repo

**Mục tiêu:** `hydragrow-simulator` tồn tại, build được, có chỗ đứng chính danh trong CI/module-rules — trước khi viết bất kỳ logic mô phỏng nào.

**Vì sao làm trước:** mọi Phase sau đều cần crate này tồn tại và đã có path-dependency đúng vào `hydragrow-controller-core` + `hydragrow-shared`. Rủi ro thấp, không đụng code hiện có.

**Files tạo mới:**
- `hydragrow-simulator/Cargo.toml`
- `hydragrow-simulator/src/main.rs`, `src/lib.rs` (chỉ cần `fn main() { println!("hydragrow-simulator boot"); }` + module rỗng, chưa cần logic)
- `docs/superpowers/specs/module-rules/simulator.md` — copy cấu trúc của `controller-core.md`, điều chỉnh nội dung: crate host-native thuần Rust, không phụ thuộc esp-idf, **chỉ được phép** phụ thuộc `hydragrow-controller-core` + `hydragrow-shared` qua `path = "../..."`, không được import ngược lại thứ gì từ `ESP32-C3-CONTROLLER-NODE` hay `hydragrow-backend`.
- `.github/workflows/simulator-ci.yml` (nếu repo dùng GitHub Actions — kiểm tra thư mục `.github/workflows/` trước khi viết để bắt chước đúng format các workflow còn lại, VD `controller-core-ci`), trigger khi PR chạm `hydragrow-simulator/` hoặc `hydragrow-controller-core/` hoặc `hydragrow-shared/`.

**Files sửa:**
- `docs/superpowers/specs/module-rules/README.md` — thêm 1 dòng vào bảng subsystem trỏ tới `simulator.md`, thêm dòng lệnh test vào bảng "Kiểm tra chung": `(cd hydragrow-simulator && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)`.
- `README.md` (root) — thêm dòng vào bảng Subsystems và bảng CI, follow đúng pattern các dòng đang có.

**Cargo.toml gợi ý (điều chỉnh version thật theo `cargo add` lúc thực thi, không chép số version ở đây làm chân lý):**
```toml
[package]
name = "hydragrow-simulator"
version = "0.1.0"
edition = "2024"

[dependencies]
hydragrow-controller-core = { path = "../hydragrow-controller-core" }
hydragrow-shared = { path = "../hydragrow-shared" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
```

**Acceptance:**
- [ ] `cd hydragrow-simulator && cargo build` chạy sạch.
- [ ] `cargo clippy --all-targets -- -D warnings` sạch.
- [ ] CI workflow mới chạy xanh trên PR test.
- [ ] `module-rules/README.md` và root `README.md` phản ánh đúng subsystem mới.

**Parallel lanes:** Phase này không chia lane — làm trước tiên, một mình.

---

## Phase 1 — Harness gọi `orchestrator::tick` + Virtual Hardware tĩnh (chưa có Plant Model)

**Mục tiêu:** Chạy được FSM thật (`hydragrow_controller_core::core::fsm::orchestrator::tick`) trong một vòng lặp do simulator điều khiển, với actuator ảo chỉ *ghi nhận* trạng thái (chưa phản hồi ngược vào sensor). Sensor vẫn lấy từ một timeline định sẵn (giống cách `tests/e2e/*.rs` đang làm tay) — mục đích Phase này là **chứng minh harness trung thực với ngữ nghĩa orchestrator**, trước khi thêm plant feedback ở Phase 2 (tách rủi ro).

**Public API cần dùng (đã xác minh có thật, xem spec mục 2.2–2.3):**
```rust
use hydragrow_controller_core::core::fsm::orchestrator;
use hydragrow_controller_core::core::fsm::context::SystemContext;
use hydragrow_controller_core::core::fsm::events::OrchestratorEvent;
use hydragrow_controller_core::hydragrow_shared::{ControllerConfig, SensorData};
```
Xem cách `ESP32-C3-CONTROLLER-NODE/src/runtime/fsm_loop.rs` áp `TickResult.delta` vào `ctx` (đọc file này TRƯỚC khi tự viết `harness.rs`, để không phát minh lại sai cách áp delta — đây là hàng thật đã chạy production).

**Files tạo mới:**
- `hydragrow-simulator/src/harness.rs` — `pub struct SimHarness { ctx: SystemContext, now_ms: u64, uptime_ms: u64, ... }`, `pub fn tick(&mut self, config: &ControllerConfig, sensors: &SensorData) -> TickResult` (bọc quanh `orchestrator::tick`, tự tăng `now_ms`/`uptime_ms`, tự áp `delta` vào `self.ctx`).
- `hydragrow-simulator/src/actuators/virtual_hw.rs` — `pub struct VirtualHardwareState { pub pump_a, pump_b, ph_up, ph_down: PumpState, pub osaka_pwm: u32, pub mist_valve, mix_valve: bool, ... }` với `PumpState { on: bool, pwm_percent: u32 }`.
- `hydragrow-simulator/src/dispatcher.rs` — `pub fn apply_event(state: &mut VirtualHardwareState, event: &OrchestratorEvent)`. **Phải match đầy đủ mọi variant** của `OrchestratorEvent` (liệt kê ở spec mục 2.3) — các variant Publish*/Save*/Trigger*/Update*/Reboot/FactoryReset ban đầu chỉ cần `tracing::debug!` (no-op có log), KHÔNG được dùng `_ => {}` (sẽ nuốt mất variant mới thêm sau này mà không ai biết — vi phạm đúng lý do enum không đánh dấu non_exhaustive).
- `hydragrow-simulator/src/scenario/timeline.rs` — tái sử dụng tinh thần của `tests/helpers/fixtures.rs`: các hàm dựng `SensorData` theo kịch bản cố định (không cần đọc file ngoài ở Phase này).

**Acceptance (bắt buộc viết test, không chỉ chạy tay):**
- [ ] Test tích hợp trong `hydragrow-simulator/tests/`: dựng lại **đúng kịch bản** của `hydragrow-controller-core/tests/e2e/full_dosing_cycle.rs` nhưng lái qua `SimHarness` + `apply_event` thay vì gọi `orchestrator::tick` trực tiếp trong file test — assert cùng chuỗi `SystemPhase` mà file gốc kỳ vọng. Đây là bằng chứng harness không lệch ngữ nghĩa so với core.
- [ ] `apply_event` có test unit cho từng variant hardware (`SetDosingPump`, `SetWaterPump`, `SetMistValve`, `SetMixValve`, `SetOsakaPump`, `StartOsakaSoft`) — input event → đúng field nào đổi trong `VirtualHardwareState`.
- [ ] CLI (`main.rs`) chạy được `cargo run -- run --ticks 100` và in ra state cuối cùng (chưa cần đẹp).

**Parallel lanes:**
- Lane A: `harness.rs` + test tích hợp (phụ thuộc đọc kỹ `fsm_loop.rs`).
- Lane B: `dispatcher.rs` + `virtual_hw.rs` + test unit từng event (độc lập với Lane A, chỉ cần biết chữ ký `OrchestratorEvent`).
Hai lane có thể chạy song song bằng worktree riêng, merge tại `main.rs`.

---

## Phase 2 — Plant Model (vòng phản hồi EC/pH/temp/water_level thật)

**Mục tiêu:** Đóng vòng phản hồi thật: bơm chạy → tank đổi EC/pH → sensor model đọc tank → `SensorData` mới → tick kế tiếp. Đây là phần giá trị cốt lõi của cả simulator (không có phần này thì Phase 1 chỉ là "test runner có vỏ CLI", không phải digital twin).

**Ràng buộc bắt buộc (xem spec mục 2.4 — không thương lượng):** mọi hệ số hoá học phải đọc từ `ControllerConfig`, KHÔNG hardcode số riêng trong `hydragrow-simulator`:
- `ec_gain_per_ml`, `ph_shift_up_per_ml`, `ph_shift_down_per_ml`
- `pump_a_capacity_ml_per_sec`, `pump_b_capacity_ml_per_sec`, `pump_ph_up_capacity_ml_per_sec`, `pump_ph_down_capacity_ml_per_sec`
- `tank_height`, `water_level_*` (cho cân bằng thể tích khi refill/drain)

**Files tạo mới:**
- `hydragrow-simulator/src/plant/tank.rs` — `pub struct Tank { pub volume_l, ec, ph, temp, water_level: f32 }`, `pub fn step(&mut self, dt_ms: u64, actuators: &VirtualHardwareState, config: &ControllerConfig)`.
  - Model tuyến tính bậc 1 trước (đúng mục 8/9 đề xuất gốc): `ec += pump_a_flow_ml * config.ec_gain_per_ml / volume_l` v.v. — ghi rõ trong doc-comment công thức này là bậc-1, cố ý đơn giản, có thể nâng cấp phi tuyến sau.
  - Thêm `mixing_delay` bằng cách không cộng dồn tức thời mà làm mịn qua vài tick (VD low-pass filter đơn giản `ec_effective += (ec_target_after_dose - ec_effective) * alpha`), để tái hiện đúng mô tả "EC không nhảy ngay" ở mục 9/10 đề xuất gốc.
- `hydragrow-simulator/src/sensors/sensor_model.rs` — `pub fn read(tank: &Tank, noise_cfg: &NoiseConfig) -> SensorData`. Noise/offset/delay là tham số có thể tắt (Mode A cần `NoiseConfig::none()` để test được deterministic).

**Files sửa:**
- `hydragrow-simulator/src/harness.rs` — thay chỗ lấy `SensorData` từ timeline tĩnh (Phase 1) bằng `sensor_model::read(&tank, ...)`, đưa `Tank` vào vòng lặp chính.

**Refactor tuỳ chọn (ghi rõ trong PR nếu làm, KHÔNG bắt buộc để coi Phase 2 là "xong"):**
- Trích phần dựng `SensorData` thủ công lặp lại trong `hydragrow-controller-core/tests/e2e/full_dosing_cycle.rs`/`water_management.rs` thành hàm dùng chung — nhưng vì các test đó nằm TRONG crate `hydragrow-controller-core` còn `Tank` nằm ở crate `hydragrow-simulator` (phụ thuộc một chiều core → không phụ thuộc ngược), không thể import thẳng. Nếu muốn dùng chung thật sự, cần thêm một module nhỏ kiểu `hydragrow-controller-core/src/test_support.rs` (feature-gated `#[cfg(any(test, feature = "test-support"))]`) chứa đúng công thức tuyến tính, rồi cả `tests/e2e/*` lẫn `hydragrow-simulator` cùng gọi nó. Đây là quyết định cần người phụ trách `controller-core` duyệt riêng (đụng vào crate lõi) — nêu thành câu hỏi trong PR, đừng tự quyết.

**Acceptance:**
- [ ] Test snapshot (`insta`, đúng crate/style repo đã dùng ở `controller-core`) cho một kịch bản dosing xác định (Mode A, không noise): EC đi từ 0.8 → hội tụ về `ec_target ± ec_tolerance` trong số tick hữu hạn, review diff bằng mắt khi snapshot đổi (không auto-accept — đúng checklist `module-rules/controller-core.md`).
- [ ] Test riêng cho `Tank::step` bằng input giả lập thuần số liệu (không cần chạy FSM) — input pump flow cố định → output EC/pH đúng công thức kỳ vọng, tính tay so sánh assert.
- [ ] Test: tắt hết actuator → tank không đổi state ngoài decay/mixing tự nhiên (nếu có mô hình đó).

**Parallel lanes:**
- Lane A: `plant/tank.rs` (EC/pH chemistry).
- Lane B: `sensors/sensor_model.rs` (noise/delay) — có thể code song song với Lane A miễn thống nhất trước chữ ký `Tank` (struct fields) làm "hợp đồng" giữa 2 lane.

---

## Phase 3 — Scenario Engine + Fault Injection + Recorder

**Mục tiêu:** Biến simulator thành "test laboratory" thật cho `SafetyGuard`/FSM (đúng tinh thần mục 12–14 đề xuất gốc), có log ra để xem lại.

**Files tạo mới:**
- `hydragrow-simulator/src/scenario/format.rs` — struct scenario serde (JSON), fields tối thiểu: `initial_tank`, `config_overrides` (partial `ControllerConfig`, chỉ field hay đổi), `faults: Vec<FaultEvent>` (mỗi fault có `at_tick` hoặc `at_ms`, loại, tham số).
- `hydragrow-simulator/src/scenario/library/*.json` — ít nhất các kịch bản tương ứng 1-1 với `FaultCode` đã liệt kê ở spec mục 2.6: `sensor_timeout.json`, `ec_stagnant.json`, `ph_oscillating.json`, `water_level_critical.json`, `too_many_refills.json`, v.v. Đặt tên file trùng tên biến thể `FaultCode` (viết snake_case) để không ai phải đoán scenario nào test fault nào.
- `hydragrow-simulator/src/faults/injector.rs` — áp fault vào `Tank`/`VirtualHardwareState`/sensor pipeline theo mô tả trong scenario (VD "pump_a_stuck" = khoá `PumpState.on = true` bất kể event; "ec_sensor_disconnected" = sensor_model trả EC cũ/đóng băng).
- `hydragrow-simulator/src/telemetry/recorder.rs` — ghi mỗi tick ra CSV: `time,phase,ec,ph,temp,level,pump_a,pump_b,ph_up,ph_down,osaka_pwm`. Ghi ra `--output <path>.csv` qua CLI flag.
- CLI: `hydragrow-sim run --scenario <name.json> --record out.csv`.

**Acceptance:**
- [ ] Với mỗi scenario trong `library/`, có một test tự động assert: sau N tick, `ctx.phase` là đúng `SystemPhase::Fault(FaultCode::X)` (hoặc `EmergencyStop`) tương ứng — KHÔNG chỉ chạy tay xem log. Đây là phần "giá trị nhất" người dùng nhắc tới ở mục 14, nên bắt buộc tự động hoá, không để lại dạng demo thủ công.
- [ ] Recorder test: chạy 1 scenario ngắn, đọc lại CSV, assert số dòng = số tick, cột không rỗng.
- [ ] `injector.rs` có test đơn vị: từng loại fault áp đúng 1 thay đổi, không side-effect chéo sang fault khác.

**Parallel lanes:**
- Lane A: `scenario/format.rs` + `library/*.json` + test assert-phase (cần Phase 2 xong).
- Lane B: `telemetry/recorder.rs` (chỉ phụ thuộc `harness.rs` của Phase 1, có thể làm sớm hơn, song song với cả Phase 2 nếu muốn — ít rủi ro nhất trong toàn roadmap).

---

## Phase 4 — Digital-Twin Bridge: nói MQTT thật vào stack quan sát có sẵn

**Mục tiêu:** Cho simulator đóng vai "thiết bị giả" publish đúng topic thật, để `hydragrow-backend` + `hydragrow-frontend` hiện có hiển thị nó y như thiết bị thật — không viết dashboard mới (xem spec mục 3.5).

**Đọc trước khi code:** `hydragrow-shared/src/topics.rs` (toàn bộ hàm `topic_*`/`MqttTopics::*`), `hydragrow-backend/README.md` (cách chạy backend + biến môi trường MQTT broker), `docker-compose.yml` (service `mosquitto` port 1883).

**Files tạo mới:**
- `hydragrow-simulator/src/telemetry/mqtt_bridge.rs`:
  - Publish `SensorData` (JSON) lên `hydragrow_shared::topics::topic_sensors(device_id)` mỗi tick — **dùng đúng struct `SensorData` với `Serialize` có sẵn**, không tự chế JSON tay (tránh lệch schema với `shared.md` rule về thay đổi hợp đồng).
  - Publish FSM/telemetry lên `topic_fsm_state`, `topic_fsm_transition`, `topic_controller_status`, `topic_dosing_report`, `topic_dosing_cycle`, `topic_water_cycle` — map từ đúng các `OrchestratorEvent::Publish*` mà `dispatcher.rs` (Phase 1) hiện đang chỉ log — nối chúng vào bridge này thay vì no-op khi feature `mqtt` bật.
  - Dùng crate MQTT client cùng họ với backend nếu hợp lý (`rumqttc` — backend đã dùng, xem `hydragrow-backend/Cargo.toml` trước khi chọn) để tránh thêm dependency không cần thiết.
- CLI: `hydragrow-sim run --scenario <name> --mqtt tcp://localhost:1883 --device-id sim-001`.

**Rõ ràng về scope KHÔNG làm (ghi trong PR để review không hiểu nhầm là thiếu):**
- Không nhận lệnh điều khiển qua `controller/command` ở Phase này (đó là `command_handler.rs` phía ESP32, gắn với kiểu esp-idf ở vài chỗ — xem spec mục 6). Simulator ở Phase 4 chạy 100% `ControlMode::Auto`, chỉ publish, không subscribe lệnh. Ghi rõ đây là stretch-goal riêng nếu người dùng muốn sau này bấm nút trên frontend thật để điều khiển thiết bị giả.

**Acceptance:**
- [ ] Chạy `docker-compose up mosquitto influxdb postgres backend` + `cd hydragrow-frontend && npm run dev:web`, chạy simulator trỏ vào `mosquitto` local, quan sát bằng mắt: dashboard frontend hiện đúng EC/pH/phase của thiết bị giả theo thời gian thực.
- [ ] Test tích hợp (có thể cần `#[ignore]` + chạy thủ công trong CI nếu cần broker thật): publish xong, subscribe lại bằng client test, assert payload deserialize đúng `SensorData`/kiểu tương ứng.
- [ ] Không có thay đổi nào trong `hydragrow-shared/src/topics.rs` hay format payload (nếu cần đổi, phải theo đúng quy trình `shared.md` — cập nhật đồng thời mọi consumer trong 1 PR).

**Parallel lanes:** Phase này phụ thuộc trực tiếp Phase 1+3 xong (cần `dispatcher.rs` và các `Publish*` event đã có chỗ móc vào) — không chạy song song với Phase 1/2, nhưng có thể chạy song song với Phase 5.

---

## Phase 5 — CLI ergonomics: Step Simulation + kịch bản debug FSM

**Mục tiêu:** Hiện thực mục 16 đề xuất gốc — khả năng bấm từng bước để debug FSM, độc lập với việc có dashboard hay không.

**Files sửa:**
- `hydragrow-simulator/src/main.rs` — thêm subcommand `step` (REPL đơn giản hoặc flag `--step 100ms|1s|10s` chạy đúng 1 bước rồi in `ContextDelta` + `Vec<OrchestratorEvent>` vừa xảy ra ra stdout dạng human-readable, không cần đẹp).

**Acceptance:**
- [ ] `cargo run -- step --duration 100ms` chạy đúng 1 tick 100ms trên state đang lưu (cần cơ chế lưu/khôi phục state giữa các lần gọi CLI trong 1 phiên — có thể đơn giản là chế độ tương tác `run --interactive` giữ process sống, nhập lệnh qua stdin, thay vì phục hồi state giữa các lần gọi binary riêng biệt — quyết định UX cụ thể để Jules tự chọn khi viết plan chi tiết, miễn thoả acceptance "xem được từng tick một").

**Parallel lanes:** Độc lập hoàn toàn, có thể làm bất cứ lúc nào sau Phase 1, song song với Phase 3/4.

---

## Tổng kết trình tự bắt buộc vs. có thể song song

```text
Phase 0 (bắt buộc trước tiên)
   │
   ▼
Phase 1 ──┬── Lane A: harness.rs
          └── Lane B: dispatcher.rs + virtual_hw.rs
   │
   ▼
Phase 2 ──┬── Lane A: plant/tank.rs
          └── Lane B: sensors/sensor_model.rs
   │
   ├──────────────┬──────────────┐
   ▼              ▼              ▼
Phase 3        Phase 5        (không phase nào khác cần chờ)
(fault+scenario) (step CLI, độc lập từ sau Phase 1)
   │
   ▼
Phase 4 (MQTT bridge — cần Phase 1+3 xong)
```

## Câu hỏi cần người phụ trách xác nhận trước khi Jules chạy toàn bộ (đừng tự đoán)

1. Chuẩn xác tên broker/host dùng trong CI cho test tích hợp MQTT (Phase 4) — chạy `mosquitto` thật trong CI hay chỉ test unit phần serialize/publish logic, để `mqtt_bridge` test không cần network trong CI?
2. Có muốn refactor `test_support` dùng chung giữa `tests/e2e/*` và `hydragrow-simulator` (mục "Refactor tuỳ chọn" ở Phase 2), hay giữ 2 bản riêng cho tới khi thấy thật sự trùng lặp gây đau?
3. Ai là người review PR đụng `hydragrow-controller-core` (nếu Phase 2 optional refactor được chọn) — theo `CONTRIBUTING.md` cần biết trước để không bị chặn ở bước review.
