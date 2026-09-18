#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

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

echo "== Frontend tests =="
cd "$ROOT/hydragrow-frontend"
npm test
