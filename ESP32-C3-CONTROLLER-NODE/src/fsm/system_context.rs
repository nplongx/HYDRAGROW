use crate::mqtt::PumpStatus;
use serde::{Deserialize, Serialize};

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
    pub adaptive_ec_ratio: f32,
    pub adaptive_ph_ratio: f32,
    pub best_ec_ratio: f32,
    pub best_ph_ratio: f32,
    pub locked: bool,
    pub abnormal_streak: u8,
    pub last_update_sec: u64,
    pub ec_delta_window: DeltaWindow,
    pub ph_delta_window: DeltaWindow,
    pub gain_learner: GainLearner,
    pub cooldown_adaptor: CooldownAdaptor,
    pub oscillation_detector: OscillationDetector,
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
}

pub struct DeltaWindow {
    pub hour_total: f32,
    pub hour_anchor_sec: u64,
    pub day_total: f32,
    pub day_anchor_sec: u64,
}

pub struct GainLearner {
    pub ema_ec_gain: f32,
    pub ema_ph_up_gain: f32,
    pub ema_ph_down_gain: f32,
    pub sample_count: u32,
    pub alpha: f32,
    pub confidence: f32,
    pub min_samples_to_trust: u32,
}

pub struct CooldownAdaptor {
    pub current_cooldown_sec: u64,
    pub min_cooldown_sec: u64,
    pub max_cooldown_sec: u64,
}

pub struct OscillationDetector {
    pub ph_direction_history: [bool; 4],
    pub streak: u8,
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
            locked: false,
            abnormal_streak: 0,
            last_update_sec: 0,
            ec_delta_window: DeltaWindow::default(),
            ph_delta_window: DeltaWindow::default(),
            gain_learner: GainLearner::default(),
            cooldown_adaptor: CooldownAdaptor::default(),
            oscillation_detector: OscillationDetector::default(),
        }
    }
}

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
        self.on_dosing_ack(response, expected, true, now_sec);
    }

    pub fn on_ph_dosing_ack(
        &mut self,
        response: f32,
        expected: f32,
        _config: &hydragrow_shared::ControllerConfig,
        is_up: bool,
        now_sec: u64,
    ) {
        let _ = is_up;
        self.on_dosing_ack(response, expected, false, now_sec);
    }

    fn on_dosing_ack(&mut self, response: f32, expected: f32, is_ec: bool, now_sec: u64) {
        if self.locked || expected <= 0.0 || !response.is_finite() || !expected.is_finite() {
            return;
        }

        // Đưa window vào scope riêng để giải phóng borrow ngay khi tính xong allowed_delta
        let allowed_delta = {
            let window = if is_ec {
                &mut self.ec_delta_window
            } else {
                &mut self.ph_delta_window
            };

            if window.hour_anchor_sec == 0 || now_sec.saturating_sub(window.hour_anchor_sec) >= 3600
            {
                window.hour_anchor_sec = now_sec;
                window.hour_total = 0.0;
            }

            let max_hour_delta: f32 = 0.08;
            (max_hour_delta - window.hour_total.abs()).max(0.0)
        };

        if allowed_delta < 0.005 {
            return;
        }

        // Áp dụng luôn _f32 để tránh lỗi E0689 tương tự
        let gain_vs_expected: f32 = response / expected.max(0.001_f32);
        let raw_delta: f32 = if gain_vs_expected > 2.0 {
            -0.01
        } else if gain_vs_expected < 1.0 {
            0.02
        } else {
            0.0
        };
        let tune_delta = raw_delta.clamp(-allowed_delta, allowed_delta);

        if tune_delta != 0.0 {
            // self đã không còn bị borrow bởi window nên gọi hàm bình thường
            self.adjust_step_ratio(is_ec, tune_delta);

            // Re-borrow window cục bộ để update hour_total
            let window = if is_ec {
                &mut self.ec_delta_window
            } else {
                &mut self.ph_delta_window
            };
            window.hour_total += tune_delta;

            if is_ec {
                self.best_ec_ratio = self.best_ec_ratio.max(self.adaptive_ec_ratio);
            } else {
                self.best_ph_ratio = self.best_ph_ratio.max(self.adaptive_ph_ratio);
            }
        }

        self.last_update_sec = now_sec;

        if (gain_vs_expected - 1.0).abs() > 1.0 {
            self.abnormal_streak = self.abnormal_streak.saturating_add(1);
        } else {
            self.abnormal_streak = 0;
        }

        if self.abnormal_streak >= 3 {
            self.locked = true;
        }
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
        self.locked
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
                "locked": self.locked,
                "abnormal_streak": self.abnormal_streak,
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
        }
    }
}

impl Default for DeltaWindow {
    fn default() -> Self {
        Self {
            hour_total: 0.0,
            hour_anchor_sec: 0,
            day_total: 0.0,
            day_anchor_sec: 0,
        }
    }
}

impl Default for GainLearner {
    fn default() -> Self {
        Self {
            ema_ec_gain: 0.0,
            ema_ph_up_gain: 0.0,
            ema_ph_down_gain: 0.0,
            sample_count: 0,
            alpha: 0.1,
            confidence: 0.0,
            min_samples_to_trust: 5,
        }
    }
}

impl Default for CooldownAdaptor {
    fn default() -> Self {
        Self {
            current_cooldown_sec: 60,
            min_cooldown_sec: 20,
            max_cooldown_sec: 300,
        }
    }
}

impl CooldownAdaptor {
    pub fn observe_cycle(
        &mut self,
        reached_target: bool,
        sensor_stabilize_sec: u64,
        config_cooldown_sec: u64,
    ) {
        if self.current_cooldown_sec == 0 {
            self.current_cooldown_sec = config_cooldown_sec.max(self.min_cooldown_sec);
        }
        if reached_target {
            self.current_cooldown_sec = self
                .current_cooldown_sec
                .saturating_sub((sensor_stabilize_sec / 3).max(1))
                .max(self.min_cooldown_sec);
        } else {
            self.current_cooldown_sec = (self.current_cooldown_sec
                + (sensor_stabilize_sec / 2).max(2))
            .min(self.max_cooldown_sec);
        }
    }

    pub fn effective_cooldown_sec(&self, config_cooldown_sec: u64) -> u64 {
        let base = if self.current_cooldown_sec == 0 {
            config_cooldown_sec
        } else {
            self.current_cooldown_sec
        };
        base.clamp(self.min_cooldown_sec, self.max_cooldown_sec)
    }
}

impl Default for OscillationDetector {
    fn default() -> Self {
        Self {
            ph_direction_history: [false; 4],
            streak: 0,
        }
    }
}

impl OscillationDetector {
    pub fn record_ph_dose(&mut self, is_up: bool) -> bool {
        self.ph_direction_history.rotate_right(1);
        self.ph_direction_history[0] = is_up;
        let oscillating = self.ph_direction_history[0] != self.ph_direction_history[1]
            && self.ph_direction_history[1] != self.ph_direction_history[2]
            && self.ph_direction_history[2] != self.ph_direction_history[3];
        if oscillating {
            self.streak = self.streak.saturating_add(1);
        } else {
            self.streak = 0;
        }
        oscillating
    }
}

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
        self.ema_ec_gain = if self.sample_count == 0 {
            observed_gain
        } else {
            self.alpha * observed_gain + (1.0 - self.alpha) * self.ema_ec_gain
        };
        self.sample_count = self.sample_count.saturating_add(1);
        self.confidence = (self.sample_count as f32 / self.min_samples_to_trust as f32).min(1.0);
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
            &mut self.ema_ph_up_gain
        } else {
            &mut self.ema_ph_down_gain
        };
        *target = if self.sample_count == 0 {
            observed_gain
        } else {
            self.alpha * observed_gain + (1.0 - self.alpha) * *target
        };
        self.sample_count = self.sample_count.saturating_add(1);
        self.confidence = (self.sample_count as f32 / self.min_samples_to_trust as f32).min(1.0);
    }

    pub fn effective_ec_gain(&self, config_gain: f32) -> f32 {
        if self.confidence >= 0.6 && self.sample_count >= self.min_samples_to_trust {
            0.6 * self.ema_ec_gain + 0.4 * config_gain
        } else {
            config_gain
        }
    }

    pub fn effective_ph_up_gain(&self, config_gain: f32) -> f32 {
        if self.confidence >= 0.6 && self.sample_count >= self.min_samples_to_trust {
            0.6 * self.ema_ph_up_gain + 0.4 * config_gain
        } else {
            config_gain
        }
    }

    pub fn effective_ph_down_gain(&self, config_gain: f32) -> f32 {
        if self.confidence >= 0.6 && self.sample_count >= self.min_samples_to_trust {
            0.6 * self.ema_ph_down_gain + 0.4 * config_gain
        } else {
            config_gain
        }
    }
}
