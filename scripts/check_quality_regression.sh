#!/usr/bin/env bash
set -euo pipefail

<<<<<<< codex/start-4-week-roadmap-execution-6lx4kz
baseline_file="${1:-docs/process/baseline-lock.txt}"

if [[ ! -f "$baseline_file" ]]; then
  echo "Baseline lock file not found: $baseline_file"
  echo "Run: ./scripts/update_quality_baseline_lock.sh"
  exit 2
fi

tmp_current=$(mktemp)
tmp_diff=$(mktemp)
trap 'rm -f "$tmp_current" "$tmp_diff"' EXIT

./scripts/update_quality_baseline_lock.sh "$tmp_current" >/dev/null

grep -v '^#' "$baseline_file" | sed '/^$/d' | sort > "${tmp_diff}.baseline"
grep -v '^#' "$tmp_current" | sed '/^$/d' | sort > "${tmp_diff}.current"

comm -13 "${tmp_diff}.baseline" "${tmp_diff}.current" > "$tmp_diff"

new_count=$(sed '/^$/d' "$tmp_diff" | wc -l | tr -d ' ')

if [[ "$new_count" -gt 0 ]]; then
  echo "Regression detected: $new_count new quality violation(s) not present in baseline lock"
  echo "--- New violations ---"
  cat "$tmp_diff"
  exit 1
fi

baseline_count=$(wc -l < "${tmp_diff}.baseline" | tr -d ' ')
current_count=$(wc -l < "${tmp_diff}.current" | tr -d ' ')

echo "Baseline lock check passed"
echo "- baseline entries: $baseline_count"
echo "- current entries: $current_count"
echo "- new entries: 0"
=======
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
>>>>>>> main
