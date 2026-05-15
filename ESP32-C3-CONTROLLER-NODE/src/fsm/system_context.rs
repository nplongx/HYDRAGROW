use crate::mqtt::PumpStatus;

use super::actors::{dosing_actor::DosingActor, safety_guard::SafetyGuard, water_actor::WaterActor};
use super::context::ControlContext;
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
    pub pending_publish_count: u32,
    pub sample_count_ec: u32,
    pub sample_count_ph_up: u32,
    pub sample_count_ph_down: u32,
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

impl SystemContext {
    pub fn from_legacy(ctx: &ControlContext) -> Self {
        let mut state = Self::default();
        state.sync_from_legacy(ctx);
        state
    }

    pub fn sync_from_legacy(&mut self, legacy: &ControlContext) {
        self.phase = SystemPhase::from(&legacy.current_state);

        self.calibration.pending_sample = legacy.pending_calibration_sample.clone();
        self.calibration.pending_publish_count = legacy.calibration_pending_publish_count;
        self.calibration.sample_count_ec = legacy.calibration_sample_count_ec;
        self.calibration.sample_count_ph_up = legacy.calibration_sample_count_ph_up;
        self.calibration.sample_count_ph_down = legacy.calibration_sample_count_ph_down;

        self.tuner.adaptive_ec_ratio = legacy.adaptive_ec_step_ratio;
        self.tuner.adaptive_ph_ratio = legacy.adaptive_ph_step_ratio;
        self.tuner.best_ec_ratio = legacy.best_known_ec_step_ratio;
        self.tuner.best_ph_ratio = legacy.best_known_ph_step_ratio;
        self.tuner.locked = legacy.auto_tune_locked;
        self.tuner.abnormal_streak = legacy.abnormal_sample_streak;
        self.tuner.last_update_sec = legacy.tuning_last_update_sec;
        self.tuner.ec_delta_window.hour_total = legacy.tuning_hour_ec_delta;
        self.tuner.ec_delta_window.hour_anchor_sec = legacy.tuning_hour_anchor_sec;
        self.tuner.ec_delta_window.day_total = legacy.tuning_day_ec_delta;
        self.tuner.ec_delta_window.day_anchor_sec = legacy.tuning_day_anchor_sec;
        self.tuner.ph_delta_window.hour_total = legacy.tuning_hour_ph_delta;
        self.tuner.ph_delta_window.hour_anchor_sec = legacy.tuning_hour_anchor_sec;
        self.tuner.ph_delta_window.day_total = legacy.tuning_day_ph_delta;
        self.tuner.ph_delta_window.day_anchor_sec = legacy.tuning_day_anchor_sec;

        self.peripherals.pump_status = legacy.pump_status.clone();
        self.peripherals.osaka_active = legacy.fsm_osaka_active;
        self.peripherals.osaka_pwm = legacy.current_osaka_pwm;
        self.peripherals.is_misting_active = legacy.is_misting_active;
        self.peripherals.last_mist_toggle_time = legacy.last_mist_toggle_time;
        self.peripherals.is_scheduled_mixing_active = legacy.is_scheduled_mixing_active;
        self.peripherals.last_mixing_start_sec = legacy.last_mixing_start_sec;
        self.peripherals.last_continuous_level = legacy.last_continuous_level;
        self.peripherals.previous_ec = legacy.previous_ec;
        self.peripherals.previous_ph = legacy.previous_ph;

        self.safety.manual_timeouts = legacy.manual_timeouts.clone();
        self.safety.safety_override_until = legacy.safety_override_until;
        self.safety.last_ec_before_dose = legacy.last_ec_before_dosing;
        self.safety.last_ph_before_dose = legacy.last_ph_before_dosing;
        self.safety.last_ph_dose_up = legacy.last_ph_dosing_is_up;
        self.safety.last_water_before_refill = legacy.last_water_before_refill;

        self.water_change_cron = legacy.current_water_change_cron_expr.clone();
        self.last_water_change_sec = legacy.last_water_change_time;
    }
}

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
            pending_publish_count: 0,
            sample_count_ec: 0,
            sample_count_ph_up: 0,
            sample_count_ph_down: 0,
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

impl Default for DeltaWindow {
    fn default() -> Self {
        Self { hour_total: 0.0, hour_anchor_sec: 0, day_total: 0.0, day_anchor_sec: 0 }
    }
}
