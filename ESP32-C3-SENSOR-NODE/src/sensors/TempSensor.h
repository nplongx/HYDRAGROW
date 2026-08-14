#pragma once

#include <Arduino.h>
#include <OneWire.h>
#include <DallasTemperature.h>

class TempSensor {
public:
    explicit TempSensor(int pin);

    void begin();

    float read();

    void setOffset(float offset);
    float getOffset() const;

    float getLastRawTemperature() const;
    float getLastTemperature() const;

private:
    int pin_;

    OneWire oneWire_;
    DallasTemperature sensor_;

    float offset_ = 0.0f;

    float lastRawTemperature_ = NAN;
    float lastTemperature_ = NAN;
};