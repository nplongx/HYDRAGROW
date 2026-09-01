#!/usr/bin/env python3
import json, sys
from pathlib import Path

p = Path(sys.argv[1])
data = json.loads(p.read_text())
required = {"requirement_id", "plan", "tasks", "status"}
missing = required - data.keys()
if missing:
    raise SystemExit(f"missing required fields: {sorted(missing)}")
if data["status"] not in {"PLANNED", "IN_PROGRESS", "BLOCKED", "VERIFIED", "ACCEPTED"}:
    raise SystemExit(f"invalid plan status: {data['status']}")
if not data["tasks"]:
    raise SystemExit("plan execution record must contain at least one task")
allowed = {"NOT_STARTED", "IN_PROGRESS", "BLOCKED", "DONE"}
for task in data["tasks"]:
    for key in ("id", "title", "status"):
        if key not in task:
            raise SystemExit(f"task missing {key}")
    if task["status"] not in allowed:
        raise SystemExit(f"invalid task status for {task['id']}: {task['status']}")
    if task["status"] == "DONE" and not task.get("evidence", "").strip():
        raise SystemExit(f"DONE task has no evidence: {task['id']}")
if data["status"] in {"VERIFIED", "ACCEPTED"}:
    incomplete = [t["id"] for t in data["tasks"] if t["status"] != "DONE"]
    if incomplete:
        raise SystemExit(f"plan marked {data['status']} with incomplete tasks: {incomplete}")
print(f"Plan execution record valid: {p}")
