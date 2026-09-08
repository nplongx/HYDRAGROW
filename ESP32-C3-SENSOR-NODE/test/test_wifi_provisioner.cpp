#include <unity.h>
#include <Arduino.h>
#include <cmath>
// Stub Preferences để test trên host
#include "../test/stubs/Preferences.h"
#include "../src/wifi/WifiProvisioner.cpp"
#include "../src/filters/HybridFilter.cpp"
#include "../src/ota/OtaVersionCheck.cpp"
#include "../src/ota/OtaValidationGate.h"

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

void test_no_update_when_tag_matches_current_version() {
    TEST_ASSERT_FALSE(OtaVersionCheck::isUpdateAvailable("v0.1.0", "v0.1.0"));
}

void test_update_available_when_tag_differs() {
    TEST_ASSERT_TRUE(OtaVersionCheck::isUpdateAvailable("v0.1.0", "v0.2.0"));
}

void test_no_update_when_tag_is_empty() {
    TEST_ASSERT_FALSE(OtaVersionCheck::isUpdateAvailable("v0.1.0", ""));
}

void test_finds_sensor_firmware_asset_url() {
    JsonDocument doc;
    DeserializationError err = deserializeJson(doc, R"JSON(
{
    "tag_name": "v0.2.0",
    "assets": [
        {"name": "firmware.bin", "browser_download_url": "https://example.com/firmware.bin"},
        {"name": "sensor-firmware.bin", "browser_download_url": "https://example.com/sensor-firmware.bin"}
    ]
}
)JSON");
    TEST_ASSERT_FALSE(err);

    String url = OtaVersionCheck::findAssetDownloadUrl(doc, "sensor-firmware.bin");
    TEST_ASSERT_EQUAL_STRING("https://example.com/sensor-firmware.bin", url.c_str());
}

void test_returns_empty_when_asset_not_found() {
    JsonDocument doc;
    deserializeJson(doc, R"JSON({"tag_name": "v0.2.0", "assets": []})JSON");

    String url = OtaVersionCheck::findAssetDownloadUrl(doc, "sensor-firmware.bin");
    TEST_ASSERT_EQUAL(0, url.length());
}

void test_returns_empty_when_assets_field_missing() {
    JsonDocument doc;
    deserializeJson(doc, R"JSON({"tag_name": "v0.2.0"})JSON");

    String url = OtaVersionCheck::findAssetDownloadUrl(doc, "sensor-firmware.bin");
    TEST_ASSERT_EQUAL(0, url.length());
}

void test_validation_gate_marks_exactly_once() {
    OtaValidationGate gate;
    TEST_ASSERT_TRUE(gate.markIfNeeded());
    TEST_ASSERT_FALSE(gate.markIfNeeded());
    TEST_ASSERT_FALSE(gate.markIfNeeded());
}

void test_validation_gate_first_call_returns_true() {
    // Nếu gate coi như "đã mark" ngay từ đầu thì sẽ không bao giờ gọi
    // esp_ota_mark_app_valid_cancel_rollback() ở lần kết nối MQTT đầu tiên.
    OtaValidationGate gate;
    TEST_ASSERT_TRUE(gate.markIfNeeded());
}

int main(int argc, char **argv) {
    UNITY_BEGIN();
    RUN_TEST(test_load_returns_empty_when_nvs_empty);
    RUN_TEST(test_save_and_load_roundtrip);
    RUN_TEST(test_load_with_fallback_secret_when_nvs_empty);
    RUN_TEST(test_ph_filter_initialization_on_boot);
    RUN_TEST(test_ph_filter_rate_limiting_step_change);
    RUN_TEST(test_no_update_when_tag_matches_current_version);
    RUN_TEST(test_update_available_when_tag_differs);
    RUN_TEST(test_no_update_when_tag_is_empty);
    RUN_TEST(test_finds_sensor_firmware_asset_url);
    RUN_TEST(test_returns_empty_when_asset_not_found);
    RUN_TEST(test_returns_empty_when_assets_field_missing);
    RUN_TEST(test_validation_gate_marks_exactly_once);
    RUN_TEST(test_validation_gate_first_call_returns_true);
    return UNITY_END();
}
