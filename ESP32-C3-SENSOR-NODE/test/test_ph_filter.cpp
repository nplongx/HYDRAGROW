#include <unity.h>
#include <cmath>
#include "filters/HybridFilter.h"

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
    RUN_TEST(test_ph_filter_initialization_on_boot);
    RUN_TEST(test_ph_filter_rate_limiting_step_change);
    return UNITY_END();
}
