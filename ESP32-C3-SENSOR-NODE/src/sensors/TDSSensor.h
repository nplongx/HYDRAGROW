#pragma once
#include <Arduino.h>
#include <Adafruit_ADS1X15.h>

struct TDSSensorConfig {
    float kValue = 1.0f;                    // Hệ số hiệu chuẩn kValue (mặc định 1.0)
    float ecOffset = 0.0f;                  // Bù trừ offset (mS/cm)
    float tdsFactor = 500.0f;               // Hệ số chuyển đổi PPM (500 scale)
    bool temperatureCompensation = true;
    float temperatureCoefficient = 0.02f;   // 2% / °C
};

class TDSSensor {
public:
    explicit TDSSensor(uint8_t i2cAddress = 0x49);
    bool begin();
    float read(float temperature);          // Trả về EC (mS/cm)
    void setConfig(const TDSSensorConfig& config);
    const TDSSensorConfig& getConfig() const;
    
    float getLastVoltageMv() const;
    float getLastEc() const;                // EC đơn vị mS/cm
    float getLastTds() const;               // TDS quy đổi ppm (EC * 500)
    bool isConnected() const { return isConnected_; }

private:
    Adafruit_ADS1115 ads_;
    uint8_t i2cAddress_;
    TDSSensorConfig config_;
    float lastVoltageMv_ = NAN;
    float lastEc_ = NAN;
    float lastTds_ = NAN;
    bool isConnected_ = false;

    float readVoltageMv();
};
