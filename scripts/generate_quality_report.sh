#!/usr/bin/env bash
set -euo pipefail

period="${1:-weekly}"
output="${2:-docs/process/reports/${period}-$(date -u +%Y-%m-%d).md}"

mkdir -p "$(dirname "$output")"

run_and_count_warnings() {
  local cmd="$1"
  local tmp
  tmp=$(mktemp)

  set +e
  timeout 30 bash -lc "$cmd" >"$tmp" 2>&1
  local status=$?
  set -e

  if [[ $status -eq 124 ]]; then
    echo "NA(timeout)"
  elif [[ $status -eq 0 ]]; then
    rg -c "warning:" "$tmp" || true
  else
    if rg -q "command not found|not installed|No such file or directory" "$tmp"; then
      echo "NA(missing-tool)"
    else
      rg -c "warning:" "$tmp" || true
    fi
  fi

  rm -f "$tmp"
}

backend_lint=$(run_and_count_warnings "cd hydragrow-backend && cargo clippy --all-targets --all-features")
controller_lint=$(run_and_count_warnings "cd ESP32-C3-CONTROLLER-NODE && cargo clippy --all-targets --all-features")
alias_count=$(./scripts/check_alias_naming.sh --count)
tech_debt_count=$(rg --glob '!ESP32-C3-SENSOR-NODE/libraries/**' --glob '!**/node_modules/**' --glob '!**/target/**' -n "TODO\\(tech-debt\\)|FIXME" . | wc -l | tr -d ' ')

cat >"$output" <<REPORT
# ${period^} quality report - $(date -u +%Y-%m-%d)

## KPI snapshot
- Backend lint warnings: ${backend_lint}
- Controller lint warnings: ${controller_lint}
- Alias naming violations: ${alias_count}
- Technical inconsistency markers (TODO(tech-debt)/FIXME): ${tech_debt_count}

## Trend vs previous report
- Lint: _fill in delta_
- Alias naming: _fill in delta_
- Technical inconsistency: _fill in delta_

## Milestone targets
- M1: Reduce total lint warnings by >= 20%.
- M2: Reduce alias naming violations by >= 50% from baseline.
- M3: Keep new technical inconsistency markers at 0 per sprint.

## Actions
- _owner / action / due date_
REPORT

echo "Generated report: $output"
