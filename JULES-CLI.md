# HYDRAGROW — Jules CLI Guide

Bộ công cụ CLI để dispatch tasks tới [Google Jules](https://jules.google.com) — AI coding agent tích hợp GitHub.

---

## Cài đặt nhanh

```bash
# 1. Cài gh CLI
brew install gh        # macOS
# hoặc: https://cli.github.com/

# 2. Đăng nhập
gh auth login

# 3. Đảm bảo Jules có quyền với repo
# → jules.google.com → Connect GitHub repo → nplongx/HYDRAGROW

# 4. Cấp quyền script
chmod +x scripts/jules

# 5. Đảm bảo labels tồn tại trong repo
gh label create jules    --color "0075ca" --description "Jules AI agent task" --repo nplongx/HYDRAGROW
gh label create review   --color "e4e669" --description "Code review"          --repo nplongx/HYDRAGROW
gh label create audit    --color "d93f0b" --description "Code audit"            --repo nplongx/HYDRAGROW
gh label create finding  --color "ee0701" --description "Jules finding"         --repo nplongx/HYDRAGROW
gh label create hotfix   --color "b60205" --description "Urgent fix"            --repo nplongx/HYDRAGROW
gh label create tech-debt --color "c2e0c6" --description "Technical debt"       --repo nplongx/HYDRAGROW
```

---

## Cách Jules hoạt động

```
Developer                   GitHub                      Jules
    │                          │                          │
    │── scripts/jules ──────►  │  create issue            │
    │   review/audit/hotfix    │  label: "jules"          │
    │                          │◄─────────────────────────│
    │                          │  Jules picks up issue     │
    │                          │  (polls for jules label)  │
    │                          │                          │──► checkout branch
    │                          │                          │    read AGENTS.md
    │                          │                          │    run 3-phase
    │                          │◄─────────────────────────│
    │                          │  Jules creates PR         │
    │◄─────────────────────────│                          │
    │  PR notification          │                          │
    │── review PR ──────────►  │                          │
```

Jules nhận task thông qua **GitHub Issues có label `jules`**. Khi issue được tạo (bởi CLI hoặc tay), Jules:
1. Đọc `AGENTS.md` — nhận operational directives
2. Đọc `.agent/jules.yml` — nhận commands, workflows, restrictions
3. Thực hiện task theo 3-phase protocol
4. Tạo PR với verification output

---

## Commands

### `review` — Code review

```bash
# Review branch hiện tại vs main
./scripts/jules review

# Review PR cụ thể
./scripts/jules review --pr 42

# Review một subsystem cụ thể
./scripts/jules review --subsystem backend

# Review từ diff file
git diff main > /tmp/my.diff
./scripts/jules review --diff /tmp/my.diff
```

Jules sẽ:
- Chạy `cargo clippy`, `cargo fmt`, `cargo test`, `tsc`, `eslint`
- Kiểm tra module-rules compliance
- Comment review summary (Critical / Warning / Suggestion)
- Tạo sub-issue cho mỗi Critical/Warning

---

### `issue` — Tạo task issue cho Jules

```bash
# Bug report
./scripts/jules issue bug \
  --title "pH sensor reads 0.0 after 2h uptime" \
  --subsystem firmware-sensor

# Feature request
./scripts/jules issue feature \
  --title "Add WebSocket reconnect with exponential backoff" \
  --subsystem frontend

# Với mô tả đầy đủ từ file
./scripts/jules issue bug \
  --title "MQTT handler panic on malformed payload" \
  --subsystem backend \
  --body-file /tmp/bug-description.md

# Tech debt
./scripts/jules issue tech-debt \
  --title "Replace .unwrap() with proper error handling in simulator" \
  --subsystem simulator
```

---

### `audit` — Full subsystem audit

```bash
# Audit toàn bộ workspace
./scripts/jules audit

# Audit một subsystem
./scripts/jules audit --subsystem backend
./scripts/jules audit --subsystem shared    # quan trọng nhất — check MQTT contract
```

Jules sẽ kiểm tra toàn diện và tạo issues cho mọi vi phạm severity ≥ Warning.

---

### `hotfix` — Urgent fix (3-phase protocol)

```bash
./scripts/jules hotfix \
  --subsystem controller-core \
  --hint "dosing actor FSM transition"
# → Jules hỏi mô tả lỗi, tạo issue với priority cao
```

---

### `task` — Dispatch từ template markdown

```bash
# Dùng template có sẵn
./scripts/jules task .agent/tasks/example-feature-ws-reconnect.md

# Hoặc tạo task template của riêng bạn
cp .agent/tasks/hotfix.md .agent/tasks/my-task.md
# Sửa template...
./scripts/jules task .agent/tasks/my-task.md
```

---

### `status` — Xem Jules tasks đang chờ

```bash
./scripts/jules status
./scripts/jules status --limit 30
```

---

## Workflow điển hình

### Scenario 1: Review trước khi merge PR

```bash
# Mở PR xong, chạy:
./scripts/jules review --pr 55

# Jules tạo issue, bắt đầu review
# → Check status:
./scripts/jules status

# Jules comment kết quả vào issue
# Với finding Critical: Jules tự tạo sub-issue
# → Fix sub-issues xong → merge PR
```

### Scenario 2: Bug report từ production

```bash
./scripts/jules issue bug \
  --title "Backend crash: thread panicked at 'called Result::unwrap() on Err'" \
  --subsystem backend \
  --body-file /tmp/crash-log.txt

# Jules nhận issue, thực hiện 3-phase:
# 1. Discovery: trace unwrap() gây crash
# 2. Test: viết failing test reproduce
# 3. Fix: minimal fix, verify, open PR
```

### Scenario 3: Audit định kỳ (monthly)

```bash
# Chạy audit tất cả subsystem quan trọng
for sub in backend shared controller-core; do
  ./scripts/jules audit --subsystem "$sub"
  sleep 2  # tránh rate limit
done
```

### Scenario 4: CI tự động dispatch review

Thêm label `needs-review` vào PR → GitHub Actions (`jules-review.yml`) tự động tạo Jules review issue.

```bash
gh pr edit 42 --add-label "needs-review" --repo nplongx/HYDRAGROW
```

---

## File structure

```
.agent/
  jules.yml                        # Jules manifest: commands, workflows, restrictions
  tasks/
    review-pr.md                   # Template: PR code review
    audit-subsystem.md             # Template: subsystem audit
    hotfix.md                      # Template: urgent fix
    example-feature-ws-reconnect.md  # Ví dụ feature task hoàn chỉnh

scripts/
  jules                            # Main CLI (bash)
  build.sh                         # Existing
  test.sh                          # Existing
  verify.sh                        # Existing

.github/
  ISSUE_TEMPLATE/
    jules-task.yml                 # Issue template: dispatch task tới Jules
    jules-finding.yml              # Issue template: Jules findings (review/audit)
  workflows/
    jules-review.yml               # CI: auto-dispatch review khi PR ready
```

---

## Tips & Gotchas

**Jules không nhận task ngay lập tức** — thường mất 1-5 phút để Jules pick up issue mới.

**Abort condition**: Nếu Jules comment `ABORT_UNRESOLVABLE` → issue quá phức tạp cho Jules, cần fix thủ công.

**Restricted files**: Jules không được sửa `.github/`, `server_wallet.json`, `migrations/`. Xem `.agent/jules.yml`.

**Shared changes**: Bất cứ khi nào sửa `hydragrow-shared/`, chạy audit riêng:
```bash
./scripts/jules audit --subsystem shared
```

**Jules hallucination guard**: `AGENTS.md` §2 rule "READ-BEFORE-WRITE" — Jules phải inspect function signature trước khi edit. Nếu Jules tạo PR với compilation error, đó là vi phạm rule này → comment vào PR để Jules retry.
