#!/usr/bin/env bash
set -euo pipefail

output="${1:-docs/process/baseline-lock.txt}"

alias_pattern='^\s*(use|import)\b.*\bas\s+[A-Za-z0-9]*_[A-Za-z0-9_]*\b'

alias_matches=$(rg --glob '!ESP32-C3-SENSOR-NODE/libraries/**' --glob '!**/node_modules/**' --glob '!**/target/**' \
  --glob '*.rs' --glob '*.ts' --glob '*.tsx' -n "$alias_pattern" . || true)

tech_debt_matches=$(rg --glob '!ESP32-C3-SENSOR-NODE/libraries/**' --glob '!**/node_modules/**' --glob '!**/target/**' \
  --glob "*.rs" --glob "*.ts" --glob "*.tsx" --glob "*.js" --glob "*.jsx" --glob "*.c" --glob "*.cpp" --glob "*.h" --glob "*.hpp" --glob "*.py" --glob "*.sh" \
  -n "TODO\\(tech-debt\\)|FIXME" . || true)

{
  echo "# quality baseline lock snapshot"
  echo "# generated_at_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# format: TYPE|path:line:content"

  if [[ -n "$alias_matches" ]]; then
    printf '%s\n' "$alias_matches" | sed 's/^/ALIAS|/' | sort
  fi

  if [[ -n "$tech_debt_matches" ]]; then
    printf '%s\n' "$tech_debt_matches" | sed 's/^/DEBT|/' | sort
  fi
} > "$output"

echo "Updated baseline lock file: $output"
