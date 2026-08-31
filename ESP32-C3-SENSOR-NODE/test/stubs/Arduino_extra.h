#pragma once
#include <cstdint>
#include <cmath>
#include <algorithm>

inline void delay(uint32_t ms) {}
inline float constrain(float amt, float low, float high) {
    return (amt < low) ? low : ((amt > high) ? high : amt);
}
