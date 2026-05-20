use hydragrow_shared::PumpStatus;
use serde::{Deserialize, Serialize};

use crate::fsm::matrix::{InteractionMatrix, KalmanCovarianceDiag};

use super::actors::{
    dosing_actor::DosingActor, safety_guard::SafetyGuard, water_actor::WaterActor,
};
use super::phases::SystemPhase;
use super::types::PendingCalibrationSample;

pub type CronSchedule = String;

pub struct SystemContext {
    pub dosing_cycle_count: u64,
    pub phase: SystemPhase,
    pub phase_finish_ms: Option<u64>,
    pub dosing: DosingActor,
    pub water: WaterActor,
    pub safety: SafetyGuard,
    pub calibration: CalibrationSampler,
    pub tuner: AutoTuner,
    pub peripherals: PeripheralState,
    pub water_change_cron: CronSchedule,
    pub scheduled_dosing_cron: CronSchedule,
    pub last_water_change_sec: u64,
    pub next_water_change_trigger_sec: Option<u64>,
    pub next_scheduled_dosing_trigger_sec: Option<u64>,
}

pub struct PeripheralState {
    pub pump_status: PumpStatus,
    pub osaka_active: bool,
    pub osaka_pwm: u32,
    pub is_misting_active: bool,
    pub last_mist_toggle_time: u64,
    pub is_scheduled_mixing_active: bool,
    pub last_mixing_start_sec: u64,
    pub last_continuous_level: bool,
    pub previous_ec: Option<f32>,
    pub previous_ph: Option<f32>,
}

pub struct CalibrationSampler {
    pub pending_sample: Option<PendingCalibrationSample>,
}

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

// pub struct InteractionMatrix {
//     pub ec_to_ec: f32,
//     pub ec_to_ph: f32,
//     pub ph_to_ec: f32,
//     pub ph_to_ph: f32,
// }
//
// impl InteractionMatrix {
//     pub fn from_scalar(value: f32) -> Self {
//         Self {
//             ec_to_ec: value,
//             ec_to_ph: value,
//             ph_to_ec: value,
//             ph_to_ph: value,
//         }
//     }
// }
//
// pub struct KalmanCovarianceDiag {
//     pub ec_variance: f32,
//     pub ph_variance: f32,
// }
//
// impl KalmanCovarianceDiag {
//     pub fn new() -> Self {
//         Self {
//             ec_variance: 1.0,
//             ph_variance: 1.0,
//         }
//     }
// }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvsSnapshot {
    pub step_ratio_ec: f32,
    pub step_ratio_ph: f32,
    pub last_water_change_sec: u64,
    pub hourly_dose_ec_ml: f32,
    pub hourly_dose_ph_ml: f32,
    pub hourly_window_start_sec: u64,
    pub retry_ec: u8,
    pub retry_ph: u8,
    pub dosing_cycle_count: u64,
    pub best_ec_ratio: f32,
    pub best_ph_ratio: f32,
    pub ema_ec_gain: f32,
    pub ema_ph_up_gain: f32,
    pub ema_ph_down_gain: f32,
    pub ec_sample_count: u32,
    pub ph_sample_count: u32,
    #[serde(default)]
    pub ph_up_sample_count: u32,
    #[serde(default)]
    pub ph_down_sample_count: u32,
    #[serde(default = "default_tuner_state")]
    pub tuner_state: u8,
    #[serde(default)]
    pub ec_variance_baseline: f32,
    #[serde(default)]
    pub ph_variance_baseline: f32,
    #[serde(default)]
    pub interaction_matrix: Option<[f32; 6]>,
    #[serde(default)]
    pub matrix_update_count: u32,
}

pub struct ConvergenceTracker {
    pub error_history: [f32; 8],
    pub head: usize,
    pub count: u8,
    pub trend: f32,
    pub oscillation: f32,
    pub stagnant_cycles: u8,
}

pub struct SingleGainLearner {
    pub ema: f32,
    pub sample_count: u32,
    pub alpha: f32,
    pub confidence: f32,
    pub min_samples: u32,
    pub variance: f32,
    pub last_observed: f32,
}

pub struct GainLearner {
    pub ec: SingleGainLearner,
    pub ph_up: SingleGainLearner,
    pub ph_down: SingleGainLearner,
}

impl SystemContext {}

impl Default for SystemContext {
    fn default() -> Self {
        Self {
            phase: SystemPhase::Booting,
            dosing_cycle_count: 0,
            phase_finish_ms: None,
            dosing: DosingActor::new(),
            water: WaterActor::new(),
            safety: SafetyGuard::new(),
            calibration: CalibrationSampler::default(),
            tuner: AutoTuner::default(),
            peripherals: PeripheralState::default(),
            water_change_cron: String::new(),
            scheduled_dosing_cron: String::new(),
            last_water_change_sec: 0,
            next_water_change_trigger_sec: None,
            next_scheduled_dosing_trigger_sec: None,
        }
    }
}

impl Default for PeripheralState {
    fn default() -> Self {
        Self {
            pump_status: PumpStatus::default(),
            osaka_active: false,
            osaka_pwm: 0,
            is_misting_active: false,
            last_mist_toggle_time: 0,
            is_scheduled_mixing_active: false,
            last_mixing_start_sec: 0,
            last_continuous_level: false,
            previous_ec: None,
            previous_ph: None,
        }
    }
}

impl Default for CalibrationSampler {
    fn default() -> Self {
        Self {
            pending_sample: None,
        }
    }
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
            interaction_matrix: InteractionMatrix::from_scalar(0.0, 0.0),
            kalman: KalmanCovarianceDiag::new(0.0, 0.0, 0.0),
            matrix_update_count: 0,
            matrix_is_warm: false,
        }
    }
}
//
// impl Default for KalmanState {
//     fn default() -> Self {
//         Self { g: [[0.0; 3]; 2] }
//     }
// }
//
// impl KalmanState {
//     pub fn predict(&mut self) {
//         // Placeholder for process-model prediction step.
//     }
// }

impl CalibrationSampler {
    pub fn start_sample(&mut self, sample: PendingCalibrationSample) {
        self.pending_sample = Some(sample);
    }

    pub fn finalize(&mut self) -> Option<PendingCalibrationSample> {
        self.pending_sample.take()
    }
}

impl AutoTuner {
    pub fn on_ec_dosing_ack(
        &mut self,
        response: f32,
        expected: f32,
        _config: &hydragrow_shared::ControllerConfig,
        now_sec: u64,
    ) {
        self.on_dosing_ack(response, expected, true, None, now_sec);
    }

    pub fn on_ph_dosing_ack(
        &mut self,
        response: f32,
        expected: f32,
        _config: &hydragrow_shared::ControllerConfig,
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

    pub fn is_locked(&self) -> bool {
        matches!(self.state, TunerState::Stable)
    }

    pub fn active_ec_ratio(&self) -> f32 {
        self.adaptive_ec_ratio
    }

    pub fn to_mqtt_payload(
        &self,
        device_id: &str,
        config: &hydragrow_shared::ControllerConfig,
        now_ms: u64,
    ) -> String {
        serde_json::json!({
            "type": "runtime_calibration_update",
            "device_id": device_id,
            "runtime_coefficients": {
                "step_ratio_ec": self.adaptive_ec_ratio,
                "step_ratio_ph": self.adaptive_ph_ratio,
                "best_ec_ratio": self.best_ec_ratio,
                "best_ph_ratio": self.best_ph_ratio,
                "state": self.state as u8,
                "ec_gain_per_ml": self.gain_learner.effective_ec_gain(config.ec_gain_per_ml),
                "ph_shift_up_per_ml": self.gain_learner.effective_ph_up_gain(config.ph_shift_up_per_ml),
                "ph_shift_down_per_ml": self.gain_learner.effective_ph_down_gain(config.ph_shift_down_per_ml),
            },
            "timestamp_ms": now_ms
        })
        .to_string()
    }
}

impl NvsSnapshot {
    pub fn from_context(ctx: &SystemContext, now_sec: u64) -> Self {
        let hourly_dose_ec_ml = ctx
            .safety
            .hourly_doses()
            .iter()
            .filter(|(pump, _)| pump.as_str() == "NutrientA" || pump.as_str() == "NutrientB")
            .map(|(_, history)| {
                history
                    .iter()
                    .filter(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600)
                    .map(|(_, ml)| ml)
                    .sum::<f32>()
            })
            .sum();

        let hourly_dose_ph_ml = ctx
            .safety
            .hourly_doses()
            .iter()
            .filter(|(pump, _)| pump.as_str() == "PhUp" || pump.as_str() == "PhDown")
            .map(|(_, history)| {
                history
                    .iter()
                    .filter(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600)
                    .map(|(_, ml)| ml)
                    .sum::<f32>()
            })
            .sum();

        Self {
            step_ratio_ec: ctx.tuner.adaptive_ec_ratio,
            step_ratio_ph: ctx.tuner.adaptive_ph_ratio,
            last_water_change_sec: ctx.last_water_change_sec,
            hourly_dose_ec_ml,
            hourly_dose_ph_ml,
            hourly_window_start_sec: now_sec.saturating_sub(3600),
            retry_ec: ctx.dosing.retry_ec,
            retry_ph: ctx.dosing.retry_ph,
            dosing_cycle_count: ctx.dosing_cycle_count,
            best_ec_ratio: ctx.tuner.best_ec_ratio,
            best_ph_ratio: ctx.tuner.best_ph_ratio,
            ema_ec_gain: ctx.tuner.gain_learner.ec.ema,
            ema_ph_up_gain: ctx.tuner.gain_learner.ph_up.ema,
            ema_ph_down_gain: ctx.tuner.gain_learner.ph_down.ema,
            ec_sample_count: ctx.tuner.gain_learner.ec.sample_count,
            ph_sample_count: ctx
                .tuner
                .gain_learner
                .ph_up
                .sample_count
                .max(ctx.tuner.gain_learner.ph_down.sample_count),
            ph_up_sample_count: ctx.tuner.gain_learner.ph_up.sample_count,
            ph_down_sample_count: ctx.tuner.gain_learner.ph_down.sample_count,
            tuner_state: ctx.tuner.state.as_u8(),
            ec_variance_baseline: ctx.tuner.ec_variance_baseline,
            ph_variance_baseline: ctx.tuner.ph_variance_baseline,
            interaction_matrix: Some(ctx.tuner.interaction_matrix.as_flat()),
            matrix_update_count: ctx.tuner.matrix_update_count,
        }
    }
}

fn default_tuner_state() -> u8 {
    TunerState::Converging as u8
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

impl Default for GainLearner {
    fn default() -> Self {
        Self {
            ec: SingleGainLearner::default(),
            ph_up: SingleGainLearner::default(),
            ph_down: SingleGainLearner::default(),
        }
    }
}

// impl Default for InteractionMatrix {
//     fn default() -> Self {
//         Self {
//             values: [0.0, 0.0, 0.0, 0.0, 1.0, 1.0],
//         }
//     }
// }

impl GainLearner {
    pub fn update_ec_gain(
        &mut self,
        dose_ml: f32,
        delta_ec: f32,
        config: &hydragrow_shared::ControllerConfig,
    ) {
        if dose_ml <= 0.0 || delta_ec <= 0.0 {
            return;
        }
        let observed_gain = delta_ec / dose_ml;
        let base = config.ec_gain_per_ml.max(0.0001);
        if observed_gain < base * 0.3 || observed_gain > base * 3.0 {
            return;
        }
        self.ec.update(observed_gain);
    }

    pub fn update_ph_gain(
        &mut self,
        dose_ml: f32,
        delta_ph: f32,
        is_up: bool,
        config: &hydragrow_shared::ControllerConfig,
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

impl SingleGainLearner {
    fn update(&mut self, observed_gain: f32) {
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

    pub fn recalculate_confidence(&mut self) {
        self.confidence = (self.sample_count as f32 / self.min_samples as f32).min(1.0);
    }
}

impl ConvergenceTracker {
    fn push(&mut self, error: f32) {
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
        let mut first = 0.0;
        let mut last = 0.0;
        let mut prev_sign = 0_i8;
        let mut sign_changes = 0_u8;
        let mut abs_sum = 0.0;
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
        let mean_abs = abs_sum / n as f32;
        if mean_abs > 0.98 {
            self.stagnant_cycles = self.stagnant_cycles.saturating_add(1);
        } else {
            self.stagnant_cycles = 0;
        }
    }
    fn current_error(&self) -> f32 {
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

impl AutoTuner {
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
        self.adaptive_ec_ratio =
            self.adaptive_ec_ratio + (self.best_ec_ratio - self.adaptive_ec_ratio) * 0.5;
        self.adaptive_ph_ratio =
            self.adaptive_ph_ratio + (self.best_ph_ratio - self.adaptive_ph_ratio) * 0.5;
        self.ec_tracker.reset();
        self.ph_tracker.reset();
        self.state = TunerState::Converging;
    }
    pub fn on_manual_reset(&mut self) {
        self.ec_tracker.reset();
        self.ph_tracker.reset();
        self.state = TunerState::Converging;
    }
    fn refresh_variance_baseline(&mut self) {
        let ec_ready = self.gain_learner.ec.sample_count >= self.gain_learner.ec.min_samples;
        if ec_ready && self.ec_variance_baseline <= 0.0 {
            self.ec_variance_baseline = self.gain_learner.ec.variance.max(1e-6);
        }
        let ph_samples =
            self.gain_learner.ph_up.sample_count + self.gain_learner.ph_down.sample_count;
        let ph_ready = ph_samples >= self.gain_learner.ph_up.min_samples;
        if ph_ready && self.ph_variance_baseline <= 0.0 {
            let ph_var =
                (self.gain_learner.ph_up.variance + self.gain_learner.ph_down.variance) * 0.5;
            self.ph_variance_baseline = ph_var.max(1e-6);
        }
        // EMA readiness != matrix RLS readiness; matrix_is_warm is updated by RLS/restore flow.
    }
    fn is_degraded(&self) -> bool {
        let ec_degraded = self.ec_variance_baseline > 0.0
            && self.gain_learner.ec.variance > self.ec_variance_baseline * 1.5;
        let ph_var = (self.gain_learner.ph_up.variance + self.gain_learner.ph_down.variance) * 0.5;
        let ph_degraded =
            self.ph_variance_baseline > 0.0 && ph_var > self.ph_variance_baseline * 1.5;
        ec_degraded || ph_degraded
    }
}
