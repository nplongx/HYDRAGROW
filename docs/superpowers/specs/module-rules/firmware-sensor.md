# HYDRAGROW — `ESP32-C3-SENSOR-NODE` Module Rules

Firmware C++/Arduino/PlatformIO, đọc cảm biến EC/pH/Temp/WaterLevel và publish qua MQTT. Không dùng chung crate Rust với 2 subsystem kia — mọi hợp đồng dữ liệu (tên topic, field JSON) phải khớp THỦ CÔNG với `hydragrow-shared` vì không thể `use` trực tiếp crate Rust từ C++.

**Khi nào chạm module này:** đọc cảm biến mới, hiệu chuẩn (calibration), `src/secrets.h` (KHÔNG BAO GIỜ commit file thật, chỉ commit `.example`), payload MQTT publish.

## Rules

- Mọi chuỗi topic MQTT hardcode trong firmware này (VD: `"AGITECH/" + device_id + "/sensor/data"`) phải khớp CHÍNH XÁC format string trong `hydragrow-shared/src/topics.rs`. Khi đổi topic ở `hydragrow-shared`, PR đó phải cập nhật cả file `.cpp`/`.h` tương ứng ở đây trong cùng PR — xem checklist tổng ở [shared.md](./shared.md#thay-đổi-tương-thích-ngược).
- Payload JSON publish lên `sensor/data` phải khớp field-name (kể cả case) với `SensorData` trong `hydragrow-shared`. Không tự đổi tên field phía firmware (VD: `ec` → `EC`) mà không đổi struct Rust tương ứng.
- `src/secrets.h` không bao giờ được commit — chỉ `src/secrets.h.example` (đây là lý do CI có bước `cp src/secrets.h.example src/secrets.h`). PR nào vô tình thêm `src/secrets.h` thật phải bị chặn ở review, không dựa vào `.gitignore` một mình.
- Test native (`test/`) dùng stub hardware (`test/stubs/Arduino.h`, `test/stubs/Preferences.h`) để chạy được trên máy host không cần board thật. Logic đọc/parse cảm biến mới nên tách ra hàm thuần (nhận input số, trả kết quả số) để test được qua stub này, thay vì viết thẳng trong `loop()`.
- Thư viện bên thứ 3 trong `libraries/` (VD: `ArduinoJson`, `DallasTemperature`) không tự sửa trực tiếp — nếu cần vá, fork lên PlatformIO registry hoặc ghi đè bằng file riêng, không diff trực tiếp vào code vendor (khó merge upgrade sau này).

## Test checklist

- [ ] Đọc cảm biến mới: có hàm parse thuần tách khỏi `loop()`, có test native trong `test/` dùng stub
- [ ] Đổi payload/topic MQTT: đã đối chiếu với `hydragrow-shared` (topics.rs + struct payload) trong CÙNG PR
- [ ] `pio run --environment esp32-c3-devkitm-1` build thành công
- [ ] `pio test --environment native` PASS (không chỉ "không có test nào")

## Build/test cục bộ

```bash
cd ESP32-C3-SENSOR-NODE
cp src/secrets.h.example src/secrets.h
pio run --environment esp32-c3-devkitm-1
pio test --environment native
```
