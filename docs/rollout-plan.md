# Rollout kế hoạch theo module ưu tiên

Tài liệu này chuẩn hóa cách tách rollout thành các PR nhỏ, mỗi PR chỉ xử lý **1 phạm vi rõ ràng**.

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
