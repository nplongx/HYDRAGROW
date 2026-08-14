#pragma once
#include <Arduino.h>
#include <ArduinoJson.h>

class SensorManager;

class MqttManager {
public:
    explicit MqttManager(SensorManager& sensors);
    void begin();
    void update();

private:
    SensorManager& sensors_;

    // Chuyển thành static
    static void mqttCallback(char* topic, byte* payload, unsigned int length);

    void reconnect();
    void handleCommand(const String& payload);
    void handleConfig(const String& payload);
    void handleConfigDocument(JsonDocument& doc);
    void publishSensorData();
    void publishSensorIfNeeded();
};