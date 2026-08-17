#include "SensorManager.h"
#include <Wire.h>
#include "Logger.h"

namespace {
// Pin configuration
constexpr int PIN_DS18B20 = 2;
constexpr int PIN_TRIG = 3;
constexpr int PIN_ECHO = 5;

constexpr int PIN_SDA     = 6; // GPIO 6 làm SDA
constexpr int PIN_SCL     = 7; // GPIO 7 làm SCL

// Địa chỉ I2C cho 2 ADS1115
constexpr uint8_t ADS_PH_ADDR  = 0x48; // Chân ADDR nối GND
constexpr uint8_t ADS_TDS_ADDR = 0x49; // Chân ADDR nối VCC
} // namespace

SensorManager::SensorManager()
    : tempSensor_(PIN_DS18B20),
      waterLevelSensor_(PIN_TRIG, PIN_ECHO),
      phSensor_(ADS_PH_ADDR),
      tdsSensor_(ADS_TDS_ADDR),
      tempFilter_(5.0f, 0.125f),
      waterFilter_(20.0f, 0.125f),
      phFilter_(1.5f, 0.125f),
      tdsFilter_(1.0f, 0.125f) {}

void SensorManager::begin() {
    // Khởi tạo bus I2C (mặc định chân SDA/SCL trên ESP32-C3)
    Wire.begin(PIN_SDA, PIN_SCL);

    tempSensor_.begin();
    waterLevelSensor_.begin();
    
    if (!phSensor_.begin()) {
        Logger::debugPrintln("Error: ADS1115 pH not found!");
    }
    if (!tdsSensor_.begin()) {
        Logger::debugPrintln("Error: ADS1115 TDS not found!");
    }
}

void SensorManager::update() {
    unsigned long now = millis();
    if (now - lastSampleTime_ < SAMPLE_INTERVAL_MS) {
        return;
    }
    lastSampleTime_ = now;

    updateTemperature();
    updateWaterLevel();
    updatePh();
    updateTds();
}

const SensorData& SensorManager::getData() const {
    return data_;
}

void SensorManager::setTankHeight(float height) {
    waterLevelSensor_.setTankHeight(height);
}

void SensorManager::enableTemperature(bool enabled) { enableTemperature_ = enabled; }
void SensorManager::enableWaterLevel(bool enabled)  { enableWaterLevel_ = enabled; }
void SensorManager::enablePh(bool enabled)          { enablePh_ = enabled; }
void SensorManager::enableTds(bool enabled)         { enableTds_ = enabled; }

// ============================================================
// Private Sensor Update Methods
// ============================================================

void SensorManager::updateTemperature() {
    if (!enableTemperature_) return;
    float raw = tempSensor_.read();
    if (isnan(raw)) {
        data_.errTemperature = true;
    } else {
        data_.errTemperature = false;
        data_.temperature = tempFilter_.update(raw);
    }
}

void SensorManager::updateWaterLevel() {
    if (!enableWaterLevel_) return;
    float raw = waterLevelSensor_.read();
    data_.rawWaterLevel = raw;
    if (isnan(raw)) {
        data_.errWaterLevel = true;
    } else {
        data_.errWaterLevel = false;
        data_.waterLevel = waterFilter_.update(raw);
    }
}

void SensorManager::updatePh() {
    if (!enablePh_) return;
    float raw = phSensor_.read(data_.temperature);
    data_.rawPh = raw;
    data_.phVoltageMv = phSensor_.getLastVoltageMv();
    if (isnan(raw)) {
        data_.errPh = true;
    } else {
        data_.errPh = false;
        data_.ph = phFilter_.update(raw);
    }
}

void SensorManager::updateTds() {
    if (!enableTds_) return;
    float raw = tdsSensor_.read(data_.temperature);
    if (isnan(raw)) {
        data_.errTds = true;
    } else {
        data_.errTds = false;
        data_.tds = tdsFilter_.update(raw);
    }
}
