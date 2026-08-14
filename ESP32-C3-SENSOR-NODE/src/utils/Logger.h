#pragma once

#include <Arduino.h>

namespace Logger {

void begin();

bool isDebugEnabled();
void setDebugEnabled(bool enabled);

void debugPrintf(const char* format, ...);
void debugPrintln(const char* message);

String redactSecret(const String& value);

}