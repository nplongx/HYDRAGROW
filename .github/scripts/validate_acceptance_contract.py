#!/usr/bin/env python3
"""Validate HYDRAGROW machine-readable acceptance contracts without third-party deps."""
import json
import pathlib
import sys


def fail(message: str) -> None:
    print(f"Acceptance contract invalid: {message}")
    raise SystemExit(1)


path = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else None
if path is None:
    fail("contract path argument is required")
if not path.is_file():
    fail(f"file not found: {path}")

try:
    data = json.loads(path.read_text(encoding="utf-8"))
except json.JSONDecodeError as exc:
    fail(f"invalid JSON: {exc}")

if not isinstance(data, dict):
    fail("root must be an object")
for key in ("requirement_id", "objective", "acceptance"):
    if key not in data:
        fail(f"missing required field '{key}'")
if not isinstance(data["requirement_id"], str) or not data["requirement_id"].strip():
    fail("requirement_id must be a non-empty string")
if not isinstance(data["objective"], str) or not data["objective"].strip():
    fail("objective must be a non-empty string")

acceptance = data["acceptance"]
if not isinstance(acceptance, list) or not acceptance:
    fail("acceptance must be a non-empty array")

allowed_ops = {"=", "!=", "<", "<=", ">", ">=", "contains", "exists"}
ids = set()
for item in acceptance:
    if not isinstance(item, dict):
        fail("each acceptance item must be an object")
    for key in ("id", "criterion", "verification"):
        if key not in item:
            fail(f"acceptance item missing '{key}'")
    item_id = item["id"]
    if not isinstance(item_id, str) or not item_id.startswith("AC-") or not item_id[3:].isdigit():
        fail(f"invalid acceptance id: {item_id!r}")
    if item_id in ids:
        fail(f"duplicate acceptance id: {item_id}")
    ids.add(item_id)
    if not isinstance(item["criterion"], str) or not item["criterion"].strip():
        fail(f"{item_id}: criterion must be non-empty")
    if not isinstance(item["verification"], str) or not item["verification"].strip():
        fail(f"{item_id}: verification must be non-empty")
    if "metric" in item:
        for key in ("operator", "target", "unit"):
            if key not in item:
                fail(f"{item_id}: quantitative acceptance requires '{key}'")
        if item["operator"] not in allowed_ops:
            fail(f"{item_id}: unsupported operator {item['operator']!r}")
    if "evidence_required" in item and not isinstance(item["evidence_required"], bool):
        fail(f"{item_id}: evidence_required must be boolean")

print(f"Acceptance contract valid: {path}")
print(f"Requirement: {data['requirement_id']}")
print(f"Acceptance criteria: {len(acceptance)}")
