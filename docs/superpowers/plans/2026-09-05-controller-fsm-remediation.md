# Controller FSM Remaining Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close remaining safety, lifecycle, and consistency gaps across the Decision → Actor → Shadow → Hardware chain (findings R1 through R15), maintaining the current architecture and enforcing strict verification contracts.

**Architecture:** Target fixes directly at existing architectural boundaries:
- **Phase & Actor boundaries (`MimoDosingPhase`, `MonitoringPhase`, `WaterRefillingPhase`, `WaterDrainingPhase`, `DosingActor`, `WaterActor`):** Ensure timeout actions abort actors, enforce water direction mutual exclusivity, remove transient `Starting` ambiguity, eliminate duplicate pulse hardware events, and prevent premature actor mutation.
- **Context & Shadow boundaries (`SystemContext`, `PeripheralDelta`, `ContextDelta`):** Ensure canonical clearing of shadow `pump_status`, track recipe identity/revision sync, and implement per-channel sensor freshness and disabled-sensor gating.
- **Safety Budget boundaries (`SafetyGuard`):** Ensure safety budget commits and actor state transitions occur transactionally with physical dispatch confirmation.
- **Dispatcher & Command boundaries (`EventDispatcher`, `command_handler.rs`, backend `control.rs`):** Best-effort all-off prior to reboot/factory reset, prevent manual dosage ceiling elevation, and enforce effective duration/PWM constraints on `force_on`.

**Tech Stack:** Rust 2024 / 2021 edition (`hydragrow-controller-core`, `hydragrow-shared`, `hydragrow-backend`, `ESP32-C3-CONTROLLER-NODE`, `hydragrow-simulator`).

---

## Global Constraints & Mandatory Invariants

- **I1. Terminal convergence:** Sau hard-timeout, fault, abort, manual stop, reboot/reset preparation:
  - `DosingActor = Idle`
  - `WaterActor = Idle`
  - `Pump A/B/PhUp/PhDown = OFF`
  - `Water IN/OUT = OFF`
  - `Mist = OFF`
  - `Mix = OFF`
  - `Osaka = OFF / PWM 0`
  - `Ownership flags = false`
  - `Water watchdog timestamp = None`
- **I2. Water mutual exclusion:** Runtime path must reject or canonicalize simultaneous IN and OUT: `assert!(!(water_pump_in && water_pump_out))`
- **I3. No phantom safety commit:** Safety history commits only after corresponding actuator command has dispatched successfully.
- **I4. One normal timeout authority per water job:** `WaterJob.max_duration_sec` is the normal timeout authority. Phase watchdog is the final failsafe.
- **I5. Recipe execution identity:** Execution identity = `recipe_id + revision + stage_index + start_time` policy, never bare `stage_index`.
- **I6. Sensor freshness per enabled channel:** Each channel has dedicated freshness; disabled channel does not generate timeout or non-finite fault.

---

## Tasks

### Task 1: Canonical Terminal Abort cho Actor + Shadow

**Files:**
- Modify: `hydragrow-controller-core/src/core/fsm/context.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/orchestrator.rs`
- Create: `hydragrow-controller-core/tests/actor_terminal_reset_tests.rs`
- Modify: `hydragrow-controller-core/tests/cross_layer_fsm_tests.rs`

**Interfaces:**
- `SystemContext::reset_active_actors_and_ownership(&mut self)`: Clears actors (`dosing.reset()`, `water.abort()`), clears calibration, resets ownership flags, and sets all fields of `self.peripherals.pump_status` to false/None.
- `PeripheralDelta::all_pumps_off()`: Produces `PeripheralDelta` with all pumps/valves explicitly `Some(false)` and `osaka_pwm = Some(0)`.

- [ ] **Step 1: Write failing test in `actor_terminal_reset_tests.rs`**
  Assert that after activating dosing, water, and peripheral shadow states, calling `reset_active_actors_and_ownership()` or applying delta with `reset_active_actors = true` completely clears all `pump_status` flags and sets actors to Idle.
- [ ] **Step 2: Run test to verify failure**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test actor_terminal_reset_tests`
  Expected: FAIL (pump_status fields still true).
- [ ] **Step 3: Implement canonical shadow clear**
  In `context.rs`:
  ```rust
  pub fn reset_active_actors_and_ownership(&mut self) {
      self.dosing.reset();
      self.water.reset();
      self.calibration.pending_sample = None;
      self.peripherals.misting_started_by_dosing = false;
      self.peripherals.mix_valve_started_by_dosing = false;
      self.peripherals.is_misting_active = false;
      self.peripherals.is_scheduled_mixing_active = false;
      self.peripherals.water_pump_started_uptime_ms = None;
      self.peripherals.pump_status = hydragrow_shared::state::PumpStatus::default();
  }
  ```
  In `orchestrator.rs`: ensure `fault_all_outputs_off()` sets all `pump_status` fields to `Some(false)`.
- [ ] **Step 4: Run test to verify pass**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test actor_terminal_reset_tests`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git add hydragrow-controller-core/src/core/fsm/context.rs hydragrow-controller-core/src/core/fsm/orchestrator.rs hydragrow-controller-core/tests/actor_terminal_reset_tests.rs`
  `git commit -m "fix(fsm): canonical terminal actor and shadow abort"`

---

### Task 2: WaterActor Timeout/Abort là Terminal

**Files:**
- Modify: `hydragrow-controller-core/src/core/actors/water_actor.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/phases/mimo_dosing.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/phases/water_phases.rs`
- Test: `hydragrow-controller-core/tests/water_actor_semantics_tests.rs`
- Test: `hydragrow-controller-core/tests/orchestrator_timeout_tests.rs`

**Interfaces:**
- `WaterActor::abort(&mut self) -> Vec<OrchestratorEvent>`: Sets state to `Idle`, returns `SetWaterPump { direction: WaterDirection::Stop }`.
- `check_water_pump_timeouts(&self, ...)`: Failsafe watchdog aborts actor and sets `peri_delta.reset_active_actors = true`.
- `WaterRefillingPhase` / `WaterDrainingPhase`: `WaterEvent::Done { success: false, .. }` transitions to `SystemPhase::Fault(FaultCode::WaterRefillFailed | WaterDrainFailed)`.

- [ ] **Step 1: Write failing regression test for STOP-then-ON oscillation**
  In `water_actor_semantics_tests.rs`, test that when water timeout occurs, subsequent ticks while remaining in the phase do not re-emit `SetWaterPump(In/Out)`. Also test dedicated refill/drain failure routing to Fault.
- [ ] **Step 2: Run test to verify failure**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test water_actor_semantics_tests`
  Expected: FAIL.
- [ ] **Step 3: Implement WaterActor::abort and phase watchdog abort**
  In `water_actor.rs`: add `pub fn abort(&mut self)` and make `reset(&mut self)` call it.
  In `mimo_dosing.rs`: in `check_water_pump_timeouts`, call `ctx.water.abort()` and set `result.delta.reset_active_actors = true`.
  In `water_phases.rs`: route `WaterEvent::Done { success: false, .. }` to `SystemPhase::Fault(FaultCode::WaterRefillFailed)` (or `WaterDrainFailed`).
- [ ] **Step 4: Run tests to verify pass**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test water_actor_semantics_tests --test orchestrator_timeout_tests`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git commit -m "fix(water): make timeout terminal and actor-owned"`

---

### Task 3: Mimo Hard-Timeout/Failure Phải Shutdown Toàn Bộ Actuator

**Files:**
- Modify: `hydragrow-controller-core/src/core/fsm/phases/mimo_dosing.rs`
- Test: `hydragrow-controller-core/tests/cross_layer_fsm_tests.rs`
- Test: `hydragrow-controller-core/tests/phase_transition_tests.rs`

**Interfaces:**
- `stop_all_dosing_and_water(ctx, result, peri_delta)`: Emits OFF for dosing pumps A, B, PhUp, PhDown, water Stop, mist/mix valves OFF, calls `ctx.dosing.reset()`, `ctx.water.abort()`.
- `DosingEvent::Failed`: Immediately triggers `fault_all_outputs_off`, resets actors, and transitions to Fault.

- [ ] **Step 1: Write failing test for MimoDosing hard-timeout with active dosing**
  In `phase_transition_tests.rs`: set `ctx.phase = MimoDosing`, set `ctx.dosing` in `PumpingA` state with `pump_a: true`. Advance uptime past hard timeout limit. Assert emitted events contain `SetDosingPump` OFF for pump A, `dosing.is_idle()`, and `phase == Cooldown`.
- [ ] **Step 2: Run test to verify failure**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test phase_transition_tests mimo_dosing_hard_timeout`
  Expected: FAIL (no dosing pump OFF emitted).
- [ ] **Step 3: Implement shutdown in hard-timeout and failure paths**
  In `mimo_dosing.rs`: replace `stop_water_and_misting` with `abort_all_dosing_and_water`:
  Emit `SetDosingPump` OFF for all 4 channels, `SetWaterPump(Stop)`, `SetMistValve(false)`, `SetMixValve(false)`.
  Call `ctx.dosing.reset()` and `ctx.water.abort()`.
- [ ] **Step 4: Run test to verify pass**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test phase_transition_tests`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git commit -m "fix(mimo): shutdown dosing outputs on hard timeout"`

---

### Task 4: Enforce Water IN/OUT Mutual Exclusion

**Files:**
- Modify: `hydragrow-controller-core/src/core/fsm/phases/monitoring.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/tick_result.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/context.rs`
- Test: `hydragrow-controller-core/tests/water_actor_semantics_tests.rs`

**Interfaces:**
- In `monitoring.rs`: if `control.water_in_sec > 0.0 && control.water_out_sec > 0.0`, reject decision deterministically.
- In `PeripheralDelta::merge_from()`: `water_pump_in == Some(true)` sets `water_pump_out = Some(false)` and vice versa.
- In `SystemContext::apply_delta()`: assert `!(self.peripherals.pump_status.water_pump_in && self.peripherals.pump_status.water_pump_out)`.

- [ ] **Step 1: Write failing test in `water_actor_semantics_tests.rs`**
  Test decision having both `water_in_sec > 0` and `water_out_sec > 0` rejects both and emits error log; test `PeripheralDelta::merge_from` clears opposite direction.
- [ ] **Step 2: Run test to verify failure**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test water_actor_semantics_tests`
  Expected: FAIL.
- [ ] **Step 3: Implement mutual exclusion invariant**
  In `monitoring.rs`:
  ```rust
  if control.water_in_sec > 0.0 && control.water_out_sec > 0.0 {
      log::error!("🚨 [SAFETY INVARIANT] Simultaneous water IN and OUT requested. Rejecting water decision.");
      control.water_in_sec = 0.0;
      control.water_out_sec = 0.0;
  }
  ```
  In `PeripheralDelta`: enforce mutual exclusion during merge.
- [ ] **Step 4: Run test to verify pass**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test water_actor_semantics_tests`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git commit -m "fix(water): enforce direction exclusivity"`

---

### Task 5: Safety Budget Transaction Phải Commit Sau Dispatch Success

**Files:**
- Modify: `hydragrow-controller-core/src/core/fsm/phases/monitoring.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/tick_result.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/context.rs`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/runtime/fsm_loop.rs`
- Test: `hydragrow-controller-core/tests/cross_layer_fsm_tests.rs`

**Interfaces:**
- `TickResult`: holds `pending_safety_commits: Vec<SafetyCommit>`.
- `SystemContext::commit_safety_transaction(&mut self, commits: &[SafetyCommit])`: Commits recorded refills, drains, and hourly doses only when invoked.
- If dispatch fails, `pending_safety_commits` are discarded without charging safety history.

- [ ] **Step 1: Write failing test in `cross_layer_fsm_tests.rs`**
  Test scenario: Decision generates refill + dosing commands. Event dispatcher fails on first event. Assert that `safety.peek_refill` and hourly doses reflect 0 committed volume (no phantom commit).
- [ ] **Step 2: Run test to verify failure**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test cross_layer_fsm_tests`
  Expected: FAIL.
- [ ] **Step 3: Decouple safety commit from decision creation**
  In `monitoring.rs`: populate `result.pending_safety_commits` instead of immediately mutating `ctx.safety`.
  In `fsm_loop.rs` (and orchestrator `apply_delta`/commit helper): commit `pending_safety_commits` only after `EventDispatcher::dispatch()` returns `None`.
- [ ] **Step 4: Run test to verify pass**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test cross_layer_fsm_tests`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git commit -m "fix(safety): commit budget only after dispatch success"`

---

### Task 6: Không Mutate Execution State Irreversibly Trước Khi Dispatch Success

**Files:**
- Modify: `hydragrow-controller-core/src/core/fsm/phases/monitoring.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/context.rs`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/runtime/fsm_loop.rs`
- Test: `hydragrow-controller-core/tests/cross_layer_fsm_tests.rs`

**Interfaces:**
- In `apply_dispatch_fault()`: call `ctx.reset_active_actors_and_ownership()` so that any partially started actor (`ctx.water`, `ctx.dosing`) is aborted to `Idle`.
- In `MonitoringPhase`: prepare actor parameters in `ContextDelta` or ensure mandatory abort on dispatch failure.

- [ ] **Step 1: Write failing test in `cross_layer_fsm_tests.rs`**
  Test that when hardware dispatch fails on the very first actuator event, actor states (`ctx.water` and `ctx.dosing`) remain or revert to `Idle`.
- [ ] **Step 2: Run test to verify failure**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test cross_layer_fsm_tests`
  Expected: FAIL (actor left in active state).
- [ ] **Step 3: Implement dispatch failure rollback**
  In `apply_dispatch_fault` and simulator dispatch failure handler, ensure `ctx.reset_active_actors_and_ownership()` is called immediately.
- [ ] **Step 4: Run test to verify pass**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test cross_layer_fsm_tests`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git commit -m "fix(fsm): prevent phantom actor state on dispatch failure"`

---

### Task 7: Recipe Identity + Cursor + Start-Time Synchronization

**Files:**
- Modify: `hydragrow-controller-core/src/core/fsm/recipe_manager.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/context.rs`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/main.rs` (boot restore path)
- Test: `hydragrow-controller-core/tests/recipe_lifecycle_tests.rs`

**Interfaces:**
- `SystemContext`: tracks `active_recipe_id: Option<String>` and `active_recipe_revision: Option<u32>`.
- `tick_recipe_engine`: if recipe ID or revision changes, resets `current_stage_index` and `recipe_completed`, forcing stage reapplication even if stage index is identical.
- `activate_recipe`: if `now_sec < recipe.start_time_sec`, does not set stage 0 as active; waits until `start_time_sec`.
- `clear_recipe`: clears runtime stage, resets context stage cursor and completion flags.

- [ ] **Step 1: Write failing tests in `recipe_lifecycle_tests.rs`**
  - Recipe A stage 3 → Recipe B stage 3 re-applies overrides.
  - Clear recipe → new recipe with same stage index starts properly.
  - Future recipe activation before `start_time_sec` does not apply stage 0.
- [ ] **Step 2: Run test to verify failure**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test recipe_lifecycle_tests`
  Expected: FAIL.
- [ ] **Step 3: Implement identity & cursor synchronization**
  Update `recipe_manager.rs` and `context.rs` to track and compare `recipe_id` + `revision`. Gate stage activation on `now_sec >= recipe.start_time_sec`.
- [ ] **Step 4: Run tests to verify pass**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test recipe_lifecycle_tests`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git commit -m "fix(recipe): synchronize execution identity and cursor"`

---

### Task 8: Per-Channel Sensor Freshness + Disabled-Sensor Policy

**Files:**
- Modify: `hydragrow-controller-core/src/core/fsm/context.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/orchestrator.rs`
- Create: `hydragrow-controller-core/tests/sensor_freshness_regression_tests.rs`
- Test: `hydragrow-controller-core/tests/orchestrator_timeout_tests.rs`

**Interfaces:**
- `ChannelFreshness`: tracks `ec_ms: u64`, `ph_ms: u64`, `temp_ms: u64`, `water_ms: u64`.
- `orchestrator::tick()`: staleness and non-finite checks evaluated only for enabled sensor channels (`config.enable_ec_sensor`, `config.enable_ph_sensor`, etc.).

- [ ] **Step 1: Write failing tests in `sensor_freshness_regression_tests.rs`**
  - EC fresh continuously + pH stale > 90s ⇒ Fault(SensorTimeout).
  - pH disabled + pH NaN/Inf ⇒ no Fault, automation continues.
- [ ] **Step 2: Run test to verify failure**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test sensor_freshness_regression_tests`
  Expected: FAIL.
- [ ] **Step 3: Implement per-channel freshness and disabled gating**
  In `orchestrator.rs`:
  ```rust
  let ec_timed_out = config.enable_ec_sensor && uptime_ms.saturating_sub(ctx.sensor_freshness.ec_ms) > 90_000;
  let ph_timed_out = config.enable_ph_sensor && uptime_ms.saturating_sub(ctx.sensor_freshness.ph_ms) > 90_000;
  let ec_non_finite = config.enable_ec_sensor && !sensors.ec.is_finite();
  let ph_non_finite = config.enable_ph_sensor && !sensors.ph.is_finite();
  ```
- [ ] **Step 4: Run test to verify pass**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test sensor_freshness_regression_tests`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git commit -m "fix(sensor): track freshness per enabled channel"`

---

### Task 9: Close Manual Control Safety Holes

**Files:**
- Modify: `hydragrow-backend/src/api/control.rs`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/runtime/command_handler.rs`
- Test: `hydragrow-backend/tests/api_control_tests.rs` (or control unit test)

**Interfaces:**
- `validate_manual_dosing_safety`: `manual_max_allowed_ml` can only clamp down, never exceed server configured ceiling:
  `let max_allowed_ml = match manual_max_allowed_ml { Some(v) if v > 0.0 => v.min(node_limit), _ => node_limit };`
- `command_handler.rs`: `force_on` sets `manual_pump_timeout` unconditionally (using `duration_sec.unwrap_or(120)`) and validates effective PWM/duration.

- [ ] **Step 1: Write failing adversarial test in `hydragrow-backend/src/api/control.rs`**
  Assert that sending `manual_max_allowed_ml = 100000.0` fails validation when estimated dose exceeds server limit.
- [ ] **Step 2: Run test to verify failure**
  Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml api::control`
  Expected: FAIL.
- [ ] **Step 3: Implement clamping in backend and timeout safety in node**
  Clamp `manual_max_allowed_ml` against `node_limit`. In `command_handler.rs`, ensure `manual_pump_timeout` is registered for all `force_on` invocations.
- [ ] **Step 4: Run test to verify pass**
  Run: `cargo test --manifest-path hydragrow-backend/Cargo.toml api::control`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git commit -m "fix(control): close manual dose safety gaps"`

---

### Task 10: Reboot/Factory-Reset Phải Terminal Ngay Cả Khi Stop Fail

**Files:**
- Modify: `ESP32-C3-CONTROLLER-NODE/src/runtime/command_handler.rs`
- Modify: `ESP32-C3-CONTROLLER-NODE/src/runtime/dispatcher.rs`
- Test: `ESP32-C3-CONTROLLER-NODE/tests/` (or code verification)

**Interfaces:**
- `EventDispatcher::dispatch_terminal_lifecycle(events, dc)`: Runs `dispatch_best_effort_all_off` on stop events, logs warnings for actuator failures, and always reaches `RebootDevice` or `FactoryReset`.

- [ ] **Step 1: Write regression test for terminal lifecycle dispatch**
  Mock failing pump driver; verify that `RebootDevice` / `FactoryReset` is reached and executed despite actuator stop errors.
- [ ] **Step 2: Run test to verify failure**
- [ ] **Step 3: Implement best-effort lifecycle dispatch**
  In `dispatcher.rs`, distinguish terminal lifecycle events so stop failures don't short-circuit the execution.
- [ ] **Step 4: Run test to verify pass**
- [ ] **Step 5: Commit**
  `git commit -m "fix(runtime): make reboot/reset terminal after stop failure"`

---

### Task 11: DosingActor Là Single Owner Của Pulse Hardware

**Files:**
- Modify: `hydragrow-controller-core/src/core/actors/dosing_actor.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/phases/mimo_dosing.rs`
- Test: `hydragrow-controller-core/tests/inflight_job_semantics_tests.rs`

**Interfaces:**
- Remove duplicate `SetDosingPump` emission in `MimoDosingPhase::tick` `DosingEvent::PulseToggle`.
- Ensure active dosing job uses snapshotted PWM/duration from job creation rather than reading live config.

- [ ] **Step 1: Write failing test in `inflight_job_semantics_tests.rs`**
  Assert that during a pulse toggle in `MimoDosingPhase`, exactly ONE `SetDosingPump` event is present in `TickResult.events`.
- [ ] **Step 2: Run test to verify failure**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test inflight_job_semantics_tests`
  Expected: FAIL (2 events found).
- [ ] **Step 3: Remove duplicate emission**
  In `mimo_dosing.rs`: remove `result.events.push(OrchestratorEvent::SetDosingPump)` from `DosingEvent::PulseToggle` branch since `ctx.dosing.tick()` already emits it into `hardware_events`.
- [ ] **Step 4: Run test to verify pass**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test inflight_job_semantics_tests`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git commit -m "fix(dosing): make actor sole pulse hardware owner"`

---

### Task 12: Config Cross-Field Validation

**Files:**
- Modify: `hydragrow-shared/src/lib.rs`
- Test: `hydragrow-shared/tests/config_validation_tests.rs` (or in `lib.rs`)

**Interfaces:**
- `ControllerConfig::validate(&self)`: checks:
  - `water_level_min <= water_level_target && water_level_target <= water_level_max`
  - `min_ec_limit <= max_ec_limit`
  - `min_ph_limit <= max_ph_limit`
  - `min_temp_limit <= max_temp_limit`
  - `water_level_critical_min < water_level_max`
  - `scheduled_drain_amount_cm <= tank_height`

- [ ] **Step 1: Write failing tests in `config_validation_tests.rs`**
  Add test cases for each cross-field invalid combination (e.g. `water_level_target < water_level_min`, `min_ec_limit > max_ec_limit`, etc.).
- [ ] **Step 2: Run test to verify failure**
  Run: `cargo test --manifest-path hydragrow-shared/Cargo.toml`
  Expected: FAIL.
- [ ] **Step 3: Implement cross-field validations**
  In `hydragrow-shared/src/lib.rs` `validate(&self)`: add the 6 relation checks.
- [ ] **Step 4: Run test to verify pass**
  Run: `cargo test --manifest-path hydragrow-shared/Cargo.toml`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git commit -m "fix(config): enforce cross-field invariants"`

---

### Task 13: Consume/Remove WaterSubState::Starting

**Files:**
- Modify: `hydragrow-controller-core/src/core/actors/water_actor.rs`
- Modify: `hydragrow-controller-core/src/core/fsm/phases/water_phases.rs`
- Test: `hydragrow-controller-core/tests/water_actor_semantics_tests.rs`

**Interfaces:**
- Remove `WaterSubState::Starting` completely from `WaterSubState` enum. Transitions go directly from `Idle` to `Filling { job }` or `Draining { job }`.

- [ ] **Step 1: Write test in `water_actor_semantics_tests.rs`**
  Assert that start_fill directly yields `Filling` state and tick does not emit spurious Stop events on first cycle.
- [ ] **Step 2: Run test to verify failure**
- [ ] **Step 3: Remove `Starting` variant**
  Remove `Starting` from `WaterSubState` and update `water_phases.rs` to start fill/drain directly from `Idle`.
- [ ] **Step 4: Run tests to verify pass**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test water_actor_semantics_tests`
  Expected: PASS.
- [ ] **Step 5: Commit**
  `git commit -m "fix(water): remove transient Starting ambiguity"`

---

### Task 14: Build Complete Terminal Transition Regression Matrix

**Files:**
- Create: `hydragrow-controller-core/tests/fsm_terminal_matrix_tests.rs`

**Scenarios covered:**
1. MimoDosing + PumpA ON + hard timeout ⇒ Cooldown, actors Idle, all pumps OFF
2. Water Filling + watchdog timeout ⇒ Cooldown/Fault, WaterActor Idle, Stop emitted
3. Water IN + Water OUT same decision ⇒ rejected deterministically
4. Actuator A succeeds + B dispatch fails ⇒ safety budget only accounts confirmed actions
5. Recipe A stage 3 → Recipe B stage 3 ⇒ stage overrides re-applied
6. Clear recipe → new recipe same stage index ⇒ overrides applied
7. Boot recipe + persisted cursor ⇒ consistent runtime state
8. Future recipe start time ⇒ stage 0 inactive until start time
9. Sensor EC fresh + pH stale ⇒ Fault(SensorTimeout)
10. pH disabled + pH NaN ⇒ no fault

- [ ] **Step 1: Write all 10 adversarial scenarios in `fsm_terminal_matrix_tests.rs`**
- [ ] **Step 2: Run tests to verify all pass**
  Run: `cargo test --manifest-path hydragrow-controller-core/Cargo.toml --test fsm_terminal_matrix_tests`
  Expected: PASS.
- [ ] **Step 3: Commit**
  `git add hydragrow-controller-core/tests/fsm_terminal_matrix_tests.rs`
  `git commit -m "test(fsm): add terminal transition matrix"`

---

### Task 15: Full Validation & Delivery Governance

- [ ] **Step 1: Run Controller Core test suite**
  `cargo test --manifest-path hydragrow-controller-core/Cargo.toml`
- [ ] **Step 2: Run Shared test suite**
  `cargo test --manifest-path hydragrow-shared/Cargo.toml`
- [ ] **Step 3: Run Simulator test suite**
  `cargo test --manifest-path hydragrow-simulator/Cargo.toml`
- [ ] **Step 4: Run Backend test suite**
  `cargo test --manifest-path hydragrow-backend/Cargo.toml`
- [ ] **Step 5: Run formatting and clippy across workspaces**
  `cargo fmt --all -- --check`
  `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] **Step 6: Update Delivery Governance contracts**
  Update `docs/acceptance/CONTROLLER-FSM-002.json`, `docs/evidence/CONTROLLER-FSM-002.json`, `docs/project-state/CURRENT-STATUS.md`, `docs/project-state/TRACEABILITY.md`.
- [ ] **Step 7: Commit governance files**
  `git commit -m "docs(governance): add acceptance contract and evidence for controller FSM remediation"`
