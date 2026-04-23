#!/usr/bin/env bash
set -euo pipefail

# Rule: aliases introduced in Rust/TS imports by `as <name>` should use
# camelCase/PascalCase and must not contain underscores.
pattern='^\s*(use|import)\b.*\bas\s+[A-Za-z0-9]*_[A-Za-z0-9_]*\b'

matches=$(rg --glob '!ESP32-C3-SENSOR-NODE/libraries/**' --glob '!**/node_modules/**' --glob '!**/target/**' \
  --glob '*.rs' --glob '*.ts' --glob '*.tsx' \
  -n "$pattern" . || true)

count=$(printf "%s" "$matches" | sed '/^$/d' | wc -l | tr -d ' ')

if [[ "${1:-}" == "--count" ]]; then
  echo "$count"
  exit 0
fi

if [[ "$count" -gt 0 ]]; then
  echo "Found $count alias naming violation(s). Use camelCase/PascalCase aliases without underscores."
  echo "$matches"
  exit 1
fi

echo "Alias naming check passed."
