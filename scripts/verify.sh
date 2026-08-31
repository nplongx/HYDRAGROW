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

echo
echo "HYDRAGROW verification passed."
