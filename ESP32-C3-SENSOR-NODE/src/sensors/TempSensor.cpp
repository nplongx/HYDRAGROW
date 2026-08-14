#include "TempSensor.h"

TempSensor::TempSensor(int pin)
    : pin_(pin), oneWire_(pin), sensor_(&oneWire_) {}

void TempSensor::begin() {
    sensor_.begin();
}

float TempSensor::read() {
    sensor_.requestTemperatures();
    float rawTemp = sensor_.getTempCByIndex(0);
    
    if (rawTemp == DEVICE_DISCONNECTED_C) {
        lastRawTemperature_ = NAN;
        lastTemperature_ = NAN;
        return NAN;
    }
    
    lastRawTemperature_ = rawTemp;
    lastTemperature_ = rawTemp + offset_;
    return lastTemperature_;
}

void TempSensor::setOffset(float offset) {
    offset_ = offset;
}

float TempSensor::getOffset() const {
    return offset_;
}

float TempSensor::getLastRawTemperature() const {
    return lastRawTemperature_;
}

float TempSensor::getLastTemperature() const {
    return lastTemperature_;
}