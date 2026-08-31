#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== Rust workspace build =="
cargo build --workspace

echo
echo "== Frontend build =="
cd "$ROOT/hydragrow-frontend"
npm run build

echo
echo "Build passed."
