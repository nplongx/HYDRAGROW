#pragma once

#include <Arduino.h>
#include <functional>

// publishEvent(level, title, message) — bắn 1 sự kiện system_log lên MQTT.
// Nhận qua tham số thay vì include trực tiếp MqttManager để tránh phụ thuộc
// vòng (MqttManager.cpp include OtaUpdater.h, không phải ngược lại).
using OtaEventPublisher = std::function<void(const char* level, const char* title, const char* message)>;

namespace OtaUpdater {

// Tên asset trong GitHub Release ứng với firmware của sensor node — phân
// biệt với "firmware.bin" của ESP32-C3-CONTROLLER-NODE. Cả 2 node chia sẻ
// chung 1 tag release nhưng khác tên file đính kèm (xem Task 9).
constexpr const char* SENSOR_FIRMWARE_ASSET_NAME = "sensor-firmware.bin";

constexpr const char* GITHUB_RELEASES_LATEST_URL =
    "https://api.github.com/repos/nplongx/HYDRAGROW/releases/latest";

// Kiểm tra GitHub Releases; nếu có bản mới, tải và flash vào phân vùng OTA
// còn lại rồi ESP.restart(). Trả về true nếu đã bắt đầu flash thành công
// (thiết bị sẽ tự khởi động lại); false nếu không có bản mới hoặc có lỗi (đã
// gọi publishEvent() mô tả lỗi trước khi return).
bool performUpdate(const String& deviceId, const OtaEventPublisher& publishEvent);

} // namespace OtaUpdater