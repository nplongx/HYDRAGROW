#pragma once
#include <Arduino.h>
#include <ArduinoJson.h>
#include "../wifi/WifiProvisioner.h"

class SensorManager;

class MqttManager {
public:
    explicit MqttManager(SensorManager& sensors, WifiProvisioner& wifiProvisioner);
    void begin();
    void update();

private:
    SensorManager& sensors_;

    WifiProvisioner& wifiProvisioner_;

    // Chuyển thành static
    static void mqttCallback(char* topic, byte* payload, unsigned int length);

    void reconnect();
    void connectWifi();
    void handleCommand(const String& payload);
    void handleConfig(const String& payload);
    void handleConfigDocument(JsonDocument& doc);
    void handleUpdateWifiList(JsonDocument& doc);
    void publishSensorData();
    void publishSensorIfNeeded();
};