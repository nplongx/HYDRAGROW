#pragma once
#include <Arduino.h>
#include <Adafruit_ADS1X15.h>

struct PhSensorConfig {
    float v686 = 2650.0f;
    float v4   = 3555.0f;
    float v918 = 1750.0f;
    String calibrationMode = "2-point";
    bool enableTemperatureCompensation = true;
    float nominalVccMv = 5000.0f; // Điện áp VCC chuẩn (mV)
};

class PhSensor {
public:
    explicit PhSensor(uint8_t i2cAddress = 0x48);
    bool begin();
    float read(float currentTemperature);
    void setConfig(const PhSensorConfig& config);
    const PhSensorConfig& getConfig() const;
    float getLastVoltageMv() const;
    float getLastRawPh() const;
    float getLastVccMv() const;
    bool isConnected() const { return isConnected_; } // Thêm hàm kiểm tra

private:
    Adafruit_ADS1115 ads_;
    uint8_t i2cAddress_;
    PhSensorConfig config_;
    float lastVoltageMv_ = NAN;
    float lastRawPh_ = NAN;
    float lastVccMv_ = NAN;
    bool isConnected_ = false;

    float readDifferentialMv();
    float readVccMv();
    float calculatePh(float voltageMv, float currentTemperature);
};