## Phạm vi PR (chỉ 1 phạm vi rõ ràng)

- Layer: <!-- core | service | api | tests -->
- Scope: <!-- ví dụ: naming chuẩn cho module user -->
- Mục tiêu chính: <!-- mô tả ngắn 1 câu -->

## Changelog ngắn

- 
- 
- 

## Checklist ảnh hưởng

- [ ] Ảnh hưởng DB schema
- [ ] Ảnh hưởng API contract
- [ ] Ảnh hưởng business rule
- [ ] Ảnh hưởng UI/integration
- [ ] Ảnh hưởng test hiện có
- [ ] Cần migration/backfill
- [ ] Cần feature flag/rollout từng phần
- [ ] Cần phối hợp team khác

## Validate trước merge

- [ ] Đã tự review theo đúng 1 phạm vi
- [ ] Đã cập nhật test liên quan (hoặc nêu lý do chưa cập nhật)
- [ ] Đã ghi chú breaking change (nếu có)


## Quality gate / SLA

- [ ] Nếu gate fail, đã gán owner + ETA theo `docs/process/quality-gate-sla.md`
- [ ] Nếu có re-baseline, đã nêu rõ lý do và phạm vi thay đổi
