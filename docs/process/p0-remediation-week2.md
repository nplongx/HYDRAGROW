# Week 2 - P0 remediation tracker (cross-module)

Cập nhật lần cuối: **2026-04-23 (UTC)**.

## Mục tiêu tuần 2

- Chốt owner + deadline cho toàn bộ P0 còn tồn.
- Đảm bảo mỗi P0 có hướng xử lý rõ, có artefact theo dõi.

## Danh sách P0

| ID | Trạng thái | Owner | Deadline | Hướng xử lý | Artefact theo dõi |
|---|---|---|---|---|---|
| P0-001 | In progress | @backend-lead + @frontend-lead | 2026-04-30 | Chuẩn hóa naming field sensor trong API response và UI mapping type. | `docs/process/roadmap-4-week-execution.md` |
| P0-002 | In progress | @backend-lead + @firmware-lead | 2026-04-30 | Đồng bộ enum command giữa scheduler service và controller constants. | `docs/process/roadmap-4-week-execution.md` |
| P0-003 | Done | @platform-lead | 2026-04-23 | Bật fail-on-new-violations bằng baseline lock trên pull request. | `.github/workflows/quality-roadmap.yml`, `scripts/check_quality_regression.sh` |

## Exit criteria tuần 2

- [x] Không còn P0 ở trạng thái “chưa có owner”.
- [x] Không còn P0 ở trạng thái “chưa có deadline”.
- [x] Có điểm kiểm soát CI warning-only hoạt động ổn định.
- [x] Có danh mục P0 rõ để chuyển sang các PR remediation của tuần 3.
