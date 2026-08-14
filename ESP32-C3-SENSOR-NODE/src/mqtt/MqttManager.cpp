#include "MqttManager.h"

#include <ArduinoJson.h>
#include <WiFi.h>
#include <PubSubClient.h>

#include "AppConfig.h"
#include "CommandSecurity.h"
#include "Logger.h"
#include "SensorManager.h"
#include "secrets.h"
#include <WiFiClientSecure.h>
#include "RootCA.h"

// ============================================================
// MQTT / WiFi objects
// ============================================================

namespace {

WiFiClientSecure wifiClient;
PubSubClient mqttClient(wifiClient);

CommandSecurity commandSecurity;

SensorManager* sensorManager = nullptr;

MqttManager* instance = nullptr;

// MQTT connection state
unsigned long lastReconnectAttempt = 0;

// ============================================================
// MQTT topics
// ============================================================

constexpr const char* TOPIC_SENSOR  = "device/sensor";
constexpr const char* TOPIC_STATUS  = "device/status";
constexpr const char* TOPIC_COMMAND = "device/command";
constexpr const char* TOPIC_CONFIG  = "device/config";

// ============================================================
// Status messages
// ============================================================

void publishStatus(
    const char* status,
    const char* message
) {
    JsonDocument doc;

    doc["status"] = status;
    doc["message"] = message;

    char buffer[256];

    serializeJson(doc, buffer, sizeof(buffer));

    mqttClient.publish(
        TOPIC_STATUS,
        buffer,
        false
    );
}

} // namespace


// ============================================================
// Constructor
// ============================================================

MqttManager::MqttManager(
    SensorManager& sensors
)
    : sensors_(sensors) {
}


// ============================================================
// begin()
// ============================================================

void MqttManager::begin() {
    instance = this;
    sensorManager = &sensors_;

    // WiFi
    WiFi.mode(WIFI_STA);
    WiFi.begin(WIFI_SSID, WIFI_PASSWORD);

    Logger::debugPrintf("Connecting WiFi: %s\n", WIFI_SSID);
    unsigned long start = millis();
    while (WiFi.status() != WL_CONNECTED && millis() - start < 15000) {
        delay(250);
        Logger::debugPrintln("Waiting for WiFi...");
    }

    if (WiFi.status() == WL_CONNECTED) {
        Logger::debugPrintf("WiFi connected. IP=%s\n", WiFi.localIP().toString().c_str());
    } else {
        Logger::debugPrintln("WiFi connection timeout");
    }

    // Bỏ qua xác thực CA (Phù hợp cho public test broker)
    wifiClient.setInsecure();

    // MQTT
    mqttClient.setServer(MQTT_HOST, MQTT_PORT);
    mqttClient.setCallback(mqttCallback);
    mqttClient.setBufferSize(2048);
    reconnect();
}


// ============================================================
// update()
// ============================================================

void MqttManager::update() {

    // WiFi mất kết nối
    if (WiFi.status() != WL_CONNECTED) {

        unsigned long now = millis();

        if (now - lastReconnectAttempt >= 5000) {

            lastReconnectAttempt = now;

            WiFi.disconnect();
            WiFi.begin(
                WIFI_SSID,
                WIFI_PASSWORD
            );
        }

        return;
    }

    // MQTT mất kết nối
    if (!mqttClient.connected()) {

        unsigned long now = millis();

        if (now - lastReconnectAttempt >= 5000) {

            lastReconnectAttempt = now;

            reconnect();
        }

        return;
    }

    mqttClient.loop();

    publishSensorIfNeeded();
}


// ============================================================
// MQTT reconnect
// ============================================================

void MqttManager::reconnect() {

    if (WiFi.status() != WL_CONNECTED) {
        return;
    }

    Logger::debugPrintln(
        "Connecting MQTT..."
    );

    bool connected = mqttClient.connect(
        MQTT_CLIENT_ID,
        MQTT_USERNAME,
        MQTT_PASSWORD
    );

    if (!connected) {

        Logger::debugPrintf(
            "MQTT connection failed, state=%d\n",
            mqttClient.state()
        );

        return;
    }

    Logger::debugPrintln(
        "MQTT connected"
    );

    mqttClient.subscribe(
        TOPIC_COMMAND
    );

    mqttClient.subscribe(
        TOPIC_CONFIG
    );

    publishStatus(
        "online",
        "device connected"
    );
}


// ============================================================
// MQTT callback
// ============================================================

void MqttManager::mqttCallback(char* topic, byte* payload, unsigned int length) {
    if (!instance) return;

    String message;
    message.reserve(length);
    for (unsigned int i = 0; i < length; i++) {
        message += static_cast<char>(payload[i]);
    }

    Logger::debugPrintf("MQTT RX topic=%s\n", topic);

    if (strcmp(topic, TOPIC_COMMAND) == 0) {
        instance->handleCommand(message);
        return;
    }

    if (strcmp(topic, TOPIC_CONFIG) == 0) {
        instance->handleConfig(message);
        return;
    }
}


// ============================================================
// Command
// ============================================================

void MqttManager::handleCommand(
    const String& payload
) {
    JsonDocument doc;

    DeserializationError error =
        deserializeJson(
            doc,
            payload
        );

    if (error) {

        Logger::debugPrintln(
            "Invalid command JSON"
        );

        publishStatus(
            "error",
            "invalid command JSON"
        );

        return;
    }

    // --------------------------------------------------------
    // Security
    // --------------------------------------------------------

    if (!commandSecurity.verify(doc)) {

        Logger::debugPrintln(
            "Command rejected: invalid signature"
        );

        publishStatus(
            "error",
            "command authentication failed"
        );

        return;
    }

    const char* command =
        doc["cmd"] | "";

    if (strcmp(command, "set_config") == 0) {

        handleConfigDocument(doc);

        return;
    }

    if (strcmp(command, "get_status") == 0) {

        publishSensorData();

        return;
    }

    if (strcmp(command, "restart") == 0) {

        publishStatus(
            "ok",
            "restarting"
        );

        delay(100);

        ESP.restart();

        return;
    }

    Logger::debugPrintf(
        "Unknown command: %s\n",
        command
    );

    publishStatus(
        "error",
        "unknown command"
    );
}


// ============================================================
// Config
// ============================================================

void MqttManager::handleConfig(
    const String& payload
) {
    JsonDocument doc;

    DeserializationError error =
        deserializeJson(
            doc,
            payload
        );

    if (error) {

        publishStatus(
            "error",
            "invalid config JSON"
        );

        return;
    }

    if (!commandSecurity.verify(doc)) {

        publishStatus(
            "error",
            "config authentication failed"
        );

        return;
    }

    handleConfigDocument(doc);
}


void MqttManager::handleConfigDocument(
    JsonDocument& doc
) {
    // Phần validate/apply configuration
    // nên nằm trong AppConfig.
    //
    // Ví dụ:
    //
    // if (!appConfig.apply(doc)) {
    //     publishStatus("error", "invalid config");
    //     return;
    // }

    Logger::debugPrintln(
        "Configuration received"
    );

    publishStatus(
        "ok",
        "configuration accepted"
    );
}


// ============================================================
// Publish sensor data
// ============================================================

void MqttManager::publishSensorData() {

    if (!mqttClient.connected()) {
        return;
    }

    const SensorData& data =
        sensors_.getData();

    JsonDocument doc;

    doc["temperature"] = data.temperature;

    doc["water_level"] =
        data.waterLevel;

    doc["ph"] =
        data.ph;

    doc["ph_raw"] =
        data.rawPh;

    doc["ph_voltage_mv"] =
        data.phVoltageMv;

    doc["tds"] =
        data.tds;

    doc["err_temperature"] =
        data.errTemperature;

    doc["err_water_level"] =
        data.errWaterLevel;

    doc["err_ph"] =
        data.errPh;

    doc["err_tds"] =
        data.errTds;

    char buffer[1024];

    size_t length =
        serializeJson(
            doc,
            buffer,
            sizeof(buffer)
        );

    mqttClient.publish(
        TOPIC_SENSOR,
        reinterpret_cast<const uint8_t*>(buffer),
        length,
        false
    );
}


// ============================================================
// Publish interval
// ============================================================

void MqttManager::publishSensorIfNeeded() {

    static unsigned long lastPublish = 0;

    unsigned long now = millis();

    if (
        now - lastPublish <
        appConfig.publishInterval
    ) {
        return;
    }

    lastPublish = now;

    publishSensorData();
}