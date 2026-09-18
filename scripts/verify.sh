#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== Rust format =="
for manifest in \
  hydragrow-shared/Cargo.toml \
  hydragrow-controller-core/Cargo.toml \
  hydragrow-simulator/Cargo.toml \
  hydragrow-backend/Cargo.toml \
  hydragrow-diagnostic-worker/Cargo.toml \
  hydragrow-supervisor-cli/Cargo.toml \
  hydragrow-supervisor-query/Cargo.toml \
  hydragrow-watchdog/Cargo.toml; do
  cargo fmt --manifest-path "$manifest" -- --check
done

echo "== Rust check =="
for manifest in \
  hydragrow-shared/Cargo.toml \
  hydragrow-controller-core/Cargo.toml \
  hydragrow-simulator/Cargo.toml \
  hydragrow-backend/Cargo.toml \
  hydragrow-diagnostic-worker/Cargo.toml \
  hydragrow-supervisor-cli/Cargo.toml \
  hydragrow-supervisor-query/Cargo.toml \
  hydragrow-watchdog/Cargo.toml; do
  cargo check --manifest-path "$manifest"
done

echo "== Rust tests =="
for manifest in \
  hydragrow-shared/Cargo.toml \
  hydragrow-controller-core/Cargo.toml \
  hydragrow-simulator/Cargo.toml \
  hydragrow-backend/Cargo.toml \
  hydragrow-diagnostic-worker/Cargo.toml \
  hydragrow-supervisor-cli/Cargo.toml \
  hydragrow-supervisor-query/Cargo.toml \
  hydragrow-watchdog/Cargo.toml; do
  cargo test --manifest-path "$manifest"
done

echo "== Frontend build =="
cd "$ROOT/hydragrow-frontend"
npm run build

echo "== Frontend lint =="
npm run lint

echo "== Frontend tests =="
npm test

echo "== Automation Design Tokens & Legacy Editor Check =="
cd "$ROOT"
if git grep -nE "bg-blue-|text-blue-|border-blue-|text-gray-|bg-gray-|slate-|Blockly|blockly/extractIr" \
    hydragrow-frontend/src/components/automation \
    hydragrow-frontend/src/pages/Automation.tsx \
    hydragrow-frontend/src/hooks/useFlowCanvas.ts; then
  echo "ERROR: Found forbidden legacy classes or Blockly references in automation files!"
  exit 1
fi

echo
echo "HYDRAGROW verification passed."
