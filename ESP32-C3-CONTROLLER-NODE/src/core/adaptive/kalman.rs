// src/core/adaptive/kalman.rs
//! Lọc Kalman đường chéo (Diagonal Kalman Covariance) cho ma trận MIMO.
//! Thuộc tầng Pure Core: Không phụ thuộc ESP-IDF, có thể test 100% bằng `cargo test`.

/// Ma trận hiệp phương sai 8 chiều dạng đường chéo (Diagonal Covariance Matrix).
#[derive(Debug, Clone)]
pub struct KalmanCovarianceDiag {
    /// Mảng ước lượng hiệp phương sai cho 8 kênh điều khiển
    pub p: [f32; 8],
    /// Nhiễu quá trình (Process noise covariance)
    pub q: f32,
    /// Nhiễu đo lường (Measurement noise covariance)
    pub r: f32,
}

impl KalmanCovarianceDiag {
    pub fn new(p0: f32, q: f32, r: f32) -> Self {
        let p0 = p0.max(0.0);
        Self {
            p: [p0; 8],
            q: q.max(0.0),
            r: r.max(1e-9),
        }
    }

    /// Bước dự đoán Kalman Predict: Tăng độ bất định (phương sai P) theo thời gian bằng nhiễu quá trình Q.
    pub fn predict(&mut self) {
        for p_i in &mut self.p {
            *p_i += self.q;
        }
    }

    /// Cập nhật phương sai và tính toán Kalman Gain K cho kênh `idx`.
    pub fn update_and_get_gain(&mut self, idx: usize) -> f32 {
        if idx >= self.p.len() {
            return 0.0;
        }

        let p_val = self.p[idx].max(0.0);
        let denom = p_val + self.r; // r là độ nhiễu cảm biến

        if denom <= 1e-9 {
            return 0.0;
        }

        let k = (p_val / denom).clamp(0.0, 1.0);

        // Cập nhật lại phương sai sai số sau khi thu thập mẫu
        self.p[idx] = ((1.0 - k) * p_val).max(1e-9);
        k
    }

    /// Tính chỉ số độ tin cậy (Confidence score) trong khoảng [0.0 -> 1.0] cho kênh `idx`.
    pub fn confidence(&self, idx: usize) -> f32 {
        if idx >= self.p.len() {
            return 0.0;
        }
        1.0 / (1.0 + self.p[idx].max(0.0))
    }
}

impl Default for KalmanCovarianceDiag {
    fn default() -> Self {
        Self::new(1.0, 0.001, 0.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_gain_convergence() {
        let mut kalman = KalmanCovarianceDiag::new(1.0, 0.01, 0.1);
        let initial_conf = kalman.confidence(0);

        // Giả lập 5 bước cập nhật Kalman
        for _ in 0..5 {
            kalman.predict();
            let gain = kalman.update_and_get_gain(0);
            assert!(gain >= 0.0 && gain <= 1.0);
        }

        // Sau khi update nhiều lần, độ tin cậy phải tăng lên
        assert!(kalman.confidence(0) > initial_conf);
    }
}