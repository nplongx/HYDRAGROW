# ESP32-C3-SENSOR-NODE

Firmware đọc cảm biến EC/pH/Nhiệt độ/Mực nước và gửi MQTT telemetry.

**Platform:** ESP32-C3 | **Lang:** C++ / Arduino | **Build:** PlatformIO

## Prerequisites

```bash
pip install platformio
```

## Build

```bash
cd ESP32-C3-SENSOR-NODE
pio run --environment esp32-c3-devkitm-1
```

## Flash

```bash
pio run --target upload --environment esp32-c3-devkitm-1
```

## Monitor serial

```bash
pio device monitor --baud 115200
```

## Native tests

```bash
pio test --environment native
```

## Cấu trúc

```
src/
├── config/     # Cấu hình WiFi/MQTT (thay thế trước khi flash)
├── sensors/    # Driver EC (ADS1115), pH (ADS1115), DS18B20, water level
├── mqtt/       # PubSubClient wrapper, topic builder
├── ota/        # OTA updater (GitHub Releases)
├── filters/    # EMA / median filter cho tín hiệu cảm biến
├── security/   # TLS cert bundle, xác thực HMAC cho lệnh MQTT (CommandSecurity)
└── utils/
```

## Thư viện (khai trong `platformio.ini`)

- `ArduinoJson` ^7.4.3
- `PubSubClient` ^2.8.0
- `DallasTemperature` ^4.0.6
- `Adafruit ADS1X15` ^2.5.0

## MQTT Payload

Gửi lên topic `AGITECH/{device_id}/sensor/data`, schema khớp `SensorData` trong `hydragrow-shared`.

## OTA

Thiết bị tự kiểm tra và cập nhật firmware qua GitHub Releases khi nhận lệnh
MQTT đã ký `trigger_ota` (topic `AGITECH/{device_id}/command`, field `action`).
Tải file `sensor-firmware.bin` từ release mới nhất của repo, flash vào phân
vùng OTA còn trống rồi tự khởi động lại. Tiến trình/kết quả được publish lên
`AGITECH/{device_id}/system_log`.

Version hiện tại của firmware nằm ở `src/config/Version.h` (`FIRMWARE_VERSION`)
— PHẢI khớp với git tag khi cắt release, nếu không workflow
`firmware-release.yml` sẽ chặn release (xem `docs/superpowers/plans/2026-09-08-sensor-node-ota.md`).
