#include "CommandSecurity.h"

#include <mbedtls/md.h>

#include "secrets.h"
#include "Logger.h"

CommandSecurity::CommandSecurity() {
}

String CommandSecurity::canonicalValue(JsonVariant value) {
    // Chuyển logic jsonValueToCanonical()
    // từ code cũ sang đây.
}

String CommandSecurity::canonicalCommandPayload(
    JsonDocument& doc
) {
    // Chuyển logic canonicalCommandPayload()
    // từ code cũ sang đây.
}

String CommandSecurity::calculateHmac(
    const String& payload
) {
    // Chuyển logic hmacSha256Hex()
    // từ code cũ sang đây.
}

bool CommandSecurity::nonceSeen(
    const String& nonce
) {
    for (int i = 0; i < nonceCount_; i++) {
        if (recentNonces_[i] == nonce) {
            return true;
        }
    }

    return false;
}

void CommandSecurity::rememberNonce(
    const String& nonce
) {
    if (nonceCount_ < MAX_NONCES) {
        recentNonces_[nonceCount_++] = nonce;
        return;
    }

    for (int i = 1; i < MAX_NONCES; i++) {
        recentNonces_[i - 1] = recentNonces_[i];
    }

    recentNonces_[MAX_NONCES - 1] = nonce;
}

bool CommandSecurity::verify(
    JsonDocument& doc
) {
    // Chuyển nguyên logic verifyCommand()
    // từ code cũ sang đây.
}