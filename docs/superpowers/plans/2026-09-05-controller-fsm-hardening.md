# HYDRAGROW Controller FSM Hardening Implementation Plan

**Goal:** Khắc phục các lỗi P0/P1 đã phát hiện trong controller FSM của HYDRAGROW, làm cho state transition, actuator shutdown, sensor freshness, dosing/water execution, recipe runtime và persistence có semantics nhất quán và kiểm thử được.

**Architecture:** Giữ kiến trúc FSM/Actor hiện tại, không rewrite toàn bộ controller. Chuẩn hóa boundary SystemContext → ContextDelta → TickResult → EventDispatcher → hardware; actor chỉ tạo execution intent, còn hardware success/failure được phản ánh ngược vào runtime. Mỗi nhóm thay đổi phải có regression test và commit riêng.

**Tech Stack:** Rust, Cargo workspace, ESP32-C3 runtime, hydragrow-controller-core, unit/integration tests.

---

## Tasks

- [x] **Task 1: Establish a Clean Build/Test Baseline**
  - Step 1: Run workspace check (cargo check or per-crate check). Verify DispatchContext boundary.
  - Step 2: Run all tests.
  - Step 3: Record baseline results.
  - Step 4: Commit only a necessary baseline compile fix; otherwise keep code-free.

- [x] **Task 2: Make ContextDelta Merging Field-Wise**
  - Step 1: Add a failing regression test in core FSM for preserving independent peripheral fields.
  - Step 2: Run targeted test and verify failure.
  - Step 3: Implement field-wise merge helper in orchestrator.rs, tick_result.rs, context.rs.
  - Step 4: Add conflict test for two producers writing the same valve with explicit ownership rule.
  - Step 5: Run core tests.
  - Step 6: Commit: fix(fsm): merge context deltas without dropping fields.

- [x] **Task 3: Make Event Dispatch Transactional on Hardware Fault**
  - Step 1: Add failing dispatcher test with events [PumpA ON, PumpB ON, Water IN] and PumpA failing; assert PumpB and Water are not attempted.
  - Step 2: Run test.
  - Step 3: Refactor borrow boundary in fsm_loop.rs / dispatcher.rs.
  - Step 4: Make emergency ALL-OFF return its own dispatch outcome (never discard shutdown dispatch result).
  - Step 5: Add test where OFF command fails during ALL-OFF: assert primary fault stays latched and secondary shutdown failure is observable.
  - Step 6: Run tests.
  - Step 7: Commit: fix(runtime): stop dispatching after actuator fault.

- [x] **Task 4: Centralize Terminal Stop / Fault / Calibration Actor Reset**
  - Step 1: Add failing sensor-fault test: PumpingA → sensor timeout → Fault/all-off → actor must be idle.
  - Step 2: Add failing calibration-entry test: active dosing/water actor → enter calibration → all actor/ownership state cleared.
  - Step 3: Add manual-disable/ManualMode stop regression: re-enabling automation must not resume previous actor job.
  - Step 4: Add centralized method reset_active_actors_and_ownership(&mut self) in SystemContext and invoke on terminal boundaries.
  - Step 5: Run core tests.
  - Step 6: Commit: fix(fsm): reset actors and ownership on terminal stops.

- [x] **Task 5: Repair Calibration Lifecycle and Timeout**
  - Step 1: Add failing timeout test: phase = SensorCalibration, phase_finish_ms = 1000, now_ms = 1001 must leave calibration safely.
  - Step 2: Run targeted test.
  - Step 3: Add missing SensorCalibration phase branch in orchestrator.rs so phase_finish_ms is evaluated.
  - Step 4: Add calibration-enter regression for Osaka PWM, mist, mix, water in/out, actor state, and ownership flags.
  - Step 5: Harden exit_calibration: clear calibration-specific pending state, reset actors/ownership, transition to Monitoring.
  - Step 6: Run core tests and commit: fix(calibration): enforce timeout and lifecycle cleanup.

- [x] **Task 6: Harden Sensor Ingress and Freshness**
  - Step 1: Add empty-packet test for {} asserting controller_received_ms does not advance.
  - Step 2: Add partial-packet test: {"ec":1.5,"err_ec":true} then {} must preserve EC and err_ec=true.
  - Step 3: Add malformed/non-finite tests for NaN/Infinity/impossible combinations.
  - Step 4: Implement field-wise sensor merge: only overwrite state field when Some; update freshness only after valid measurement packet.
  - Step 5: Keep watchdog monotonic: retain saturating_sub uptime comparison; refresh sensor_last_update_ms only on accepted packet.
  - Step 6: Run tests.
  - Step 7: Commit: fix(sensors): refresh freshness only for valid measurements.

- [x] **Task 7: Make WaterActor Result Semantics Explicit**
  - Step 1: Add timeout/failure regression: WaterEvent::Done { success: false, duration_sec: 42 } must never advance as successful completion.
  - Step 2: Add duration propagation regression: cycle accounting must use actor duration_sec, not configured maximum.
  - Step 3: Change MIMO matching to explicit branches: success: true vs success: false.
  - Step 4: Carry actual duration through ContextDelta/event accounting into cycle report and calibration.
  - Step 5: Add fill+drain same-tick test. Assert actor cannot be started in both directions in one tick.
  - Step 6: Implement explicit mutual-exclusion rule.
  - Step 7: Run core tests and commit: fix(water): preserve result and measured duration.

- [x] **Task 8: Make Dosing Planning Transactional With Safety Budget**
  - Step 1: Add failing test: flow A unavailable → no A budget commit.
  - Step 2: Add A=0/B>0 regression: B job must survive and be budgeted/executed.
  - Step 3: Add PH Up + PH Down regression: both must not collapse into one pending slot; execute serially or reject explicitly.
  - Step 4: Change start_matrix_cycle() to return an explicit prepared plan/result: Prepared(Vec<Job>) or Rejected(Reason).
  - Step 5: Commit/charge budget from the prepared plan only.
  - Step 6: Add invariant test: committed hourly dose equals the sum of prepared executable jobs.
  - Step 7: Run tests and commit: fix(dosing): charge safety budget only for executable jobs.

- [x] **Task 9: Correct Delivered-Volume Accounting and Pump-B Start Semantics**
  - Step 1: Add failing test: ON event generated, hardware write fails → delivered_ml == 0.
  - Step 2: Separate requested/reserved volume from executed/delivered volume. Only confirmed hardware success increments delivered volume.
  - Step 3: Add PumpB start test. Explicitly model state as intent and guarantee fault reconciliation.
  - Step 4: Run tests.
  - Step 5: Commit: fix(dosing): account delivered volume after hardware success.

- [x] **Task 10: Make Water Hardware Stop and Retry Reliable**
  - Step 1: Add emergency-stop test where Water IN OFF fails; Water OUT OFF must still be attempted.
  - Step 2: Remove early-return behavior for STOP. Attempt both physical outputs and aggregate any errors.
  - Step 3: Add cached-direction recovery test: software cache says IN while hardware is actually OFF; next IN request must issue real write.
  - Step 4: Invalidate direction cache on hardware reset/fault or add readback/reassertion.
  - Step 5: Run tests and commit: fix(water-hw): complete stop and recover stale direction cache.

- [x] **Task 11: Make Peripheral Decisions Use Final Same-Tick Intent**
  - Step 1: Add stale-state test: phase turns misting ON; same tick Osaka decision must use the new intent, not prior is_misting_active.
  - Step 2: Add mix-valve conflict test: phase says ON, scheduler says OFF; assert explicit owner wins and only one command is emitted.
  - Step 3: Reorder the flow: phase decisions → merge deltas → resolve peripheral ownership/conflicts → apply final intent → generate side effects.
  - Step 4: Remove dead _is_dosing_active parameter or make it authoritative for ownership.
  - Step 5: Run core tests and commit: fix(fsm): resolve same-tick peripheral intent deterministically.

- [x] **Task 12: Unify Recipe Activation, Persistence, and Clear**
  - Step 1: Add recipe/set activation test: valid signed recipe/set updates runtime active recipe, revision, and stage state.
  - Step 2: Add reboot persistence test: set → persist → fresh load returns same active recipe.
  - Step 3: Add recipe/clear test: handler clears runtime active recipe and canonical persistence, stops applying old stages.
  - Step 4: Introduce one canonical activation path: activate_recipe(&mut self, recipe: ValidatedRecipe) -> Result<()>.
  - Step 5: Align NVS write/read keys to one source of truth (active_recipe).
  - Step 6: Add recipe-id/revision reset test: changing either resets stage cursor and recipe_completed.
  - Step 7: Run tests and commit: fix(recipe): unify runtime activation and persistence.

- [x] **Task 13: Route Scheduled Water Through Common Amount + Safety Planning**
  - Step 1: Add scheduled-water test where scheduled_drain_amount_cm != max_drain_duration_sec; assert requested amount drives planning.
  - Step 2: Add safety-equivalence test: scheduled operation violating manual/recipe guardrail must be rejected by same rule.
  - Step 3: Create one common water planning function that converts direction + amount + flow/calibration + max duration into validated plan.
  - Step 4: Run core tests and commit: fix(water): use common planning for scheduled changes.

- [x] **Task 14: Define Runtime Config/Recipe Semantics for In-Flight Jobs**
  - Step 1: Add test: start dosing with config A, update to B while active; assert in-flight job keeps A values and next job uses B.
  - Step 2: Add safety-critical update abort test: required abort clears actor/ownership and emits OFF intent.
  - Step 3: Add recipe id/revision hot-swap regression: new recipe cannot inherit old stage/completion state.
  - Step 4: Store minimal job snapshot needed by actor.
  - Step 5: Run tests and commit: fix(fsm): define deterministic semantics for active jobs.

- [x] **Task 15: Formalize Safety-Budget Scope and Numeric Validation**
  - Step 1: Write tests defining whether max_dose_per_hour is global or per pump/group.
  - Step 2: Test EC then PH budget interaction against that policy.
  - Step 3: Add node-side validation tests before casts to unsigned numeric types: reject negative, NaN, Infinity, zero where forbidden.
  - Step 4: Run workspace tests and commit: fix(safety): formalize budget scope and numeric validation.

- [x] **Task 16: Add Cross-Layer FSM Regression Scenarios**
  - Step 1: Sensor timeout recovery: Monitoring → active dosing → timeout → Fault/all-off → sensor recovery; no stale actor resumes.
  - Step 2: Dispatcher failure: first ON fails, ALL-OFF runs, one OFF fails; assert primary + shutdown failures observable.
  - Step 3: Water timeout: start → failure → no success accounting → actor reset.
  - Step 4: Recipe swap: recipe A stage 2 → recipe B revision 1 → B stage 0.
  - Step 5: Recipe reboot: set → persist → restart/load → same active recipe.
  - Step 6: Partial sensor packet: error flag remains until explicitly replaced/cleared.
  - Step 7: Run workspace tests.
  - Step 8: Commit: test(fsm): add cross-layer controller regressions.

- [x] **Task 17: Final Static Audit / Verification Gate**
  - Step 1: Run final build/test (cargo check, cargo test).
  - Step 2: Search all stop/reset paths for fault_all_outputs_off, stop_all_hardware, dosing.reset, water.reset.
  - Step 3: Search wildcard event handling (Done { .. }, _ =>).
  - Step 4: Search budget commits (commit_hourly_dose, commit_refill, commit_drain).
  - Step 5: Search direct PeripheralDelta replacement.
  - Step 6: Verify every P0/P1 finding has a regression test.
  - Step 7: Produce defect matrix.
  - Step 8: Confirm release readiness.
