#include "CommandSecurity.h"

#include <mbedtls/md.h>
#include <string.h>

#include "secrets.h"
#include "Logger.h"

CommandSecurity::CommandSecurity() {}

// Canonical = chính JSON đã nhận (đã bỏ field "signature"), serialize lại
// compact — KHÔNG tự sort key thủ công. Khớp với cách backend tạo chữ ký
// (hydragrow-backend/src/api/mqtt_utils.rs::canonical_payload): backend parse
// vào serde_json::Value, xoá "signature", serialize lại bằng
// serde_json::to_vec. Vì serde_json::Value dùng BTreeMap mặc định (không bật
// feature preserve_order), CHÍNH bytes JSON backend gửi lên MQTT đã ở dạng
// key-sorted sẵn — nên chỉ cần bỏ "signature" khỏi document đã parse rồi
// serialize lại (ArduinoJson giữ nguyên thứ tự đã parse) là ra đúng canonical
// bytes, không cần tự sort field ở firmware. Đã mô phỏng lại toàn bộ vòng
// ký/xác minh bằng Python (hmac + json.dumps(sort_keys=True)) để xác nhận.
String CommandSecurity::canonicalCommandPayload(JsonDocument& doc) {
    String out;
    serializeJson(doc, out);
    return out;
}

String CommandSecurity::calculateHmac(const String& payload) {
    const mbedtls_md_info_t* mdInfo = mbedtls_md_info_from_type(MBEDTLS_MD_SHA256);
    if (mdInfo == nullptr) {
        return String("");
    }

    unsigned char digest[32];
    int rc = mbedtls_md_hmac(
        mdInfo,
        reinterpret_cast<const unsigned char*>(COMMAND_HMAC_KEY),
        strlen(COMMAND_HMAC_KEY),
        reinterpret_cast<const unsigned char*>(payload.c_str()),
        payload.length(),
        digest
    );
    if (rc != 0) {
        return String("");
    }

    static const char hexChars[] = "0123456789abcdef";
    String hex;
    hex.reserve(sizeof(digest) * 2);
    for (unsigned char b : digest) {
        hex += hexChars[(b >> 4) & 0x0F];
        hex += hexChars[b & 0x0F];
    }
    return hex;
}

bool CommandSecurity::nonceSeen(const String& nonce) {
    for (int i = 0; i < nonceCount_; i++) {
        if (recentNonces_[i] == nonce) {
            return true;
        }
    }
    return false;
}

void CommandSecurity::rememberNonce(const String& nonce) {
    if (nonceCount_ < MAX_NONCES) {
        recentNonces_[nonceCount_++] = nonce;
        return;
    }
    for (int i = 1; i < MAX_NONCES; i++) {
        recentNonces_[i - 1] = recentNonces_[i];
    }
    recentNonces_[MAX_NONCES - 1] = nonce;
}

bool CommandSecurity::verify(JsonDocument& doc) {
    if (!doc["signature"].is<const char*>()) {
        Logger::debugPrintln("[SECURITY] Thieu truong signature");
        return false;
    }
    String signature = doc["signature"].as<String>();
    if (signature.length() == 0) {
        return false;
    }

    if (doc["ts"].isNull()) {
        Logger::debugPrintln("[SECURITY] Thieu truong ts");
        return false;
    }

    if (!doc["nonce"].is<const char*>()) {
        Logger::debugPrintln("[SECURITY] Thieu truong nonce");
        return false;
    }
    String nonce = doc["nonce"].as<String>();
    if (nonce.length() == 0) {
        return false;
    }
    if (nonceSeen(nonce)) {
        Logger::debugPrintln("[SECURITY] Nonce da duoc su dung (replay)");
        return false;
    }

    doc.remove("signature");
    String canonical = canonicalCommandPayload(doc);
    // Khoi phuc lai field "signature" - khong anh huong logic verify, chi de
    // code goi sau (vd log toan bo payload) van thay du lieu goc neu can.
    doc["signature"] = signature;

    String expected = calculateHmac(canonical);
    if (expected.length() == 0 || expected != signature) {
        Logger::debugPrintln("[SECURITY] Chu ky khong hop le");
        return false;
    }

    rememberNonce(nonce);
    return true;
}