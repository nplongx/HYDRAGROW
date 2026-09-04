#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== Rust format =="
cargo fmt --all -- --check

echo "== Rust check =="
cargo check --workspace

echo "== Rust tests =="
cargo test --workspace

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
