#include "PhSensor.h"
#include <math.h>

PhSensor::PhSensor(uint8_t i2cAddress)
    : i2cAddress_(i2cAddress) {}

bool PhSensor::begin() {
    isConnected_ = ads_.begin(i2cAddress_);
    if (isConnected_) {
        ads_.setGain(GAIN_TWOTHIRDS);
    }
    return isConnected_;
}

void PhSensor::setConfig(const PhSensorConfig& config) {
    config_ = config;
}

const PhSensorConfig& PhSensor::getConfig() const {
    return config_;
}

float PhSensor::getLastVoltageMv() const { return lastVoltageMv_; }
float PhSensor::getLastRawPh() const { return lastRawPh_; }
float PhSensor::getLastVccMv() const { return lastVccMv_; }

float PhSensor::readDifferentialMv() {
    int32_t sum = 0;
    constexpr int samples = 10;
    for (int i = 0; i < samples; i++) {
        sum += ads_.readADC_Differential_0_1();
        delay(5);
    }
    float rawAvg = static_cast<float>(sum) / samples;
    return ads_.computeVolts(static_cast<int16_t>(rawAvg)) * 1000.0f;
}

float PhSensor::readVccMv() {
    // Đo Single-Ended trên chân A3
    int16_t rawA3 = ads_.readADC_SingleEnded(3);
    return ads_.computeVolts(rawA3) * 1000.0f;
}

float PhSensor::calculatePh(float voltageMv, float currentTemperature) {
    float slope;
    float basePh;
    float baseV;

    if (config_.calibrationMode == "3-point") {
        if (voltageMv > config_.v686) {
            float diff = config_.v4 - config_.v686;
            slope = (fabs(diff) < 0.1f) ? -0.006f : ((4.0f - 6.86f) / diff);
            basePh = 6.86f;
            baseV = config_.v686;
        } else {
            float diff = config_.v686 - config_.v918;
            slope = (fabs(diff) < 0.1f) ? -0.006f : ((6.86f - 9.18f) / diff);
            basePh = 9.18f;
            baseV = config_.v918;
        }
    } else {
        float diff = config_.v4 - config_.v686;
        slope = (fabs(diff) < 0.1f) ? -0.006f : ((4.0f - 6.86f) / diff);
        basePh = 6.86f;
        baseV = config_.v686;
    }

    if (config_.enableTemperatureCompensation) {
        float tempRatio = (currentTemperature + 273.15f) / (25.0f + 273.15f);
        slope /= tempRatio;
    }

    float result = basePh + slope * (voltageMv - baseV);
    return constrain(result, 0.0f, 14.0f);
}

float PhSensor::read(float currentTemperature) {

    if (!isConnected_) { // Bỏ qua nếu không kết nối ADS1115
        lastVoltageMv_ = NAN;
        lastRawPh_ = NAN;
        return NAN;
    }

    float diffMv = readDifferentialMv();
    float vccMv = readVccMv();
    lastVccMv_ = vccMv;

    if (isnan(diffMv) || isnan(vccMv) || vccMv <= 1000.0f || diffMv <= 500.0f) { // Kiểm tra sụt áp quá mức, lỗi phần cứng hoặc chưa ready (< 500mV ngoài dải pH 0-14)
        lastVoltageMv_ = NAN;
        lastRawPh_ = NAN;
        return NAN;
    }

    // Bù sai lệch điện áp VCC thực tế so với chuẩn (5000mV)
    float compensatedMv = diffMv * (config_.nominalVccMv / vccMv);
    lastVoltageMv_ = compensatedMv;

    float ph = calculatePh(compensatedMv, currentTemperature);
    lastRawPh_ = ph;
    return ph;
}