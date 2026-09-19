#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

test -f AGENTS.md
test -f PLANS.md
test -f GOALS.md
test -f PROMPTS.md
test -f .agent/verify.yml
test -f docs/DELIVERY-GOVERNANCE.md
test -f harness/build/02-frontend-migration.md
test -f harness/build/03-hardening.md

grep -q 'repository-local harness' AGENTS.md
grep -q 'P3.2 — Frontend migration' PLANS.md
grep -q 'harness/build/' AGENTS.md

echo 'HYDRAGROW harness structural checks: PASS'
