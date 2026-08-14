#include "HybridFilter.h"

HybridFilter::HybridFilter(float deltaMax, float alpha)
    : xPrev_(0.0f),
      yPrev_(0.0f),
      errorStreak_(0),
      deltaMax_(deltaMax),
      alpha_(alpha),
      initialized_(false) {
}

void HybridFilter::setAlpha(float alpha) {
    alpha_ = alpha;
}

void HybridFilter::setDelta(float delta) {
    deltaMax_ = delta;
}

void HybridFilter::reset() {
    xPrev_ = 0.0f;
    yPrev_ = 0.0f;
    errorStreak_ = 0;
    initialized_ = false;
}

float HybridFilter::update(float value) {
    if (!initialized_) {
        xPrev_ = value;
        yPrev_ = value;
        initialized_ = true;

        return value;
    }

    float delta = value - xPrev_;

    if (delta > deltaMax_) {
        delta = deltaMax_;
    } else if (delta < -deltaMax_) {
        delta = -deltaMax_;
    }

    float xLimited = xPrev_ + delta;

    float y = alpha_ * xLimited
            + (1.0f - alpha_) * yPrev_;

    xPrev_ = value;
    yPrev_ = y;

    return y;
}