# ESP32-C3-CONTROLLER-NODE

Firmware điều khiển bơm và quản lý FSM cho thiết bị HYDRAGROW.

**Platform:** ESP32-C3 | **Lang:** Rust | **Toolchain:** esp-rs nightly

## Prerequisites

```bash
# Cài espup (quản lý Rust toolchain cho Espressif)
cargo install espup
espup install --targets esp32c3

# Source env (thêm vào ~/.bashrc hoặc chạy mỗi lần)
source ~/export-esp.sh
```

## Build

```bash
cd ESP32-C3-CONTROLLER-NODE
cargo build
```

## Flash lên device

```bash
# Cài espflash
cargo install espflash

# Flash + monitor
cargo run
# hoặc
espflash flash --monitor target/riscv32imc-esp-espidf/debug/esp32-c3-controller-node
```

## Cấu hình thiết bị

Lần đầu khởi động: device phát WiFi AP `HydraGrow-Setup`. Kết nối và truy cập `192.168.4.1` để nhập WiFi credentials + MQTT config (lưu vào NVS).

## Cấu trúc

```
src/
├── core/
│   ├── actors/     # DosingActor, WaterActor, SafetyGuard
│   ├── adaptive/   # Kalman filter, MIMO solver, gain learner
│   └── fsm/        # Finite state machine phases (monitoring, dosing, mixing, stabilizing, cooldown)
├── hw/             # Hardware abstraction (WiFi, MQTT, NVS, OTA, I2C pump controller)
├── runtime/        # FSM loop, command handler, health reporter, observers
└── main.rs
```

## CI

Xem `.github/workflows/firmware-controller-ci.yml` — cargo check trên ESP32-C3 target.
