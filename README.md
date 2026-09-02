# HYDRAGROW

Hệ thống điều khiển và giám sát thủy canh thông minh.

## Kiến trúc tổng quan

```
[ESP32-C3-SENSOR-NODE]          [ESP32-C3-CONTROLLER-NODE]
 C++/PlatformIO/Arduino            Rust/esp-rs (nightly)
 Đọc cảm biến EC/pH/Temp           - FSM điều khiển bơm dosing
 Gửi MQTT telemetry                - Kalman filter + MIMO adaptive
        │                                    │
        └──────────────┬─────────────────────┘
                       │ MQTT (TLS)
                       ▼
              [hydragrow-backend]
               Rust/Actix-web
               - REST API (auth Firebase + API key)
               - MQTT broker client (rumqttc)
               - InfluxDB (sensor timeseries)
               - PostgreSQL (config, users, seasons)
               - WebSocket fan-out → frontend
               - Solana wallet integration
                       │
        ┌──────────────┴──────────────┐
        │ HTTP/WebSocket              │
        ▼                             ▼
[hydragrow-frontend]          [hydragrow-shared]
 React/TS/Tauri                Rust (crate dùng chung)
 Vite + Zustand                - Types: SensorData, FsmSnapshot
 Firebase Auth                 - MQTT topics constants
                               - Schema roundtrip tests
```

## Subsystems

| Subsystem | Ngôn ngữ | Thư mục | README |
|---|---|---|---|
| Backend API | Rust / Actix-web | `hydragrow-backend/` | [README](hydragrow-backend/README.md) |
| Frontend | React / TypeScript / Tauri | `hydragrow-frontend/` | [README](hydragrow-frontend/README.md) |
| Shared types | Rust crate | `hydragrow-shared/` | [README](hydragrow-shared/README.md) |
| Controller firmware | Rust / esp-rs | `ESP32-C3-CONTROLLER-NODE/` | [README](ESP32-C3-CONTROLLER-NODE/README.md) |
| Sensor firmware | C++ / PlatformIO | `ESP32-C3-SENSOR-NODE/` | [README](ESP32-C3-SENSOR-NODE/README.md) |
| Simulator | Rust | `hydragrow-simulator/` | N/A |

## MQTT Topics chính

| Topic pattern | Publisher | Subscriber | Payload type |
|---|---|---|---|
| `AGITECH/{device_id}/sensor/data` | SENSOR-NODE | Backend | `SensorData` |
| `AGITECH/{device_id}/controller/status` | CONTROLLER-NODE | Backend | `DeviceHealthSnapshot` |
| `AGITECH/{device_id}/system/log` | CONTROLLER-NODE | Backend | `UnifiedSystemLog` |
| `AGITECH/{device_id}/fsm/transition` | CONTROLLER-NODE | Backend | `FsmTransitionEvent` |
| `AGITECH/{device_id}/command/#` | Backend | CONTROLLER-NODE | JSON commands |

## Databases

| DB | Dùng bởi | Lưu gì |
|---|---|---|
| InfluxDB | Backend | Timeseries sensor (EC, pH, Temp, WaterLevel) |
| PostgreSQL | Backend | Users, device config, crop seasons, system events |

## Quick Start

Xem README của từng subsystem để build/run chi tiết. Thứ tự khởi động:

1. PostgreSQL + InfluxDB (Docker Compose — xem `hydragrow-backend/README.md`)
2. `hydragrow-backend` (cargo run)
3. `hydragrow-frontend` (npm run dev:web)
4. Flash firmware lên phần cứng (xem README của từng firmware)

## CI

| Workflow | Trigger | Kiểm tra |
|---|---|---|
| `shared-schema-check` | push/PR chạm `hydragrow-shared/` | cargo fmt + clippy + test (schema snapshots) |
| `backend-ci` | push/PR chạm `hydragrow-backend/` hoặc `hydragrow-shared/` | cargo fmt + check + test + clippy (-D warnings) |
| `simulator-ci` | push/PR chạm `hydragrow-simulator/`, `hydragrow-controller-core/` hoặc `hydragrow-shared/` | cargo fmt + check + clippy + test |
| `controller-core-ci` | push/PR chạm `hydragrow-controller-core/` hoặc `hydragrow-shared/` | cargo fmt + clippy + test |
| `frontend-ci` | push/PR chạm `hydragrow-frontend/` | tsc + eslint + vitest + cargo check (src-tauri) |
| `firmware-controller-ci` | push/PR chạm `ESP32-C3-CONTROLLER-NODE/` | cargo check + fmt + clippy (esp-rs nightly) |
| `firmware-sensor-ci` | push/PR chạm `ESP32-C3-SENSOR-NODE/` | pio run + pio test (native) |
| `code-quality` | mọi PR; push `main` (trừ docs-only) | Rust fmt + locked check + clippy `-D warnings` + tests; frontend TypeScript + ESLint `--max-warnings=0` + tests + production build |

`code-quality` là quality gate bổ sung ở cấp repository: PR không đạt các tiêu chí chất lượng code sẽ fail CI. Các workflow subsystem hiện hữu vẫn giữ vai trò kiểm tra chuyên biệt.

Xem [CONTRIBUTING.md](CONTRIBUTING.md) cho quy trình PR và [docs/superpowers/specs/module-rules/](docs/superpowers/specs/module-rules/README.md) cho ràng buộc từng subsystem.

## Không đụng vào

- `server_wallet.json` — Solana wallet key, không commit, không sửa trong quá trình dọn dẹp code
- `hydragrow-backend/migrations/` — schema DB production, thay đổi cần migration plan riêng
