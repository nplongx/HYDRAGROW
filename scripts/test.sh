#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== Rust tests =="
cargo test --workspace

echo "== Frontend tests =="
cd "$ROOT/hydragrow-frontend"
npm test
