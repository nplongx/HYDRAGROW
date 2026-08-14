#pragma once

class HybridFilter {
public:
    HybridFilter(float deltaMax, float alpha);

    void setAlpha(float alpha);
    void setDelta(float delta);

    float update(float value);

    void reset();

private:
    float xPrev_;
    float yPrev_;

    int errorStreak_;

    float deltaMax_;
    float alpha_;

    bool initialized_;
};