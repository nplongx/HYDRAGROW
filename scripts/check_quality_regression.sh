#!/usr/bin/env bash
set -euo pipefail

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
