#include "TDSSensor.h"
#include <math.h>

TDSSensor::TDSSensor(uint8_t i2cAddress)
    : i2cAddress_(i2cAddress) {}

bool TDSSensor::begin() {
    isConnected_ = ads_.begin(i2cAddress_);
    if (isConnected_) {
        // GAIN_ONE: dải đo ±4.096V (tín hiệu TDS analog từ 0 đến ~2.3V)
        ads_.setGain(GAIN_ONE);
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

float TDSSensor::readVoltageMv() {
    int32_t sum = 0;
    constexpr int samples = 10;
    for (int i = 0; i < samples; i++) {
        // Đọc chân A0 chế độ Single-Ended (tham chiếu GND)
        sum += ads_.readADC_SingleEnded(0);
        delay(5);
    }
    float rawAvg = static_cast<float>(sum) / samples;
    return ads_.computeVolts(static_cast<int16_t>(rawAvg)) * 1000.0f;
}

float TDSSensor::read(float temperature) {
    if (!isConnected_) {
        lastVoltageMv_ = NAN;
        lastEc_ = NAN;
        lastTds_ = NAN;
        return NAN;
    }

    float voltageMv = readVoltageMv();
    lastVoltageMv_ = voltageMv;

    // Chuyển sang Volt để áp dụng công thức đặc tính DFRobot
    float voltageV = voltageMv / 1000.0f;

    // 1. Bù nhiệt độ chuẩn hóa về 25°C
    float compensationCoefficient = 1.0f;
    if (config_.temperatureCompensation && !isnan(temperature) && temperature > 0.0f) {
        compensationCoefficient = 1.0f + config_.temperatureCoefficient * (temperature - 25.0f);
        if (compensationCoefficient <= 0.0f) compensationCoefficient = 1.0f;
    }
    float compensationVoltage = voltageV / compensationCoefficient;

    // 2. Đa thức bậc 3 chuẩn DFRobot tính độ dẫn điện EC (µS/cm)
    float ec_uS = (133.42f * powf(compensationVoltage, 3) 
                 - 255.86f * powf(compensationVoltage, 2) 
                 + 857.39f * compensationVoltage) * config_.kValue;

    if (ec_uS < 0.0f) ec_uS = 0.0f;

    // 3. Quy đổi ra đơn vị chuẩn EC (mS/cm) và TDS (ppm)
    float ec_mS = (ec_uS / 1000.0f) + config_.ecOffset;
    if (ec_mS < 0.0f) ec_mS = 0.0f;

    float tds_ppm = ec_mS * config_.tdsFactor; // ppm = mS/cm * 500

    lastEc_ = ec_mS;
    lastTds_ = tds_ppm;

    // Trả về EC (mS/cm) làm giá trị đo chính
    return ec_mS;
}
