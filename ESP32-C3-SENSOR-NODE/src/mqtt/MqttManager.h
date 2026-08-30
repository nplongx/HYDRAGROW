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

    // Defer publish ra ngoài MQTT callback để tránh buffer corruption
    bool pendingStatusOk_ = false;
    bool pendingPublishSensor_ = false;

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
