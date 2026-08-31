#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "== Rust build =="
cargo build --workspace

echo "== Frontend build =="
cd "$ROOT/hydragrow-frontend"
npm run build
