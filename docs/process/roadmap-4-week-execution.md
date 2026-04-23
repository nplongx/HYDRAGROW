# Theo dõi thực thi lộ trình 4 tuần

Ngày khởi động: **2026-04-23 (UTC)**.
Tài liệu gốc: `docs/rollout-plan.md`.

## Trạng thái tổng quan

- [x] Tuần 1 — Standards + glossary + baseline được chốt trong `docs/engineering/*`.
- [x] Tuần 1 — Có baseline báo cáo ban đầu tại `docs/process/reports/weekly-2026-04-23.md`.
- [x] Tuần 2 — Bật quality gate ở chế độ warning-only qua workflow `.github/workflows/quality-roadmap.yml`.
<<<<<<< codex/start-4-week-roadmap-execution-6lx4kz
- [x] Tuần 2 — Backlog P0 đã được triage đầy đủ (có owner + deadline cho toàn bộ mục tồn).
- [x] Tuần 3 — Fail-on-new-violations + baseline lock đã bật trên pull request.
=======
- [ ] Tuần 2 — Đóng toàn bộ backlog P0 cross-module.
- [~] Tuần 3 — Chuẩn bị fail-on-new-violations + baseline lock (đã có script + toggle CI).
>>>>>>> main
- [ ] Tuần 4 — Bật full governance gates (pre-push + reviewer gate + SLA xử lý vi phạm).

## Backlog P0 mở (khởi tạo)

<<<<<<< codex/start-4-week-roadmap-execution-6lx4kz
| ID | Phạm vi | Mô tả | Owner | Hạn | Trạng thái |
|---|---|---|---|---|---|
| P0-001 | Backend/Frontend contract | Chuẩn hóa naming field dữ liệu sensor giữa API và UI model | @backend-lead + @frontend-lead | 2026-04-30 | In progress |
| P0-002 | Backend/Firmware command | Kiểm tra đồng bộ enum command giữa scheduler service và controller | @backend-lead + @firmware-lead | 2026-04-30 | In progress |
| P0-003 | Governance | Chốt rule baseline lock để chuẩn bị fail-on-new ở tuần 3 | @platform-lead | 2026-04-23 | Done |

Tham chiếu chi tiết: `docs/process/p0-remediation-week2.md`.
=======
| ID | Phạm vi | Mô tả | Owner | Hạn |
|---|---|---|---|---|
| P0-001 | Backend/Frontend contract | Chuẩn hóa naming field dữ liệu sensor giữa API và UI model | TBD | 2026-04-30 |
| P0-002 | Backend/Firmware command | Kiểm tra đồng bộ enum command giữa scheduler service và controller | TBD | 2026-04-30 |
| P0-003 | Governance | Chốt rule baseline lock để chuẩn bị fail-on-new ở tuần 3 | TBD | 2026-05-07 |
>>>>>>> main

## Nhịp cập nhật

- Cập nhật báo cáo tuần bằng:

```bash
./scripts/generate_quality_report.sh weekly
```

- Khi hoàn tất một P0, tick trạng thái + bổ sung PR link ngay trong bảng backlog.


## Baseline lock (chuẩn bị tuần 3)

<<<<<<< codex/start-4-week-roadmap-execution-6lx4kz
- Baseline lock hiện tại lưu tại `docs/process/baseline-lock.txt`.
- Script kiểm tra regression: `./scripts/check_quality_regression.sh`.
- GitHub Actions chạy kiểm tra này mặc định cho pull request (không cần toggle).

- Khi cần re-baseline có chủ đích (sau khi cleanup nợ cũ), chạy: `./scripts/update_quality_baseline_lock.sh`.
=======
- Baseline hiện tại lưu tại `docs/process/baseline-metrics.env`.
- Script kiểm tra regression: `./scripts/check_quality_regression.sh`.
- Trên GitHub Actions, bật khóa bằng cách đặt Repository Variable `ENABLE_BASELINE_LOCK=true`.
>>>>>>> main
