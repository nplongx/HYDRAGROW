# P2.5 Firmware Canonical Contract Pipeline

**Status:** PROPOSED
**Phase:** P2.5
**Depends on:** P1.9 schema alignment, canonical registry, existing firmware fixture work

## Goal

Make the canonical schema registry executable against all supported firmware implementations and explicitly define C++/Rust sensor ownership.

## Supported consumers

```text
ESP32-C3-CONTROLLER-NODE
ESP32-C3-SENSOR-NODE
ESP32-C3-SENSOR-NODE-RUST
hydragrow-shared
hydragrow-backend
hydragrow-simulator
```

## Rules

- `schema/contracts/registry.json` is the only canonical wire contract.
- Fixtures are test inputs, not a new runtime protocol.
- Firmware tests MUST validate exact topic/payload compatibility for supported contracts.
- Host-only fixture validation is not physical HIL.
- If both C++ and Rust sensor nodes remain supported, both are independently validated.
- If one becomes successor/legacy, the repository MUST record that ownership explicitly before removing CI coverage.

## Acceptance criteria

- **AC-1:** Controller command/status fixtures validate in the controller firmware test pipeline.
- **AC-2:** C++ sensor payload fixtures validate in the C++ sensor pipeline.
- **AC-3:** Rust sensor payload fixtures validate in the Rust sensor pipeline.
- **AC-4:** Shared registry tests and firmware fixture tests fail on incompatible field/topic changes.
- **AC-5:** Firmware material changes trigger the correct acceptance/evidence/schema governance checks.
- **AC-6:** No firmware CI job claims physical hardware verification when only host/native tests ran.
- **AC-7:** Production-vs-successor ownership of C++/Rust sensor implementations is documented.
