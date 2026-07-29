// src/core/adaptive/tuner.rs
//! AutoTuner & ConvergenceTracker — Đánh giá độ hội tụ và tự động điều chỉnh bước châm (step ratio).
//! Thuộc tầng Pure Core: Không phụ thuộc ESP-IDF, có thể test 100% bằng `cargo test`.

use hydragrow_shared::ControllerConfig;
use serde::{Deserialize, Serialize};
use crate::core::fsm::types::PendingCalibrationSample;
use super::gain_learner::GainLearner;
use super::kalman::KalmanCovarianceDiag;
use super::matrix::InteractionMatrix;

/// Trạng thái học máy của AutoTuner
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum TunerState {
    Exploring = 0,  // Đang thăm dò gain ban đầu
    Converging = 1, // Đang hội tụ về target
    Stable = 2,     // Đạt trạng thái ổn định
    Degraded = 3,   // Phương hại / Nhiễu cao
}

impl TunerState {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Exploring,
            1 => Self::Converging,
            2 => Self::Stable,
            3 => Self::Degraded,
            _ => Self::Converging,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Theo dõi lịch sử sai số 8 chu kỳ gần nhất để phát hiện xu hướng (trend) & dao động (oscillation).
#[derive(Debug, Clone)]
pub struct ConvergenceTracker {
    pub error_history: [f32; 8],
    pub head: usize,
    pub count: u8,
    pub trend: f32,
    pub oscillation: f32,
    pub stagnant_cycles: u8,
}

impl Default for ConvergenceTracker {
    fn default() -> Self {
        Self {
            error_history: [0.0; 8],
            head: 0,
            count: 0,
            trend: 0.0,
            oscillation: 0.0,
            stagnant_cycles: 0,
        }
    }
}

impl ConvergenceTracker {
    pub fn push(&mut self, error: f32) {
        self.error_history[self.head] = error;
        self.head = (self.head + 1) % self.error_history.len();
        self.count = self
            .count
            .saturating_add(1)
            .min(self.error_history.len() as u8);
        self.recompute();
    }

    fn recompute(&mut self) {
        let n = self.count as usize;
        if n < 2 {
            return;
        }

        let (mut first, mut last, mut prev_sign, mut sign_changes, mut abs_sum) =
            (0.0, 0.0, 0_i8, 0_u8, 0.0);

        for i in 0..n {
            let idx = (self.head + self.error_history.len() - n + i) % self.error_history.len();
            let v = self.error_history[idx];

            if i == 0 {
                first = v;
            }
            if i == n - 1 {
                last = v;
            }

            abs_sum += v.abs();
            let sign = if v > 0.0 {
                1
            } else if v < 0.0 {
                -1
            } else {
                0
            };

            if prev_sign != 0 && sign != 0 && sign != prev_sign {
                sign_changes = sign_changes.saturating_add(1);
            }
            if sign != 0 {
                prev_sign = sign;
            }
        }

        self.trend = first.abs() - last.abs();
        self.oscillation = sign_changes as f32 / (n.saturating_sub(1) as f32);

        if (abs_sum / n as f32) > 0.98 {
            self.stagnant_cycles = self.stagnant_cycles.saturating_add(1);
        } else {
            self.stagnant_cycles = 0;
        }
    }

    pub fn current_error(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }
        let idx = (self.head + self.error_history.len() - 1) % self.error_history.len();
        self.error_history[idx]
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// AutoTuner chính điều phối việc học ma trận và thích ứng tỷ lệ bước châm
#[derive(Debug, Clone)]
pub struct AutoTuner {
    pub state: TunerState,
    pub adaptive_ec_ratio: f32,
    pub adaptive_ph_ratio: f32,
    pub best_ec_ratio: f32,
    pub best_ph_ratio: f32,
    pub last_update_sec: u64,
    pub ec_tracker: ConvergenceTracker,
    pub ph_tracker: ConvergenceTracker,
    pub gain_learner: GainLearner,
    pub ec_variance_baseline: f32,
    pub ph_variance_baseline: f32,
    pub interaction_matrix: InteractionMatrix,
    pub kalman: KalmanCovarianceDiag,
    pub matrix_update_count: u32,
    pub matrix_is_warm: bool,
}

impl Default for AutoTuner {
    fn default() -> Self {
        Self {
            adaptive_ec_ratio: 0.4,
            adaptive_ph_ratio: 0.2,
            best_ec_ratio: 0.4,
            best_ph_ratio: 0.2,
            state: TunerState::Exploring,
            last_update_sec: 0,
            ec_tracker: ConvergenceTracker::default(),
            ph_tracker: ConvergenceTracker::default(),
            gain_learner: GainLearner::default(),
            ec_variance_baseline: 0.0,
            ph_variance_baseline: 0.0,
            interaction_matrix: InteractionMatrix::from_scalar(0.015, 0.02, 0.025, 0.05, 0.04),
            kalman: KalmanCovarianceDiag::new(1.0, 0.001, 0.1),
            matrix_update_count: 0,
            matrix_is_warm: false,
        }
    }
}

impl AutoTuner {
    pub fn is_locked(&self) -> bool {
        matches!(self.state, TunerState::Stable)
    }

    pub fn active_ec_ratio(&self) -> f32 {
        self.adaptive_ec_ratio
    }

    pub fn effective_ec_tolerance(&self, base: f32) -> f32 {
        adaptive_solver_tolerance(base, self.state, self.ec_tracker.oscillation)
    }

    pub fn effective_ph_tolerance(&self, base: f32) -> f32 {
        adaptive_solver_tolerance(base, self.state, self.ph_tracker.oscillation)
    }

    pub fn on_ec_dosing_ack(
        &mut self,
        response: f32,
        expected: f32,
        _config: &ControllerConfig,
        now_sec: u64,
    ) {
        self.on_dosing_ack(response, expected, true, None, now_sec);
    }

    pub fn on_ph_dosing_ack(
        &mut self,
        response: f32,
        expected: f32,
        _config: &ControllerConfig,
        is_up: bool,
        now_sec: u64,
    ) {
        self.on_dosing_ack(response, expected, false, Some(is_up), now_sec);
    }

    fn on_dosing_ack(
        &mut self,
        response: f32,
        expected: f32,
        is_ec: bool,
        is_ph_up: Option<bool>,
        now_sec: u64,
    ) {
        if expected <= 0.0 || !response.is_finite() || !expected.is_finite() {
            return;
        }

        let gain_vs_expected: f32 = response / expected.max(0.001_f32);
        let tracker = if is_ec {
            &mut self.ec_tracker
        } else {
            &mut self.ph_tracker
        };
        tracker.push(gain_vs_expected - 1.0);

        let tune_delta = self.compute_delta(is_ec).clamp(-0.08, 0.08);
        if tune_delta != 0.0 {
            self.adjust_step_ratio(is_ec, tune_delta);
            if is_ec {
                self.best_ec_ratio = self.best_ec_ratio.max(self.adaptive_ec_ratio);
            } else {
                self.best_ph_ratio = self.best_ph_ratio.max(self.adaptive_ph_ratio);
            }
        }

        self.last_update_sec = now_sec;
        self.update_state(is_ec, is_ph_up);
    }

    pub fn adjust_step_ratio(&mut self, is_ec: bool, delta: f32) {
        let ratio = if is_ec {
            &mut self.adaptive_ec_ratio
        } else {
            &mut self.adaptive_ph_ratio
        };
        *ratio = (*ratio + delta).clamp(0.1_f32, 2.0_f32);
    }

    fn compute_delta(&self, is_ec: bool) -> f32 {
        let tracker = if is_ec {
            &self.ec_tracker
        } else {
            &self.ph_tracker
        };
        let error = tracker.current_error();
        let p_term = error * 0.04;
        let d_term = tracker.trend * (-0.015);
        let damp = 1.0 - (tracker.oscillation * 0.6).clamp(0.0, 0.8);
        let scale = if matches!(self.state, TunerState::Stable) {
            0.1
        } else {
            1.0
        };

        (p_term + d_term) * damp * scale
    }

    fn update_state(&mut self, is_ec: bool, is_ph_up: Option<bool>) {
        let (err, tracker_count) = if is_ec {
            (self.ec_tracker.current_error().abs(), self.ec_tracker.count)
        } else {
            (self.ph_tracker.current_error().abs(), self.ph_tracker.count)
        };

        let confidence = if is_ec {
            self.gain_learner.ec.confidence
        } else {
            match is_ph_up {
                Some(true) => self.gain_learner.ph_up.confidence,
                Some(false) => self.gain_learner.ph_down.confidence,
                None => self
                    .gain_learner
                    .ph_up
                    .confidence
                    .max(self.gain_learner.ph_down.confidence),
            }
        };

        self.refresh_variance_baseline();

        if self.is_degraded() {
            self.state = TunerState::Degraded;
            return;
        }

        self.state = match self.state {
            TunerState::Exploring if confidence > 0.3 => TunerState::Converging,
            TunerState::Converging if err < 0.1 && tracker_count >= 3 => TunerState::Stable,
            TunerState::Stable if err > 0.2 => TunerState::Converging,
            TunerState::Degraded if err <= 0.2 => TunerState::Converging,
            state => state,
        };
    }

    pub fn on_water_change(&mut self) {
        self.adaptive_ec_ratio += (self.best_ec_ratio - self.adaptive_ec_ratio) * 0.5;
        self.adaptive_ph_ratio += (self.best_ph_ratio - self.adaptive_ph_ratio) * 0.5;
        self.ec_tracker.reset();
        self.ph_tracker.reset();
        self.state = TunerState::Converging;
    }

    pub fn on_manual_reset(&mut self) {
        self.ec_tracker.reset();
        self.ph_tracker.reset();
        self.state = TunerState::Converging;
    }

    /// Entry point duy nhất cho adaptive learning pipeline sau mỗi chu kỳ châm.
    pub fn learn_from_cycle(
        &mut self,
        sample: &PendingCalibrationSample,
        post_ec: f32,
        post_ph: f32,
        post_water: f32,
        post_temp: f32,
        config: &ControllerConfig,
        now_sec: u64,
    ) -> bool {
        if sample.invalid_by_noise || sample.invalid_by_water_change {
            return false;
        }

        let actual_delta_ec = post_ec - sample.start_ec;
        let actual_delta_ph = post_ph - sample.start_ph;
        let total_nutrient_ml = sample.dose_a_ml + sample.dose_b_ml;
        let ph_dose_ml = sample.dose_ph_up_ml + sample.dose_ph_down_ml;
        let is_ph_up = sample.dose_ph_up_ml > sample.dose_ph_down_ml;

        // 1. Cập nhật GainLearner
        if total_nutrient_ml > 0.5 && actual_delta_ec > 0.01 {
            self.gain_learner
                .update_ec_gain(total_nutrient_ml, actual_delta_ec, config);
        }

        if ph_dose_ml > 0.1 && actual_delta_ph.abs() > 0.01 {
            self.gain_learner
                .update_ph_gain(ph_dose_ml, actual_delta_ph.abs(), is_ph_up, config);
        }

        // 2. AutoTuner ACK
        if total_nutrient_ml > 0.5 {
            let expected_ec_delta = total_nutrient_ml * config.ec_gain_per_ml;
            if expected_ec_delta > 1e-6 {
                self.on_ec_dosing_ack(actual_delta_ec, expected_ec_delta, config, now_sec);
            }
        }

        if ph_dose_ml > 0.1 {
            let expected_ph_delta = if is_ph_up {
                ph_dose_ml * config.ph_shift_up_per_ml
            } else {
                ph_dose_ml * config.ph_shift_down_per_ml
            };
            if expected_ph_delta > 1e-6 {
                self.on_ph_dosing_ack(
                    actual_delta_ph,
                    expected_ph_delta,
                    config,
                    is_ph_up,
                    now_sec,
                );
            }
        }

        // 3. Cập nhật InteractionMatrix via Kalman filter
        self.interaction_matrix.update_matrix_adaptive(
            &mut self.kalman,
            sample,
            post_ec,
            post_ph,
            post_water,
            post_temp,
        );

        // 4. Update tracking
        self.matrix_update_count = self.matrix_update_count.saturating_add(1);
        if !self.matrix_is_warm && self.matrix_update_count >= 10 {
            self.matrix_is_warm = true;
            log::info!(
                "🔥 [ADAPTIVE] InteractionMatrix đã ẤM sau {} cycles! Chuyển sang WarmPathSolver.",
                self.matrix_update_count
            );
        }

        log::info!(
            "📊 [ADAPTIVE] Cycle học #{}: ΔEC={:.3}, ΔpH={:.3}, Matrix warm={}, Updates={}",
            self.matrix_update_count,
            actual_delta_ec,
            actual_delta_ph,
            self.matrix_is_warm,
            self.matrix_update_count
        );

        true
    }

    fn refresh_variance_baseline(&mut self) {
        if self.gain_learner.ec.sample_count >= self.gain_learner.ec.min_samples
            && self.ec_variance_baseline <= 0.0
        {
            self.ec_variance_baseline = self.gain_learner.ec.variance.max(1e-6);
        }

        if (self.gain_learner.ph_up.sample_count + self.gain_learner.ph_down.sample_count)
            >= self.gain_learner.ph_up.min_samples
            && self.ph_variance_baseline <= 0.0
        {
            self.ph_variance_baseline =
                ((self.gain_learner.ph_up.variance + self.gain_learner.ph_down.variance) * 0.5)
                    .max(1e-6);
        }
    }

    fn is_degraded(&self) -> bool {
        let ec_degraded = self.ec_variance_baseline > 0.0
            && self.gain_learner.ec.variance > self.ec_variance_baseline * 1.5;
        let ph_degraded = self.ph_variance_baseline > 0.0
            && ((self.gain_learner.ph_up.variance + self.gain_learner.ph_down.variance) * 0.5)
                > self.ph_variance_baseline * 1.5;
        ec_degraded || ph_degraded
    }

    pub fn to_mqtt_payload(
        &self,
        device_id: &str,
        config: &ControllerConfig,
        adaptive_mixing_sec: u32,
        adaptive_stabilize_sec: u32,
        now_ms: u64,
    ) -> String {
        let flat = self.interaction_matrix.as_flat();
        serde_json::json!({
            "type": "runtime_calibration_update",
            "device_id": device_id,
            "runtime_coefficients": {
                "step_ratio_ec": self.adaptive_ec_ratio,
                "step_ratio_ph": self.adaptive_ph_ratio,
                "best_ec_ratio": self.best_ec_ratio,
                "best_ph_ratio": self.best_ph_ratio,
                "state": self.state.as_u8(),
                "adaptive_mixing_sec": adaptive_mixing_sec,
                "adaptive_stabilize_sec": adaptive_stabilize_sec,
                "effective_ec_tolerance": self.effective_ec_tolerance(config.ec_tolerance),
                "effective_ph_tolerance": self.effective_ph_tolerance(config.ph_tolerance),
                "ec_gain_per_ml": self.gain_learner.effective_ec_gain(config.ec_gain_per_ml),
                "ph_shift_up_per_ml": self.gain_learner.effective_ph_up_gain(config.ph_shift_up_per_ml),
                "ph_shift_down_per_ml": self.gain_learner.effective_ph_down_gain(config.ph_shift_down_per_ml),
                "interaction_matrix": flat,
                "matrix_update_count": self.matrix_update_count,
                "matrix_is_warm": self.matrix_is_warm,
                "kalman_confidence": [
                    self.kalman.confidence(0), // Nutrient A
                    self.kalman.confidence(1), // Nutrient B
                    self.kalman.confidence(2), // pH Up
                    self.kalman.confidence(3), // pH Down
                    self.kalman.confidence(4), // Water In
                    self.kalman.confidence(5), // Water Out
                    self.kalman.confidence(6), // Osaka Mixing
                    self.kalman.confidence(7), // Misting
                ],
            },
            "timestamp_ms": now_ms
        })
        .to_string()
    }
}

pub fn adaptive_solver_tolerance(base: f32, state: TunerState, oscillation: f32) -> f32 {
    if !base.is_finite() || base <= 0.0 {
        return 0.0;
    }
    let state_multiplier = match state {
        TunerState::Stable => 1.0,
        TunerState::Converging => 1.25,
        TunerState::Exploring => 1.5,
        TunerState::Degraded => 1.75,
    };
    let oscillation_multiplier = 1.0 + oscillation.clamp(0.0, 1.0) * 0.75;
    let multiplier = (state_multiplier * oscillation_multiplier).clamp(1.0, 2.5);
    base * multiplier
}