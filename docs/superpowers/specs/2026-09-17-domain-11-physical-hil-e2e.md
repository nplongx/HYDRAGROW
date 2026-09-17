# HYDRAGROW — Domain Spec 11: Physical / HIL E2E

**Domain ID:** `DOMAIN-11-PHYSICAL-HIL-E2E`  
**Domain:** Physical / Hardware-in-the-Loop end-to-end verification  
**Status:** `IMPLEMENTED / VERIFICATION BLOCKED`

## 1. Objective

Verify the real HYDRAGROW device path against physical ESP32-C3 hardware and a controlled test bench after Domain 10 cross-system verification.

The purpose is to prove that repository-level contracts survive the physical boundary:

- sensor firmware boots and publishes canonical telemetry;
- controller firmware boots, connects, receives commands/configuration, and drives hardware outputs;
- backend receives physical telemetry and exposes the resulting device state;
- frontend observes the same physical device state;
- configuration synchronization reaches the physical controller and reports the expected revision/status;
- safety/fault conditions stop actuators and recover only under the documented conditions;
- restart/power-cycle preserves the durable state required by the roadmap;
- no physical test requires production credentials, production PostgreSQL, or uncontrolled chemical/actuator operation.

This domain is execution-first. Source inspection, host tests, simulator tests, and firmware compilation are prerequisites/evidence inputs but are **not** substitutes for physical/HIL execution.

## 2. Preconditions and safety boundary

- Preserve the existing dirty worktree. Never reset, clean, revert, or stash unrelated changes.
- Never connect to production PostgreSQL or production MQTT infrastructure.
- Use a dedicated/staging MQTT broker, backend instance, and isolated PostgreSQL database/container.
- Use a bench/device identity that is explicitly non-production and unique to the test run.
- Before every ESP32-C3 controller firmware build/check/flash command, run:
  `source ~/export-esp.sh`
- Before sensor firmware build/flash, use the repository's PlatformIO environment.
- Physical actuator tests must start with pumps/valves disconnected from hazardous fluid, or with a mechanically safe test load, unless a test explicitly requires fluid flow and the operator has confirmed containment.
- Do not run dry-pump or high-power tests outside the device/vendor-safe operating envelope.
- Emergency stop/power cut must be physically reachable before any actuator test.
- No test may intentionally create an unsafe EC/pH/water condition in a live crop/reservoir.
- Credentials/secrets must not be committed or written into evidence artifacts.
- If required hardware, broker, bench instrumentation, or safe test load is unavailable, record the exact blocker and stop; do not convert a simulated/host result into a physical PASS.

## 3. Required test topology

The minimum HIL topology is:

```text
                         isolated/staging LAN
                                │
                ┌───────────────┴────────────────┐
                │                                │
        HYDRAGROW backend                  MQTT broker
        + isolated PostgreSQL                    │
                │                                │
                └───────────────┬────────────────┘
                                │ MQTT
                     ┌──────────┴──────────┐
                     │                     │
             ESP32-C3 Sensor         ESP32-C3 Controller
             physical node           physical node
                     │                     │
              EC / pH / temp /       pump/valve I/O,
              water-level inputs     status/command path
                     │                     │
                     └──── controlled bench ────┘
                                │
                         frontend browser
```

### Minimum physical assets

1. ESP32-C3 sensor node.
2. ESP32-C3 controller node.
3. USB serial connection(s) or an equivalent documented flashing/monitoring path.
4. Controlled sensor stimulus or calibrated signal source for EC/pH/temperature/water-level inputs.
5. Safe actuator load or disconnected-output harness with observable GPIO/I2C state.
6. Emergency power cut / emergency stop.
7. Isolated/staging network with MQTT reachable from both devices.
8. Backend + isolated PostgreSQL.
9. Operator-visible serial logs and MQTT telemetry capture.
10. Frontend instance pointing at the same staging backend.

## 4. Environment setup

### 4.1 Controller

Before any controller build/check/flash command:

```text
source ~/export-esp.sh
```

Then use the repository-supported ESP32-C3 target and `espflash` flow from `ESP32-C3-CONTROLLER-NODE/README.md`.

### 4.2 Sensor

Use the repository-supported PlatformIO environment:

```text
pio run --project-dir ESP32-C3-SENSOR-NODE --environment esp32-c3-devkitm-1
pio run --project-dir ESP32-C3-SENSOR-NODE --target upload --environment esp32-c3-devkitm-1
pio device monitor --baud 115200
```

### 4.3 Backend / frontend / MQTT

Use only isolated/staging endpoints. Record endpoint identity and isolated PostgreSQL database name, but redact passwords, tokens, API keys, and WiFi credentials.

The HIL run must capture:

- device IDs;
- firmware versions;
- test start/end timestamps;
- MQTT topic names used (topic paths are acceptable; credentials are not);
- backend API base URL identity without secrets;
- isolated database identity;
- serial/USB port identifiers where needed;
- exact command and exit code for every executable gate.

## 5. Physical/HIL verification matrix

### H-01 — Sensor boot and canonical telemetry

**Setup:** Flash the sensor node with the test firmware, connect the controlled sensor stimulus, connect to staging WiFi/MQTT.

**Procedure:**
1. Cold boot the sensor node.
2. Confirm serial boot reaches normal runtime without a fatal/reboot loop.
3. Confirm MQTT connection.
4. Produce a controlled, known sensor input.
5. Capture `AGITECH/{device_id}/sensor/data`.
6. Verify the payload is accepted by the backend's canonical `SensorData` path.

**Pass:** Physical sensor node publishes telemetry that backend accepts as the canonical contract; device identity and required fields are correct; no credential material is present in telemetry.

### H-02 — Controller boot, identity, and connectivity

**Setup:** Flash controller firmware; configure only staging/test WiFi and MQTT.

**Procedure:**
1. Cold boot.
2. Capture serial boot log.
3. Confirm WiFi association and MQTT connection.
4. Confirm controller reports the expected device identity/version/status.

**Pass:** Device reaches normal connected state without reboot loop and reports the expected non-production identity.

### H-03 — Physical telemetry → backend → frontend

**Setup:** Both physical nodes connected; staging backend/frontend running.

**Procedure:**
1. Generate a distinctive but safe sensor change.
2. Observe physical sensor MQTT publication.
3. Observe backend ingestion.
4. Observe frontend device/sensor state update.
5. Correlate by device ID and timestamp.

**Pass:** The same physical measurement is traceable from sensor node → MQTT → backend → frontend without manual data injection.

### H-04 — Configuration synchronization to physical controller

**Setup:** Controller connected; staging backend has a known baseline configuration revision.

**Procedure:**
1. Read current configuration/revision.
2. Apply one safe configuration mutation through the supported backend/frontend path.
3. Observe MQTT/controller receipt.
4. Observe controller runtime state or status reporting.
5. Confirm backend/frontend converge on the same revision/status.
6. Repeat with a second revision to prove the update is not a one-shot artifact.

**Pass:** The physical controller receives the canonical configuration, applies the expected safe value, and reports a revision/status that converges with backend/frontend state.

### H-05 — Actuator command with safe load

**Setup:** Actuator output connected to a safe observable load, not a hazardous crop/reservoir path.

**Procedure:**
1. Issue one supported command with the minimum safe duration/output.
2. Observe controller serial/event output.
3. Observe the physical output transition.
4. Confirm backend command/lifecycle state, where applicable.
5. Confirm output returns to safe/off state.

**Pass:** Command crosses backend → MQTT → controller → physical output and returns to the documented terminal state, with no duplicate or stuck output.

### H-06 — Safety stop / fault path

**Setup:** Safe actuator load; controlled fault stimulus available.

**Procedure:**
1. Start from a normal connected state.
2. Inject one documented sensor/data fault that the controller treats as unsafe (for example a sensor timeout or equivalent supported failure condition).
3. Observe controller fault state.
4. Verify all relevant physical outputs stop.
5. Verify backend/frontend expose the fault/status without inventing a healthy fallback.
6. Restore valid sensor input.
7. Verify recovery occurs only under the documented recovery condition.

**Pass:** Fault causes physical outputs to enter the safe state; recovery requires the expected valid condition; no automatic unsafe continuation occurs.

### H-07 — Configuration durability across restart

**Setup:** A safe configuration revision has been successfully applied to the physical controller.

**Procedure:**
1. Record applied revision/value.
2. Power-cycle the controller using the safe bench procedure.
3. Observe boot and reconnect.
4. Verify the durable configuration is restored/applied as documented.
5. Confirm backend/frontend status converges after reconnect.

**Pass:** Required durable configuration survives restart and synchronization does not regress to an unrelated/default revision.

### H-08 — Sensor/controller reconnect recovery

**Setup:** Both devices initially healthy and connected.

**Procedure:**
1. Temporarily interrupt the test network path for one device only.
2. Confirm the system enters the documented disconnected/stale state.
3. Restore the network.
4. Confirm MQTT reconnect and telemetry/status recovery.
5. Confirm no duplicate/stale command is replayed unexpectedly.

**Pass:** Reconnect is bounded and convergent; stale data is not presented as fresh; no unintended actuator command is generated solely by reconnect.

### H-09 — End-to-end safe dosing/mixing cycle (bench only)

**Setup:** Safe physical load or contained test fluid, emergency stop ready, all configured limits conservative.

**Procedure:**
1. Start from healthy Monitoring state.
2. Apply a controlled sensor stimulus that requests a bounded operation.
3. Observe controller FSM transition.
4. Observe physical actuator sequence.
5. Observe sensor telemetry during/after the operation.
6. Confirm the controller reaches the documented terminal phase and outputs stop.

**Pass:** The physical sequence matches the controller's documented FSM path and safety limits; no actuator remains active after the terminal state.

This test must be skipped if the bench cannot safely contain the operation. A skipped H-09 is a blocker for full Domain 11 verification, not a PASS.

### H-10 — Power-loss / recovery safety

**Setup:** Safe actuator load; configuration and device identity already provisioned.

**Procedure:**
1. Confirm normal connected state.
2. Ensure actuator is either off or at a safe test state.
3. Remove device power using the controlled power-cut procedure.
4. Restore power.
5. Verify boot, connectivity, durable state, and actuator safe-state behavior.

**Pass:** Power recovery does not leave an actuator unexpectedly energized and required durable state/reconnect behavior is preserved.

## 6. Cross-system HIL assertions

The HIL run must explicitly correlate these boundaries:

| Boundary | Physical evidence | Expected invariant |
|---|---|---|
| Sensor → MQTT | serial + MQTT capture | canonical `SensorData`, correct device ID |
| MQTT → backend | broker capture + backend log/API | accepted canonical payload, no fabricated fallback |
| Backend → frontend | API response + UI state | same device/revision/status |
| Backend → controller | command/config MQTT + serial | correct target identity and revision |
| Controller → actuator | serial/event + measured output | requested bounded output only |
| Fault → safety | injected fault + physical output | safe/off state |
| Restart → durability | before/after revision | required state survives restart |
| Reconnect → convergence | disconnect/reconnect traces | stale state clears; no unintended replay |

## 7. Evidence requirements

Create:

`docs/evidence/DOMAIN-11-PHYSICAL-HIL-E2E-001.json`

Each HIL case must contain:

- `test_id`;
- `status`: `PASS`, `FAIL`, or `BLOCKED`;
- exact command(s) executed and exit code(s), when command-based;
- physical device identifiers (non-secret IDs only);
- firmware versions;
- test timestamps;
- observed serial/MQTT/API/UI evidence references;
- expected result;
- actual result;
- operator/environment notes;
- blocker classification if not PASS.

Never place passwords, API keys, private certificates, WiFi passwords, HMAC secrets, or database passwords in the evidence file.

## 8. Acceptance criteria

- **AC-01:** A real ESP32-C3 sensor node boots and publishes canonical telemetry to the isolated/staging MQTT broker.
- **AC-02:** The physical sensor telemetry is accepted by the staging backend and reaches the frontend without manual injection.
- **AC-03:** A real ESP32-C3 controller boots with `source ~/export-esp.sh` used before controller build/check/flash commands and reaches normal connected state.
- **AC-04:** A supported configuration mutation reaches the physical controller and revision/status converges across controller, backend, and frontend.
- **AC-05:** At least one supported command crosses backend → MQTT → controller → physical output using a safe test load and returns to safe/off state.
- **AC-06:** A documented unsafe sensor/data fault causes the physical actuator outputs to stop and recovery requires the documented valid condition.
- **AC-07:** Required configuration/state survives a physical controller restart/power-cycle and re-converges with backend/frontend.
- **AC-08:** Device network interruption and reconnect recover without stale-as-fresh telemetry or unintended command replay.
- **AC-09:** A safe physical end-to-end dosing/mixing cycle executes and reaches the documented terminal state, or the case is explicitly BLOCKED because the bench cannot safely execute it.
- **AC-10:** Power-loss/recovery leaves physical actuators safe and preserves required durable state.
- **AC-11:** Every PASS has executable physical evidence; host/simulator/static evidence cannot satisfy a physical criterion by itself.
- **AC-12:** No production PostgreSQL, production MQTT, production credentials, or live crop/hazardous process is used.
- **AC-13:** HIL evidence is redacted and reproducible from the recorded setup/commands.
- **AC-14:** `git diff --check` passes and all pre-existing dirty worktree changes remain preserved.
- **AC-15:** Domain 11 does not start Domain 12 roadmap-wide evidence refresh or final release-readiness audit.

## 9. Failure classification

Use exactly one primary classification for every non-PASS case:

- `HARDWARE_UNAVAILABLE` — required physical board/device/load absent.
- `FLASH_OR_TOOLCHAIN_BLOCKED` — firmware cannot be flashed or started with the supported toolchain.
- `NETWORK_OR_BROKER_BLOCKED` — isolated/staging network or MQTT unavailable.
- `BACKEND_OR_DATABASE_BLOCKED` — staging backend/isolated DB unavailable or fails independently of physical execution.
- `FIRMWARE_RUNTIME_FAILURE` — physical firmware boots but violates the expected runtime contract.
- `CROSS_SYSTEM_CONTRACT_FAILURE` — physical path reaches a boundary with incompatible payload/state/revision semantics.
- `SAFETY_FAILURE` — actuator does not enter the required safe state; stop the run immediately and escalate.
- `TEST_BENCH_UNSAFE` — the required physical operation cannot be executed within the defined safety boundary.

A `BLOCKED` case is not a PASS and must not be hidden by a host/simulator substitute.

## 10. Non-goals

- No schema redesign.
- No new production feature.
- No production deployment.
- No production database migration.
- No uncontrolled actuator operation.
- No replacement of physical evidence with simulator evidence.
- No Domain 12 roadmap-wide evidence refresh.
- No unrelated refactor.
- No Domain 7 rework.

## 11. Execution order

1. Patch this spec before implementation.
2. Capture baseline worktree state.
3. Confirm isolated/staging backend, PostgreSQL, MQTT, and frontend endpoints.
4. Confirm physical boards, serial links, safe load, stimulus source, and emergency stop.
5. Source `~/export-esp.sh`; build/flash/controller-monitor the Rust controller.
6. Build/flash/monitor the C++ sensor node.
7. Execute H-01 through H-04 first; stop if identity/connectivity/canonical telemetry is broken.
8. Execute H-05 through H-08; stop immediately on any safety failure.
9. Execute H-09 only if the physical bench is explicitly safe for the bounded operation.
10. Execute H-10 power-loss/recovery.
11. Correlate physical serial, MQTT, backend, and frontend evidence.
12. Run `git diff --check` and verify dirty worktree preservation.
13. Create/update the Domain 11 evidence artifact.
14. Mark `COMPLETE / VERIFIED` only when every required physical acceptance criterion has executable evidence; otherwise mark `IMPLEMENTED / VERIFICATION BLOCKED` with exact blockers. Current execution has physical controller and sensor MQTT evidence, but remains blocked by the required isolated sensor-to-backend topology and by missing safe physical actuator/fault/power-cycle instrumentation; see the Domain 11 evidence artifact.

## 12. Current execution result

The Domain 11 specification was patched and execution subsequently reached both physical ESP32-C3 nodes. The controller was built/flashed after `source ~/export-esp.sh`, booted with `device_001`, joined the isolated LAN MQTT path, and produced physical status/command/fault observations. The sensor firmware was rebuilt with a reversible staging-only MQTT transport configuration, flashed to both OTA app partitions, booted on the physical sensor, joined staging WiFi at `192.168.1.7`, connected to the isolated MQTT broker at `192.168.1.9:1883`, and published canonical-shaped telemetry on `AGITECH/device_001/sensors`. A staging backend run then physically received those sensor messages, but InfluxDB writes returned HTTP 401, so canonical persistence is not fully proven. Full HIL remains `IMPLEMENTED / VERIFICATION BLOCKED` because production physical telemetry is not yet observed on the same broker path as the deployed backend, and the safe physical actuator/fault/restart/reconnect/power-cycle instrumentation required by H-05 through H-10 is not complete. The production browser was rechecked after the merged Domain-11 deployment: the selected device was authenticated and visible, `/config/unified` truthfully returned `device_config_missing`, `/sensors/latest` returned `503 dependency_unavailable`, and `/health-summary` remained permission-protected. A follow-up backend fix now backfills the minimal disabled/manual base config for pre-existing ownership rows at startup. No physical PASS is claimed where the spec requires physical output/recovery evidence.

## 13. Evidence provenance rule

A physical PASS requires an observation from the physical device boundary. The following are supporting evidence only:

- host unit/integration tests;
- simulator/HIL-like virtual tests;
- firmware compilation;
- static source inspection;
- CI success.

They may explain readiness or a blocker, but cannot be promoted to physical PASS.
