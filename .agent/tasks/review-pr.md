# [Jules Review] PR #{{PR_NUMBER}} — {{PR_TITLE}}

> **Thay thế `{{PR_NUMBER}}` và `{{PR_TITLE}}` trước khi dispatch.**
> Dùng: `./scripts/jules review --pr <number>` để tự điền.

## 🤖 Task type: Code Review

Jules thực hiện review PR này theo **3-phase protocol** và tạo sub-issues cho mọi phát hiện nghiêm trọng.

---

## Phase 1 — Discovery (viết KHÔNG code)

- [ ] Checkout PR branch: `gh pr checkout {{PR_NUMBER}}`
- [ ] Đọc module-rule cho mọi subsystem bị chạm:
  - Backend: `docs/superpowers/specs/module-rules/backend.md`
  - Shared: `docs/superpowers/specs/module-rules/shared.md`
  - Frontend: `docs/superpowers/specs/module-rules/frontend.md`
  - Controller: `docs/superpowers/specs/module-rules/controller-core.md`
  - Simulator: `docs/superpowers/specs/module-rules/simulator.md`
- [ ] List tất cả file thay đổi: `git diff main...HEAD --name-only`
- [ ] Xác định subsystem(s) bị chạm

## Phase 2 — Static Analysis

Chạy từng lệnh, paste output đầy đủ vào comment:

```bash
# Rust subsystems (chạy với từng subsystem bị chạm)
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace

# Frontend (nếu hydragrow-frontend/ bị chạm)
cd hydragrow-frontend
npx tsc --noEmit
npx eslint .
npx vitest run
```

## Phase 3 — Review Findings

### Checklist bắt buộc

**Module-rules compliance**
- [ ] SQL/DB logic chỉ trong `db/` module — không trong handler
- [ ] MQTT topic constant lấy từ `hydragrow-shared` — không hardcode string
- [ ] Shared type thay đổi → cập nhật đồng thời tất cả consumer trong cùng PR
- [ ] Không `unwrap()`/`.expect()` trên production code path

**Code quality**
- [ ] Không dead code, unused import
- [ ] Error handling đúng: dùng `?` operator hoặc match, không ignore
- [ ] Naming nhất quán với convention hiện tại

**Security**
- [ ] Không hardcode secret, API key, credential
- [ ] Auth middleware áp dụng đúng cho mọi protected route
- [ ] Input validation trước khi write DB

**Test quality**
- [ ] Logic mới có unit test với assertion rõ ràng (không empty body, không tautology)
- [ ] Test name mô tả behavior, không chỉ mô tả code

---

## Output format

Jules post comment trên issue này với cấu trúc sau:

```markdown
## Review Summary — PR #{{PR_NUMBER}}

### 🔴 Critical (block merge)
| File | Line | Issue |
|------|------|-------|
| ... | ... | ... |

### 🟡 Warning (should fix)
| File | Line | Issue |
|------|------|-------|

### 🟢 Suggestion (optional)
- ...

### Verification
| Check | Command | Exit | Notes |
|-------|---------|------|-------|
| cargo fmt   | `cargo fmt --all -- --check`    | 0 ✓ | |
| clippy      | `cargo clippy -- -D warnings`   | 0 ✓ | |
| cargo test  | `cargo test --workspace`        | 0 ✓ | |
| tsc         | `npx tsc --noEmit`              | 0 ✓ | |
| eslint      | `npx eslint .`                  | 0 ✓ | |
| vitest      | `npx vitest run`                | 0 ✓ | |

### Verdict
- [ ] LGTM — có thể merge
- [ ] Needs changes — xem Critical/Warning ở trên
```

**Với mỗi Critical/Warning**: Jules tạo 1 issue riêng dùng template `jules-finding`.

---

*Template: `.agent/tasks/review-pr.md` | Dispatch: `./scripts/jules review --pr {{PR_NUMBER}}`*
