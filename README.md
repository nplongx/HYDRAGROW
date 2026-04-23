# HYDRAGROW Monorepo

Repository này gồm nhiều module cho hệ thống HYDRAGROW:

- `hydragrow-frontend`: Tauri + React + TypeScript.
- `hydragrow-backend`: Rust backend service.
- `ESP32-C3-CONTROLLER-NODE`: Rust firmware cho controller node.
- `ESP32-C3-SENSOR-NODE`: ESP32 sensor node (PlatformIO/C++).

## Pre-commit: chạy format/lint

Trước khi mở PR, chạy **format + lint/check** theo từng module bạn thay đổi.

### 1) Rust modules (`hydragrow-backend`, `ESP32-C3-CONTROLLER-NODE`)

```bash
# Format
cargo fmt --all --check

# Lint
cargo clippy --all-targets --all-features -- -D warnings
```

> Chạy trong thư mục module tương ứng.

### 2) Frontend module (`hydragrow-frontend`)

```bash
# Type/lint check tối thiểu
npm run build
```

> `npm run build` sẽ chạy `tsc` trước khi build nên giúp bắt phần lớn lỗi type/lint mức biên dịch.

### 3) Sensor node (`ESP32-C3-SENSOR-NODE`)

Hiện tại module này chưa chuẩn hoá script lint/format ở cấp repo.
Khi thay đổi C/C++, vui lòng giữ naming/style theo `CONTRIBUTING.md` và format nhất quán trước khi commit.

## Quy ước đóng góp

Xem chi tiết tại `CONTRIBUTING.md`.
