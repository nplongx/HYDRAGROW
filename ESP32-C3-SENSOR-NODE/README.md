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
├── filters/    # EMA / median filter cho tín hiệu cảm biến
├── security/   # TLS cert bundle
└── utils/
```

## Thư viện (khai trong `platformio.ini`)

- `ArduinoJson` ^7.4.3
- `PubSubClient` ^2.8.0
- `DallasTemperature` ^4.0.6
- `Adafruit ADS1X15` ^2.5.0

## MQTT Payload

Gửi lên topic `AGITECH/{device_id}/sensor/data`, schema khớp `SensorData` trong `hydragrow-shared`.
