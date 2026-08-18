// src/core/adaptive/tuner.rs
//! AutoTuner & ConvergenceTracker đánh giá và điều chỉnh bước châm thích ứng (step ratio).

use hydragrow_shared::ControllerConfig;
use serde::{Deserialize, Serialize};

use super::gain_learner::GainLearner;
use super::kalman::KalmanCovarianceDiag;
use super::matrix::InteractionMatrix;
use crate::core::fsm::types::PendingCalibrationSample;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum TunerState {
    Exploring = 0,
    Converging = 1,
    Stable = 2,
    Degraded = 3,
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

#[derive(Debug, Clone, Copy, PartialEq)]
enum TuneChannel {
    EcA,
    EcB,
    PhUp,
    PhDown,
}

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

/// AutoTuner Tách Biệt 4 Kênh: EcA, EcB, PhUp, PhDown
#[derive(Debug, Clone)]
pub struct AutoTuner {
    pub state: TunerState,

    // Tách riêng Step Ratio cho từng kênh
    pub adaptive_ec_a_ratio: f32,
    pub adaptive_ec_b_ratio: f32,
    pub adaptive_ph_up_ratio: f32,
    pub adaptive_ph_down_ratio: f32,

    pub best_ec_a_ratio: f32,
    pub best_ec_b_ratio: f32,
    pub best_ph_up_ratio: f32,
    pub best_ph_down_ratio: f32,

    pub last_update_sec: u64,

    pub ec_a_tracker: ConvergenceTracker,
    pub ec_b_tracker: ConvergenceTracker,
    pub ph_up_tracker: ConvergenceTracker,
    pub ph_down_tracker: ConvergenceTracker,

    pub gain_learner: GainLearner,

    pub ec_a_variance_baseline: f32,
    pub ec_b_variance_baseline: f32,
    pub ph_variance_baseline: f32,

    pub interaction_matrix: InteractionMatrix,
    pub kalman: KalmanCovarianceDiag,
    pub matrix_update_count: u32,
    pub matrix_is_warm: bool,
}

impl Default for AutoTuner {
    fn default() -> Self {
        Self {
            adaptive_ec_a_ratio: 0.4,
            adaptive_ec_b_ratio: 0.4,
            adaptive_ph_up_ratio: 0.2,
            adaptive_ph_down_ratio: 0.2,
            best_ec_a_ratio: 0.4,
            best_ec_b_ratio: 0.4,
            best_ph_up_ratio: 0.2,
            best_ph_down_ratio: 0.2,
            state: TunerState::Exploring,
            last_update_sec: 0,
            ec_a_tracker: ConvergenceTracker::default(),
            ec_b_tracker: ConvergenceTracker::default(),
            ph_up_tracker: ConvergenceTracker::default(),
            ph_down_tracker: ConvergenceTracker::default(),
            gain_learner: GainLearner::default(),
            ec_a_variance_baseline: 0.0,
            ec_b_variance_baseline: 0.0,
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

    pub fn active_ec_a_ratio(&self) -> f32 {
        self.adaptive_ec_a_ratio
    }
    pub fn active_ec_b_ratio(&self) -> f32 {
        self.adaptive_ec_b_ratio
    }
    pub fn active_ph_up_ratio(&self) -> f32 {
        self.adaptive_ph_up_ratio
    }
    pub fn active_ph_down_ratio(&self) -> f32 {
        self.adaptive_ph_down_ratio
    }

    // Helpers cho tương thích report
    pub fn adaptive_ec_ratio(&self) -> f32 {
        (self.adaptive_ec_a_ratio + self.adaptive_ec_b_ratio) * 0.5
    }
    pub fn best_ec_ratio(&self) -> f32 {
        (self.best_ec_a_ratio + self.best_ec_b_ratio) * 0.5
    }
    pub fn adaptive_ph_ratio(&self) -> f32 {
        (self.adaptive_ph_up_ratio + self.adaptive_ph_down_ratio) * 0.5
    }
    pub fn best_ph_ratio(&self) -> f32 {
        (self.best_ph_up_ratio + self.best_ph_down_ratio) * 0.5
    }

    pub fn effective_ec_tolerance(&self, base: f32) -> f32 {
        let max_osc = self
            .ec_a_tracker
            .oscillation
            .max(self.ec_b_tracker.oscillation);
        adaptive_solver_tolerance(base, self.state, max_osc)
    }

    pub fn effective_ph_tolerance(&self, base: f32) -> f32 {
        let max_osc = self
            .ph_up_tracker
            .oscillation
            .max(self.ph_down_tracker.oscillation);
        adaptive_solver_tolerance(base, self.state, max_osc)
    }

    pub fn on_nutrient_dosing_ack(
        &mut self,
        dose_a: f32,
        dose_b: f32,
        actual_delta_ec: f32,
        config: &ControllerConfig,
        now_sec: u64,
    ) {
        let total = dose_a + dose_b;
        if total <= 0.0 || actual_delta_ec <= 0.0 {
            return;
        }

        if dose_a > 0.0 {
            let delta_a = actual_delta_ec * (dose_a / total);
            let expected_a = dose_a * config.ec_gain_per_ml;
            self.on_dosing_ack(delta_a, expected_a, TuneChannel::EcA, now_sec);
        }
        if dose_b > 0.0 {
            let delta_b = actual_delta_ec * (dose_b / total);
            let expected_b = dose_b * config.ec_gain_per_ml;
            self.on_dosing_ack(delta_b, expected_b, TuneChannel::EcB, now_sec);
        }
    }

    pub fn on_ph_dosing_ack(&mut self, response: f32, expected: f32, is_up: bool, now_sec: u64) {
        let channel = if is_up {
            TuneChannel::PhUp
        } else {
            TuneChannel::PhDown
        };
        self.on_dosing_ack(response, expected, channel, now_sec);
    }

    fn on_dosing_ack(&mut self, response: f32, expected: f32, channel: TuneChannel, now_sec: u64) {
        if expected <= 0.0 || !response.is_finite() || !expected.is_finite() {
            return;
        }

        let gain_vs_expected: f32 = response / expected.max(0.001_f32);

        let tracker = match channel {
            TuneChannel::EcA => &mut self.ec_a_tracker,
            TuneChannel::EcB => &mut self.ec_b_tracker,
            TuneChannel::PhUp => &mut self.ph_up_tracker,
            TuneChannel::PhDown => &mut self.ph_down_tracker,
        };

        tracker.push(gain_vs_expected - 1.0);

        let tune_delta = self.compute_delta(channel).clamp(-0.08, 0.08);
        if tune_delta != 0.0 {
            self.adjust_step_ratio(channel, tune_delta);
        }

        self.last_update_sec = now_sec;
        self.update_state();
    }

    fn adjust_step_ratio(&mut self, channel: TuneChannel, delta: f32) {
        match channel {
            TuneChannel::EcA => {
                self.adaptive_ec_a_ratio = (self.adaptive_ec_a_ratio + delta).clamp(0.1, 2.0);
                self.best_ec_a_ratio = self.best_ec_a_ratio.max(self.adaptive_ec_a_ratio);
            }
            TuneChannel::EcB => {
                self.adaptive_ec_b_ratio = (self.adaptive_ec_b_ratio + delta).clamp(0.1, 2.0);
                self.best_ec_b_ratio = self.best_ec_b_ratio.max(self.adaptive_ec_b_ratio);
            }
            TuneChannel::PhUp => {
                self.adaptive_ph_up_ratio = (self.adaptive_ph_up_ratio + delta).clamp(0.05, 1.0);
                self.best_ph_up_ratio = self.best_ph_up_ratio.max(self.adaptive_ph_up_ratio);
            }
            TuneChannel::PhDown => {
                self.adaptive_ph_down_ratio =
                    (self.adaptive_ph_down_ratio + delta).clamp(0.05, 1.0);
                self.best_ph_down_ratio = self.best_ph_down_ratio.max(self.adaptive_ph_down_ratio);
            }
        }
    }

    fn compute_delta(&self, channel: TuneChannel) -> f32 {
        let tracker = match channel {
            TuneChannel::EcA => &self.ec_a_tracker,
            TuneChannel::EcB => &self.ec_b_tracker,
            TuneChannel::PhUp => &self.ph_up_tracker,
            TuneChannel::PhDown => &self.ph_down_tracker,
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

    fn update_state(&mut self) {
        let err_ec_a = self.ec_a_tracker.current_error().abs();
        let err_ec_b = self.ec_b_tracker.current_error().abs();
        let err_ph_up = self.ph_up_tracker.current_error().abs();
        let err_ph_down = self.ph_down_tracker.current_error().abs();

        let max_err = err_ec_a.max(err_ec_b).max(err_ph_up).max(err_ph_down);
        let max_tracker_count = self
            .ec_a_tracker
            .count
            .max(self.ec_b_tracker.count)
            .max(self.ph_up_tracker.count)
            .max(self.ph_down_tracker.count);

        let confidence = self
            .gain_learner
            .ec_a
            .confidence
            .max(self.gain_learner.ec_b.confidence)
            .max(self.gain_learner.ph_up.confidence)
            .max(self.gain_learner.ph_down.confidence);

        self.refresh_variance_baseline();

        if self.is_degraded() {
            self.state = TunerState::Degraded;
            return;
        }

        self.state = match self.state {
            TunerState::Exploring if confidence > 0.3 => TunerState::Converging,
            TunerState::Converging if max_err < 0.1 && max_tracker_count >= 3 => TunerState::Stable,
            TunerState::Stable if max_err > 0.2 => TunerState::Converging,
            TunerState::Degraded if max_err <= 0.2 => TunerState::Converging,
            state => state,
        };
    }

    pub fn on_water_change(&mut self) {
        self.adaptive_ec_a_ratio += (self.best_ec_a_ratio - self.adaptive_ec_a_ratio) * 0.5;
        self.adaptive_ec_b_ratio += (self.best_ec_b_ratio - self.adaptive_ec_b_ratio) * 0.5;
        self.adaptive_ph_up_ratio += (self.best_ph_up_ratio - self.adaptive_ph_up_ratio) * 0.5;
        self.adaptive_ph_down_ratio +=
            (self.best_ph_down_ratio - self.adaptive_ph_down_ratio) * 0.5;

        self.ec_a_tracker.reset();
        self.ec_b_tracker.reset();
        self.ph_up_tracker.reset();
        self.ph_down_tracker.reset();
        self.state = TunerState::Converging;
    }

    pub fn on_manual_reset(&mut self) {
        self.ec_a_tracker.reset();
        self.ec_b_tracker.reset();
        self.ph_up_tracker.reset();
        self.ph_down_tracker.reset();
        self.state = TunerState::Converging;
    }

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

        // 1. Cập nhật GainLearner (Tách riêng A và B)
        if total_nutrient_ml > 0.5 && actual_delta_ec > 0.01 {
            self.gain_learner.update_nutrient_gains(
                sample.dose_a_ml,
                sample.dose_b_ml,
                actual_delta_ec,
                config,
            );
        }
        if ph_dose_ml > 0.1 && actual_delta_ph.abs() > 0.01 {
            self.gain_learner
                .update_ph_gain(ph_dose_ml, actual_delta_ph.abs(), is_ph_up, config);
        }

        // 2. AutoTuner ACK
        if total_nutrient_ml > 0.5 {
            self.on_nutrient_dosing_ack(
                sample.dose_a_ml,
                sample.dose_b_ml,
                actual_delta_ec,
                config,
                now_sec,
            );
        }
        if ph_dose_ml > 0.1 {
            let expected_ph_delta = if is_ph_up {
                ph_dose_ml * config.ph_shift_up_per_ml
            } else {
                ph_dose_ml * config.ph_shift_down_per_ml
            };
            if expected_ph_delta > 1e-6 {
                self.on_ph_dosing_ack(actual_delta_ph, expected_ph_delta, is_ph_up, now_sec);
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
        }

        true
    }

    fn refresh_variance_baseline(&mut self) {
        if self.gain_learner.ec_a.sample_count >= self.gain_learner.ec_a.min_samples
            && self.ec_a_variance_baseline <= 0.0
        {
            self.ec_a_variance_baseline = self.gain_learner.ec_a.variance.max(1e-6);
        }
        if self.gain_learner.ec_b.sample_count >= self.gain_learner.ec_b.min_samples
            && self.ec_b_variance_baseline <= 0.0
        {
            self.ec_b_variance_baseline = self.gain_learner.ec_b.variance.max(1e-6);
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
        let ec_a_deg = self.ec_a_variance_baseline > 0.0
            && self.gain_learner.ec_a.variance > self.ec_a_variance_baseline * 1.5;
        let ec_b_deg = self.ec_b_variance_baseline > 0.0
            && self.gain_learner.ec_b.variance > self.ec_b_variance_baseline * 1.5;
        let ph_deg = self.ph_variance_baseline > 0.0
            && ((self.gain_learner.ph_up.variance + self.gain_learner.ph_down.variance) * 0.5)
                > self.ph_variance_baseline * 1.5;

        ec_a_deg || ec_b_deg || ph_deg
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
                "step_ratio_ec": self.adaptive_ec_ratio(),
                "step_ratio_ph": self.adaptive_ph_ratio(),
                "best_ec_ratio": self.best_ec_ratio(),
                "best_ph_ratio": self.best_ph_ratio(),
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
                    self.kalman.confidence(0),
                    self.kalman.confidence(1),
                    self.kalman.confidence(2),
                    self.kalman.confidence(3),
                    self.kalman.confidence(4),
                    self.kalman.confidence(5),
                    self.kalman.confidence(6),
                    self.kalman.confidence(7),
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
