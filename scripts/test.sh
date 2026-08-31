#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT"
echo "== Rust workspace tests =="
cargo test --workspace

echo
echo "== Frontend tests =="
cd "$ROOT/hydragrow-frontend"

if [[ ! -d node_modules ]]; then
    echo "ERROR: hydragrow-frontend/node_modules is missing."
    echo "Run: npm ci"
    exit 1
fi

npm test

echo
echo "Tests passed."
