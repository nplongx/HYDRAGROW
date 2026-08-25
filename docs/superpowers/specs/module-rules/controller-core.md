# HYDRAGROW — `hydragrow-controller-core` Module Rules

Crate Rust thuần (không phụ thuộc `esp-idf`), chứa toàn bộ logic FSM/dosing/adaptive-control có thể test trên máy host (không cần flash firmware). Đây là lý do crate này tách khỏi `ESP32-C3-CONTROLLER-NODE` — giữ ranh giới đó, không import `esp-idf-hal`/`esp-idf-svc` vào crate này.

**Khi nào chạm module này:** `src/core/fsm/*` (orchestrator, phase_tick, events, phases), `src/core/actors/*` (dosing_actor, water_actor, safety_guard), `src/core/adaptive/*` (kalman, gain_learner, tuner, solver), `src/pump_types.rs`.

## Rules

- **`src/core/fsm/orchestrator.rs` là nơi DUY NHẤT được phép chuyển `SystemPhase`.** Actor (`dosing_actor.rs`, `water_actor.rs`, `safety_guard.rs`) chỉ được trả `TickResult`/event về orchestrator, không tự set phase.
- Thêm `SystemPhase` mới (trong `hydragrow-shared::fsm`) bắt buộc phải:
  1. Thêm nhánh xử lý trong `src/core/fsm/phase_tick.rs`
  2. Thêm test chuyển pha trong `tests/phase_transition_tests.rs`
  3. Cập nhật rule tương ứng nếu phase mới publish MQTT event mới (xem [shared.md](./shared.md))
- `safety_guard.rs` có quyền phủ quyết (veto) mọi actor khác — không actor nào được set `EmergencyStop`/`Fault` ngoài qua `safety_guard`. Nếu một actor phát hiện điều kiện nguy hiểm, nó return `TickResult::SafetyViolation` và để `safety_guard` quyết định phase tiếp theo.
- Thuật toán trong `core/adaptive/` (Kalman filter, gain learner) không được gọi trực tiếp I/O hay side-effect — nhận input là số liệu thuần (`&[f64]`/struct), trả kết quả thuần, để test được bằng dữ liệu giả lập mà không cần mock hardware.
- Timeout dùng trong FSM (VD: dosing timeout, mixing timeout) phải là hằng số đặt tên rõ ràng ở đầu file liên quan (không magic number rải rác) — vì `tests/orchestrator_timeout_tests.rs` test trực tiếp các hằng số này.

## Test checklist

- [ ] Test chuyển pha: pha A + event X → đúng pha B kỳ vọng (`tests/phase_transition_tests.rs`)
- [ ] Test timeout: pha treo quá hạn → đúng hành vi (retry/fault/abort) (`tests/orchestrator_timeout_tests.rs`)
- [ ] Test actor: input giả lập → đúng `TickResult` kỳ vọng, kể cả nhánh lỗi (`tests/dosing_actor_tests.rs` hoặc file tương ứng)
- [ ] Nếu thêm hành vi end-to-end (nhiều pha nối tiếp): thêm kịch bản trong `tests/e2e/`
- [ ] Snapshot test (`insta`) cho output phức tạp: review diff bằng mắt, không auto-accept

## Chạy test cục bộ (không cần esp-rs toolchain — đây là crate host-native)

```bash
cd hydragrow-controller-core
cargo test
cargo test --test e2e
```
