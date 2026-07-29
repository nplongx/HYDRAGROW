// src/core/adaptive/gain_learner.rs
//! GainLearner — Học động hệ số phản ứng thực tế (Response Gain) từ phản hồi dung dịch.
//! Thuộc tầng Pure Core: Không phụ thuộc ESP-IDF, có thể test 100% bằng `cargo test`.

use hydragrow_shared::ControllerConfig;

/// Học hệ số gain cho từng kênh đơn lẻ (EC / pH Up / pH Down)
#[derive(Debug, Clone)]
pub struct SingleGainLearner {
    pub ema: f32,
    pub sample_count: u32,
    pub alpha: f32,
    pub confidence: f32,
    pub min_samples: u32,
    pub variance: f32,
    pub last_observed: f32,
}

impl Default for SingleGainLearner {
    fn default() -> Self {
        Self {
            ema: 0.0,
            sample_count: 0,
            alpha: 0.1,
            confidence: 0.0,
            min_samples: 5,
            variance: 0.0,
            last_observed: 0.0,
        }
    }
}

impl SingleGainLearner {
    pub fn update(&mut self, observed_gain: f32) {
        self.ema = if self.sample_count == 0 {
            observed_gain
        } else {
            self.alpha * observed_gain + (1.0 - self.alpha) * self.ema
        };

        let diff = observed_gain - self.ema;
        self.variance = (1.0 - self.alpha) * self.variance + self.alpha * diff * diff;
        self.last_observed = observed_gain;
        self.sample_count = self.sample_count.saturating_add(1);
        self.confidence = (self.sample_count as f32 / self.min_samples as f32).min(1.0);
    }
}

/// Bộ học Gain tổng hợp cho EC, pH Up và pH Down
#[derive(Debug, Clone, Default)]
pub struct GainLearner {
    pub ec: SingleGainLearner,
    pub ph_up: SingleGainLearner,
    pub ph_down: SingleGainLearner,
}

impl GainLearner {
    /// Cập nhật EC Gain quan sát được sau chu kỳ châm
    pub fn update_ec_gain(&mut self, dose_ml: f32, delta_ec: f32, config: &ControllerConfig) {
        if dose_ml <= 0.0 || delta_ec <= 0.0 {
            return;
        }
        let observed_gain = delta_ec / dose_ml;
        let base = config.ec_gain_per_ml.max(0.0001);

        // Bỏ qua ngoại lệ bất thường (Outliers > 3x hoặc < 0.3x base)
        if observed_gain < base * 0.3 || observed_gain > base * 3.0 {
            return;
        }
        self.ec.update(observed_gain);
    }

    /// Cập nhật pH Gain (Up/Down) quan sát được sau chu kỳ châm
    pub fn update_ph_gain(
        &mut self,
        dose_ml: f32,
        delta_ph: f32,
        is_up: bool,
        config: &ControllerConfig,
    ) {
        if dose_ml <= 0.0 || delta_ph <= 0.0 {
            return;
        }
        let observed_gain = delta_ph / dose_ml;
        let base = if is_up {
            config.ph_shift_up_per_ml
        } else {
            config.ph_shift_down_per_ml
        }
        .max(0.0001);

        if observed_gain < base * 0.3 || observed_gain > base * 3.0 {
            return;
        }

        let target = if is_up {
            &mut self.ph_up
        } else {
            &mut self.ph_down
        };
        target.update(observed_gain);
    }

    /// Tính EC Gain hiệu dụng (Blend 60% learned EMA + 40% static config) nếu đủ độ tin cậy
    pub fn effective_ec_gain(&self, config_gain: f32) -> f32 {
        if self.ec.confidence >= 0.6
            && self.ec.sample_count >= self.ec.min_samples
            && self.ec.ema.is_finite()
            && self.ec.ema > 0.0
        {
            0.6 * self.ec.ema + 0.4 * config_gain
        } else {
            config_gain
        }
    }

    /// Tính pH Up Gain hiệu dụng
    pub fn effective_ph_up_gain(&self, config_gain: f32) -> f32 {
        if self.ph_up.confidence >= 0.6
            && self.ph_up.sample_count >= self.ph_up.min_samples
            && self.ph_up.ema.is_finite()
            && self.ph_up.ema > 0.0
        {
            0.6 * self.ph_up.ema + 0.4 * config_gain
        } else {
            config_gain
        }
    }

    /// Tính pH Down Gain hiệu dụng
    pub fn effective_ph_down_gain(&self, config_gain: f32) -> f32 {
        if self.ph_down.confidence >= 0.6
            && self.ph_down.sample_count >= self.ph_down.min_samples
            && self.ph_down.ema.is_finite()
            && self.ph_down.ema > 0.0
        {
            0.6 * self.ph_down.ema + 0.4 * config_gain
        } else {
            config_gain
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_gain_learner_confidence_increase() {
        let mut learner = SingleGainLearner::default();
        assert_eq!(learner.confidence, 0.0);

        for _ in 0..5 {
            learner.update(0.02);
        }

        assert_eq!(learner.sample_count, 5);
        assert_eq!(learner.confidence, 1.0);
        assert!((learner.ema - 0.02).abs() < f32::EPSILON);
    }
}