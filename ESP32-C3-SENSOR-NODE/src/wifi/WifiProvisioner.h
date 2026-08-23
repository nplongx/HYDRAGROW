#pragma once
#include <Arduino.h>
#include <Preferences.h>
#include <vector>

struct WifiCandidate {
    String ssid;
    String password;
    uint8_t priority;
};

class WifiProvisioner {
public:
    // fallbackSsid/Pass: từ secrets.h, dùng khi NVS rỗng
    WifiProvisioner(Preferences& prefs,
                    const char* fallbackSsid = "",
                    const char* fallbackPass = "");

    // Load danh sách từ NVS, sorted by priority. Nếu rỗng, trả về fallback.
    std::vector<WifiCandidate> load();

    // Ghi danh sách lên NVS (gọi từ MQTT handler).
    bool save(const std::vector<WifiCandidate>& candidates);

private:
    Preferences& prefs_;
    const char* fallbackSsid_;
    const char* fallbackPass_;

    static constexpr const char* kNamespace = "agitech";
    static constexpr const char* kKey       = "wifi_list";
};
