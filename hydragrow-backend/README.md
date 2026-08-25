# hydragrow-backend

REST API + MQTT handler cho HYDRAGROW.

**Tech:** Rust, Actix-web, rumqttc, sqlx (PostgreSQL), influxdb2, Firebase Admin SDK, Solana SDK.

## Prerequisites

- Rust stable toolchain (`rustup toolchain install stable`)
- PostgreSQL 15+
- InfluxDB 2.x
- MQTT broker (e.g. EMQX, Mosquitto) accessible từ backend

## Environment Variables

Copy `.env.example` → `.env` (nếu chưa có, xem `src/main.rs` để biết list env vars cần thiết):

```bash
DATABASE_URL=postgres://user:password@localhost:5432/hydragrow
INFLUX_URL=http://localhost:8086
INFLUX_TOKEN=dev_only_token
INFLUX_ORG=hydragrow
INFLUX_BUCKET=sensors
MQTT_HOST=localhost
MQTT_PORT=8883
MQTT_USERNAME=...
MQTT_PASSWORD=...
FIREBASE_PROJECT_ID=...
API_KEY=...
```

## Build & Run

```bash
cd hydragrow-backend

# Chạy migrations
sqlx migrate run

# Dev
cargo run

# Release
cargo build --release
./target/release/hydragrow-backend
```

## Test

```bash
cargo test
```

## Lint

```bash
cargo clippy -- -D warnings
```

## Cấu trúc thư mục

```
src/
├── api/          # HTTP handlers (REST endpoints)
├── db/           # Database layer (PostgreSQL + InfluxDB)
├── models/       # Serde types cho request/response
├── mqtt/
│   └── handlers/ # MQTT message handlers (sensor, status, system_log, fsm, dosing)
├── services/     # Business logic (command, fcm, firebase_auth, solana, ph_calibration)
└── main.rs       # Khởi tạo AppState, routes, MQTT client
```
