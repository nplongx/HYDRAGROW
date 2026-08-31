#include <iostream>
#include <cassert>
#include <cmath>
#include "filters/HybridFilter.h"
#include "sensors/PhSensor.h"

static int16_t g_mock_diff_counts = 0; // Starts at 0 (unready on reboot)
static int16_t g_mock_vcc_counts = 26667; // 5000mV VCC

Adafruit_ADS1115::Adafruit_ADS1115() {}

// Concrete implementation of Adafruit_ADS1115 for testing PhSensor logic
bool Adafruit_ADS1X15::begin(uint8_t i2c_addr, TwoWire *wire) { return true; }
void Adafruit_ADS1X15::setGain(adsGain_t gain) {}
int16_t Adafruit_ADS1X15::readADC_Differential_0_1() { return g_mock_diff_counts; }
int16_t Adafruit_ADS1X15::readADC_SingleEnded(uint8_t channel) { return g_mock_vcc_counts; }
float Adafruit_ADS1X15::computeVolts(int16_t counts) { return (counts * 0.1875f) / 1000.0f; }

void test_reboot_sequence_prevents_zero_initialization() {
    PhSensor phSensor(0x48);
    phSensor.begin();

    PhSensorConfig config;
    config.v686 = 2650.0f;
    config.v4 = 3555.0f;
    config.v918 = 1750.0f;
    phSensor.setConfig(config);

    HybridFilter phFilter(1.5f, 0.125f);

    // Boot Cycle 1: ADS1115 differential is 0 (unready / power-up)
    g_mock_diff_counts = 0;
    float raw_boot = phSensor.read(25.0f);
    std::cout << "Boot cycle 1 raw pH: " << raw_boot << std::endl;
    assert(std::isnan(raw_boot));

    // SensorManager logic: if raw is NAN, filter is not updated
    if (!std::isnan(raw_boot)) {
        phFilter.update(raw_boot);
    }

    // Boot Cycle 2: ADS1115 conversion completes (~2650mV -> 14133 counts)
    g_mock_diff_counts = 14133;
    float raw_ready = phSensor.read(25.0f);
    std::cout << "Boot cycle 2 raw pH: " << raw_ready << std::endl;
    assert(!std::isnan(raw_ready));
    assert(std::abs(raw_ready - 6.86f) < 0.1f);

    float filtered_ph = phFilter.update(raw_ready);
    std::cout << "Filtered pH output on first valid cycle: " << filtered_ph << std::endl;

    // Filter must initialize directly at ~6.86 without rate-limiting from 0.0
    assert(std::abs(filtered_ph - 6.86f) < 0.1f);
}

int main() {
    std::cout << "Running test_ph_sensor_reboot..." << std::endl;
    test_reboot_sequence_prevents_zero_initialization();
    std::cout << "Reboot test passed successfully!" << std::endl;
    return 0;
}
