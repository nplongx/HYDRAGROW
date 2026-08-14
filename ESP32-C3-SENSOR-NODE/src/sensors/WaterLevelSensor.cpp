#include "WaterLevelSensor.h"

namespace {

constexpr float SOUND_SPEED_CM_PER_US = 0.0343f;
constexpr unsigned long ECHO_TIMEOUT_US = 20000;

}

WaterLevelSensor::WaterLevelSensor(
    int trigPin,
    int echoPin
)
    : trigPin_(trigPin),
      echoPin_(echoPin) {
}

void WaterLevelSensor::begin() {
    pinMode(trigPin_, OUTPUT);
    pinMode(echoPin_, INPUT);

    digitalWrite(trigPin_, LOW);
}

void WaterLevelSensor::setTankHeight(float height) {
    tankHeight_ = height;
}

float WaterLevelSensor::getTankHeight() const {
    return tankHeight_;
}

float WaterLevelSensor::getLastDistance() const {
    return lastDistance_;
}

float WaterLevelSensor::getLastWaterLevel() const {
    return lastWaterLevel_;
}

float WaterLevelSensor::read() {
    // Trigger ultrasonic pulse
    digitalWrite(trigPin_, LOW);
    delayMicroseconds(2);

    digitalWrite(trigPin_, HIGH);
    delayMicroseconds(20);

    digitalWrite(trigPin_, LOW);

    // Đo thời gian echo
    unsigned long duration =
        pulseIn(
            echoPin_,
            HIGH,
            ECHO_TIMEOUT_US
        );

    if (duration == 0) {
        lastDistance_ = NAN;
        lastWaterLevel_ = NAN;

        return NAN;
    }

    // Tính khoảng cách từ sensor đến mặt nước
    float distance =
        (duration / 2.0f) *
        SOUND_SPEED_CM_PER_US;

    lastDistance_ = distance;

    // Mực nước = chiều cao bồn - khoảng cách
    float waterLevel =
        tankHeight_ - distance;

    // Không cho phép âm
    if (waterLevel < 0.0f) {
        waterLevel = 0.0f;
    }

    lastWaterLevel_ = waterLevel;

    return waterLevel;
}