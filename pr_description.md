## Requirement
- Requirement ID: HIFI-D1-D3-001
- Change class (`C0`-`C7`): C2

## Objective
Lấp 4 khoảng trống chức năng giữa Figma "Hi-Fi — Full App (v1)" (D1 Dashboard, D2 Điều khiển, D3 Tự động hóa) và code hiện tại: khoá chéo an toàn pH Up/Down, nút Dừng khẩn cấp toàn cục, banner xung đột lịch trình tự động hóa, và trạng thái gửi lệnh real-time + PWM inline.

## Acceptance Contract
- Acceptance contract: `docs/acceptance/HIFI-D1-D3-001.json`
- Evidence contract: `docs/evidence/HIFI-D1-D3-001.json`
- For C1-C7 changes, commit both contracts in the same PR.
- Schemas: `docs/schemas/acceptance-contract.schema.json`, `docs/schemas/evidence-contract.schema.json`

## Acceptance Criteria

| ID | Criterion | Target / Expected | Actual | Evidence |
|---|---|---|---|---|
| AC-1 | pH Up/Down interlock | Pass tests | PASS | docs/evidence/HIFI-D1-D3-001.json |
| AC-2 | Emergency stop command | Pass tests | PASS | docs/evidence/HIFI-D1-D3-001.json |
| AC-3 | Automation schedule conflict | Pass tests | PASS | docs/evidence/HIFI-D1-D3-001.json |
| AC-4 | Command status pill and PWM | Pass tests | PASS | docs/evidence/HIFI-D1-D3-001.json |

## Verification
- [x] Acceptance contract gate
- [x] Evidence contract gate
- [x] Delivery governance gate
- [x] Unit / integration tests

## Documentation
- [x] Required architecture/API/operations docs updated
- [x] `docs/project-state/CURRENT-STATUS.md` updated if project state changed
- [x] `docs/project-state/TRACEABILITY.md` updated for material requirements

## Risks / Known Gaps
None.

## Final Acceptance
- [x] Code verification passed
- [x] Acceptance criteria passed
- [x] Documentation synchronized
- [x] Project state synchronized
