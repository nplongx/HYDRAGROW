#include <Arduino.h>

#include "Logger.h"
#include "SensorManager.h"
#include "MqttManager.h"
#include "AppConfig.h"

SensorManager sensorManager;
MqttManager mqttManager(sensorManager);

void setup() {
    Serial.begin(115200);

    Logger::begin();
    Logger::setDebugEnabled(
        appConfig.debugLog
    );

    sensorManager.begin();
    mqttManager.begin();
}

void loop() {
    sensorManager.update();
    mqttManager.update();
}