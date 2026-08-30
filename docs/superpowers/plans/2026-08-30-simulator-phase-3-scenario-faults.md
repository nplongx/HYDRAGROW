# Simulator Phase 3 - Scenario Engine and Recorder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform the simulator into a "test laboratory" by introducing scenario definitions (JSON), fault injection to simulate physical failures mapped to `FaultCode`, and a telemetry recorder to export simulation runs to CSV.

**Architecture:** Create a `Scenario` struct (Serde deserializable) that drives fault injection. The `Injector` intercepts hardware commands or alters sensor readings based on scheduled faults. Create a `Recorder` to dump the telemetry each tick.

**Tech Stack:** Rust, `serde`, `serde_json`, `csv` (if needed, or simple string formatting).

---

### Task 1: Scenario Format and Library

**Files:**
- Create: `hydragrow-simulator/src/scenario/format.rs`
- Create: `hydragrow-simulator/src/scenario/mod.rs`
- Create: `hydragrow-simulator/src/scenario/library/ec_stagnant.json`
- Modify: `hydragrow-simulator/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// hydragrow-simulator/src/scenario/format.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_scenario() {
        let json = r#"{
            "initial_tank": { "volume_l": 10.0, "ec": 1.0, "ph": 6.0, "temp": 25.0, "water_level": 50.0 },
            "faults": [
                { "at_ms": 5000, "kind": "PumpStuckOn", "pump": "PUMP_A" }
            ]
        }"#;
        let scenario: Scenario = serde_json::from_str(json).unwrap();
        assert_eq!(scenario.faults.len(), 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hydragrow-simulator && cargo test --lib scenario`
Expected: FAIL.

- [ ] **Step 3: Write minimal implementation**

```rust
// hydragrow-simulator/src/scenario/format.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialTank {
    pub volume_l: f32,
    pub ec: f32,
    pub ph: f32,
    pub temp: f32,
    pub water_level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FaultEventKind {
    PumpStuckOn { pump: String },
    PumpStuckOff { pump: String },
    SensorFrozen { sensor: String },
    // more as needed mapping to fsm faults
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultEvent {
    pub at_ms: u64,
    #[serde(flatten)]
    pub kind: FaultEventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub initial_tank: InitialTank,
    pub faults: Vec<FaultEvent>,
}
```
*Note: Also create the library JSON file matching the spec vocabulary.*

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hydragrow-simulator && cargo test --lib scenario`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-simulator/src/lib.rs hydragrow-simulator/src/scenario/
git commit -m "feat(simulator): define scenario and fault formats"
```

### Task 2: Fault Injector

**Files:**
- Create: `hydragrow-simulator/src/faults/injector.rs`
- Create: `hydragrow-simulator/src/faults/mod.rs`
- Modify: `hydragrow-simulator/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// hydragrow-simulator/src/faults/injector.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::actuators::virtual_hw::VirtualHardwareState;

    #[test]
    fn test_injector_pump_stuck() {
        let mut hw = VirtualHardwareState::default();
        let mut injector = Injector::new();
        injector.add_active_fault(FaultEventKind::PumpStuckOn { pump: "PUMP_A".to_string() });

        injector.apply_hardware_faults(&mut hw);
        assert_eq!(hw.pump_a.on, true); // Forced on despite default false
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hydragrow-simulator && cargo test --lib faults`
Expected: FAIL.

- [ ] **Step 3: Write minimal implementation**

```rust
// hydragrow-simulator/src/faults/injector.rs
use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::scenario::format::FaultEventKind;

pub struct Injector {
    pub active_faults: Vec<FaultEventKind>,
}

impl Injector {
    pub fn new() -> Self {
        Self { active_faults: vec![] }
    }

    pub fn add_active_fault(&mut self, fault: FaultEventKind) {
        self.active_faults.push(fault);
    }

    pub fn apply_hardware_faults(&self, hw: &mut VirtualHardwareState) {
        for fault in &self.active_faults {
            match fault {
                FaultEventKind::PumpStuckOn { pump } => {
                    if pump == "PUMP_A" { hw.pump_a.on = true; }
                }
                FaultEventKind::PumpStuckOff { pump } => {
                    if pump == "PUMP_A" { hw.pump_a.on = false; }
                }
                _ => {} // Sensor faults handled separately
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hydragrow-simulator && cargo test --lib faults`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-simulator/src/lib.rs hydragrow-simulator/src/faults/
git commit -m "feat(simulator): implement fault injector for hardware overrides"
```

### Task 3: Telemetry Recorder

**Files:**
- Create: `hydragrow-simulator/src/telemetry/recorder.rs`
- Create: `hydragrow-simulator/src/telemetry/mod.rs`
- Modify: `hydragrow-simulator/src/lib.rs`

- [ ] **Step 1: Write the failing test**

```rust
// hydragrow-simulator/src/telemetry/recorder.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_record_csv_line() {
        let mut recorder = Recorder::new("test_out.csv");
        recorder.record(100, "Idle", 1.2, 6.0, 25.0, 50.0, true, false);
        drop(recorder);
        let content = fs::read_to_string("test_out.csv").unwrap();
        assert!(content.contains("100,Idle,1.2,6,25,50,true,false"));
        fs::remove_file("test_out.csv").unwrap();
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd hydragrow-simulator && cargo test --lib telemetry`
Expected: FAIL.

- [ ] **Step 3: Write minimal implementation**

```rust
// hydragrow-simulator/src/telemetry/recorder.rs
use std::fs::File;
use std::io::Write;

pub struct Recorder {
    file: File,
}

impl Recorder {
    pub fn new(path: &str) -> Self {
        let mut file = File::create(path).unwrap();
        writeln!(file, "time,phase,ec,ph,temp,level,pump_a,pump_b").unwrap();
        Self { file }
    }

    pub fn record(&mut self, time: u64, phase: &str, ec: f32, ph: f32, temp: f32, level: f32, pa: bool, pb: bool) {
        writeln!(self.file, "{},{},{},{},{},{},{},{}", time, phase, ec, ph, temp, level, pa, pb).unwrap();
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd hydragrow-simulator && cargo test --lib telemetry`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-simulator/src/lib.rs hydragrow-simulator/src/telemetry/
git commit -m "feat(simulator): implement telemetry recorder to CSV"
```

### Task 4: Integrate Scenario Engine into Harness

**Files:**
- Modify: `hydragrow-simulator/src/harness.rs`
- Create: `hydragrow-simulator/tests/scenario_ec_stagnant.rs`

- [ ] **Step 1: Integrate and Write test asserting FaultCode**

```rust
// Hydrate initial state from scenario, load faults, inject them on the fly based on uptime_ms, and assert that after X ticks the context phase is `SystemPhase::Fault(FaultCode::EcStagnant)`.
```

- [ ] **Step 2: Run test**

Run: `cd hydragrow-simulator && cargo test --test scenario_ec_stagnant`

- [ ] **Step 3: Commit**

```bash
git add hydragrow-simulator/src/harness.rs hydragrow-simulator/tests/
git commit -m "test(simulator): integrate scenario and injector to test EcStagnant fault"
```
