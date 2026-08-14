#pragma once

#include <Arduino.h>
#include <ArduinoJson.h>

class CommandSecurity {
public:
    CommandSecurity();

    bool verify(JsonDocument& doc);

private:
    static constexpr int MAX_NONCES = 20;

    String canonicalValue(JsonVariant value);
    String canonicalCommandPayload(JsonDocument& doc);

    String calculateHmac(
        const String& payload
    );

    bool nonceSeen(
        const String& nonce
    );

    void rememberNonce(
        const String& nonce
    );

    String recentNonces_[MAX_NONCES];
    int nonceCount_ = 0;
};