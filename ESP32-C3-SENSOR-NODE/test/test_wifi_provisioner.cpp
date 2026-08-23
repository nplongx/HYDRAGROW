#include <unity.h>
#include <Arduino.h>
// Stub Preferences để test trên host
#include "../test/stubs/Preferences.h"
#include "../src/wifi/WifiProvisioner.cpp"

void test_load_returns_empty_when_nvs_empty() {
    Preferences prefs;  // stub: không có key nào
    WifiProvisioner prov(prefs);
    auto candidates = prov.load();
    TEST_ASSERT_EQUAL(0, candidates.size());
}

void test_save_and_load_roundtrip() {
    Preferences prefs;
    WifiProvisioner prov(prefs);
    std::vector<WifiCandidate> list = {
        {"SSID_A", "pass_a", 0},
        {"SSID_B", "pass_b", 1}
    };
    prov.save(list);
    auto loaded = prov.load();
    TEST_ASSERT_EQUAL(2, loaded.size());
    TEST_ASSERT_EQUAL_STRING("SSID_A", loaded[0].ssid.c_str());
    TEST_ASSERT_EQUAL(0, loaded[0].priority);
}

void test_load_with_fallback_secret_when_nvs_empty() {
    Preferences prefs;
    WifiProvisioner prov(prefs, "FALLBACK_SSID", "FALLBACK_PASS");
    auto candidates = prov.load();
    TEST_ASSERT_EQUAL(1, candidates.size());
    TEST_ASSERT_EQUAL_STRING("FALLBACK_SSID", candidates[0].ssid.c_str());
}

int main(int argc, char **argv) {
    UNITY_BEGIN();
    RUN_TEST(test_load_returns_empty_when_nvs_empty);
    RUN_TEST(test_save_and_load_roundtrip);
    RUN_TEST(test_load_with_fallback_secret_when_nvs_empty);
    return UNITY_END();
}
