# HydraGrow

HydraGrow is a comprehensive, smart hydroponic system combining IoT firmware, a robust backend, and a modern frontend interface. It is composed of 5 distinct subsystems designed to monitor and automatically dose hydroponic nutrients while offering complete control and system visibility.

## 🏗 System Architecture

The project consists of 5 main subsystems:

1. **`ESP32-C3-CONTROLLER-NODE` (Firmware - Rust)**
   - **Role:** The main controller node. Reads sensor data, manages the Finite State Machine (FSM), controls pumps and valves based on dosing cycles.
   - **Communication:** Connects to the backend via **MQTT** (publishes telemetry/health, subscribes to control commands).

2. **`ESP32-C3-SENSOR-NODE` (Firmware - C++/PlatformIO)**
   - **Role:** A dedicated sensor node for continuously monitoring specific environmental parameters like pH, EC, Temp, and Water levels.
   - **Communication:** Sends telemetry data to the main controller or backend via **MQTT**.

3. **`hydragrow-backend` (Backend - Rust / Actix)**
   - **Role:** The central server. Handles HTTP APIs for the frontend, manages the WebSocket/MQTT connections, processes telemetry, and coordinates Firebase Authentication.
   - **Communication:** Uses **PostgreSQL** (via `sqlx`) for persistent data, **InfluxDB** for time-series metrics, and communicates with firmware via **MQTT**. Serves a REST/HTTP API to the frontend.

4. **`hydragrow-shared` (Library - Rust)**
   - **Role:** A shared library containing common schemas, payload definitions, and constants used by both the Rust backend and the Rust firmware controller. Ensures type safety and protocol consistency across boundaries.

5. **`hydragrow-frontend` (Frontend - React / TS / Tauri)**
   - **Role:** The user interface. Available as a web app or desktop application (via Tauri). Allows users to monitor dashboards, configure dosing recipes, review system logs, and pair new devices.
   - **Communication:** Communicates with `hydragrow-backend` via HTTP REST APIs.

## 🚀 Getting Started

### Prerequisites
- [Rust Toolchain](https://rustup.rs/)
- [Node.js](https://nodejs.org/) & npm
- [PlatformIO](https://platformio.org/)
- PostgreSQL & InfluxDB instances (for the backend)
- MQTT Broker (e.g., Mosquitto)

### 1. hydragrow-frontend
Read the detailed [Frontend README](./hydragrow-frontend/README.md).
```sh
cd hydragrow-frontend
npm install
npm run dev:web  # Web mode
# or
npm run dev:tauri # Desktop mode
```

### 2. hydragrow-backend
Ensure `DATABASE_URL` and `INFLUX_URL` are configured in your `.env`.
```sh
cd hydragrow-backend
cargo run
```

### 3. ESP32-C3-CONTROLLER-NODE
Requires the `esp-rs` toolchain.
```sh
cd ESP32-C3-CONTROLLER-NODE
cargo build --release
# Flash to device:
cargo run --release
```

### 4. ESP32-C3-SENSOR-NODE
Configure `src/secrets.h` from `src/secrets.h.example`.
```sh
cd ESP32-C3-SENSOR-NODE
pio run -t upload
```

## 🛡 Quality Control
This project enforces quality through:
- **Frontend:** ESLint, TypeScript compilation, and Vitest.
- **Backend & Firmware:** `cargo clippy`, `cargo check`, and `cargo test`.
- **CI/CD:** Automated GitHub Actions workflows ensure code consistency across all 5 subsystems before merging.
