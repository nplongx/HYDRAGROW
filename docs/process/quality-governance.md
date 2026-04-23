# Quality governance baseline

This document defines the baseline controls to reduce technical inconsistency progressively by milestone.

## 1) Pre-commit gate before push

- Standard hooks are defined in `.pre-commit-config.yaml`.
- A local Git pre-push hook is provided at `.githooks/pre-push`.
- Setup (one-time per clone):

```bash
git config core.hooksPath .githooks
pipx install pre-commit
pre-commit install
```

### Current gates
- Basic hygiene: merge conflict markers, trailing whitespace, EOF, YAML/JSON/TOML validation.
- CI warning-only gate cho lộ trình tuần 2 được cấu hình tại `.github/workflows/quality-roadmap.yml` (không chặn merge, chỉ hiển thị cảnh báo).
- Baseline lock script cho tuần 3: `scripts/check_quality_regression.sh` với baseline lock snapshot tại `docs/process/baseline-lock.txt` (enforce mặc định trên pull request).
- Rust formatting checks for:
  - `hydragrow-backend`
  - `ESP32-C3-CONTROLLER-NODE`
- Alias naming convention check via `scripts/check_alias_naming.sh`.

## 2) CODEOWNERS + reviewer checklist for critical areas

- Ownership rules are declared in `.github/CODEOWNERS`.
- Review checklist is enforced through `.github/pull_request_template.md`.

Critical areas currently covered:
- Backend API/service/db/migrations
- Controller firmware source
- Frontend control/context source
- Governance tooling (`.github`, hooks, process docs)

## 3) Weekly/Monthly reporting

Use `scripts/generate_quality_report.sh` to generate periodic reports.

```bash
./scripts/generate_quality_report.sh weekly
./scripts/generate_quality_report.sh monthly
```

Reports are generated under `docs/process/reports/`.

### Required KPIs
- Number of lint warnings (backend + controller).
- Number of alias naming violations.
- Technical inconsistency trend (currently proxied by `TODO(tech-debt)` and `FIXME` markers).

## 4) Milestone targets (decreasing inconsistency)

- **M1**: Reduce total lint warnings by at least **20%** from baseline.
- **M2**: Reduce alias naming violations by at least **50%** from baseline.
- **M3**: Keep newly introduced technical inconsistency markers at **0 per sprint**.

If a milestone misses target, define a corrective action owner and date in the next report.
