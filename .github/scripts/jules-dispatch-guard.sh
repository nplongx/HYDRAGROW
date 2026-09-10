#!/usr/bin/env bash
set -euo pipefail

MAX_JULES_SESSIONS="${MAX_JULES_SESSIONS:-15}"

active="$(juleson sessions list --state QUEUED,RUNNING --json state 2>/dev/null \
  | jq '[.[] | select(.state == "QUEUED" or .state == "RUNNING")] | length' || echo 0)"

echo "Active Jules sessions: ${active}/${MAX_JULES_SESSIONS}"

if [ "${active}" -ge "${MAX_JULES_SESSIONS}" ]; then
  echo "Jules capacity exhausted"
  exit 78
fi

echo "Jules slot available"
