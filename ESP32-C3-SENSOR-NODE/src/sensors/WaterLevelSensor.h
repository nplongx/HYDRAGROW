#pragma once

#include <Arduino.h>

class WaterLevelSensor {
public:
    WaterLevelSensor(
        int trigPin,
        int echoPin
    );

    void begin();

    float read();

    void setTankHeight(float height);
    float getTankHeight() const;

    float getLastDistance() const;
    float getLastWaterLevel() const;

private:
    int trigPin_;
    int echoPin_;

    float tankHeight_ = 100.0f;

    float lastDistance_ = NAN;
    float lastWaterLevel_ = NAN;
};