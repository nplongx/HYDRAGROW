#!/usr/bin/env bash
set -euo pipefail

baseline_file="${1:-docs/process/baseline-metrics.env}"

if [[ ! -f "$baseline_file" ]]; then
  echo "Baseline file not found: $baseline_file"
  exit 2
fi

# shellcheck disable=SC1090
source "$baseline_file"

current_alias=$(./scripts/check_alias_naming.sh --count)
current_tech_debt=$(rg --glob '!ESP32-C3-SENSOR-NODE/libraries/**' --glob '!**/node_modules/**' --glob '!**/target/**' -n "TODO\\(tech-debt\\)|FIXME" . | wc -l | tr -d ' ')

status=0

if [[ "$current_alias" -gt "${BASELINE_ALIAS_VIOLATIONS:-0}" ]]; then
  echo "Regression: alias naming violations increased (${BASELINE_ALIAS_VIOLATIONS} -> ${current_alias})"
  status=1
fi

if [[ "$current_tech_debt" -gt "${BASELINE_TECH_DEBT_MARKERS:-0}" ]]; then
  echo "Regression: tech-debt markers increased (${BASELINE_TECH_DEBT_MARKERS} -> ${current_tech_debt})"
  status=1
fi

echo "Baseline check summary"
echo "- alias violations: baseline=${BASELINE_ALIAS_VIOLATIONS:-0}, current=${current_alias}"
echo "- tech-debt markers: baseline=${BASELINE_TECH_DEBT_MARKERS:-0}, current=${current_tech_debt}"

exit "$status"
