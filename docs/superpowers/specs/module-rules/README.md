# HYDRAGROW — Module Contribution Rules (Index)

Mỗi subsystem có 1 file rule riêng. Khi thêm/sửa tính năng, đọc rule của **mọi** subsystem mà thay đổi của bạn chạm tới trước khi viết code.

| Subsystem | File rule | Khi nào đọc |
|---|---|---|
| Backend API (`hydragrow-backend/`) | [backend.md](./backend.md) | Sửa DB, API handler, MQTT handler, auth middleware |
| Shared types (`hydragrow-shared/`) | [shared.md](./shared.md) | Sửa struct dùng chung, MQTT topic, schema version |
| Controller core (`hydragrow-controller-core/`) | [controller-core.md](./controller-core.md) | Sửa FSM, dosing actor, safety guard, adaptive control |
| Frontend (`hydragrow-frontend/`) | [frontend.md](./frontend.md) | Sửa UI, store, WebSocket client, Tauri command |
| Controller firmware (`ESP32-C3-CONTROLLER-NODE/`) | [firmware-controller.md](./firmware-controller.md) | Sửa firmware Rust/esp-rs chạy trên MCU điều khiển |
| Sensor firmware (`ESP32-C3-SENSOR-NODE/`) | [firmware-sensor.md](./firmware-sensor.md) | Sửa firmware C++/PlatformIO đọc cảm biến |

---

## 📋 General Rules (áp dụng cho MỌI subsystem)

1. **Một trách nhiệm mỗi file.** File thay đổi vì 2 lý do không liên quan → tách thành 2 file.
2. **Không đặt logic dùng chung ở 2 nơi.** Nếu cả backend và firmware cần cùng 1 hằng số/type (topic MQTT, tên field payload), nó phải sống trong `hydragrow-shared`, không copy tay.
3. **Đổi hợp đồng liên-subsystem (MQTT topic, payload JSON, schema_version) phải cập nhật đồng thời:** `hydragrow-shared` (nguồn sự thật) + mọi subsystem tiêu thụ nó (backend handler, firmware publisher/subscriber, frontend type) + rule file tương ứng — trong cùng 1 PR. Xem thêm [shared.md](./shared.md#thay-đổi-tương-thích-ngược).
4. **Không `unwrap()`/`.expect()` trên đường code chạy production** (backend, controller-core, firmware). Chỉ cho phép trong `#[cfg(test)]`, script build, hoặc assertion lúc khởi động (`main.rs`/`app_main`) khi panic sớm là hành vi mong muốn.
5. **PR đổi rule ở file này hoặc bất kỳ file rule con nào phải nêu rõ trong mô tả PR: "Cập nhật module-rules: <lý do>".**

---

## 🔧 Kiểm tra chung trước mọi PR

| Subsystem | Bộ lệnh kiểm tra (chạy từ repo root) |
|---|---|
| Rust workspaces (chạy riêng từng thư mục — không có Cargo workspace gộp) | `(cd hydragrow-shared && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)`<br>`(cd hydragrow-backend && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)`<br>`(cd hydragrow-controller-core && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)` |
| Frontend | `(cd hydragrow-frontend && npx tsc --noEmit && npx eslint . && npx vitest run)` |
