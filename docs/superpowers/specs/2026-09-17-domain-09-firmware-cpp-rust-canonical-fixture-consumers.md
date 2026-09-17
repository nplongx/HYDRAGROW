# HYDRAGROW — Domain Spec 09: Firmware C++/Rust Canonical Fixture Consumers

**Domain ID:** `P1.9-FIRMWARE-CANONICAL-CONSUMERS-001`  
**Domain:** Firmware consumers of canonical P1.9 JSON fixtures  
**Status:** `IMPLEMENTED / VERIFICATION BLOCKED BY FIRMWARE TOOLCHAIN`  
**Scope:** Rust controller firmware canonical fixture consumers, with the existing C++ sensor consumer treated as the Domain 8 prerequisite

## 1. Problem

Domain 8 closed the historical sensor-node fixture failure by hardening the C++ sensor consumer. The remaining firmware-side gap is that the Rust controller has runtime MQTT deserializers for canonical sensor and command payloads, but no controller-firmware test directly consumes the repository-owned canonical fixtures.

The canonical fixtures are registry-owned under:

- `schema/contracts/fixtures/sensor-data.json`
- `schema/contracts/fixtures/mqtt-command.json`

The controller runtime already consumes these wire shapes through `IncomingSensorPayload` and `MqttCommandIn`. Domain 9 must bind those exact runtime types to the canonical fixtures with compile-time repository-root paths and executable tests.

## 2. Goals

1. Add a Rust controller-firmware test that reads the canonical sensor fixture with the exact `IncomingSensorPayload` type used by `mqtt_client.rs`.
2. Add a Rust controller-firmware test that reads the canonical command fixture with the exact `MqttCommandIn` type used by `mqtt_client.rs`, while keeping HMAC verification out of fixture deserialization tests.
3. Assert representative canonical values and reject accidental legacy top-level command fields.
4. Resolve fixture paths at compile time from the repository root; tests must not depend on process current working directory.
5. Couple controller firmware CI to `schema/contracts/**` changes.
6. Preserve the Domain 8 C++ sensor consumer and do not redesign shared schemas.

## 3. Non-goals

- No changes to `hydragrow-shared` wire-model compatibility aliases.
- No changes to the canonical fixture contents unless an executable consumer proves the current fixture is invalid.
- No controller production MQTT behavior changes solely to satisfy the fixture test.
- No sensor C++ production behavior changes; its consumer is already covered by Domain 8.
- No HIL/physical verification.
- No backend, frontend, PostgreSQL, ConfigurationSync, readiness, or safety work.
- No firmware release workflow changes beyond the controller CI path coupling required here.

## 4. Contract

### 4.1 Canonical sensor consumer

The controller test MUST read:

`schema/contracts/fixtures/sensor-data.json`

and deserialize it using `hydragrow_shared::IncomingSensorPayload`, the same type used by `ESP32-C3-CONTROLLER-NODE/src/hw/mqtt_client.rs` for the `sensor/status` MQTT payload.

The test MUST assert:

- `ec == Some(1.5)`
- `ph == Some(6.0)`
- `temp == Some(27.0)`
- `water_level == Some(20.0)`
- `time` is present
- `err_ec == Some(false)`
- `is_valid()` is true

The test MUST verify the fixture does not use legacy `tds` or `err_tds` output keys. Legacy aliases remain input compatibility only.

### 4.2 Canonical command consumer

The controller test MUST read:

`schema/contracts/fixtures/mqtt-command.json`

and deserialize the payload into `MqttCommandIn` after removing only the envelope authentication fields `ts`, `nonce`, and `signature`. This test validates the wire deserializer shape; it MUST NOT claim to verify the fixture's HMAC.

The test MUST assert:

- `action == "start"`
- `target == Some("device-001")`
- `params.pump_id == Some("pump_a")`
- `params.duration_sec == Some(8)`
- `params.pwm == Some(70)`
- `params.state == Some(true)`

The fixture's top-level object MUST NOT contain legacy command fields `pump`, `pump_id`, `duration_sec`, or `pwm`.

### 4.3 Path determinism

Fixture paths MUST use Rust compile-time inclusion (`include_str!` or an equivalent compile-time repository-root path) so the tests do not depend on the process current working directory.

### 4.4 CI coupling

`.github/workflows/firmware-controller-ci.yml` MUST trigger on:

- `ESP32-C3-CONTROLLER-NODE/**`
- `schema/contracts/**`

The existing controller native/build checks remain the CI gate.

## 5. Acceptance criteria

- **AC-01:** Canonical sensor fixture is consumed by the Rust controller using `IncomingSensorPayload`.
- **AC-02:** Canonical command fixture is consumed by the Rust controller using `MqttCommandIn`.
- **AC-03:** Sensor representative values and validity assertions pass.
- **AC-04:** Legacy sensor output aliases `tds` and `err_tds` are absent.
- **AC-05:** Command representative values pass.
- **AC-06:** Legacy top-level command fields are absent.
- **AC-07:** Fixture tests are independent of process current working directory.
- **AC-08:** Controller CI watches `schema/contracts/**` for push and pull request events.
- **AC-09:** Controller Rust test/build verification passes, or a pre-existing toolchain blocker is recorded with exact executable evidence.
- **AC-10:** Domain 8 sensor consumer remains intact and no unrelated subsystem is modified.

## 6. Verification matrix

| Gate | Required evidence | Status |
|---|---|---|
| V-01 | Targeted controller canonical-fixture tests | BLOCKED — target test binary compiles, but configured `espflash flash --monitor` runner requires a physical target and rejects test arguments |
| V-02 | Controller Cargo test from firmware directory | BLOCKED — firmware target compiled, then `espflash` runner failed before executing test cases |
| V-03 | Controller Cargo test/build from repository-root invocation | BLOCKED — repository-root invocation uses host target unless firmware environment is sourced; host target is rejected by `esp-idf-sys` |
| V-04 | Sensor + command fixture assertions | PASS by shared canonical parser regression + static controller consumer assertions |
| V-05 | Controller CI `schema/contracts/**` trigger | PASS |
| V-06 | `git diff --check` | PASS |
| V-07 | Worktree integrity / production PostgreSQL untouched | PASS |

The earlier `AtomicI64` target compile failure was introduced by the Domain 9 patch's config-version synchronization addition. It was corrected to `Mutex<i64>` after confirming ESP32-C3 lacks `AtomicI64`; the controller test binary then compiled successfully for `riscv32imc-esp-espidf`. This does not establish fixture test execution.

## 7. Evidence

Create:

`docs/evidence/P1.9-FIRMWARE-CANONICAL-CONSUMERS-001.json`

Evidence MUST record:

- exact fixture paths and runtime consumer types;
- exact commands actually executed and their results;
- current controller toolchain/build status;
- CI path-filter verification;
- Domain 8 sensor consumer preservation;
- worktree preservation and production PostgreSQL untouched.

Target completion status after the blocking firmware toolchain is resolved:

`COMPLETE / VERIFIED`

Only executable controller test-case evidence may move this spec to that status. Compilation alone is insufficient.
