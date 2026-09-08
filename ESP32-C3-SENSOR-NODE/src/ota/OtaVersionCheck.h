#pragma once

#include <Arduino.h>
#include <ArduinoJson.h>

// Logic thuần (không phụ thuộc WiFi/HTTP/Update) để kiểm tra phiên bản và tìm
// asset trong response GitHub Releases — tách riêng để test được trên env
// native, mirror 2 hàm tương ứng trong ESP32-C3-CONTROLLER-NODE/src/hw/ota.rs.
namespace OtaVersionCheck {

// true nếu tagName khác currentVersion và không rỗng. Không so sánh semver —
// mirror đúng logic đơn giản của controller (Rust): so sánh chuỗi bằng nhau.
bool isUpdateAvailable(const String& currentVersion, const String& tagName);

// Tìm asset tên `assetName` trong mảng "assets" của response GitHub Release
// (đã parse vào releaseDoc), trả về "browser_download_url" tương ứng.
// Trả về chuỗi rỗng nếu không tìm thấy mảng assets hoặc không khớp tên.
String findAssetDownloadUrl(JsonDocument& releaseDoc, const char* assetName);

} // namespace OtaVersionCheck