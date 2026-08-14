#pragma once
#include <Arduino.h>
#include <Adafruit_ADS1X15.h>

struct TDSSensorConfig {
    float ecFactor = 0.88f;
    float ecOffset = 0.0f;
    float tdsFactor = 500.0f;
    bool temperatureCompensation = true;
    float temperatureCoefficient = 0.02f;
    float nominalVccMv = 5000.0f;
};

class TDSSensor {
public:
    explicit TDSSensor(uint8_t i2cAddress = 0x49);
    bool begin();
    float read(float temperature);
    void setConfig(const TDSSensorConfig& config);
    const TDSSensorConfig& getConfig() const;
    float getLastVoltageMv() const;
    float getLastEc() const;
    float getLastTds() const;
    float getLastVccMv() const;
    bool isConnected() const { return isConnected_; } // Thêm hàm kiểm tra

private:
    Adafruit_ADS1115 ads_;
    uint8_t i2cAddress_;
    TDSSensorConfig config_;
    float lastVoltageMv_ = NAN;
    float lastEc_ = NAN;
    float lastTds_ = NAN;
    float lastVccMv_ = NAN;
    bool isConnected_ = false;

    float readDifferentialMv();
    float readVccMv();
    float calculateEc(float voltageMv, float temperature);
    float calculateTds(float ec);
};