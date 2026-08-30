#include "WifiProvisioner.h"
#include <ArduinoJson.h>
#include <algorithm>

WifiProvisioner::WifiProvisioner(Preferences& prefs,
                                 const char* fallbackSsid,
                                 const char* fallbackPass)
    : prefs_(prefs), fallbackSsid_(fallbackSsid), fallbackPass_(fallbackPass) {}

std::vector<WifiCandidate> WifiProvisioner::load() {
    prefs_.begin(kNamespace, /*readOnly=*/false);
    String raw = prefs_.getString(kKey, "");
    prefs_.end();

    std::vector<WifiCandidate> result;

    if (raw.length() > 0) {
        JsonDocument doc;
        if (deserializeJson(doc, raw) == DeserializationError::Ok) {
            for (JsonObject obj : doc["candidates"].as<JsonArray>()) {
                WifiCandidate c;
                c.ssid     = obj["ssid"].as<String>();
                c.password = obj["password"].as<String>();
                c.priority = obj["priority"] | 0;
                if (c.ssid.length() > 0) {
                    result.push_back(c);
                }
            }
        }
    }

    if (result.empty() && strlen(fallbackSsid_) > 0) {
        result.push_back({fallbackSsid_, fallbackPass_, 0});
    }

    std::sort(result.begin(), result.end(),
              [](const WifiCandidate& a, const WifiCandidate& b) {
                  return a.priority < b.priority;
              });
    return result;
}

bool WifiProvisioner::save(const std::vector<WifiCandidate>& candidates) {
    JsonDocument doc;
    JsonArray arr = doc["candidates"].to<JsonArray>();
    for (const auto& c : candidates) {
        if (c.ssid.length() == 0) continue;
        JsonObject obj = arr.add<JsonObject>();
        obj["ssid"]     = c.ssid;
        obj["password"] = c.password;
        obj["priority"] = c.priority;
    }
    String out;
    serializeJson(doc, out);

    prefs_.begin(kNamespace, /*readOnly=*/false);
    bool ok = prefs_.putString(kKey, out.c_str());
    prefs_.end();
    return ok;
}
