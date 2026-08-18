// src/core/adaptive/gain_learner.rs
//! GainLearner đồng bộ hoá đáp ứng thực tế (Response Gain) từ các liều châm dung dịch.
//! Thuộc tầng Pure Core: Không phụ thuộc ESP-IDF, có thể test 100% bằng `cargo test`.

use hydragrow_shared::ControllerConfig;

/// Học gain cho từng kênh riêng biệt (EC A / EC B / pH Up / pH Down)
#[derive(Debug, Clone)]
pub struct SingleGainLearner {
    pub ema: f32,          // Observed gain trung bình trượt theo EMA
    pub sample_count: u32, // Số mẫu đã thu thập
    pub alpha: f32,        // Hệ số học (mặc định 0.1)
    pub confidence: f32,   // Độ tin cậy [0.0 -> 1.0]
    pub min_samples: u32,  // Số mẫu tối thiểu để tin cậy (mặc định 5)
    pub variance: f32,     // Phương sai của sai số đo
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
        }
    }
}

impl SingleGainLearner {
    pub fn update(&mut self, observed_gain: f32) {
        if !observed_gain.is_finite() || observed_gain <= 0.0 {
            return;
        }

        self.ema = if self.sample_count == 0 {
            observed_gain
        } else {
            self.alpha * observed_gain + (1.0 - self.alpha) * self.ema
        };

        let diff = observed_gain - self.ema;
        self.variance = (1.0 - self.alpha) * self.variance + self.alpha * diff * diff;
        self.sample_count = self.sample_count.saturating_add(1);

        // Độ tin cậy tăng dần theo số lượng mẫu và giảm khi phương sai cao
        let c_n = (self.sample_count as f32 / self.min_samples as f32).min(1.0);
        let c_v = (-c_n * self.variance.max(0.0)).exp();
        self.confidence = (c_n * c_v).clamp(0.0, 1.0);
    }

    /// Phát hiện giá trị ngoại lai vượt quá 3 độ lệch chuẩn (3-Sigma Rule)
    pub fn outlier(&self, observed_gain: f32) -> bool {
        if self.sample_count < self.min_samples {
            return false;
        }
        let diff = (observed_gain - self.ema).abs();
        diff > (3.0 * self.variance.sqrt())
    }

    /// Trộn hệ số đã học với hệ số cấu hình dựa trên độ tin cậy
    pub fn effective_gain(&self, config_gain: f32) -> f32 {
        if self.confidence >= 0.6
            && self.sample_count >= self.min_samples
            && self.ema.is_finite()
            && self.ema > 0.0
        {
            self.confidence * self.ema + (1.0 - self.confidence) * config_gain
        } else {
            config_gain
        }
    }
}

/// Bộ học Gain tổng hợp tách biệt cho Dinh Dưỡng A, B và pH Up, Down
#[derive(Debug, Clone, Default)]
pub struct GainLearner {
    pub ec: SingleGainLearner,      // Gain tổng hợp của toàn bộ dinh dưỡng EC
    pub ec_a: SingleGainLearner,    // Gain riêng của Bơm A
    pub ec_b: SingleGainLearner,    // Gain riêng của Bơm B
    pub ph_up: SingleGainLearner,   // Gain riêng của pH Up (Kiềm)
    pub ph_down: SingleGainLearner, // Gain riêng của pH Down (Axit)
}

impl GainLearner {
    /// Cập nhật độc lập Gain cho từng Bơm A và B dựa trên tỷ lệ thực bơm
    pub fn update_nutrient_gains(
        &mut self,
        dose_a_ml: f32,
        dose_b_ml: f32,
        delta_ec: f32,
        config: &ControllerConfig,
    ) {
        let total_dose = dose_a_ml + dose_b_ml;
        if total_dose <= 0.0 || delta_ec <= 0.0 {
            return;
        }

        let base_gain = config.ec_gain_per_ml.max(0.0001);

        // 1. Cập nhật Gain tổng hợp EC
        let observed_total_gain = delta_ec / total_dose;
        if observed_total_gain >= base_gain * 0.3 && observed_total_gain <= base_gain * 3.0 {
            if !self.ec.outlier(observed_total_gain) {
                self.ec.update(observed_total_gain);
            }
        }

        // 2. Phân tách và cập nhật Gain riêng cho Bơm A
        if dose_a_ml > 0.0 {
            let ec_share_a = delta_ec * (dose_a_ml / total_dose);
            let observed_gain_a = ec_share_a / dose_a_ml;
            if observed_gain_a >= base_gain * 0.3 && observed_gain_a <= base_gain * 3.0 {
                if !self.ec_a.outlier(observed_gain_a) {
                    self.ec_a.update(observed_gain_a);
                }
            }
        }

        // 3. Phân tách và cập nhật Gain riêng cho Bơm B
        if dose_b_ml > 0.0 {
            let ec_share_b = delta_ec * (dose_b_ml / total_dose);
            let observed_gain_b = ec_share_b / dose_b_ml;
            if observed_gain_b >= base_gain * 0.3 && observed_gain_b <= base_gain * 3.0 {
                if !self.ec_b.outlier(observed_gain_b) {
                    self.ec_b.update(observed_gain_b);
                }
            }
        }
    }

    /// Hàm tương thích ngược nhận tổng liều EC
    pub fn update_ec_gain(&mut self, total_dose_ml: f32, delta_ec: f32, config: &ControllerConfig) {
        let half = total_dose_ml * 0.5;
        self.update_nutrient_gains(half, half, delta_ec, config);
    }

    /// Cập nhật pH Gain (Up hoặc Down) độc lập
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
        let base_gain = if is_up {
            config.ph_shift_up_per_ml
        } else {
            config.ph_shift_down_per_ml
        }
        .max(0.0001);

        if observed_gain < base_gain * 0.3 || observed_gain > base_gain * 3.0 {
            return;
        }

        let target = if is_up {
            &mut self.ph_up
        } else {
            &mut self.ph_down
        };

        if target.outlier(observed_gain) {
            return;
        }

        target.update(observed_gain);
    }

    /// Tính EC Gain tổng hợp hiệu dụng
    pub fn effective_ec_gain(&self, config_gain: f32) -> f32 {
        self.ec.effective_gain(config_gain)
    }

    /// Tính EC Gain riêng biệt của Bơm A
    pub fn effective_ec_a_gain(&self, config_gain: f32) -> f32 {
        self.ec_a.effective_gain(config_gain)
    }

    /// Tính EC Gain riêng biệt của Bơm B
    pub fn effective_ec_b_gain(&self, config_gain: f32) -> f32 {
        self.ec_b.effective_gain(config_gain)
    }

    /// Tính pH Up Gain hiệu dụng
    pub fn effective_ph_up_gain(&self, config_gain: f32) -> f32 {
        self.ph_up.effective_gain(config_gain)
    }

    /// Tính pH Down Gain hiệu dụng
    pub fn effective_ph_down_gain(&self, config_gain: f32) -> f32 {
        self.ph_down.effective_gain(config_gain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_gain_learner_converges_and_increases_confidence() {
        let mut learner = SingleGainLearner::default();
        assert_eq!(learner.confidence, 0.0);

        for _ in 0..5 {
            learner.update(0.02);
        }

        assert_eq!(learner.sample_count, 5);
        assert!(learner.confidence > 0.8);
        assert!((learner.ema - 0.02).abs() < 1e-4);
    }

    #[test]
    fn outlier_detection_rejects_abnormal_samples() {
        let mut learner = SingleGainLearner::default();
        for _ in 0..10 {
            learner.update(0.02);
        }

        // Giá trị tăng vọt gấp 5 lần phải bị coi là outlier
        assert!(learner.outlier(0.10));
        // Giá trị gần với EMA không phải là outlier
        assert!(!learner.outlier(0.021));
    }

    #[test]
    fn separate_nutrient_gains_learn_independently() {
        let mut learner = GainLearner::default();
        let config = ControllerConfig {
            ec_gain_per_ml: 0.02,
            ..Default::default()
        };

        // Giả sử Bơm A vào 4ml, Bơm B vào 6ml, tổng EC tăng 0.20 mS/cm
        for _ in 0..5 {
            learner.update_nutrient_gains(4.0, 6.0, 0.20, &config);
        }

        assert!(learner.ec_a.sample_count == 5);
        assert!(learner.ec_b.sample_count == 5);
        assert!((learner.effective_ec_gain(0.02) - 0.02).abs() < 1e-3);
    }
}
