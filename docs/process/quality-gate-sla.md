# Quality gate incident response & SLA

## Mục tiêu

Chuẩn hóa cách xử lý khi quality gate fail ở local hoặc trên pull request, đảm bảo vi phạm được xử lý trong thời hạn rõ ràng.

## Phân loại mức độ

- **P0 (critical regression)**
  - Ví dụ: `check_quality_regression.sh` phát hiện vi phạm mới ở module critical.
  - SLA: fix hoặc rollback trong **24 giờ**.
- **P1 (quality gate failure non-critical)**
  - Ví dụ: lint/format fail ở module không critical.
  - SLA: fix trong **2 ngày làm việc**.
- **P2 (hygiene/process issue)**
  - Ví dụ: checklist thiếu thông tin, tài liệu chưa cập nhật.
  - SLA: fix trong **5 ngày làm việc**.

## Quy trình xử lý chuẩn

1. **Xác định loại fail**: alias naming, baseline regression, format/lint, process checklist.
2. **Gán owner trực tiếp** theo vùng CODEOWNERS.
3. **Tạo action item** trong weekly report gần nhất (`docs/process/reports/*`).
4. **Khắc phục + xác nhận lại gate** bằng local command tương ứng.
5. **Nếu quá SLA**: escalate reviewer gate (tag platform lead + module lead).

## Bộ lệnh xác nhận trước khi re-push

```bash
./scripts/check_alias_naming.sh
./scripts/check_quality_regression.sh
pre-commit run --all-files
```

## Chính sách re-baseline

Chỉ re-baseline khi cleanup có chủ đích và được reviewer phê duyệt.

```bash
./scripts/update_quality_baseline_lock.sh
```

Khi re-baseline, PR bắt buộc mô tả lý do và phạm vi thay đổi baseline.
