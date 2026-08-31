#include <unity.h>
#include <Arduino.h>
#include <cmath>
// Stub Preferences để test trên host
#include "../test/stubs/Preferences.h"
#include "../src/wifi/WifiProvisioner.cpp"
#include "../src/filters/HybridFilter.cpp"

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

void test_ph_filter_initialization_on_boot() {
    // HybridFilter configured as in SensorManager for pH (deltaMax = 1.5f, alpha = 0.125f)
    HybridFilter phFilter(1.5f, 0.125f);

    // Boot step 1: PhSensor returns NAN when ADC is unready / diffMv <= 500mV.
    // SensorManager checks std::isnan(raw) and skips phFilter.update(raw).
    float raw_boot = NAN;
    if (!std::isnan(raw_boot)) {
        phFilter.update(raw_boot);
    }

    // Boot step 2: First valid sample (6.86f) arrives once ADC conversion completes.
    float ph_output = phFilter.update(6.86f);

    // Filter initializes cleanly at 6.86f without rate-limiting from 0.0
    TEST_ASSERT_FLOAT_WITHIN(0.001f, 6.86f, ph_output);
}

void test_ph_filter_rate_limiting_step_change() {
    HybridFilter phFilter(1.5f, 0.125f);
    phFilter.update(6.86f);

    // Large jump (delta = 3.0 > deltaMax 1.5)
    float updated = phFilter.update(9.86f);

    // Limited delta = 1.5 => xLimited = 6.86 + 1.5 = 8.36
    // y = 0.125 * 8.36 + 0.875 * 6.86 = 1.045 + 6.0025 = 7.0475
    TEST_ASSERT_FLOAT_WITHIN(0.01f, 7.0475f, updated);
}

int main(int argc, char **argv) {
    UNITY_BEGIN();
    RUN_TEST(test_load_returns_empty_when_nvs_empty);
    RUN_TEST(test_save_and_load_roundtrip);
    RUN_TEST(test_load_with_fallback_secret_when_nvs_empty);
    RUN_TEST(test_ph_filter_initialization_on_boot);
    RUN_TEST(test_ph_filter_rate_limiting_step_change);
    return UNITY_END();
}
