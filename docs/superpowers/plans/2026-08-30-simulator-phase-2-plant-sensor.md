# Simulator Phase 2 - Plant and Sensor Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the physical Plant Model (Tank) representing EC, pH, temperature, and water level, using configuration values as the source of truth, and implement the Sensor Model to add realistic noise.

**Architecture:** Create a `Tank` struct that updates its state based on `VirtualHardwareState` and `ControllerConfig`. Create a `test_support` module in `hydragrow-controller-core` to house the chemistry math so both the simulator and core E2E tests can share it. Update the harness to use the Tank model.

**Tech Stack:** Rust, `insta` for snapshot testing.

---

### Task 1: Refactor chemistry math into core's test_support

**Files:**
- Modify: `hydragrow-controller-core/Cargo.toml`
- Create: `hydragrow-controller-core/src/test_support.rs`
- Modify: `hydragrow-controller-core/src/lib.rs`

- [ ] **Step 1: Write the failing test in core**

```rust
// hydragrow-controller-core/src/test_support.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ControllerConfig;

    #[test]
    fn test_calculate_ec_change() {
        let config = ControllerConfig {
            ec_gain_per_ml: 0.1,
            ..Default::default()
        };
        let change = calculate_ec_change(5.0, 10.0, &config);
        // pump flow (5.0) * ec_gain_per_ml (0.1) / volume (10.0)
        assert_eq!(change, 0.05);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hydragrow-controller-core && cargo test test_calculate_ec_change`
Expected: FAIL due to missing types.

- [ ] **Step 3: Write minimal implementation**

```rust
// hydragrow-controller-core/src/test_support.rs
use crate::config::ControllerConfig;

/// Calculates the change in EC for a given pump flow and tank volume
/// First-order linear model as specified in Phase 2.
pub fn calculate_ec_change(pump_flow_ml: f32, volume_l: f32, config: &ControllerConfig) -> f32 {
    if volume_l <= 0.0 {
        return 0.0;
    }
    (pump_flow_ml * config.ec_gain_per_ml) / volume_l
}

/// Calculates the change in pH for a given pump flow, tank volume, and direction (up/down)
pub fn calculate_ph_change(pump_flow_ml: f32, volume_l: f32, is_up: bool, config: &ControllerConfig) -> f32 {
    if volume_l <= 0.0 {
        return 0.0;
    }
    let shift = if is_up { config.ph_shift_up_per_ml } else { -config.ph_shift_down_per_ml };
    (pump_flow_ml * shift) / volume_l
}
```
*Note: Add `test-support` feature to `Cargo.toml` if needed, or simply make it public under `#[cfg(feature = "test-support")]` and expose it. Ensure `src/lib.rs` exports `pub mod test_support;` when feature is enabled.*

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hydragrow-controller-core && cargo test --features test-support` (or simply `cargo test` if it's test-only).
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-controller-core/
git commit -m "feat(core): extract chemistry formulas into test_support for simulator and E2E"
```

### Task 2: Implement the Plant (Tank) Model

**Files:**
- Create: `hydragrow-simulator/src/plant/tank.rs`
- Create: `hydragrow-simulator/src/plant/mod.rs`
- Modify: `hydragrow-simulator/src/lib.rs`
- Modify: `hydragrow-simulator/Cargo.toml` (Add path dependency to feature `test-support` of core)

- [ ] **Step 1: Write the failing test**

```rust
// hydragrow-simulator/src/plant/tank.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actuators::virtual_hw::{VirtualHardwareState, VirtualPump};
    use hydragrow_controller_core::config::ControllerConfig;

    #[test]
    fn test_tank_step_dosing_ec() {
        let mut tank = Tank { volume_l: 10.0, ec: 1.0, ph: 6.0, temp: 25.0, water_level: 50.0 };
        let mut config = ControllerConfig::default();
        config.ec_gain_per_ml = 0.5;
        config.pump_a_capacity_ml_per_sec = 2.0;

        let mut hw = VirtualHardwareState::default();
        hw.pump_a = VirtualPump { on: true, pwm: 100 };

        // 1 second step
        tank.step(1000, &hw, &config);

        // Flow = 2.0 ml. EC gain = 2.0 * 0.5 / 10.0 = 0.1
        assert_eq!(tank.ec, 1.1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hydragrow-simulator && cargo test --lib plant`
Expected: FAIL due to missing types.

- [ ] **Step 3: Write minimal implementation**

```rust
// hydragrow-simulator/src/plant/tank.rs
use crate::actuators::virtual_hw::VirtualHardwareState;
use hydragrow_controller_core::config::ControllerConfig;
#[cfg(feature = "test-support")]
use hydragrow_controller_core::test_support::{calculate_ec_change, calculate_ph_change};

#[derive(Debug, Clone)]
pub struct Tank {
    pub volume_l: f32,
    pub ec: f32,
    pub ph: f32,
    pub temp: f32,
    pub water_level: f32,
}

impl Tank {
    pub fn step(&mut self, dt_ms: u64, actuators: &VirtualHardwareState, config: &ControllerConfig) {
        let dt_sec = dt_ms as f32 / 1000.0;

        let mut ec_change = 0.0;
        let mut ph_change = 0.0;

        // Calculate flow for each pump based on capacity and PWM (assuming linear for simplicity)
        if actuators.pump_a.on {
            let flow = config.pump_a_capacity_ml_per_sec * dt_sec * (actuators.pump_a.pwm as f32 / 100.0);
            ec_change += calculate_ec_change(flow, self.volume_l, config);
        }
        if actuators.pump_b.on {
            let flow = config.pump_b_capacity_ml_per_sec * dt_sec * (actuators.pump_b.pwm as f32 / 100.0);
            ec_change += calculate_ec_change(flow, self.volume_l, config);
        }
        if actuators.pump_ph_up.on {
            let flow = config.pump_ph_up_capacity_ml_per_sec * dt_sec * (actuators.pump_ph_up.pwm as f32 / 100.0);
            ph_change += calculate_ph_change(flow, self.volume_l, true, config);
        }
        if actuators.pump_ph_down.on {
            let flow = config.pump_ph_down_capacity_ml_per_sec * dt_sec * (actuators.pump_ph_down.pwm as f32 / 100.0);
            ph_change += calculate_ph_change(flow, self.volume_l, false, config);
        }

        // Incorporate mixing delay via a simple low-pass filter if necessary (skipped here for pure math correctness, add alpha later)
        self.ec += ec_change;
        self.ph += ph_change;

        // Water level adjustments
        if actuators.water_pump_in.on {
            self.water_level += dt_sec; // simplified
        }
        if actuators.water_pump_out.on {
            self.water_level -= dt_sec; // simplified
        }
    }
}
```
*Make sure `Cargo.toml` has `hydragrow-controller-core = { path = "../hydragrow-controller-core", features = ["test-support"] }`*

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hydragrow-simulator && cargo test --lib plant`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-simulator/src/lib.rs hydragrow-simulator/src/plant/ hydragrow-simulator/Cargo.toml
git commit -m "feat(simulator): implement linear Plant model reading chemistry constants from config"
```

### Task 3: Implement the Sensor Model

**Files:**
- Create: `hydragrow-simulator/src/sensors/sensor_model.rs`
- Create: `hydragrow-simulator/src/sensors/mod.rs`
- Modify: `hydragrow-simulator/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// hydragrow-simulator/src/sensors/sensor_model.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::plant::tank::Tank;

    #[test]
    fn test_sensor_read_no_noise() {
        let tank = Tank { volume_l: 10.0, ec: 1.5, ph: 6.2, temp: 24.5, water_level: 40.0 };
        let cfg = NoiseConfig::none();
        let sensor = read_sensor(&tank, &cfg);
        assert_eq!(sensor.ec, 1.5);
        assert_eq!(sensor.ph, 6.2);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hydragrow-simulator && cargo test --lib sensors`
Expected: FAIL due to missing types.

- [ ] **Step 3: Write minimal implementation**

```rust
// hydragrow-simulator/src/sensors/sensor_model.rs
use crate::plant::tank::Tank;
use hydragrow_shared::SensorData;

#[derive(Debug, Clone)]
pub struct NoiseConfig {
    pub ec_noise_std_dev: f32,
    pub ph_noise_std_dev: f32,
}

impl NoiseConfig {
    pub fn none() -> Self {
        Self { ec_noise_std_dev: 0.0, ph_noise_std_dev: 0.0 }
    }
}

pub fn read_sensor(tank: &Tank, config: &NoiseConfig) -> SensorData {
    // For now, no actual random generation to keep it simple, just add 0.0 if none.
    // Future: Use rand crate and normal distribution for noise.
    let ec_noise = 0.0;
    let ph_noise = 0.0;

    SensorData {
        ec: tank.ec + ec_noise,
        ph: tank.ph + ph_noise,
        water_temp: tank.temp,
        room_temp: 25.0, // static for now
        humidity: 50.0,
        water_level: tank.water_level,
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hydragrow-simulator && cargo test --lib sensors`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-simulator/src/lib.rs hydragrow-simulator/src/sensors/
git commit -m "feat(simulator): implement basic sensor model with noise config"
```

### Task 4: Integrate Tank into Harness and Snapshot Test

**Files:**
- Modify: `hydragrow-simulator/src/harness.rs`
- Create: `hydragrow-simulator/tests/snapshot_dosing.rs`

- [ ] **Step 1: Update Harness**

Modify `Harness` to own a `Tank` and step it before calling the orchestrator.

```rust
// In hydragrow-simulator/src/harness.rs
use crate::plant::tank::Tank;
use crate::sensors::sensor_model::{read_sensor, NoiseConfig};

pub struct Harness {
    pub config: ControllerConfig,
    pub ctx: FsmContext,
    pub hw: VirtualHardwareState,
    pub dispatcher: SimDispatcher,
    pub tank: Tank,
    pub noise: NoiseConfig,
    uptime_ms: u64,
}

impl Harness {
    pub fn new(config: ControllerConfig, tank: Tank, noise: NoiseConfig) -> Self {
        // ... update initialization
    }

    pub fn tick(&mut self, dt_ms: u64) -> TickResult {
        // Step the plant first
        self.tank.step(dt_ms, &self.hw, &self.config);
        let sensor = read_sensor(&self.tank, &self.noise);

        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
        self.uptime_ms += dt_ms;

        let result = orchestrator::tick(now_ms, self.uptime_ms, &self.config, sensor, &[], &mut self.ctx);
        for event in &result.events { self.dispatcher.dispatch(event, &mut self.hw); }

        result
    }
}
```

- [ ] **Step 2: Write Insta Snapshot Test**

```rust
// hydragrow-simulator/tests/snapshot_dosing.rs
use hydragrow_simulator::harness::Harness;
use hydragrow_simulator::plant::tank::Tank;
use hydragrow_simulator::sensors::sensor_model::NoiseConfig;
use hydragrow_controller_core::config::ControllerConfig;
// Write a test that ticks N times and logs the EC, then verify with `insta::assert_snapshot!`.
```

- [ ] **Step 3: Run Insta to generate and accept snapshot**

Run: `cd hydragrow-simulator && cargo test --test snapshot_dosing`
Then: `cargo insta review` (or update).

- [ ] **Step 4: Commit**

```bash
git add hydragrow-simulator/src/harness.rs hydragrow-simulator/tests/
git commit -m "test(simulator): integrate tank into harness and add snapshot test"
```
