#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "========================================"
echo " HYDRAGROW Verification"
echo "========================================"

echo
echo "== Rust format =="
cargo fmt --all -- --check

echo
echo "== Rust check =="
cargo check --workspace

echo
echo "== Rust tests =="
cargo test --workspace

echo
echo "== Frontend build =="
cd "$ROOT/hydragrow-frontend"

if [[ ! -d node_modules ]]; then
    echo "ERROR: hydragrow-frontend/node_modules is missing."
    echo "Run: npm ci"
    exit 1
fi

npm run build

echo
echo "== Frontend lint =="
npm run lint

echo
echo "== Frontend tests =="
npm test

echo
echo "========================================"
echo " HYDRAGROW VERIFICATION PASSED"
echo "========================================"
