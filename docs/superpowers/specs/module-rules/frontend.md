# HYDRAGROW — `hydragrow-frontend` Module Rules

React + TypeScript + Vite, đóng gói desktop qua Tauri (`src-tauri/`), state qua Zustand. Đã có 2 doc bổ trợ: [route-content-audit.md](../../../hydragrow-frontend/docs/route-content-audit.md) và [ui-writing-guideline.md](../../../hydragrow-frontend/docs/ui-writing-guideline.md) — đọc thêm 2 file đó khi sửa nội dung/copy hiển thị cho người dùng.

**Khi nào chạm module này:** component React, Zustand store, client WebSocket/REST gọi `hydragrow-backend`, Tauri command (`src-tauri/src/`).

## Rules

- Type của dữ liệu nhận từ backend (`SensorData`, `FsmSnapshot`, ...) phải khớp 1-1 với struct tương ứng trong `hydragrow-shared` — khi struct phía Rust đổi (xem [shared.md](./shared.md)), PR đó phải cập nhật type TS trong cùng lần đổi, không để lệch schema âm thầm giữa 2 phía.
- Không gọi `fetch`/WebSocket trực tiếp trong component — đi qua lớp client/service tập trung để có 1 chỗ xử lý auth header, retry, lỗi mạng.
- Thêm Tauri command mới (`src-tauri/src/`): phải có test Rust (`cargo test` trong `src-tauri/`) cho logic phía Rust, và phải khai báo trong `capabilities/` nếu cần quyền hệ thống mới — không mở quyền rộng hơn mức command thực sự cần.
- Zustand store: state cập nhật từ WebSocket phải qua 1 action rõ ràng (VD: `applySensorUpdate(data)`), không set state trực tiếp từ callback socket rải rác nhiều nơi — dễ test và dễ trace nguồn gốc thay đổi state.
- Copy/text hiển thị cho người dùng: theo [ui-writing-guideline.md](../../../hydragrow-frontend/docs/ui-writing-guideline.md).

## Test checklist

- [ ] Component mới có ít nhất 1 test render + 1 test tương tác chính (`vitest` + Testing Library)
- [ ] Store action mới có test đơn vị (input event → state kỳ vọng)
- [ ] Tauri command mới có test Rust trong `src-tauri/`
- [ ] `npx tsc --noEmit` sạch — không dùng `any` để né lỗi type từ backend

## Chạy test cục bộ

```bash
cd hydragrow-frontend
npx tsc --noEmit
npx eslint .
npx vitest run
cd src-tauri && cargo check && cargo clippy -- -D warnings
```
