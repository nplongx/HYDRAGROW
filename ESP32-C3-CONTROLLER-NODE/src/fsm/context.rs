use hydragrow_shared::ControllerConfig;
use log::warn;
use std::collections::HashMap;

use super::types::{PendingCalibrationSample, SystemState};
use crate::mqtt::{PumpStatus, SensorData};
use crate::pump::{PumpController, PumpType, WaterDirection};

// ---------------------------------------------------------------------------
// ControlContext – toàn bộ trạng thái runtime của FSM
// ---------------------------------------------------------------------------
pub struct ControlContext {
    pub current_state: SystemState,
    pub last_water_change_time: u64,
    pub last_scheduled_dose_time_sec: u64,

    pub next_cron_trigger_sec: Option<u64>,
    pub current_cron_expr: String,
    pub next_water_change_trigger_sec: Option<u64>,
    pub current_water_change_cron_expr: String,

    pub ec_retry_count: u8,
    pub ph_retry_count: u8,
    pub water_refill_retry_count: u8,
    pub last_ec_before_dosing: Option<f32>,
    pub last_ph_before_dosing: Option<f32>,
    pub last_ph_dosing_is_up: Option<bool>,
    pub last_water_before_refill: Option<f32>,
    pub last_water_before_drain: Option<f32>,
    pub previous_ec: Option<f32>,
    pub previous_ph: Option<f32>,
    pub last_continuous_level: bool,
    pub is_misting_active: bool,
    pub last_mist_toggle_time: u64,
    pub is_scheduled_mixing_active: bool,
    pub last_mixing_start_sec: u64,
    pub fsm_osaka_active: bool,
    pub current_osaka_pwm: u32,
    pub pump_status: PumpStatus,
    pub manual_timeouts: HashMap<String, u64>,
    pub safety_override_until: u64,

    // --- Auto-tune ---
    pub adaptive_ec_step_ratio: f32,
    pub adaptive_ph_step_ratio: f32,
    pub best_known_ec_step_ratio: f32,
    pub best_known_ph_step_ratio: f32,
    pub auto_tune_locked: bool,
    pub abnormal_sample_streak: u8,
    pub tuning_last_update_sec: u64,
    pub tuning_hour_anchor_sec: u64,
    pub tuning_day_anchor_sec: u64,
    pub tuning_hour_ec_delta: f32,
    pub tuning_hour_ph_delta: f32,
    pub tuning_day_ec_delta: f32,
    pub tuning_day_ph_delta: f32,

    // --- Lịch sử bơm theo giờ ---
    pub hourly_dose_history_ml_by_pump: HashMap<String, Vec<(u64, f32)>>,
    pub hourly_refill_history: Vec<u64>,
    pub hourly_drain_history: Vec<u64>,

    // --- Calibration ---
    pub pending_calibration_sample: Option<PendingCalibrationSample>,
    pub calibration_pending_publish_count: u32,
}

impl Default for ControlContext {
    fn default() -> Self {
        Self {
            current_state: SystemState::SystemBooting,
            last_water_change_time: 0,
            last_scheduled_dose_time_sec: 0,
            next_cron_trigger_sec: None,
            current_cron_expr: String::new(),
            next_water_change_trigger_sec: None,
            current_water_change_cron_expr: String::new(),
            ec_retry_count: 0,
            ph_retry_count: 0,
            water_refill_retry_count: 0,
            last_ec_before_dosing: None,
            last_ph_before_dosing: None,
            last_ph_dosing_is_up: None,
            last_water_before_refill: None,
            last_water_before_drain: None,
            previous_ec: None,
            previous_ph: None,
            last_continuous_level: false,
            is_misting_active: false,
            last_mist_toggle_time: 0,
            is_scheduled_mixing_active: false,
            last_mixing_start_sec: 0,
            fsm_osaka_active: false,
            current_osaka_pwm: 0,
            pump_status: PumpStatus::default(),
            manual_timeouts: HashMap::new(),
            safety_override_until: 0,
            adaptive_ec_step_ratio: 0.4,
            adaptive_ph_step_ratio: 0.2,
            best_known_ec_step_ratio: 0.4,
            best_known_ph_step_ratio: 0.2,
            auto_tune_locked: false,
            abnormal_sample_streak: 0,
            tuning_last_update_sec: 0,
            tuning_hour_anchor_sec: 0,
            tuning_day_anchor_sec: 0,
            tuning_hour_ec_delta: 0.0,
            tuning_hour_ph_delta: 0.0,
            tuning_day_ec_delta: 0.0,
            tuning_day_ph_delta: 0.0,
            hourly_dose_history_ml_by_pump: HashMap::new(),
            hourly_refill_history: Vec::new(),
            hourly_drain_history: Vec::new(),
            pending_calibration_sample: None,
            calibration_pending_publish_count: 0,
        }
    }
}

impl ControlContext {
    // -----------------------------------------------------------------------
    // Pump control helpers
    // -----------------------------------------------------------------------

    pub fn stop_all_pumps(&mut self, pump_ctrl: &mut PumpController) {
        let _ = pump_ctrl.stop_all();
        self.pump_status = PumpStatus::default();
        self.is_misting_active = false;
        self.is_scheduled_mixing_active = false;
        self.fsm_osaka_active = false;
        self.current_osaka_pwm = 0;
        self.manual_timeouts.clear();
    }

    pub fn set_pulse_status(&mut self, active: bool, pulse_count: u32) {
        self.pump_status.dosing_pulse_active = active;
        self.pump_status.dosing_pulse_count = pulse_count;
    }

    pub fn reset_faults(&mut self) {
        self.ec_retry_count = 0;
        self.ph_retry_count = 0;
        self.water_refill_retry_count = 0;
        self.last_ec_before_dosing = None;
        self.last_ph_before_dosing = None;
        self.last_water_before_refill = None;
        self.fsm_osaka_active = false;
        self.current_state = SystemState::Monitoring;
    }

    pub fn turn_off_pump(&mut self, pump_name: &str, pump_ctrl: &mut PumpController) {
        let _ = match pump_name {
            "A" | "PUMP_A" => {
                self.pump_status.pump_a = false;
                self.pump_status.pump_a_pwm = Some(0);
                pump_ctrl.set_pump_state(PumpType::NutrientA, false)
            }
            "B" | "PUMP_B" => {
                self.pump_status.pump_b = false;
                self.pump_status.pump_b_pwm = Some(0);
                pump_ctrl.set_pump_state(PumpType::NutrientB, false)
            }
            "PH_UP" | "PUMP_PH_UP" => {
                self.pump_status.ph_up = false;
                self.pump_status.ph_up_pwm = Some(0);
                pump_ctrl.set_pump_state(PumpType::PhUp, false)
            }
            "PH_DOWN" | "PUMP_PH_DOWN" => {
                self.pump_status.ph_down = false;
                self.pump_status.ph_down_pwm = Some(0);
                pump_ctrl.set_pump_state(PumpType::PhDown, false)
            }
            "OSAKA_PUMP" | "OSAKA" => {
                self.pump_status.osaka_pump = false;
                self.pump_status.osaka_pwm = Some(0);
                pump_ctrl.set_osaka_pump(false)
            }
            "MIST_VALVE" | "MIST" => {
                self.pump_status.mist_valve = false;
                self.is_misting_active = false;
                pump_ctrl.set_mist_valve(false)
            }
            "WATER_PUMP" | "WATER_PUMP_IN" | "PUMP_IN" => {
                self.pump_status.water_pump_in = false;
                pump_ctrl.set_water_pump(WaterDirection::Stop)
            }
            "DRAIN_PUMP" | "WATER_PUMP_OUT" | "PUMP_OUT" => {
                self.pump_status.water_pump_out = false;
                pump_ctrl.set_water_pump(WaterDirection::Stop)
            }
            _ => Ok(()),
        };
        self.set_pulse_status(false, 0);
    }

    // -----------------------------------------------------------------------
    // Noise / sensor ACK
    // -----------------------------------------------------------------------

    pub fn check_and_update_noise(
        &mut self,
        sensors: &SensorData,
        config: &ControllerConfig,
    ) -> bool {
        let mut is_noisy = false;
        if config.enable_ec_sensor && !sensors.err_ec {
            if let Some(prev_ec) = self.previous_ec {
                if (sensors.ec - prev_ec).abs() > config.max_ec_delta {
                    warn!("⚠️ Nhiễu EC. Bỏ qua nhịp này!");
                    is_noisy = true;
                }
            }
            self.previous_ec = Some(sensors.ec);
        }
        if config.enable_ph_sensor && !sensors.err_ph {
            if let Some(prev_ph) = self.previous_ph {
                if (sensors.ph - prev_ph).abs() > config.max_ph_delta {
                    warn!("⚠️ Nhiễu pH. Bỏ qua nhịp này!");
                    is_noisy = true;
                }
            }
            self.previous_ph = Some(sensors.ph);
        }
        is_noisy
    }

    pub fn mark_pending_sample_noise_violation(&mut self) {
        if let Some(sample) = self.pending_calibration_sample.as_mut() {
            sample.invalid_by_noise = true;
        }
    }

    pub fn mark_pending_sample_water_change_violation(&mut self) {
        if let Some(sample) = self.pending_calibration_sample.as_mut() {
            sample.invalid_by_water_change = true;
        }
    }

    pub fn verify_sensor_ack(
        &mut self,
        sensors: &SensorData,
        config: &ControllerConfig,
        now_sec: u64,
    ) {
        if config.enable_ec_sensor && !sensors.err_ec {
            if let Some(last_ec) = self.last_ec_before_dosing {
                let response = sensors.ec - last_ec;
                if response >= config.ec_ack_threshold {
                    self.ec_retry_count = 0;
                    if !self.auto_tune_locked {
                        let gain_vs_expected = response / config.ec_ack_threshold.max(0.001);
                        let tune_delta = if gain_vs_expected > 2.0 {
                            -0.01
                        } else if gain_vs_expected < 1.0 {
                            0.02
                        } else {
                            0.0
                        };
                        self.adjust_ec_step_ratio(config, now_sec, tune_delta);
                        self.best_known_ec_step_ratio = self.adaptive_ec_step_ratio;
                    }
                } else {
                    self.ec_retry_count += 1;
                    warn!("⚠️ EC không tăng! Lần thử: {}/3", self.ec_retry_count);
                    if !self.auto_tune_locked {
                        self.adjust_ec_step_ratio(config, now_sec, 0.03);
                    }
                }
                self.last_ec_before_dosing = None;
            }
        }

        if config.enable_ph_sensor && !sensors.err_ph {
            if let Some(last_ph) = self.last_ph_before_dosing {
                let is_up = self.last_ph_dosing_is_up.unwrap_or(true);
                let response = if is_up {
                    sensors.ph - last_ph
                } else {
                    last_ph - sensors.ph
                };
                if response >= config.ph_ack_threshold {
                    self.ph_retry_count = 0;
                    if !self.auto_tune_locked {
                        let gain_vs_expected = response / config.ph_ack_threshold.max(0.001);
                        let tune_delta = if gain_vs_expected > 2.0 {
                            -0.01
                        } else if gain_vs_expected < 1.0 {
                            0.02
                        } else {
                            0.0
                        };
                        self.adjust_ph_step_ratio(config, now_sec, tune_delta);
                        self.best_known_ph_step_ratio = self.adaptive_ph_step_ratio;
                    }
                } else {
                    self.ph_retry_count += 1;
                    warn!("⚠️ pH không đổi hướng! Lần thử: {}/3", self.ph_retry_count);
                    if !self.auto_tune_locked {
                        self.adjust_ph_step_ratio(config, now_sec, 0.03);
                    }
                }
                self.last_ph_before_dosing = None;
                self.last_ph_dosing_is_up = None;
            }
        }

        if config.enable_water_level_sensor && !sensors.err_water {
            if let Some(w) = self.last_water_before_refill {
                if (sensors.water_level - w) >= config.water_ack_threshold {
                    self.water_refill_retry_count = 0;
                } else {
                    self.water_refill_retry_count += 1;
                }
                self.last_water_before_refill = None;
            }
            if let Some(w) = self.last_water_before_drain {
                if (w - sensors.water_level) >= config.water_ack_threshold {
                    self.water_refill_retry_count = 0;
                } else {
                    self.water_refill_retry_count += 1;
                }
                self.last_water_before_drain = None;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Auto-tune
    // -----------------------------------------------------------------------

    pub fn sync_adaptive_ratios_from_config(&mut self, config: &ControllerConfig) {
        if self.tuning_last_update_sec == 0 {
            self.adaptive_ec_step_ratio = config.ec_step_ratio;
            self.adaptive_ph_step_ratio = config.ph_step_ratio;
            self.best_known_ec_step_ratio = config.ec_step_ratio;
            self.best_known_ph_step_ratio = config.ph_step_ratio;
        }
    }

    fn ensure_tuning_windows(&mut self, now_sec: u64) {
        if self.tuning_hour_anchor_sec == 0 {
            self.tuning_hour_anchor_sec = now_sec;
        }
        if self.tuning_day_anchor_sec == 0 {
            self.tuning_day_anchor_sec = now_sec;
        }
        if now_sec.saturating_sub(self.tuning_hour_anchor_sec) >= 3600 {
            self.tuning_hour_anchor_sec = now_sec;
            self.tuning_hour_ec_delta = 0.0;
            self.tuning_hour_ph_delta = 0.0;
        }
        if now_sec.saturating_sub(self.tuning_day_anchor_sec) >= 86_400 {
            self.tuning_day_anchor_sec = now_sec;
            self.tuning_day_ec_delta = 0.0;
            self.tuning_day_ph_delta = 0.0;
        }
    }

    pub fn adjust_ec_step_ratio(
        &mut self,
        config: &ControllerConfig,
        now_sec: u64,
        requested_delta: f32,
    ) {
        self.ensure_tuning_windows(now_sec);
        let min_ratio = (config.ec_step_ratio * 0.4).max(0.05);
        let max_ratio = (config.ec_step_ratio * 1.8).min(2.5);
        let allowed_hour = (0.08 - self.tuning_hour_ec_delta.abs()).max(0.0);
        let allowed_day = (0.25 - self.tuning_day_ec_delta.abs()).max(0.0);
        let applied_delta = requested_delta.clamp(
            -allowed_hour.min(allowed_day),
            allowed_hour.min(allowed_day),
        );
        self.adaptive_ec_step_ratio =
            (self.adaptive_ec_step_ratio + applied_delta).clamp(min_ratio, max_ratio);
        self.tuning_hour_ec_delta += applied_delta;
        self.tuning_day_ec_delta += applied_delta;
        self.tuning_last_update_sec = now_sec;
    }

    pub fn adjust_ph_step_ratio(
        &mut self,
        config: &ControllerConfig,
        now_sec: u64,
        requested_delta: f32,
    ) {
        self.ensure_tuning_windows(now_sec);
        let min_ratio = (config.ph_step_ratio * 0.4).max(0.05);
        let max_ratio = (config.ph_step_ratio * 1.8).min(2.5);
        let allowed_hour = (0.08 - self.tuning_hour_ph_delta.abs()).max(0.0);
        let allowed_day = (0.25 - self.tuning_day_ph_delta.abs()).max(0.0);
        let applied_delta = requested_delta.clamp(
            -allowed_hour.min(allowed_day),
            allowed_hour.min(allowed_day),
        );
        self.adaptive_ph_step_ratio =
            (self.adaptive_ph_step_ratio + applied_delta).clamp(min_ratio, max_ratio);
        self.tuning_hour_ph_delta += applied_delta;
        self.tuning_day_ph_delta += applied_delta;
        self.tuning_last_update_sec = now_sec;
    }

    pub fn update_auto_tune_health(&mut self, abnormal_sample: bool) {
        if abnormal_sample {
            self.abnormal_sample_streak = self.abnormal_sample_streak.saturating_add(1);
            if self.abnormal_sample_streak >= 3 && !self.auto_tune_locked {
                self.auto_tune_locked = true;
                self.adaptive_ec_step_ratio = self.best_known_ec_step_ratio;
                self.adaptive_ph_step_ratio = self.best_known_ph_step_ratio;
                warn!(
                    "🔒 Khóa auto-tune do 3 mẫu liên tiếp bất thường. Fallback hệ số EC={:.3}, pH={:.3}",
                    self.adaptive_ec_step_ratio, self.adaptive_ph_step_ratio
                );
            }
        } else {
            self.abnormal_sample_streak = 0;
        }
    }

    // -----------------------------------------------------------------------
    // Hourly dose / refill / drain rate limiting
    // -----------------------------------------------------------------------

    pub fn get_hourly_total_dose_ml(&mut self, pump: &str, now_sec: u64) -> f32 {
        let history = self
            .hourly_dose_history_ml_by_pump
            .entry(pump.to_string())
            .or_default();
        history.retain(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600);
        history.iter().map(|(_, ml)| *ml).sum()
    }

    pub fn reserve_dose_if_within_hourly_limit(
        &mut self,
        pump: &str,
        now_sec: u64,
        dose_ml: f32,
        max_hourly_ml: f32,
    ) -> bool {
        let used = self.get_hourly_total_dose_ml(pump, now_sec);
        if used + dose_ml > max_hourly_ml {
            warn!(
                "⚠️ Giới hạn giờ cho {}: đã dùng {:.2}ml, yêu cầu thêm {:.2}ml (max {:.2}ml/h)",
                pump, used, dose_ml, max_hourly_ml
            );
            return false;
        }
        self.hourly_dose_history_ml_by_pump
            .entry(pump.to_string())
            .or_default()
            .push((now_sec, dose_ml));
        true
    }

    pub fn can_dose_within_hourly_limit(
        &mut self,
        pump: &str,
        now_sec: u64,
        dose_ml: f32,
        max_hourly_ml: f32,
    ) -> bool {
        let used = self.get_hourly_total_dose_ml(pump, now_sec);
        used + dose_ml <= max_hourly_ml
    }

    /// Ghi nhận và kiểm tra giới hạn số lần bơm nước vào / giờ.
    pub fn check_and_record_refill_limit(&mut self, now_sec: u64, limit: u32) -> bool {
        self.hourly_refill_history
            .retain(|&ts| now_sec.saturating_sub(ts) <= 3600);
        if self.hourly_refill_history.len() >= limit as usize {
            warn!(
                "⚠️ Quá giới hạn bơm nước vào bồn trong 1 giờ (max: {} lần). Ngắt an toàn để chống kẹt phao!",
                limit
            );
            false
        } else {
            self.hourly_refill_history.push(now_sec);
            true
        }
    }

    /// Ghi nhận và kiểm tra giới hạn số lần mở van xả / giờ.
    pub fn check_and_record_drain_limit(&mut self, now_sec: u64, limit: u32) -> bool {
        self.hourly_drain_history
            .retain(|&ts| now_sec.saturating_sub(ts) <= 3600);
        if self.hourly_drain_history.len() >= limit as usize {
            warn!(
                "⚠️ Quá giới hạn mở van xả nước ra trong 1 giờ (max: {} lần). Ngắt an toàn!",
                limit
            );
            false
        } else {
            self.hourly_drain_history.push(now_sec);
            true
        }
    }
}
