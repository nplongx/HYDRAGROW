# ESP32-C3-CONTROLLER-NODE

The central firmware component of the HydraGrow system written in Rust. It manages the Finite State Machine (FSM), reads sensors, and executes dosing logic.

## Build Instructions
This project requires the `esp-rs` toolchain.

```bash
# Build
cargo build --release

# Flash
cargo run --release
```
