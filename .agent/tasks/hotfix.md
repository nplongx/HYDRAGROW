# [Jules Hotfix] {{SUBSYSTEM}}: {{DESCRIPTION}}

> Dùng: `./scripts/jules hotfix --subsystem <name> --hint "<gợi ý>"` để tự điền.

## 🚨 Task type: Hotfix — 3-Phase Protocol bắt buộc

**Subsystem:** `{{SUBSYSTEM}}`
**Lỗi:** {{DESCRIPTION}}
**Gợi ý vị trí:** {{HINT}}

> Jules PHẢI theo đúng 3 phase. Không được bỏ qua Phase 1 để viết code ngay.

---

## Phase 1 — Discovery (viết KHÔNG code)

Jules cần hoàn thành checklist này trước khi viết bất kỳ dòng code nào:

- [ ] Grep symbol liên quan: `grep -r "{{HINT}}" --include="*.rs" -n`
- [ ] Trace call graph từ entry point đến nơi lỗi xảy ra
- [ ] List file sẽ thay đổi (tối đa, không expand scope)
- [ ] Comment trên issue: `Phase 1 complete — root cause: <mô tả>`

**Abort nếu Phase 1 không xác định được root cause** — comment và đóng issue.

## Phase 2 — Oracle & Failing Test

- [ ] Viết test reproduce lỗi trước khi fix:

```rust
// Ví dụ cho Rust subsystem
#[test]
fn test_{{SNAKE_DESCRIPTION}}_reproduces_bug() {
    // setup: tạo state trigger lỗi
    // act: thực hiện hành động gây lỗi
    // assert: expected behavior (sẽ FAIL trước khi fix)
    assert_eq!(actual, expected);
}
```

- [ ] Chạy test, xác nhận FAIL với **đúng lý do** (không phải compile error):

```bash
cargo test test_{{SNAKE_DESCRIPTION}} -- --nocapture
```

Expected: `FAILED` với message liên quan đến bug, không phải `error[E...]: ...`

- [ ] Commit test riêng: `git commit -m "test({{SUBSYSTEM}}): reproduce {{DESCRIPTION}}"`

## Phase 3 — Surgical Fix & Verify

- [ ] Implement fix **tối thiểu** — không refactor ngoài phạm vi
- [ ] Chạy failing test → phải PASS:

```bash
# Chạy test vừa viết
cargo test test_{{SNAKE_DESCRIPTION}}

# Chạy full suite — không được có regression
{{TEST_CMD}}
```

- [ ] Chạy full verification:

```bash
{{VERIFY_CMD}}
```

- [ ] **Không weakening**: không xóa assertion, không comment out check, không `#[ignore]`
- [ ] Commit: `git commit -m "fix({{SUBSYSTEM}}): {{DESCRIPTION}}"`

### Abort condition

Nếu test vẫn FAIL sau **4 lần thử** khác nhau:

```xml
<status>ABORT_UNRESOLVABLE</status>
<reason>4 attempts failed: <mô tả từng attempt></reason>
```

Comment vào issue và **đóng mà không merge**.

---

## PR description template (Jules điền khi tạo PR)

```markdown
## Fix: {{DESCRIPTION}}

**Root cause:** <Jules điền từ Phase 1>

**Approach:** <mô tả fix ngắn gọn>

**Verification:**
```
{{VERIFY_CMD}}
```

Exit code: 0 ✓

**Test added:** `test_{{SNAKE_DESCRIPTION}}` in `<file>`

Closes #<issue-number>
```

---

*Template: `.agent/tasks/hotfix.md` | Dispatch: `./scripts/jules hotfix`*
