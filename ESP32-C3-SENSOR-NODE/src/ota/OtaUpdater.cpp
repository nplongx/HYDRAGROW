#include "OtaUpdater.h"

#include <ArduinoJson.h>
#include <HTTPClient.h>
#include <Update.h>
#include <WiFiClientSecure.h>

#include "Logger.h"
#include "OtaVersionCheck.h"
#include "Version.h"

namespace OtaUpdater {

namespace {

// GitHub yêu cầu User-Agent hợp lệ, nếu không trả về 403.
constexpr const char* GITHUB_USER_AGENT = "Hydragrow-ESP32-Sensor-OTA";

bool fetchLatestRelease(JsonDocument& out) {
    WiFiClientSecure client;
    // Xem Global Constraints trong plan sensor-node-ota: mirror trade-off
    // setInsecure() đã dùng cho MQTT trong MqttManager.cpp — chưa bundle
    // root CA riêng cho GitHub.
    client.setInsecure();

    HTTPClient http;
    http.setUserAgent(GITHUB_USER_AGENT);
    if (!http.begin(client, GITHUB_RELEASES_LATEST_URL)) {
        return false;
    }
    http.addHeader("Accept", "application/vnd.github.v3+json");

    int code = http.GET();
    if (code != HTTP_CODE_OK) {
        Logger::debugPrintf("[OTA] GitHub API tra ve HTTP %d\n", code);
        http.end();
        return false;
    }

    String body = http.getString();
    http.end();

    DeserializationError err = deserializeJson(out, body);
    if (err) {
        Logger::debugPrintf("[OTA] Loi parse JSON release: %s\n", err.c_str());
        return false;
    }
    return true;
}

bool streamAndFlash(const String& downloadUrl, const OtaEventPublisher& publishEvent) {
    WiFiClientSecure client;
    client.setInsecure();
    HTTPClient http;
    http.setUserAgent(GITHUB_USER_AGENT);
    http.setFollowRedirects(HTTPC_FORCE_FOLLOW_REDIRECTS);
    if (!http.begin(client, downloadUrl)) {
        publishEvent("Critical", "OTA that bai", "Khong the mo ket noi tai firmware");
        return false;
    }
    int code = http.GET();
    if (code != HTTP_CODE_OK) {
        char msg[64];
        snprintf(msg, sizeof(msg), "Tai firmware that bai, HTTP %d", code);
        publishEvent("Critical", "OTA that bai", msg);
        http.end();
        return false;
    }
    int contentLength = http.getSize();
    bool updateStarted = (contentLength > 0)
        ? Update.begin(static_cast<size_t>(contentLength))
        : Update.begin(UPDATE_SIZE_UNKNOWN);
    if (!updateStarted) {
        publishEvent("Critical", "OTA that bai", "Khong du bo nho phan vung OTA");
        http.end();
        return false;
    }
    WiFiClient* stream = http.getStreamPtr();
    uint8_t buffer[1024];
    size_t totalWritten = 0;
    size_t lastReportedKb = 0;
    unsigned long lastActivity = millis();
    while (http.connected() && (contentLength < 0 || static_cast<int>(totalWritten) < contentLength)) {
        size_t available = stream->available();
        if (available == 0) {
            if (millis() - lastActivity > 15000) {
                publishEvent("Critical", "OTA that bai", "Timeout khi tai firmware");
                Update.abort();
                http.end();
                return false;
            }
            delay(10);
            continue;
        }
        size_t toRead = available > sizeof(buffer) ? sizeof(buffer) : available;
        size_t readBytes = stream->readBytes(buffer, toRead);
        if (readBytes == 0) continue;
        if (Update.write(buffer, readBytes) != readBytes) {
            publishEvent("Critical", "OTA that bai", Update.errorString());
            Update.abort();
            http.end();
            return false;
        }
        totalWritten += readBytes;
        lastActivity = millis();
        size_t kb = totalWritten / 1024;
        if (kb - lastReportedKb >= 100) {
            lastReportedKb = kb;
            char msg[48];
            snprintf(msg, sizeof(msg), "Da tai %u KB", static_cast<unsigned>(kb));
            publishEvent("Info", "OTA dang tien hanh", msg);
        }
    }
    http.end();

    if (contentLength > 0 && static_cast<int>(totalWritten) != contentLength) {
        publishEvent("Critical", "OTA that bai", "Firmware tai ve bi thieu du lieu");
        Update.abort();
        return false;
    }

    if (!Update.end(true)) {
        publishEvent("Critical", "OTA that bai", Update.errorString());
        return false;
    }

    publishEvent("Success", "OTA thanh cong", "Da flash firmware moi, dang khoi dong lai");
    return true;
}

} // namespace

bool performUpdate(const String& deviceId, const OtaEventPublisher& publishEvent) {
    (void)deviceId; // Giu tham so de dong nhat chu ky voi perform_ota_update()
    // ben controller (Rust); danh cho log/telemetry theo
    // device_id sau nay.

    JsonDocument release;
    if (!fetchLatestRelease(release)) {
        publishEvent("Warning", "OTA kiem tra that bai", "Khong the doc GitHub Releases");
        return false;
    }

    const char* tagName = release["tag_name"] | "";
    if (!OtaVersionCheck::isUpdateAvailable(FIRMWARE_VERSION, tagName)) {
        publishEvent("Info", "OTA", "Firmware da la ban moi nhat");
        return false;
    }

    String downloadUrl = OtaVersionCheck::findAssetDownloadUrl(release, SENSOR_FIRMWARE_ASSET_NAME);
    if (downloadUrl.length() == 0) {
        publishEvent("Critical", "OTA that bai", "Khong tim thay sensor-firmware.bin trong release");
        return false;
    }

    publishEvent("Info", "OTA dang tien hanh", "Bat dau tai firmware moi");

    bool ok = streamAndFlash(downloadUrl, publishEvent);
    if (ok) {
        delay(1000);
        ESP.restart();
    }
    return ok;
}

} // namespace OtaUpdater