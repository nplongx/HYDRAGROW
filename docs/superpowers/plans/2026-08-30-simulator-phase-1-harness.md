# Simulator Phase 1 - Dispatcher and Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the event dispatcher and run loop harness for the FSM to simulate the controller without hardware, mapping output events to virtual hardware state.

**Architecture:** We will create virtual pump and valve structs (virtual actuators) and a dispatcher that matches `OrchestratorEvent`s to update these virtual states. We will then build a basic run loop (`harness.rs`) that steps the FSM tick by tick.

**Tech Stack:** Rust, `hydragrow-controller-core`.

---

### Task 1: Create Virtual Hardware Actuators

**Files:**
- Create: `hydragrow-simulator/src/actuators/virtual_hw.rs`
- Create: `hydragrow-simulator/src/actuators/mod.rs`
- Modify: `hydragrow-simulator/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// hydragrow-simulator/src/actuators/virtual_hw.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_pump_initial_state() {
        let pump = VirtualPump::new();
        assert_eq!(pump.on, false);
        assert_eq!(pump.pwm, 0);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hydragrow-simulator && cargo test --lib actuators`
Expected: FAIL due to missing types.

- [ ] **Step 3: Write minimal implementation**

```rust
// hydragrow-simulator/src/actuators/virtual_hw.rs
#[derive(Debug, Clone, Default)]
pub struct VirtualPump {
    pub on: bool,
    pub pwm: u8,
}

impl VirtualPump {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct VirtualHardwareState {
    pub pump_a: VirtualPump,
    pub pump_b: VirtualPump,
    pub pump_ph_up: VirtualPump,
    pub pump_ph_down: VirtualPump,
    pub water_pump_in: VirtualPump,
    pub water_pump_out: VirtualPump,
    pub mist_valve: bool,
    pub osaka_pwm: u8,
}
```
*Also export `actuators` in `src/lib.rs` and `mod.rs`.*

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hydragrow-simulator && cargo test --lib actuators`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-simulator/src/lib.rs hydragrow-simulator/src/actuators/
git commit -m "feat(simulator): implement virtual hardware actuators for Phase 1"
```

### Task 2: Create the Event Dispatcher

**Files:**
- Create: `hydragrow-simulator/src/dispatcher.rs`
- Modify: `hydragrow-simulator/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// hydragrow-simulator/src/dispatcher.rs
#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_controller_core::events::OrchestratorEvent;

    #[test]
    fn test_dispatcher_pump_update() {
        let mut hw = VirtualHardwareState::default();
        let mut dispatcher = SimDispatcher::new();
        dispatcher.dispatch(&OrchestratorEvent::PumpA(true, 50), &mut hw);
        assert_eq!(hw.pump_a.on, true);
        assert_eq!(hw.pump_a.pwm, 50);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hydragrow-simulator && cargo test --lib dispatcher`
Expected: FAIL due to missing types.

- [ ] **Step 3: Write minimal implementation**

```rust
// hydragrow-simulator/src/dispatcher.rs
use crate::actuators::virtual_hw::VirtualHardwareState;
use hydragrow_controller_core::events::OrchestratorEvent;

pub struct SimDispatcher;

impl SimDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch(&mut self, event: &OrchestratorEvent, hw: &mut VirtualHardwareState) {
        match event {
            OrchestratorEvent::PumpA(on, pwm) => {
                hw.pump_a.on = *on;
                hw.pump_a.pwm = *pwm;
            }
            OrchestratorEvent::PumpB(on, pwm) => {
                hw.pump_b.on = *on;
                hw.pump_b.pwm = *pwm;
            }
            OrchestratorEvent::PumpPhUp(on, pwm) => {
                hw.pump_ph_up.on = *on;
                hw.pump_ph_up.pwm = *pwm;
            }
            OrchestratorEvent::PumpPhDown(on, pwm) => {
                hw.pump_ph_down.on = *on;
                hw.pump_ph_down.pwm = *pwm;
            }
            OrchestratorEvent::WaterPumpIn(on) => {
                hw.water_pump_in.on = *on;
                hw.water_pump_in.pwm = if *on { 100 } else { 0 };
            }
            OrchestratorEvent::WaterPumpOut(on) => {
                hw.water_pump_out.on = *on;
                hw.water_pump_out.pwm = if *on { 100 } else { 0 };
            }
            OrchestratorEvent::MistValve(on) => {
                hw.mist_valve = *on;
            }
            OrchestratorEvent::OsakaPwm(pwm) => {
                hw.osaka_pwm = *pwm;
            }
            _ => {
                // Publish* and other events will be handled in Phase 4 (MQTT bridge)
                // For now, log them if tracing is active
            }
        }
    }
}
```
*Also export `dispatcher` in `src/lib.rs`.*

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hydragrow-simulator && cargo test --lib dispatcher`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-simulator/src/lib.rs hydragrow-simulator/src/dispatcher.rs
git commit -m "feat(simulator): implement dispatcher to map OrchestratorEvents to VirtualHardwareState"
```

### Task 3: Create the Run Loop Harness

**Files:**
- Create: `hydragrow-simulator/src/harness.rs`
- Modify: `hydragrow-simulator/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// hydragrow-simulator/src/harness.rs
#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_controller_core::config::ControllerConfig;
    use hydragrow_shared::SensorData;
    use std::time::SystemTime;

    #[test]
    fn test_harness_single_tick() {
        let mut config = ControllerConfig::default();
        let sensor = SensorData::default();
        let mut harness = Harness::new(config);

        let delta_ms = 100;
        harness.tick(delta_ms, sensor);
        assert_eq!(harness.uptime_ms(), 100);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hydragrow-simulator && cargo test --lib harness`
Expected: FAIL due to missing types.

- [ ] **Step 3: Write minimal implementation**

```rust
// hydragrow-simulator/src/harness.rs
use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::dispatcher::SimDispatcher;
use hydragrow_controller_core::{
    config::ControllerConfig,
    fsm::core::{orchestrator, FsmContext},
    events::TickResult,
};
use hydragrow_shared::{SensorData, FsmSnapshot};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Harness {
    pub config: ControllerConfig,
    pub ctx: FsmContext,
    pub hw: VirtualHardwareState,
    pub dispatcher: SimDispatcher,
    uptime_ms: u64,
}

impl Harness {
    pub fn new(config: ControllerConfig) -> Self {
        Self {
            config,
            ctx: FsmContext::default(),
            hw: VirtualHardwareState::default(),
            dispatcher: SimDispatcher::new(),
            uptime_ms: 0,
        }
    }

    pub fn uptime_ms(&self) -> u64 {
        self.uptime_ms
    }

    pub fn tick(&mut self, dt_ms: u64, sensor: SensorData) -> TickResult {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.uptime_ms += dt_ms;

        // Pass external commands as empty for now in basic harness step
        let result = orchestrator::tick(
            now_ms,
            self.uptime_ms,
            &self.config,
            sensor,
            &[],
            &mut self.ctx,
        );

        for event in &result.events {
            self.dispatcher.dispatch(event, &mut self.hw);
        }

        result
    }
}
```
*Also export `harness` in `src/lib.rs`.*

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hydragrow-simulator && cargo test --lib harness`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-simulator/src/lib.rs hydragrow-simulator/src/harness.rs
git commit -m "feat(simulator): implement main simulation harness for Phase 1"
```
