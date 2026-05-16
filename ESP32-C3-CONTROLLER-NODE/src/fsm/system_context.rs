use crate::mqtt::PumpStatus;

use super::actors::{
    dosing_actor::DosingActor, safety_guard::SafetyGuard, water_actor::WaterActor,
};
use super::phases::SystemPhase;
use super::types::PendingCalibrationSample;

pub type CronSchedule = String;

pub struct SystemContext {
    pub phase: SystemPhase,
    pub phase_finish_ms: Option<u64>,
    pub dosing: DosingActor,
    pub water: WaterActor,
    pub safety: SafetyGuard,
    pub calibration: CalibrationSampler,
    pub tuner: AutoTuner,
    pub peripherals: PeripheralState,
    pub water_change_cron: CronSchedule,
    pub last_water_change_sec: u64,
    pub next_water_change_trigger_sec: Option<u64>,
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
}

pub struct DeltaWindow {
    pub hour_total: f32,
    pub hour_anchor_sec: u64,
    pub day_total: f32,
    pub day_anchor_sec: u64,
}

impl SystemContext {}

impl Default for SystemContext {
    fn default() -> Self {
        Self {
            phase: SystemPhase::Booting,
            phase_finish_ms: None,
            dosing: DosingActor::new(),
            water: WaterActor::new(),
            safety: SafetyGuard::new(),
            calibration: CalibrationSampler::default(),
            tuner: AutoTuner::default(),
            peripherals: PeripheralState::default(),
            water_change_cron: String::new(),
            last_water_change_sec: 0,
            next_water_change_trigger_sec: None,
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
        now_sec: u64,
    ) {
        self.on_dosing_ack(response, expected, false, now_sec);
    }

    fn on_dosing_ack(&mut self, response: f32, expected: f32, is_ec: bool, now_sec: u64) {
        if self.locked || expected <= 0.0 || !response.is_finite() || !expected.is_finite() {
            return;
        }

        let window = if is_ec {
            &mut self.ec_delta_window
        } else {
            &mut self.ph_delta_window
        };

        if window.hour_anchor_sec == 0 || now_sec.saturating_sub(window.hour_anchor_sec) >= 3600 {
            window.hour_anchor_sec = now_sec;
            window.hour_total = 0.0;
        }

        let max_hour_delta: f32 = 0.08;
        let allowed_delta = (max_hour_delta - window.hour_total.abs()).max(0.0);
        if allowed_delta < 0.005 {
            return;
        }

        let gain_vs_expected = response / expected.max(0.001);
        let raw_delta = if gain_vs_expected > 2.0 {
            -0.01
        } else if gain_vs_expected < 1.0 {
            0.02
        } else {
            0.0
        };
        let tune_delta = raw_delta.clamp(-allowed_delta, allowed_delta);

        if tune_delta != 0.0 {
            self.adjust_step_ratio(is_ec, tune_delta);
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
        *ratio = (*ratio + delta).clamp(0.1, 2.0);
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    pub fn active_ec_ratio(&self) -> f32 {
        self.adaptive_ec_ratio
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
