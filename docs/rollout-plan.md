# Rollout kế hoạch theo module ưu tiên

Tài liệu này chuẩn hóa cách tách rollout thành các PR nhỏ, mỗi PR chỉ xử lý **1 phạm vi rõ ràng**.

## Lộ trình 4 tuần (execution plan)

### Tuần 1 — Chốt standards + glossary + baseline

**Mục tiêu**
- Chốt phiên bản chính thức cho:
  - `docs/engineering/code-standards.md`
  - `docs/engineering/domain-glossary.md`
  - `docs/engineering/quality-baseline.md`
- Freeze phạm vi thay đổi chuẩn trong kỳ (tránh đổi luật giữa chừng).

**Đầu việc chính**
- Review chéo giữa BE/FE/Firmware để thống nhất thuật ngữ domain và naming.
- Chụp baseline KPI đầu kỳ (lint warnings, alias violations, tech-debt markers).
- Tạo danh sách P0 cần sửa ngay ở tuần 2.

**Definition of Done (DoD)**
- Bộ standards + glossary + baseline được approve và công bố là SSOT.
- Có báo cáo baseline với số liệu đo được, làm mốc so sánh cho tuần sau.

---

### Tuần 2 — Bật formatter/linter ở chế độ cảnh báo, sửa nhóm P0

**Mục tiêu**
- Kích hoạt formatter/linter trong CI ở chế độ **warning-only** (không fail build).
- Hoàn thành remediation cho nhóm vi phạm **P0** đã chốt.

**Đầu việc chính**
- Bổ sung/chuẩn hóa script lint/format cho từng khối (backend/frontend/firmware).
- Tạo dashboard hoặc báo cáo tuần hiển thị warning theo module.
- Ưu tiên fix các lỗi gây bất nhất contract/naming xuyên module.

**Definition of Done (DoD)**
- CI hiển thị warning ổn định cho toàn repo.
- Backlog P0 giảm về 0 hoặc có owner + deadline cụ thể cho phần tồn.

---

### Tuần 3 — Migrate module lõi theo PR nhỏ, bật CI fail cho vi phạm mới

**Mục tiêu**
- Refactor/migrate các module lõi theo từng PR nhỏ, độc lập.
- Chuyển CI từ warning sang **fail-on-new-violations**.

**Đầu việc chính**
- Thực hiện migration theo thứ tự bắt buộc:
  1. Core/Domain
  2. Service/Business
  3. API/Interface
  4. Tests & scripts
- Thêm cơ chế baseline lock: chỉ fail với vi phạm mới, không chặn vì nợ cũ chưa xử lý hết.

**Definition of Done (DoD)**
- Module lõi đã migrate theo chuẩn mới, có changelog rõ cho từng PR.
- CI chặn được regression chất lượng mới phát sinh.

---

### Tuần 4 — Migrate phần còn lại, dọn alias cũ, khóa governance

**Mục tiêu**
- Hoàn tất migrate phần còn lại.
- Xóa alias cũ và chốt cơ chế tự duy trì chất lượng.

**Đầu việc chính**
- Dọn dẹp alias/deprecated mapping còn sót lại.
- Bật đầy đủ governance gates:
  - pre-commit/pre-push hooks
  - PR checklist bắt buộc
  - reviewer gate/CODEOWNERS cho vùng critical
- Chốt tài liệu vận hành: quy trình xử lý khi fail gate, SLA fix vi phạm.

**Definition of Done (DoD)**
- Không còn alias cũ trong code mới/chạm vào.
- Governance được enforce ở cả local flow và PR flow.

---

## Đầu ra cuối kỳ (expected outcome)

- Repo đạt chuẩn nhất quán về naming, formatting, linting và module boundary.
- CI + pre-commit + review process đủ để **tự duy trì chất lượng**, tránh tái phát nợ kỹ thuật cũ.
- Báo cáo tuần/tháng theo KPI cho phép theo dõi xu hướng và kích hoạt corrective action sớm.

## Thứ tự rollout bắt buộc

1. **Core / Domain models**
2. **Service / Business layer**
3. **API / Controller / Interface layer**
4. **Tests và scripts hỗ trợ**

> Không mở PR cho tầng sau nếu tầng trước chưa ổn định ở phạm vi liên quan.

---

## Nguyên tắc tách PR

Mỗi PR phải thỏa đủ các tiêu chí sau:

- Chỉ có **1 mục tiêu chính** (ví dụ: `naming chuẩn cho module user`).
- Không trộn refactor không liên quan.
- Nếu bắt buộc có thay đổi phụ, liệt kê rõ trong phần **Ảnh hưởng**.
- Có **changelog ngắn** và **checklist ảnh hưởng**.

### Cách đặt tên PR đề xuất

`[<layer>] <scope ngắn> - <mục tiêu>`

Ví dụ:

- `[core] user model - naming chuẩn`
- `[service] irrigation service - chuẩn hóa validation`
- `[api] user controller - thống nhất response schema`
- `[tests] user module - bổ sung regression tests`

---

## Checklist theo từng giai đoạn

### 1) Core / Domain models

- [ ] Chuẩn hóa tên model/value object/entity theo convention.
- [ ] Không đổi hành vi nghiệp vụ ngoài phạm vi model.
- [ ] Rà soát mapping/typing liên quan trực tiếp.
- [ ] Cập nhật changelog ngắn trong PR.

### 2) Service / Business layer

- [ ] Refactor service/use-case bám theo domain đã chốt.
- [ ] Không đổi API contract nếu chưa sang phase API.
- [ ] Giữ logic idempotent/transaction (nếu có).
- [ ] Cập nhật changelog ngắn trong PR.

### 3) API / Controller / Interface layer

- [ ] Đồng bộ naming/DTO/schema theo service/domain mới.
- [ ] Kiểm tra backward compatibility hoặc ghi rõ breaking change.
- [ ] Cập nhật docs interface (nếu có).
- [ ] Cập nhật changelog ngắn trong PR.

### 4) Tests và scripts hỗ trợ

- [ ] Cập nhật unit/integration/regression tests tương ứng.
- [ ] Cập nhật script lint/test/migration (nếu bị ảnh hưởng).
- [ ] Bổ sung test case cho nhánh lỗi quan trọng.
- [ ] Cập nhật changelog ngắn trong PR.

---

## Định dạng changelog ngắn (bắt buộc cho mỗi PR)

Sử dụng 3–5 bullet:

- Thay đổi chính trong phạm vi PR.
- Lý do thay đổi.
- Ảnh hưởng trực tiếp tới module liên quan.
- (Nếu có) Breaking change / migration note.

Ví dụ:

- Chuẩn hóa naming cho `UserStatus` và `UserProfile` trong domain user.
- Loại bỏ alias cũ để giảm trùng nghĩa giữa model và DTO.
- Service user đã cập nhật import tương ứng, không đổi hành vi nghiệp vụ.
- Không có breaking change ở API.

---

## Checklist ảnh hưởng (attach trong PR)

- [ ] Ảnh hưởng DB schema
- [ ] Ảnh hưởng API contract
- [ ] Ảnh hưởng business rule
- [ ] Ảnh hưởng UI/integration
- [ ] Ảnh hưởng test hiện có
- [ ] Cần migration/backfill
- [ ] Cần feature flag/rollout từng phần
- [ ] Cần phối hợp team khác

Nếu tick mục nào, thêm 1 dòng ghi rõ phạm vi.
