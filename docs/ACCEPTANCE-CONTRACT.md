# Machine-readable Acceptance Contract

Each material feature or system behavior change should include a machine-readable acceptance contract at the repository path declared by the PR.

## Contract

```yaml
requirement_id: REQ-123
objective: "Restore telemetry after a network interruption"
acceptance:
  - id: AC-1
    criterion: "Device reconnects within the target window"
    metric: reconnect_time
    operator: "<="
    target: 10
    unit: seconds
    verification: integration
    evidence_required: true
  - id: AC-2
    criterion: "Packet loss remains below the allowed threshold"
    metric: packet_loss
    operator: "<"
    target: 1
    unit: percent
    verification: integration
    evidence_required: true
```

## Rules

- `requirement_id` must identify the requirement/issue being delivered.
- Every acceptance item must have a stable `AC-N` identifier.
- Quantitative acceptance must declare a metric, operator, target, and unit.
- `evidence_required: true` means the PR must provide reproducible evidence before delivery can be accepted.
- The contract describes what must be true; tests and deployment artifacts prove whether it is true.
- Do not use vague targets such as `fast`, `good`, or `works` for quantitative requirements.

The JSON Schema is `docs/schemas/acceptance-contract.schema.json`.
