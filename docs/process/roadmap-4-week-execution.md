# Theo dõi thực thi lộ trình 4 tuần

Ngày khởi động: **2026-04-23 (UTC)**.
Tài liệu gốc: `docs/rollout-plan.md`.

## Trạng thái tổng quan

- [x] Tuần 1 — Standards + glossary + baseline được chốt trong `docs/engineering/*`.
- [x] Tuần 1 — Có baseline báo cáo ban đầu tại `docs/process/reports/weekly-2026-04-23.md`.
- [x] Tuần 2 — Bật quality gate ở chế độ warning-only qua workflow `.github/workflows/quality-roadmap.yml`.
- [ ] Tuần 2 — Đóng toàn bộ backlog P0 cross-module.
- [ ] Tuần 3 — Chuyển sang fail-on-new-violations + baseline lock.
- [ ] Tuần 4 — Bật full governance gates (pre-push + reviewer gate + SLA xử lý vi phạm).

## Backlog P0 mở (khởi tạo)

| ID | Phạm vi | Mô tả | Owner | Hạn |
|---|---|---|---|---|
| P0-001 | Backend/Frontend contract | Chuẩn hóa naming field dữ liệu sensor giữa API và UI model | TBD | 2026-04-30 |
| P0-002 | Backend/Firmware command | Kiểm tra đồng bộ enum command giữa scheduler service và controller | TBD | 2026-04-30 |
| P0-003 | Governance | Chốt rule baseline lock để chuẩn bị fail-on-new ở tuần 3 | TBD | 2026-05-07 |

## Nhịp cập nhật

- Cập nhật báo cáo tuần bằng:

```bash
./scripts/generate_quality_report.sh weekly
```

- Khi hoàn tất một P0, tick trạng thái + bổ sung PR link ngay trong bảng backlog.
