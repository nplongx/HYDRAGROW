/// Port của HybridFilter từ C++ sang Rust.
/// Kết hợp rate-limiting (clamp delta) + EMA để lọc nhiễu cảm biến.
pub struct HybridFilter {
    x_prev: f32,
    y_prev: f32,
    delta_max: f32,
    alpha: f32,
    initialized: bool,
}

impl HybridFilter {
    pub fn new(delta_max: f32, alpha: f32) -> Self {
        Self {
            x_prev: 0.0,
            y_prev: 0.0,
            delta_max,
            alpha,
            initialized: false,
        }
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }

    pub fn set_delta(&mut self, delta: f32) {
        self.delta_max = delta;
    }

    pub fn reset(&mut self) {
        self.x_prev = 0.0;
        self.y_prev = 0.0;
        self.initialized = false;
    }

    pub fn update(&mut self, value: f32) -> f32 {
        if !self.initialized {
            self.x_prev = value;
            self.y_prev = value;
            self.initialized = true;
            return value;
        }

        // Rate-limit delta
        let delta = (value - self.x_prev).clamp(-self.delta_max, self.delta_max);
        let x_limited = self.x_prev + delta;

        // EMA
        let y = self.alpha * x_limited + (1.0 - self.alpha) * self.y_prev;

        self.x_prev = value;
        self.y_prev = y;
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_value_passthrough() {
        let mut f = HybridFilter::new(5.0, 0.5);
        assert_eq!(f.update(10.0), 10.0);
    }

    #[test]
    fn test_rate_limiting() {
        let mut f = HybridFilter::new(1.0, 1.0); // alpha=1 -> no EMA smoothing
        f.update(0.0);
        let out = f.update(100.0); // big jump, clamped to delta_max=1
        assert!((out - 1.0).abs() < 0.001, "Expected ~1.0, got {}", out);
    }

    #[test]
    fn test_ema_smoothing() {
        let mut f = HybridFilter::new(100.0, 0.125);
        f.update(0.0);
        let out = f.update(8.0); // delta=8 < delta_max=100, EMA: 0.125*8 + 0.875*0 = 1.0
        assert!((out - 1.0).abs() < 0.001, "Expected 1.0, got {}", out);
    }

    #[test]
    fn test_reset() {
        let mut f = HybridFilter::new(5.0, 0.5);
        f.update(10.0);
        f.reset();
        assert_eq!(f.update(20.0), 20.0); // initialized lại -> passthrough
    }
}
