# Đóng góp cho HYDRAGROW

## Trước khi sửa bất kỳ subsystem nào

Đọc [docs/superpowers/specs/module-rules/README.md](docs/superpowers/specs/module-rules/README.md) — chọn file rule đúng subsystem bạn đụng tới. Đây là **bắt buộc**, không phải gợi ý: PR vi phạm rule (VD: SQL viết trong handler thay vì `db/`, thêm topic MQTT mà không cập nhật `hydragrow-shared`) sẽ bị yêu cầu sửa lại trong review dù CI xanh.

## Quy trình PR

1. Nhánh đặt tên `feat/<mô-tả-ngắn>`, `fix/<mô-tả-ngắn>`, hoặc `docs/<mô-tả-ngắn>`.
2. Nếu thay đổi chạm nhiều subsystem không phụ thuộc lẫn nhau, cân nhắc tách thành nhiều PR nhỏ theo subsystem — dễ review, CI chạy nhanh hơn (mỗi workflow chỉ trigger khi path của nó bị chạm, xem bảng CI trong README.md).
3. Trước khi mở PR, chạy bộ lệnh "Kiểm tra chung" ở cuối [module-rules/README.md](docs/superpowers/specs/module-rules/README.md) cho (các) subsystem bạn đã sửa.
4. Mô tả PR nêu rõ: subsystem nào bị chạm, có đổi hợp đồng MQTT/schema không, có migration DB mới không.

## CI

Xem bảng ánh xạ workflow → subsystem trong [README.md](README.md#ci). Mỗi workflow chỉ chạy khi PR chạm đúng path của nó — PR chỉ sửa 1 subsystem sẽ không phải chờ CI của subsystem khác.

## Không đụng vào

- `server_wallet.json` — Solana wallet key, không commit.
- `hydragrow-backend/migrations/` — cần migration plan riêng, xem [module-rules/backend.md](docs/superpowers/specs/module-rules/backend.md#-migrations-checklist-run-before-every-pr).
