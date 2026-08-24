#pragma once
#include <Arduino.h>
#include <ArduinoJson.h>

struct SensorConfig {
    float phV686;           // ph_v7
    float phV4;             // ph_v4
    float phV918;           // ph_v10
    float tdsFactor;        // ec_factor
    float tdsOffset;        // ec_offset
    float tempOffset;       // temp_offset
    float tankHeight;
    bool enablePh;          // enable_ph_sensor
    bool enableTds;         // enable_ec_sensor
    bool enableTemp;        // enable_temp_sensor
    bool enableWater;       // enable_water_level_sensor
};

struct AppConfig {
    SensorConfig sensor;
    unsigned long publishInterval;   // publish_interval (ms)
    bool debugLog;
    bool continuousLevel;

    /// Áp cấu hình từ JSON document do backend gửi.
    /// Chỉ cập nhật field nào có trong JSON (bảo toàn giá trị còn lại).
    void applyFromJson(const JsonDocument& doc) {
        if (!doc["ph_v7"].isNull())              sensor.phV686  = doc["ph_v7"].as<float>();
        if (!doc["ph_v4"].isNull())              sensor.phV4    = doc["ph_v4"].as<float>();
        if (!doc["ph_v10"].isNull())             sensor.phV918  = doc["ph_v10"].as<float>();
        if (!doc["ec_factor"].isNull())          sensor.tdsFactor = doc["ec_factor"].as<float>();
        if (!doc["ec_offset"].isNull())          sensor.tdsOffset = doc["ec_offset"].as<float>();
        if (!doc["temp_offset"].isNull())        sensor.tempOffset = doc["temp_offset"].as<float>();
        if (!doc["enable_ph_sensor"].isNull())   sensor.enablePh  = doc["enable_ph_sensor"].as<bool>();
        if (!doc["enable_ec_sensor"].isNull())   sensor.enableTds = doc["enable_ec_sensor"].as<bool>();
        if (!doc["enable_temp_sensor"].isNull()) sensor.enableTemp = doc["enable_temp_sensor"].as<bool>();
        if (!doc["enable_water_level_sensor"].isNull())
            sensor.enableWater = doc["enable_water_level_sensor"].as<bool>();
        if (!doc["publish_interval"].isNull())
            publishInterval = (unsigned long)doc["publish_interval"].as<int>();
    }
};

extern AppConfig appConfig;
