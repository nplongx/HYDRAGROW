#!/usr/bin/env python3
"""Validate evidence against an acceptance contract using only the Python stdlib."""
import json
import operator
import pathlib
import sys

OPS = {
    "=": operator.eq,
    "!=": operator.ne,
    "<": operator.lt,
    "<=": operator.le,
    ">": operator.gt,
    ">=": operator.ge,
}


def fail(message: str) -> None:
    print(f"Evidence contract invalid: {message}")
    raise SystemExit(1)


def load(path: pathlib.Path):
    if not path.is_file():
        fail(f"file not found: {path}")
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        fail(f"invalid JSON in {path}: {exc}")


def numeric(value, label):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        fail(f"{label} must be numeric for quantitative acceptance")
    return value


if len(sys.argv) != 3:
    fail("usage: validate_evidence_contract.py <acceptance.json> <evidence.json>")

acceptance = load(pathlib.Path(sys.argv[1]))
evidence = load(pathlib.Path(sys.argv[2]))

if not isinstance(acceptance, dict) or not isinstance(evidence, dict):
    fail("both contracts must have an object root")
if acceptance.get("requirement_id") != evidence.get("requirement_id"):
    fail("requirement_id does not match")

criteria = {item["id"]: item for item in acceptance.get("acceptance", [])}
entries = evidence.get("evidence")
if not isinstance(entries, list) or not entries:
    fail("evidence must be a non-empty array")

seen = set()
for item in entries:
    aid = item.get("acceptance_id")
    if aid not in criteria:
        fail(f"evidence references unknown acceptance id: {aid!r}")
    if aid in seen:
        fail(f"duplicate evidence for {aid}")
    seen.add(aid)
    status = item.get("status")
    if status not in {"PASS", "FAIL", "BLOCKED"}:
        fail(f"{aid}: invalid status")
    criterion = criteria[aid]
    if status != "PASS":
        continue
    if criterion.get("evidence_required", True) and not item.get("source"):
        fail(f"{aid}: PASS requires a source")
    if "metric" in criterion:
        actual = numeric(item.get("actual"), f"{aid}.actual")
        target = numeric(criterion.get("target"), f"{aid}.target")
        op = criterion.get("operator")
        if op not in OPS:
            fail(f"{aid}: unsupported quantitative operator {op!r}")
        expected_unit = criterion.get("unit")
        if expected_unit and item.get("unit") != expected_unit:
            fail(f"{aid}: evidence unit {item.get('unit')!r} does not match {expected_unit!r}")
        if not OPS[op](actual, target):
            fail(f"{aid}: actual {actual!r} does not satisfy {op} target {target!r}")

missing = set(criteria) - seen
required_missing = [aid for aid in missing if criteria[aid].get("evidence_required", True)]
if required_missing:
    fail(f"missing evidence for required criteria: {', '.join(sorted(required_missing))}")

failed = [item["acceptance_id"] for item in entries if item.get("status") != "PASS"]
if failed:
    fail(f"acceptance criteria not fully passed: {', '.join(sorted(failed))}")

print("Evidence contract PASS")
print(f"Requirement: {acceptance['requirement_id']}")
print(f"Verified criteria: {len(seen)}")
