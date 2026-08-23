#include <Arduino.h>

#include "Logger.h"
#include <Preferences.h>
#include "SensorManager.h"
#include "MqttManager.h"
#include "AppConfig.h"
#include "wifi/WifiProvisioner.h"
#include "secrets.h"

SensorManager sensorManager;
Preferences prefs;
WifiProvisioner wifiProvisioner(prefs, WIFI_SSID, WIFI_PASSWORD);
MqttManager mqttManager(sensorManager, wifiProvisioner);

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