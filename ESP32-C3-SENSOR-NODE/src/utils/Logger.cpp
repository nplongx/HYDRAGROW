#include "Logger.h"

#include <stdarg.h>

namespace {

bool debugEnabled = false;

}

namespace Logger {

void begin() {
    // Hiện Serial được khởi tạo ở main.cpp.
}

bool isDebugEnabled() {
    return debugEnabled;
}

void setDebugEnabled(bool enabled) {
    debugEnabled = enabled;
}

void debugPrintf(const char* format, ...) {
    if (!debugEnabled) {
        return;
    }

    char buffer[256];

    va_list args;
    va_start(args, format);

    vsnprintf(buffer, sizeof(buffer), format, args);

    va_end(args);

    Serial.print(buffer);
}

void debugPrintln(const char* message) {
    if (!debugEnabled) {
        return;
    }

    Serial.println(message);
}

String redactSecret(const String& value) {
    if (value.length() <= 8) {
        return "********";
    }

    return value.substring(0, 4)
         + "****"
         + value.substring(value.length() - 4);
}

}