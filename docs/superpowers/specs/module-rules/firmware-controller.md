# HYDRAGROW — `ESP32-C3-CONTROLLER-NODE` Module Rules

Firmware Rust/esp-rs (nightly) chạy trên MCU điều khiển thật. Đây là lớp **mỏng** bọc quanh `hydragrow-controller-core` — logic FSM/dosing KHÔNG được viết mới ở đây, chỉ viết driver hardware (GPIO, PWM bơm, cảm biến qua I2C/ADC) và gọi vào `hydragrow-controller-core` để quyết định hành vi.

**Khi nào chạm module này:** driver hardware mới, wiring GPIO, cấu hình `sdkconfig.defaults`/`partitions.csv`, publish/subscribe MQTT thật (dùng `esp-idf-svc`'s MQTT client), OTA (`firmware-controller-release.yml`).

## Rules

- Không viết logic quyết định pha/FSM trực tiếp trong `src/` của firmware này — nếu thấy mình đang viết `if phase == X { ... }` để quyết định hành vi nghiệp vụ, logic đó thuộc về `hydragrow-controller-core` (xem [controller-core.md](./controller-core.md)), không phải ở đây.
- Payload publish lên MQTT phải serialize từ type của `hydragrow-shared` (crate này depend `hydragrow-shared` qua `path = "../hydragrow-shared"`) — không tự tay build chuỗi JSON.
- Đổi `partitions.csv` hoặc `sdkconfig.defaults`: bắt buộc note trong PR rằng thiết bị đã flash firmware cũ sẽ cần flash lại từ đầu (không OTA được) nếu partition table đổi kích thước.
- Bump version trong `Cargo.toml` PHẢI khớp tag git khi release — đã được `firmware-controller-release.yml` chặn cứng (`Verify tag matches Cargo.toml version`), không tự ý sửa bước check đó để "cho qua".
- `harness = false` trong `Cargo.toml` là chủ đích (tránh lỗi rust-analyzer trên target no_std-ish của esp-idf) — logic cần unit test phải nằm ở `hydragrow-controller-core` nơi có test harness bình thường, không cố thêm `#[test]` vào crate firmware này.

## Test checklist

- [ ] Thay đổi logic nghiệp vụ: test đã được thêm ở `hydragrow-controller-core` (xem [controller-core.md](./controller-core.md)), không phải ở đây
- [ ] Thay đổi driver hardware: build thành công `cargo check --locked` (CI) — verify thật trên board vật lý trước khi merge vào `main` vì CI không thể test hardware thật
- [ ] Đổi payload MQTT: đã cập nhật type nguồn ở `hydragrow-shared` (xem [shared.md](./shared.md)) trong CÙNG PR

## Build/check cục bộ (cần cài esp-rs qua `espup`)

```bash
cd ESP32-C3-CONTROLLER-NODE
source "$HOME/export-esp.sh"
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo check --locked
```
