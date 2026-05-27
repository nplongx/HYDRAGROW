# FSM Tracing Logging Design

## Goal

Replace ad hoc FSM status logging in the ESP32 controller with `tracing` while preserving the existing MQTT `system_log` schema consumed by the backend and frontend.

## Scope

- Add `tracing` to `ESP32-C3-CONTROLLER-NODE`.
- Route FSM status and system-log records through a small tracing-based bridge.
- Keep `UnifiedSystemLog` JSON payloads, MQTT topics, and backend/frontend contracts unchanged.
- Keep unrelated frontend and generated target changes untouched.

## Architecture

FSM code emits structured `tracing` events instead of calling the manual log helper directly. Events that should reach MQTT are converted by a narrow bridge into `UnifiedSystemLog` and sent through the existing `Sender<String>`. Console/runtime logging continues to work through the existing ESP logger path using `tracing-log` compatibility when possible.

The bridge owns only the conversion from typed log metadata to the existing wire format. It does not change business state, phase transition rules, pump dispatching, or MQTT topic routing.

## Components

- `fsm::tracing_log`: helper functions/macros for FSM system logs and phase status events.
- `fsm::utils`: keeps time and drop-count helpers; manual `send_system_log` is removed or reduced to compatibility only if existing callers still require it during migration.
- `fsm::observers::system_log`: emits structured tracing records and uses the bridge for MQTT publishing.
- `Cargo.toml`: adds `tracing` and the minimal compatibility crate needed for ESP logging.

## Data Flow

1. FSM detects a state/status event.
2. FSM emits a structured `tracing` event with fields such as device ID, level, category, title, source, message, and optional cycle ID.
3. The bridge builds `UnifiedSystemLog`.
4. The JSON is sent on the existing MQTT channel.
5. Drop count continues to increment when the channel is full.

## Error Handling

- JSON serialization failures are logged with `tracing::error!`.
- MQTT channel send failures increment the existing drop counter and emit throttled `tracing::warn!`.
- Existing hardware dispatch errors move from `log::warn!` toward `tracing::warn!` only where touched by the FSM logging migration.

## Testing

- Add focused unit tests for the bridge:
  - builds the same `UnifiedSystemLog` JSON fields as the current helper;
  - increments drop count when the receiving side is closed;
  - preserves optional `cycle_id`.
- Run targeted controller tests/checks first, then a broader Rust check where feasible.

