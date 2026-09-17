# HYDRAGROW — Domain Spec 08: P1.9 Firmware Fixture Failure

**Domain ID:** `P1.9-FIRMWARE-FIXTURE-001`  
**Domain:** Sensor firmware canonical P1.9 fixture verification  
**Status:** `COMPLETE / VERIFIED`  
**Scope:** `ESP32-C3-SENSOR-NODE` native fixture test and its CI trigger for `schema/contracts` changes

## 1. Problem

P1.9 introduced canonical cross-system fixtures, but historical verification recorded a failing sensor-node native test:

`test_p1_9_sensor_fixture_matches_arduinojson_wire_shape: Expected TRUE Was FALSE`

Historical P1.9 evidence also recorded that no direct C++ sensor-node fixture consumer existed. A later firmware commit added a native ArduinoJson fixture test and the repository currently passes that test, but Domain 8 must close the failure as an explicit, independently verifiable contract rather than rely on historical status.

The canonical sensor fixture is:

`schema/contracts/fixtures/sensor-data.json`

The C++ sensor node publishes sensor JSON through `MqttManager::publishSensorData()` and uses ArduinoJson. The native test must consume the same repository fixture and reject accidental legacy-wire drift.

## 2. Goals

1. Make the P1.9 sensor fixture test deterministic from any PlatformIO native-test invocation.
2. Verify the canonical fixture can be parsed by ArduinoJson.
3. Verify representative required values and canonical names used by the sensor node.
4. Verify legacy `tds` / `err_tds` names are not emitted by the canonical fixture.
5. Ensure firmware CI reruns the native fixture test when canonical schema/fixture files change.
6. Preserve the distinction between historical failure evidence and current executable verification.

## 3. Non-goals

- No redesign of P1.9 shared schema ownership.
- No new schema format or generated-code system.
- No controller firmware fixture consumer; that is Domain 9.
- No physical/HIL verification.
- No changes to MQTT synchronization, ConfigurationSync, safety semantics, backend, frontend, or production PostgreSQL.
- No change to sensor business behavior solely to satisfy a fixture test.

## 4. Contract

### 4.1 Fixture source

The native test MUST read only:

`schema/contracts/fixtures/sensor-data.json`

The path MUST resolve from the repository root through an explicit PlatformIO compile-time project-root definition. The test MUST NOT depend on the process current working directory.

### 4.2 Canonical parse

ArduinoJson deserialization MUST succeed.

The fixture MUST contain and parse as:

- `device_id`: string
- `ec`: finite numeric value
- `ph`: finite numeric value
- `temp`: finite numeric value
- `water_level`: finite numeric value
- `pump_status`: object
- `time`: string
- `err_ec`: boolean

The fixture MUST NOT contain canonical-output legacy aliases:

- `tds`
- `err_tds`

### 4.3 Representative values

The test MUST assert representative fixture values sufficient to detect fixture path, field-name, type, and parser drift:

- `device_id == device-001`
- `ec == 1.5`
- `ph == 6.0`
- `temp == 27.0`
- `water_level == 20.0`
- `time` is a string
- `err_ec == false`

### 4.4 CI coupling

The sensor firmware CI workflow MUST run when either:

- `ESP32-C3-SENSOR-NODE/**` changes, or
- `schema/contracts/**` changes.

This keeps canonical fixture changes coupled to the firmware fixture consumer without making firmware CI responsible for Domain 9 controller consumers.

## 5. Acceptance criteria

- **AC-01:** Historical P1.9 fixture failure is explicitly documented as historical, not current.
- **AC-02:** Native sensor fixture test reads canonical `sensor-data.json` from repository root without cwd dependence.
- **AC-03:** ArduinoJson parses the canonical fixture successfully.
- **AC-04:** Required representative sensor fields and values are asserted.
- **AC-05:** Canonical `err_ec` is asserted and legacy `err_tds` is absent.
- **AC-06:** Canonical `tds` is absent.
- **AC-07:** Native sensor test command passes with zero failures.
- **AC-08:** Firmware CI path filters include `schema/contracts/**` so fixture changes execute the test.
- **AC-09:** Existing sensor native tests remain green.
- **AC-10:** No unrelated subsystem is modified and no production PostgreSQL is accessed.

## 6. Verification matrix

| Gate | Required evidence | Status |
|---|---|---|
| V-01 | `pio test --environment native` | PASS |
| V-02 | Dedicated P1.9 fixture test passes | PASS |
| V-03 | Native suite reports zero failures | PASS |
| V-04 | Fixture contains canonical names and no `tds`/`err_tds` | PASS |
| V-05 | Firmware CI includes `schema/contracts/**` trigger | PASS |
| V-06 | Worktree integrity / no unrelated revert | PASS |

## 7. Evidence

Create:

`docs/evidence/P1.9-FIRMWARE-FIXTURE-001.json`

Evidence MUST record:

- historical failure source and exact failure;
- current commands actually executed;
- native test result;
- fixture path and assertions;
- CI path-filter change;
- worktree preservation;
- production PostgreSQL untouched.

Target completion status:

`COMPLETE / VERIFIED`

Only executable evidence may move this spec to that status.
