#include "TDSSensor.h"
#include <math.h>

TDSSensor::TDSSensor(uint8_t i2cAddress)
    : i2cAddress_(i2cAddress) {}

bool TDSSensor::begin() {
    isConnected_ = ads_.begin(i2cAddress_);
    if (isConnected_) {
        ads_.setGain(GAIN_TWOTHIRDS);
    }
    return isConnected_;
}

void TDSSensor::setConfig(const TDSSensorConfig& config) {
    config_ = config;
}

const TDSSensorConfig& TDSSensor::getConfig() const {
    return config_;
}

float TDSSensor::getLastVoltageMv() const { return lastVoltageMv_; }
float TDSSensor::getLastEc() const { return lastEc_; }
float TDSSensor::getLastTds() const { return lastTds_; }
float TDSSensor::getLastVccMv() const { return lastVccMv_; }

float TDSSensor::readDifferentialMv() {
    int32_t sum = 0;
    constexpr int samples = 10;
    for (int i = 0; i < samples; i++) {
        sum += ads_.readADC_Differential_0_1();
        delay(5);
    }
    float rawAvg = static_cast<float>(sum) / samples;
    return ads_.computeVolts(static_cast<int16_t>(rawAvg)) * 1000.0f;
}

float TDSSensor::readVccMv() {
    int16_t rawA3 = ads_.readADC_SingleEnded(3);
    return ads_.computeVolts(rawA3) * 1000.0f;
}

float TDSSensor::calculateEc(float voltageMv, float temperature) {
    float rawEc = (voltageMv / 1000.0f) * config_.ecFactor + config_.ecOffset;
    if (config_.temperatureCompensation) {
        float coefficient = 1.0f + config_.temperatureCoefficient * (temperature - 25.0f);
        if (coefficient > 0.0f) {
            rawEc /= coefficient;
        }
    }
    return max(rawEc, 0.0f);
}

float TDSSensor::calculateTds(float ec) {
    return ec * config_.tdsFactor;
}

float TDSSensor::read(float temperature) {
    if (!isConnected_) { // Bỏ qua nếu không kết nối được ADS1115 TDS
        lastVoltageMv_ = NAN;
        lastEc_ = NAN;
        lastTds_ = NAN;
        return NAN;
    }

    float diffMv = readDifferentialMv();
    float vccMv = readVccMv();
    lastVccMv_ = vccMv;

    if (vccMv <= 1000.0f) {
        lastVoltageMv_ = NAN;
        lastEc_ = NAN;
        lastTds_ = NAN;
        return NAN;
    }

    float compensatedMv = diffMv * (config_.nominalVccMv / vccMv);
    lastVoltageMv_ = compensatedMv;

    float ec = calculateEc(compensatedMv, temperature);
    lastEc_ = ec;

    float tds = calculateTds(ec);
    lastTds_ = tds;
    return tds;
}