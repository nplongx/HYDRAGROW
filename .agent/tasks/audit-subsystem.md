# [Jules Audit] {{SUBSYSTEM}} — {{DATE}}

> **Thay thế `{{SUBSYSTEM}}` (vd: `backend`) và `{{DATE}}` (vd: `2025-09`).**
> Dùng: `./scripts/jules audit --subsystem <name>` để tự điền.

## 🔍 Task type: Full Audit

Jules thực hiện audit toàn diện subsystem `{{SUBSYSTEM}}` và **tạo issue riêng** cho mỗi vi phạm severity ≥ Warning.

---

## Phase 1 — Discovery (viết KHÔNG code)

- [ ] Đọc module-rule: `docs/superpowers/specs/module-rules/{{SUBSYSTEM}}.md`
- [ ] List tất cả file trong subsystem
- [ ] Xác định entry points, public API boundaries

## Phase 2 — Static Analysis

```bash
# Rust
cd {{SUBSYSTEM_DIR}}
cargo fmt --all -- --check            # format compliance
cargo clippy --all-targets -- -D warnings  # lint
cargo test                            # test suite

# Frontend only
npx tsc --noEmit                      # type check
npx eslint .                          # lint
npx vitest run --coverage             # test + coverage
```

Paste **toàn bộ output** (kể cả khi pass) vào comment.

## Phase 3 — Deep Audit Checklist

### 🏗 Architecture compliance

**Layering rules** (theo module-rule)
- [ ] Handler → Service → Repository pattern được giữ đúng
- [ ] Không cross-layer call trực tiếp
- [ ] DB access chỉ qua `db/` module (backend)

**Shared contract**
- [ ] Mọi MQTT topic import từ `hydragrow_shared::topics` — grep: `"AGITECH/` không được xuất hiện ngoài shared
- [ ] Payload struct `SensorData`, `FsmSnapshot`, v.v. import từ `hydragrow_shared` — không redefine local
- [ ] Schema version được set đúng khi thay đổi payload

### 🔒 Safety & Security

- [ ] Không `unwrap()`/`.expect()` trong production path — `grep -r "\.unwrap()\|\.expect(" src/` (loại trừ `#[cfg(test)]`)
- [ ] Không hardcode credential — `grep -r "password\|secret\|api_key\|token" src/ --include="*.rs" -i`
- [ ] Auth middleware cover đủ route (backend)
- [ ] MQTT publish không leak internal state không nên public

### 🧪 Test quality

- [ ] Mọi `pub fn` trong business logic có ít nhất 1 test
- [ ] Test assertion explicit — không `assert!(true)`, không empty body
- [ ] Test name dạng `test_<behavior>_<condition>_<expected>` hoặc tương đương
- [ ] Không `#[ignore]` mà không có TODO + issue link

### 📦 Dependency hygiene

- [ ] Không circular dependency giữa modules
- [ ] Feature flag dùng đúng (không enable feature dư)
- [ ] `Cargo.toml`: version pin hợp lý, không `*` wildcard

### 🗃 Migration safety (backend only)

- [ ] Mọi migration có cả `up` và `down`
- [ ] Migration idempotent — có thể chạy lại an toàn
- [ ] Không DROP COLUMN/TABLE mà không có data migration plan

---

## Output format

Jules post comment với bảng:

```markdown
## Audit Report — {{SUBSYSTEM}} ({{DATE}})

### Summary

| Severity | Count |
|----------|-------|
| 🔴 Critical | N |
| 🟡 Warning  | N |
| 🟢 Info     | N |

### Findings

| # | Severity | File | Line | Rule | Description |
|---|----------|------|------|------|-------------|
| 1 | 🔴 | src/handler.rs | 42 | no-sql-in-handler | Raw SQL query in handler |
| 2 | 🟡 | src/service.rs | 78 | no-unwrap-prod | `.unwrap()` on non-test path |

### Static analysis output

\`\`\`
<cargo clippy output>
\`\`\`

### Recommended fix order

1. Fix Critical issues first (unblock functionality)
2. ...
```

**Mỗi Critical/Warning**: Jules tạo issue riêng với label `jules,bug` hoặc `jules,tech-debt`.

---

*Template: `.agent/tasks/audit-subsystem.md` | Dispatch: `./scripts/jules audit --subsystem {{SUBSYSTEM}}`*
