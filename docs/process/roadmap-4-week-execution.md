# Theo dõi thực thi lộ trình 4 tuần

Ngày khởi động: **2026-04-23 (UTC)**.
Tài liệu gốc: `docs/rollout-plan.md`.

## Trạng thái tổng quan

- [x] Tuần 1 — Standards + glossary + baseline được chốt trong `docs/engineering/*`.
- [x] Tuần 1 — Có baseline báo cáo ban đầu tại `docs/process/reports/weekly-2026-04-23.md`.
- [x] Tuần 2 — Bật quality gate ở chế độ warning-only qua workflow `.github/workflows/quality-roadmap.yml`.
- [x] Tuần 2 — Backlog P0 đã được triage đầy đủ (có owner + deadline cho toàn bộ mục tồn).
- [x] Tuần 3 — Fail-on-new-violations + baseline lock đã bật trên pull request.
- [x] Tuần 4 — Full governance gates đã bật (pre-push + reviewer gate + SLA xử lý vi phạm).

## Backlog P0 mở (khởi tạo)

| ID | Phạm vi | Mô tả | Owner | Hạn | Trạng thái |
|---|---|---|---|---|---|
| P0-001 | Backend/Frontend contract | Chuẩn hóa naming field dữ liệu sensor giữa API và UI model | @backend-lead + @frontend-lead | 2026-04-30 | In progress |
| P0-002 | Backend/Firmware command | Kiểm tra đồng bộ enum command giữa scheduler service và controller | @backend-lead + @firmware-lead | 2026-04-30 | In progress |
| P0-003 | Governance | Chốt rule baseline lock để chuẩn bị fail-on-new ở tuần 3 | @platform-lead | 2026-04-23 | Done |

Tham chiếu chi tiết: `docs/process/p0-remediation-week2.md`.

## Nhịp cập nhật

- Cập nhật báo cáo tuần bằng:

```bash
./scripts/generate_quality_report.sh weekly
```

- Khi hoàn tất một P0, tick trạng thái + bổ sung PR link ngay trong bảng backlog.


## Baseline lock (chuẩn bị tuần 3)

- Baseline lock hiện tại lưu tại `docs/process/baseline-lock.txt`.
- Script kiểm tra regression: `./scripts/check_quality_regression.sh`.
- GitHub Actions chạy kiểm tra này mặc định cho pull request (không cần toggle).

- Khi cần re-baseline có chủ đích (sau khi cleanup nợ cũ), chạy: `./scripts/update_quality_baseline_lock.sh`.


## Week 4 governance lock

- Pre-push hook chạy `check_quality_regression.sh` trước `pre-commit run --all-files`.
- PR template yêu cầu khai báo xử lý theo SLA khi quality gate fail.
- Runbook SLA: `docs/process/quality-gate-sla.md`.
