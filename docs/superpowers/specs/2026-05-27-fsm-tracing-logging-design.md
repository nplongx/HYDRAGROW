# FSM Tracing Logging Design

## Goal

Replace ad hoc FSM status logging in the ESP32 controller with `tracing` while preserving the existing MQTT `system_log` schema consumed by the backend and frontend.

## Scope

- Add `tracing-subscriber` Layer support to `hydragrow-shared`.
- Route FSM status and system-log records through structured tracing events consumed by a custom Layer.
- Keep `UnifiedSystemLog` JSON payloads, MQTT topics, and backend/frontend contracts unchanged.
- Keep unrelated frontend and generated target changes untouched.

## Architecture

FSM code emits structured `tracing` events instead of calling the manual log helper directly. Events that should reach MQTT are consumed by a `SystemLogLayer` implementing `tracing_subscriber::Layer`. The Layer uses a `Visit` implementation to extract event fields, converts them into `UnifiedSystemLog`, and sends JSON through the existing `Sender<String>`.

The Layer owns only the conversion from tracing fields to the existing wire format. It does not change business state, phase transition rules, pump dispatching, or MQTT topic routing.

## Components

- `hydragrow_shared::log::SystemLogLayer`: custom `tracing_subscriber::Layer` that listens to `hydragrow.system_log` events.
- `hydragrow_shared::log::SystemLogVisitor`: `Visit` implementation that captures string, debug, and numeric fields from tracing events.
- `hydragrow_shared::log::emit_basic_system_log`: helper that emits structured tracing events with stable field names.
- `fsm::utils`: keeps time and drop-count helpers; `send_system_log` becomes a compatibility wrapper that emits tracing events.
- `fsm::observers::system_log`: emits structured tracing records instead of serializing JSON directly.
- `Cargo.toml`: adds `tracing`, `tracing-subscriber`, and any minimal features needed by the Layer.

## Data Flow

1. FSM detects a state/status event.
2. FSM emits a structured `tracing` event with fields such as device ID, level, category, title, source, message, optional cycle ID, and timestamp.
3. `SystemLogLayer::on_event` receives only matching `hydragrow.system_log` events.
4. `SystemLogVisitor` extracts fields using `Visit`.
5. The Layer builds `UnifiedSystemLog`.
6. The JSON is sent on the existing MQTT channel.
7. Drop count continues to increment when the channel is full.

## Error Handling

- JSON serialization failures are logged with `tracing::error!` from the Layer.
- MQTT channel send failures increment the existing drop counter and emit throttled `tracing::warn!`.
- Incomplete or malformed system-log events are ignored by the Layer and reported with debug-level tracing to avoid malformed MQTT payloads.
- Existing hardware dispatch errors move from `log::warn!` toward `tracing::warn!` only where touched by the FSM logging migration.

## Testing

- Add focused unit tests for the Layer:
  - `Visit` captures string and debug fields used by the FSM;
  - `SystemLogLayer` publishes the same `UnifiedSystemLog` JSON fields as the current helper;
  - channel failure increments drop count;
  - events with unrelated targets are ignored;
  - optional `cycle_id` is preserved.
- Run targeted controller tests/checks first, then a broader Rust check where feasible.
