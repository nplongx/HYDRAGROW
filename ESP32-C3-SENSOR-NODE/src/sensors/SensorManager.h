#pragma once

#include <Arduino.h>

#include "TempSensor.h"
#include "WaterLevelSensor.h"
#include "PhSensor.h"
#include "TDSSensor.h"
#include "HybridFilter.h"

struct SensorData {
    float temperature = 25.0f;

    float waterLevel = 20.0f;
    float rawWaterLevel = 20.0f;

    float ph = 6.86f;
    float rawPh = 6.86f;
    float phVoltageMv = NAN;

    float tds = 0.0f;

    bool errTemperature = false;
    bool errWaterLevel = false;
    bool errPh = false;
    bool errTds = false;
};

class SensorManager {
public:
    SensorManager();

    void begin();
    void update();

    const SensorData& getData() const;

    void setPublishInterval(int interval);
    void setTankHeight(float height);

    void enableTemperature(bool enabled);
    void enableWaterLevel(bool enabled);
    void enablePh(bool enabled);
    void enableTds(bool enabled);

private:
    TempSensor tempSensor_;
    WaterLevelSensor waterLevelSensor_;
    PhSensor phSensor_;
    TDSSensor tdsSensor_;

    HybridFilter tempFilter_;
    HybridFilter waterFilter_;
    HybridFilter phFilter_;
    HybridFilter tdsFilter_;

    SensorData data_;

    bool enableTemperature_ = true;
    bool enableWaterLevel_ = true;
    bool enablePh_ = true;
    bool enableTds_ = true;

    unsigned long lastSampleTime_ = 0;

    static constexpr unsigned long SAMPLE_INTERVAL_MS = 200;

    void updateTemperature();
    void updateWaterLevel();
    void updatePh();
    void updateTds();
};