#include "MqttManager.h"

#include <ArduinoJson.h>
#include <WiFi.h>
#include <PubSubClient.h>
#include <WiFiClientSecure.h>
#include <time.h>

#include "../config/RootCA.h"
#include "AppConfig.h"
#include "CommandSecurity.h"
#include "../portal/CaptivePortal.h"
#include "Logger.h"
#include "SensorManager.h"
#include "secrets.h"

namespace {

WiFiClientSecure wifiClient;
PubSubClient mqttClient(wifiClient);
CommandSecurity commandSecurity;
SensorManager* sensorManager = nullptr;
MqttManager* instance = nullptr;

unsigned long lastReconnectAttempt = 0;

const String TOPIC_PREFIX   = String("AGITECH/") + DEVICE_ID + "/";
const String TOPIC_SENSOR   = TOPIC_PREFIX + "sensors";
const String TOPIC_STATUS   = TOPIC_PREFIX + "sensor/status";
const String TOPIC_COMMAND  = TOPIC_PREFIX + "command";
const String TOPIC_CONFIG   = TOPIC_PREFIX + "sensors/config";

// *** KEY FIX: publish KHÔNG được gọi từ bên trong callback ***
// Dùng flag để defer ra update() loop
void publishStatus(const char* status, const char* message) {
    JsonDocument doc;
    doc["device_id"] = MQTT_CLIENT_ID;
    doc["status"] = status;
    doc["message"] = message;

    char buffer[256];
    size_t len = serializeJson(doc, buffer, sizeof(buffer));
    mqttClient.publish(TOPIC_STATUS.c_str(), reinterpret_cast<const uint8_t*>(buffer), len, false);
}

} // namespace

MqttManager::MqttManager(SensorManager& sensors, WifiProvisioner& wifiProvisioner)
    : sensors_(sensors), wifiProvisioner_(wifiProvisioner) {}

void MqttManager::begin() {
    instance = this;
    sensorManager = &sensors_;

    connectWifi();

    if (WiFi.status() != WL_CONNECTED) {
        Logger::debugPrintln("[PORTAL] Khong co WiFi. Mo Captive Portal...");
        bool configured = runCaptivePortal(wifiProvisioner_, 0);
        if (configured) {
            Logger::debugPrintln("[PORTAL] Da luu WiFi. Khoi dong lai...");
            delay(500);
            ESP.restart();
        }
    }

    if (WiFi.status() == WL_CONNECTED) {
        configTime(7 * 3600, 0, "pool.ntp.org", "time.nist.gov");
    }

    wifiClient.setInsecure();
    // wifiClient.setCACert(ROOT_CA);
    mqttClient.setServer(MQTT_HOST, MQTT_PORT);
    mqttClient.setCallback(mqttCallback);
    mqttClient.setBufferSize(2048);
    mqttClient.setKeepAlive(60);      // FIX: tăng từ default 15s lên 60s
    mqttClient.setSocketTimeout(15);  // FIX: timeout rõ ràng
    reconnect();
}

void MqttManager::connectWifi() {
    auto candidates = wifiProvisioner_.load();
    WiFi.mode(WIFI_STA);
    for (const auto& c : candidates) {
        Logger::debugPrintf("Dang ket noi WiFi: %s\n", c.ssid.c_str());
        WiFi.begin(c.ssid.c_str(), c.password.c_str());
        unsigned long start = millis();
        while (WiFi.status() != WL_CONNECTED && millis() - start < 12000) {
            delay(250);
        }
        if (WiFi.status() == WL_CONNECTED) {
            Logger::debugPrintf("WiFi da ket noi. IP=%s\n", WiFi.localIP().toString().c_str());
            configTime(7 * 3600, 0, "pool.ntp.org", "time.nist.gov");
            return;
        }
        Logger::debugPrintf("That bai voi '%s', thu SSID tiep theo...\n", c.ssid.c_str());
        WiFi.disconnect();
    }
    Logger::debugPrintln("Khong ket noi duoc WiFi nao!");
}

void MqttManager::update() {
    if (WiFi.status() != WL_CONNECTED) {
        unsigned long now = millis();
        if (now - lastReconnectAttempt >= 5000) {
            lastReconnectAttempt = now;
            WiFi.disconnect();
            WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
        }
        return;
    }

    if (!mqttClient.connected()) {
        unsigned long now = millis();
        if (now - lastReconnectAttempt >= 5000) {
            lastReconnectAttempt = now;
            reconnect();
        }
        return;
    }

    mqttClient.loop();

    // *** KEY FIX: flush deferred actions SAU khi loop() hoàn thành ***
    // Lúc này buffer của PubSubClient đã free, publish an toàn
    if (pendingStatusOk_) {
        pendingStatusOk_ = false;
        publishStatus("ok", "configuration applied");
    }

    publishSensorIfNeeded();
}

void MqttManager::reconnect() {
    if (WiFi.status() != WL_CONNECTED) return;

    Logger::debugPrintln("Dang ket noi MQTT...");
    bool connected = mqttClient.connect(MQTT_CLIENT_ID, MQTT_USERNAME, MQTT_PASSWORD);

    if (!connected) {
        Logger::debugPrintf("Ket noi MQTT that bai, rc=%d\n", mqttClient.state());
        return;
    }

    Logger::debugPrintln("MQTT da ket noi thanh cong!");

    // FIX: subscribe QoS 0 cho config topic — tránh QoS 1 PUBACK bị corrupt
    // bởi publish() gọi trong callback (cùng buffer)
    mqttClient.subscribe(TOPIC_COMMAND.c_str(), 0);
    mqttClient.subscribe(TOPIC_CONFIG.c_str(), 0);  // ← QoS 0, không cần PUBACK

    publishStatus("online", "Sensor node connected");
}

void MqttManager::mqttCallback(char* topic, byte* payload, unsigned int length) {
    if (!instance) return;

    String message;
    message.reserve(length);
    for (unsigned int i = 0; i < length; i++) {
        message += static_cast<char>(payload[i]);
    }

    if (strcmp(topic, TOPIC_COMMAND.c_str()) == 0) {
        instance->handleCommand(message);
    } else if (strcmp(topic, TOPIC_CONFIG.c_str()) == 0) {
        instance->handleConfig(message);
        // KHÔNG publishStatus ở đây — set flag, flush ở update()
    }
}

void MqttManager::handleCommand(const String& payload) {
    JsonDocument doc;
    if (deserializeJson(doc, payload)) {
        publishStatus("error", "invalid command JSON");
        return;
    }

    if (!commandSecurity.verify(doc)) {
        publishStatus("error", "command authentication failed");
        return;
    }

    const char* command = doc["cmd"] | "";
    if (strcmp(command, "get_status") == 0) {
        publishSensorData();
    } else if (strcmp(command, "restart") == 0) {
        publishStatus("ok", "restarting");
        delay(100);
        ESP.restart();
    } else if (strcmp(command, "update_wifi_list") == 0) {
        handleUpdateWifiList(doc);
    }
}

void MqttManager::handleUpdateWifiList(JsonDocument& doc) {
    JsonArray arr = doc["params"]["candidates"].as<JsonArray>();
    if (arr.isNull()) {
        publishStatus("error", "update_wifi_list: missing candidates array");
        return;
    }
    std::vector<WifiCandidate> candidates;
    for (JsonObject obj : arr) {
        String ssid = obj["ssid"] | "";
        if (ssid.length() == 0) continue;
        candidates.push_back({ssid, obj["password"] | "", obj["priority"] | 0});
    }
    if (candidates.empty()) {
        publishStatus("error", "update_wifi_list: no valid SSIDs");
        return;
    }
    wifiProvisioner_.save(candidates);
    publishStatus("ok", "wifi list updated");
    Logger::debugPrintf("Saved %d WiFi candidates to NVS\n", (int)candidates.size());
}

void MqttManager::handleConfig(const String& payload) {
    JsonDocument doc;
    if (deserializeJson(doc, payload)) {
        // Lỗi parse — không publish từ đây, set flag lỗi nếu cần
        Logger::debugPrintln("[CONFIG] Loi parse JSON config");
        return;
    }
    handleConfigDocument(doc);
}

void MqttManager::handleConfigDocument(JsonDocument& doc) {
    Logger::debugPrintln("[CONFIG] Nhan cau hinh tu Backend, dang ap dung...");
    appConfig.applyFromJson(doc);

    Logger::debugPrintf("[CONFIG] publish_interval=%lu ms, enablePh=%d, enableTds=%d\n",
        appConfig.publishInterval,
        (int)appConfig.sensor.enablePh,
        (int)appConfig.sensor.enableTds);

    // *** KEY FIX: KHÔNG gọi publishStatus() trực tiếp ở đây ***
    // publishStatus() ghi vào mqttClient buffer, nhưng lúc này
    // PubSubClient đang dùng buffer đó để gửi PUBACK (nếu QoS 1).
    // Dù đã subscribe QoS 0, vẫn defer để an toàn tuyệt đối.
    pendingStatusOk_ = true;
}

void MqttManager::publishSensorData() {
    if (!mqttClient.connected()) return;

    const SensorData& data = sensors_.getData();
    JsonDocument doc;

    time_t now = time(nullptr);
    char timeBuffer[30] = "2026-08-18T00:00:00Z";
    if (now > 100000) {
        struct tm* timeinfo = gmtime(&now);
        strftime(timeBuffer, sizeof(timeBuffer), "%Y-%m-%dT%H:%M:%SZ", timeinfo);
    }

    doc["device_id"]      = DEVICE_ID;
    doc["ec"]             = data.tds;
    doc["ph"]             = data.ph;
    doc["temp"]           = data.temperature;
    doc["water_level"]    = data.waterLevel;
    doc["ph_voltage_mv"]  = data.phVoltageMv;
    doc["time"]           = timeBuffer;
    doc["rssi"]           = WiFi.RSSI();
    doc["free_heap"]      = ESP.getFreeHeap();

    doc["err_temp"]       = data.errTemperature;
    doc["err_water"]      = data.errWaterLevel;
    doc["err_ph"]         = data.errPh;
    doc["err_tds"]        = data.errTds;

    char buffer[1024];
    size_t length = serializeJson(doc, buffer, sizeof(buffer));

    mqttClient.publish(TOPIC_SENSOR.c_str(), reinterpret_cast<const uint8_t*>(buffer), length, false);
}

void MqttManager::publishSensorIfNeeded() {
    static unsigned long lastPublish = 0;
    unsigned long now = millis();

    if (now - lastPublish < appConfig.publishInterval) {
        return;
    }
    lastPublish = now;
    publishSensorData();
}
