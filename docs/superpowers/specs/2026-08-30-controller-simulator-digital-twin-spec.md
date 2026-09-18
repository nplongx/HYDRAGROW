# SPEC — `hydragrow-simulator`: Digital-Twin / Controller-in-the-Loop Simulator

Status: **Implementation baseline / actively tracked** — Phase 0–4 have been implemented incrementally; this document is now the traceability baseline for the Digital Twin Runtime work. It does not claim overall Definition of Done until the remaining command/configuration/recipe/fault/scenario/backend-E2E evidence is green.
Liên quan: [module-rules/controller-core.md](./module-rules/controller-core.md), [module-rules/shared.md](./module-rules/shared.md), README.md (bảng CI + kiến trúc tổng quan).

## 1. Bối cảnh

Ý tưởng gốc (người dùng đề xuất): xây một PC simulator bao quanh `hydragrow-controller-core`, không dùng Xcos/Simulink, không mô phỏng ESP32, chỉ mô phỏng "thế giới bên ngoài" (sensor + actuator + plant model), theo 4 layer: Controller / Virtual Hardware / Plant Model / GUI.

Đã clone và đọc trực tiếp repo (`nplongx/HYDRAGROW`, nhánh mặc định) để xác minh đề xuất trước khi lập roadmap. Kết luận: **đề xuất đúng hướng ở phần lõi (Layer 1 + 2 + 3)**, nhưng có 5 điểm cần sửa/khớp lại với thực tế repo trước khi giao cho agent tự triển khai — nêu ở mục 3.

## 2. Sự thật đã xác minh trong repo (không suy đoán)

Tất cả trích dẫn dưới đây lấy trực tiếp từ source, không phải từ mô tả của người dùng.

### 2.1. `hydragrow-controller-core` đã hoàn toàn hardware-free

`hydragrow-controller-core/Cargo.toml`:
```toml
[dependencies]
serde = "1.0.228"
chrono = "0.4"
cron = "0.16.0"
log = "0.4"
tracing = { version = "0.1", features = ["log"] }
hydragrow-shared = { path = "../hydragrow-shared" }
anyhow = "1.0.104"
serde_json = "1.0.151"
```
Không có `esp-idf-hal`, `esp-idf-svc`, `embedded-hal` — những crate này CHỈ nằm trong `ESP32-C3-CONTROLLER-NODE/Cargo.toml`. Điều này khớp đúng với `module-rules/controller-core.md`: *"Crate Rust thuần (không phụ thuộc esp-idf) ... giữ ranh giới đó, không import esp-idf-hal/esp-idf-svc vào crate này."* → Simulator có thể `path`-depend thẳng vào `hydragrow-controller-core` mà không kéo theo bất kỳ dependency phần cứng nào. Đây chính là điều kiện tiên quyết để cả kiến trúc simulator khả thi, và nó **đã có sẵn**, không cần refactor gì thêm ở bước này.

### 2.2. Entry point pure-function đã tồn tại — không cần bịa ra interface mới

```rust
// hydragrow-controller-core/src/core/fsm/orchestrator.rs
pub fn tick(
    now_ms: u64,
    uptime_ms: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    sensor_last_update_ms: u64,
    ctx: &mut SystemContext,
) -> TickResult
```
```rust
// hydragrow-controller-core/src/core/fsm/tick_result.rs
pub struct TickResult {
    pub delta: ContextDelta,          // thay đổi state (phase, peripherals, calibration, ...)
    pub events: Vec<OrchestratorEvent>, // side-effect cần thực thi
}
```
`orchestrator::tick` **đã là** đúng "hàm nhân" mà simulator cần gọi lặp lại theo simulated clock. Không cần một `EventDispatcher` trait mới ở tầng core — simulator chỉ cần: build `SensorData` + `ControllerConfig`, gọi `tick()`, áp `TickResult.delta` vào `SystemContext` (xem cách `fsm_loop.rs` phía ESP32 áp delta để bắt chước đúng), rồi diễn giải `TickResult.events: Vec<OrchestratorEvent>`.

### 2.3. `OrchestratorEvent` đúng là "hardware abstraction contract" như người dùng nhận định

```rust
// hydragrow-controller-core/src/core/fsm/events.rs
pub enum OrchestratorEvent {
    SetDosingPump { pump: DosingPumpTarget, on: bool, pwm_percent: u32 },
    SetWaterPump { direction: WaterDirection },
    SetMistValve { on: bool },
    SetMixValve { on: bool },
    SetOsakaPump { pwm_percent: u32 },
    StartOsakaSoft { target_pwm_percent: u32 },
    SaveNvsSnapshot, SaveLastWaterChange{..}, SaveCurrentStageIndex{..},
    PublishFsmState, PublishCalibrationUpdate, PublishDosingReport{..},
    PublishSystemLog{..}, PublishRecipeStageChanged{..}, PublishFsmTransition{..},
    PublishDosingCycle{..},
    RequestSensorForcePublish, SetSensorContinuousMode{..},
    TriggerOtaUpdate, UpdateWifiList{..}, RebootDevice, FactoryReset,
}
```
Enum không có `#[non_exhaustive]` → một `match` đầy đủ ở phía simulator sẽ **buộc phải cập nhật** nếu core thêm variant mới (compile error), nên đây là ranh giới an toàn để dựa vào lâu dài.

### 2.4. Config đã chứa sẵn công thức plant model — không được bịa hằng số mới

```rust
// hydragrow-shared/src/lib.rs — struct ControllerConfig
pub ec_gain_per_ml: f32,
pub ph_shift_up_per_ml: f32,
pub ph_shift_down_per_ml: f32,
pub pump_a_capacity_ml_per_sec: f32,
pub pump_b_capacity_ml_per_sec: f32,
pub pump_ph_up_capacity_ml_per_sec: f32,
pub pump_ph_down_capacity_ml_per_sec: f32,
```
Đây chính xác là mô hình tuyến tính "EC += nutrient_a_effect + nutrient_b_effect" mà đề xuất gốc mô tả ở mục 8/9 — nhưng nó **đã tồn tại như một phần hợp đồng cấu hình thật** (dùng bởi `AutoTuner`/`gain_learner` để học `best_ec_ratio`, `best_ph_ratio`). Nếu Plant Model của simulator tự bịa ra hệ số riêng thay vì đọc từ `ControllerConfig`, hai bên sẽ lệch nhau âm thầm — vi phạm trực tiếp module-rules chung #2 ("không đặt logic dùng chung ở 2 nơi"). Plant Model **bắt buộc** dùng đúng các field này làm nguồn sự thật duy nhất.

### 2.5. Đã có "phôi" plant simulation nằm rải rác trong test — cần thu gom, không viết lại từ đầu

```
hydragrow-controller-core/tests/
├── helpers/fixtures.rs        # auto_config(), balanced_sensors(), low_ec_sensors(), noisy_ec_sensors(prev_ec)...
└── e2e/
    ├── full_dosing_cycle.rs   # 284 dòng — dựng sensor SỬA TAY qua từng bước để mô phỏng chu trình dosing
    ├── water_management.rs    # 246 dòng
    └── fault_recovery.rs      # 273 dòng
```
Các file e2e hiện tại **mô phỏng thủ công** việc EC/pH thay đổi sau khi bơm chạy (gán tay `SensorData { ec: ..., ..prev }` giữa các lần gọi `tick()`), vì chưa có Plant Model thật. Đây chính là logic mà Phase 2 (Plant Model) sẽ trích xuất và thay bằng một implementation dùng chung — vừa làm nguồn cho simulator, vừa có thể (tuỳ chọn, không bắt buộc) làm cho chính các test e2e này thật hơn.

### 2.6. Vocabulary lỗi đã có sẵn — fault injection phải map vào đây, không đặt tên song song

```rust
// hydragrow-shared/src/fsm.rs
pub enum FaultCode {
    EcDosingFailed, PhDosingFailed, WaterRefillFailed, WaterDrainFailed,
    TooManyRefills, TooManyDrains, MaxHourlyDoseEc, MaxHourlyDosePh,
    SensorTimeout, EcStagnant, PhOscillating, WaterLevelCritical, EmergencyStop,
}
```

### 2.7. Repo KHÔNG dùng Cargo workspace gộp — đây là quy ước có chủ đích

`docs/superpowers/specs/module-rules/README.md`:
> *"Rust workspaces (chạy riêng từng thư mục — không có Cargo workspace gộp)"* — kèm bảng lệnh kiểm tra chạy riêng `cd hydragrow-shared && cargo test`, `cd hydragrow-controller-core && cargo test`, v.v.

→ Đề xuất gốc ghi "Tạo workspace member mới" (mục 19) hơi lệch thuật ngữ: `hydragrow-simulator` sẽ là **một crate độc lập ngang hàng** (giống `hydragrow-backend`, `hydragrow-controller-core`), path-dependency trực tiếp vào `hydragrow-controller-core` + `hydragrow-shared`, **không** tạo root `Cargo.toml [workspace]` mới. Vị trí thư mục (`HYDRAGROW/hydragrow-simulator/`) mà người dùng đề xuất là đúng, chỉ cần sửa cách gọi nó.

### 2.8. Toàn bộ hạ tầng quan sát/dashboard đã tồn tại và chạy được ngay hôm nay

```yaml
# docker-compose.yml
services: [postgres, mosquitto, nodered, influxdb, backend]
```
```rust
// hydragrow-shared/src/topics.rs
MqttTopics::sensors(device_id)            // AGITECH/{id}/sensors
MqttTopics::fsm_state(device_id)          // AGITECH/{id}/fsm/state
MqttTopics::controller_status(device_id)  // AGITECH/{id}/controller/status
MqttTopics::dosing_report(device_id)      // AGITECH/{id}/dosing_report
topic_fsm_transition(device_id)           // AGITECH/{id}/fsm/transition
topic_dosing_cycle / topic_water_cycle / topic_calibration / topic_controller_command ...
```
`hydragrow-backend` (Actix-web + rumqttc + InfluxDB + Postgres + WebSocket) và `hydragrow-frontend` (React/TS/Tauri, đã có dashboard EC/pH/phase/pump) **đã chạy production**, tiêu thụ đúng các topic trên. Đây là điểm khác biệt lớn nhất so với đề xuất gốc — xem mục 3.5.

## 3. Các điểm sửa so với đề xuất gốc

1. **Không tạo Cargo workspace gộp** (2.7) — giữ `hydragrow-simulator` là crate độc lập, đúng quy ước repo.
2. **Không bịa hệ số EC/pH mới cho Plant Model** (2.4) — đọc thẳng từ `ControllerConfig`.
3. **Không viết Plant Model từ số 0** (2.5) — trích xuất logic mô phỏng tay đã có trong `tests/e2e/*.rs` thành module dùng chung.
4. **Fault injection map vào `FaultCode` có sẵn** (2.6), không đặt tên song song (`sensor_frozen`, `pump_stuck`... ở đề xuất gốc chỉ là *nguyên nhân*, kết quả kỳ vọng vẫn phải là một `FaultCode` cụ thể đã tồn tại — nếu kịch bản nào không map được vào `FaultCode` nào cả, đó là dấu hiệu cần dừng lại hỏi người phụ trách, không tự thêm `FaultCode` mới mà không có xác nhận, vì thêm `SystemPhase`/fault mới kéo theo nghĩa vụ cập nhật `phase_tick.rs` + test theo đúng `module-rules/controller-core.md`).
5. **Không tự xây dashboard Python/FastAPI/Plotly mới** (mục 15 đề xuất gốc) làm mặc định. Repo đã có full stack quan sát thật (mosquitto + InfluxDB + backend + frontend) tiêu thụ đúng `hydragrow-shared::topics`. Cách rẻ nhất để có GUI "xịn" là cho simulator **đóng vai một thiết bị giả** (fake sensor-node + fake controller-node) nói đúng giao thức MQTT hiện có, rồi chạy `docker-compose up mosquitto influxdb postgres backend` + `npm run dev:web` — không viết thêm một dòng UI nào. Dashboard tự-viết (CLI bảng ASCII hoặc web riêng) vẫn có chỗ dùng cho Mode A/B (test nhanh, không cần Docker) nhưng là lớp phụ, không phải lớp chính. Chi tiết ở roadmap Phase 4.
6. **Repo đã có sẵn quy trình spec → plan → module-rules** dưới `docs/superpowers/` (xem `docs/superpowers/plans/2026-08-30-action-blocks-dosing-water-estop.md` làm mẫu format: mỗi Task có Files / Step 1 viết test fail / Step 2 chạy xác nhận fail / Step 3 implement / Step 4 chạy xác nhận pass). Roadmap ở file đi kèm được viết ở granularity "Phase" để Jules tự mở rộng thành plan chi tiết theo đúng format này cho từng Phase bằng skill `writing-plans` sẵn có, thay vì tôi viết sẵn toàn bộ TDD step (làm vậy sẽ hàng nghìn dòng và có thể lệch với các quyết định nhỏ Jules phát hiện ra khi đọc code, giống chính commit `action-blocks` đã tự đính chính kế hoạch cũ của nó).

## 4. Kiến trúc cuối cùng

```text
                    hydragrow-controller-core (không đổi, host-native)
                              │  orchestrator::tick(now_ms, uptime_ms, config, sensors, ..., &mut ctx)
                              │  -> TickResult { delta, events: Vec<OrchestratorEvent> }
              ┌───────────────┴────────────────┐
              │ (đã có, ESP32 crate)            │ (MỚI, crate riêng)
              ▼                                 ▼
     ESP32 EventDispatcher              hydragrow-simulator::SimDispatcher
     (runtime/dispatcher.rs)                    │
              │                        ┌────────┴─────────┐
              ▼                        ▼                  ▼
        Hardware thật          Virtual Actuators     Telemetry sinks
                               (pump/valve state)    (CSV/JSON, hoặc
                                       │              MQTT fake-device
                                       ▼              → stack thật)
                                 Plant Model
                            (Tank: ec/ph/temp/level,
                             dùng ec_gain_per_ml/
                             ph_shift_*_per_ml/
                             pump_*_capacity_ml_per_sec
                             từ chính ControllerConfig)
                                       │
                                       ▼
                                 Sensor Model
                            (noise/delay/dropout)
                                       │
                                       ▼
                             hydragrow_shared::SensorData
                                       │
                                       └──────────► vòng lặp tick kế tiếp
```

## 5. Cấu trúc thư mục crate mới

```text
HYDRAGROW/
├── hydragrow-simulator/                 # crate độc lập, ngang hàng, KHÔNG phải workspace member
│   ├── Cargo.toml                       # path-dep: hydragrow-controller-core, hydragrow-shared
│   └── src/
│       ├── main.rs                      # CLI (clap): run / step / scenario list
│       ├── lib.rs
│       ├── harness.rs                   # vòng lặp gọi orchestrator::tick + áp ContextDelta
│       ├── dispatcher.rs                # match Vec<OrchestratorEvent> -> virtual hardware
│       ├── plant/
│       │   ├── tank.rs                  # EC/pH/temp/water_level state + step(dt, actuators)
│       │   └── chemistry.rs             # công thức đọc từ ControllerConfig (2.4)
│       ├── actuators/
│       │   └── virtual_hw.rs            # VirtualPump, VirtualValve, trạng thái PWM/flow
│       ├── sensors/
│       │   └── sensor_model.rs          # noise/delay/dropout quanh giá trị Plant thật
│       ├── faults/
│       │   └── injector.rs              # scenario fault -> tác động lên plant/sensor/actuator
│       ├── scenario/
│       │   ├── format.rs                # struct scenario (serde, JSON/YAML)
│       │   └── library/*.json           # kịch bản mẫu
│       └── telemetry/
│           ├── recorder.rs              # CSV/JSON theo tick
│           └── mqtt_bridge.rs           # (Phase 4) publish/subscribe đúng hydragrow_shared::topics
└── docs/superpowers/specs/module-rules/
    └── simulator.md                     # MỚI — rule riêng cho crate này (xem roadmap Phase 0)
```

## 6. Việc KHÔNG làm ở scope này (out of scope, cần xác nhận riêng nếu muốn mở rộng)

- Không mô phỏng CPU/FreeRTOS/GPIO của ESP32 (đúng như đề xuất gốc mục 4) — chỉ cần khi test firmware thật, không phải mục tiêu digital-twin controller-logic.
- Không refactor `command_handler.rs`/`process_mqtt_commands` (ESP32 crate, phụ thuộc kiểu esp-idf ở một số điểm) để dùng chung với simulator — Phase 4 chỉ *publish* telemetry giả, việc *nhận lệnh MQTT thủ công* vào simulator là stretch-goal riêng, ghi rõ trong roadmap, không âm thầm mở rộng scope.
- Không thêm `SystemPhase`/`FaultCode` mới trừ khi có xác nhận — theo đúng ràng buộc của `module-rules/controller-core.md`.
- Không sửa `hydragrow-backend/migrations/` hay `server_wallet.json` (nằm trong danh sách "Không đụng vào" ở README.md/CONTRIBUTING.md).

## 7. Implementation status — 2026-09-18

This section records the current repository state against the Digital Twin Runtime target. **Implemented** means code exists and has been exercised by repository tests/build checks; **in progress** means the architectural path exists but the full contract/e2e acceptance is not yet demonstrated; **not implemented** means no claim is made.

| Spec capability | Status | Current repository evidence / boundary |
|---|---|---|
| Host-native simulator crate | **Implemented** | `hydragrow-simulator` is an independent crate with path dependencies on `hydragrow-controller-core` and `hydragrow-shared`; no ESP-IDF dependency. |
| Deterministic virtual clock | **Implemented** | `src/clock.rs` provides `VirtualClock` with deterministic `now_ms`, uptime, advance and restart semantics. |
| Virtual controller lifecycle | **Implemented / in progress** | `src/controller.rs` models BOOT/CONNECTING/CONNECTED/RUNNING/DEGRADED/OFFLINE and restart/boot identity. Full MQTT/backend lifecycle acceptance remains. |
| Real controller-core FSM execution | **Implemented** | Harness drives the real `hydragrow-controller-core` orchestrator and applies its `ContextDelta`; no duplicate FSM was introduced. |
| Virtual actuator desired vs actual state | **Implemented** | Virtual pumps retain desired and actual state; actuator faults can create mismatch. |
| Causal virtual plant | **Implemented** | Tank dynamics consume actuator actual state and controller configuration rather than independent random sensor values. |
| Deterministic sensor model | **Implemented / in progress** | Sensor values originate from plant state; deterministic no-noise mode and sensor fault injection exist. Remaining timing/dropout semantics need scenario coverage. |
| Canonical shared MQTT topics/schema | **Implemented** | MQTT bridge uses `hydragrow-shared` topic helpers and shared payload types; no production `/twin/...` protocol was introduced. |
| Twin inbound MQTT subscription | **Implemented / in progress** | Bridge subscribes to controller config/command/recipe topics and tracks connection state. Full typed routing and end-to-end contract confirmation remain tracked work. |
| Command semantic reuse | **Implemented / in progress** | Command processing has been moved behind `hydragrow-controller-core` semantics, with the ESP adapter retaining persistent dedupe. Confirmation must still be proven from observed actuator/telemetry state rather than self-declaration. |
| Command lifecycle REQUESTED → SENT → ACKNOWLEDGED → CONFIRMED | **In progress** | Lifecycle publish plumbing exists; complete observed-state confirmation, timeout, retry and duplicate-command behavior still require executable cross-system scenarios. |
| Configuration sync normal/offline/reconnect/restart | **In progress** | MQTT reconnect/subscription plumbing exists; convergence against backend `ConfigurationSync` is not yet accepted as complete. |
| Recipe set/clear parity | **In progress** | Relevant shared MQTT subscriptions exist; complete Twin-side recipe semantics and tests remain. |
| Communication faults | **In progress** | MQTT disconnect state exists; delay/drop/duplicate and deterministic reconnect scenario coverage remain. |
| Sensor missing/invalid/outlier faults | **Implemented** | Scenario fault types and injector behavior exist, including explicit error classification and deterministic outlier behavior. |
| Sensor stale / timing faults | **In progress** | Virtual clock foundation exists; stale, delayed and out-of-order telemetry need explicit executable scenarios. |
| Actuator stuck on/off | **Implemented** | Faults alter actual actuator state while preserving desired state, enabling command-vs-observed mismatch. |
| Actuator delayed | **Not implemented / not accepted** | No acceptance evidence yet. |
| Controller restart | **Implemented / in progress** | Restart resets controller/runtime state, hardware state and clock and creates a new boot identity; reconnect/resync acceptance remains. |
| Boot loop / telemetry pause / configuration loss | **Not implemented / not accepted** | No complete executable acceptance evidence yet. |
| Scenario engine | **Implemented / in progress** | Scenario format/injector exist; baseline library now includes normal operation, controller restart, sensor unavailable/invalid, actuator stuck on/off, plus existing stagnant/frozen-sensor cases. Communication/timing/controller-fault matrix is still incomplete. |
| Recorder / deterministic replay evidence | **Implemented / in progress** | Existing recorder and deterministic clock infrastructure exist; broader scenario evidence remains. |
| Backend + Twin E2E | **Not yet demonstrated** | Required path is real backend + real MQTT contract + Twin + actuator/plant + telemetry/lifecycle confirmation. No claim of completion until this is executable and green. |
| Physical HIL gate | **Separate / still required** | Twin verification supplements software cross-system verification; it does not replace the physical controller/sensor HIL gate. |

### 7.1 Current implementation boundary

The current architecture is intentionally:

```text
hydragrow-backend / scheduler / safety
                ↕ real MQTT contract
        hydragrow-simulator
          ├─ VirtualController
          ├─ controller-core::orchestrator::tick()
          ├─ VirtualHardware (desired / actual)
          ├─ VirtualPlant (causal)
          ├─ VirtualSensors
          ├─ FaultInjector + ScenarioEngine
          └─ MQTT bridge / telemetry
```

The authoritative controller state machine remains in `hydragrow-controller-core`. The Twin is an adapter/runtime around that core, not a second implementation of the controller FSM. The authoritative wire contract remains `hydragrow-shared`.

### 7.2 Verification snapshot

At this checkpoint, the following previously executed checks are green: simulator library tests, simulator integration tests, controller-core tests, ESP32 controller host-side `cargo check --locked --target riscv32imc-esp-espidf`, and `git diff --check`. These checks establish implementation safety for the completed increments; they **do not** constitute completion of the Digital Twin Definition of Done.

The remaining acceptance gate is the executable cross-system matrix covering command lifecycle, configuration/recipe synchronization, communication/timing faults, required scenario library, and backend↔Twin E2E behavior.

### 7.3 Explicit non-claims

- No 3D/full-physics/AI-ML/cloud Digital Twin platform is implied.
- No second database or fake backend/authentication layer is part of the Twin.
- No production MQTT `/twin/...` protocol is introduced.
- Passing Twin tests does not remove the need for physical HIL verification.
- Existing unrelated working-tree changes are outside this implementation status and must not be reverted as part of Twin work.
