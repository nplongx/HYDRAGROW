use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition, EspNvs};
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{Local, TimeZone};
use cron::Schedule;
use std::str::FromStr;

use crate::config::{ControlMode, DeviceConfig, SharedConfig};
use crate::mqtt::{MqttCommandPayload, PumpStatus, SensorData};
use crate::pump::{PumpController, PumpType, WaterDirection};

pub type SharedSensorData = Arc<RwLock<SensorData>>;

#[derive(Debug, Clone, PartialEq)]
pub enum PendingDose {
    EC {
        dose_ml: f32,
        target_ec: f32,
        pwm_percent: u32,
    },
    PH {
        is_up: bool,
        dose_ml: f32,
        target_ph: f32,
        pwm_percent: u32,
    },
    ScheduledDose {
        dose_a_ml: f32,
        dose_b_ml: f32,
        pwm_percent: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum SystemState {
    SystemBooting,
    ManualMode,
    DosingCycleComplete,
    Monitoring,
    EmergencyStop(String),
    SystemFault(String),
    SensorCalibration {
        step: String,
        finish_time: u64,
    },
    WaterRefilling {
        target_level: f32,
        start_time: u64,
    },
    WaterDraining {
        target_level: f32,
        start_time: u64,
    },
    StartingOsakaPump {
        finish_time: u64,
        pending_action: PendingDose,
    },
    DosingPumpA {
        next_toggle_time: u64,
        dose_target_ml: f32,
        delivered_ml_est: f32,
        dose_b_ml: f32,
        pulse_on: bool,
        pulse_count: u32,
        max_pulse_count: u32,
        pulse_on_ms: u64,
        pulse_off_ms: u64,
        pwm_percent: u32,
        active_capacity_ml_per_sec: f32,
        target_ec: f32,
        start_ec: f32,
        start_ph: f32,
    },
    WaitingBetweenDose {
        finish_time: u64,
        dose_b_ml: f32,
        target_ec: f32,
        start_ec: f32,
        start_ph: f32,
        dose_a_ml_reported: f32,
    },
    DosingPumpB {
        next_toggle_time: u64,
        dose_target_ml: f32,
        delivered_ml_est: f32,
        pulse_on: bool,
        pulse_count: u32,
        max_pulse_count: u32,
        pulse_on_ms: u64,
        pulse_off_ms: u64,
        pwm_percent: u32,
        active_capacity_ml_per_sec: f32,
        target_ec: f32,
        start_ec: f32,
        start_ph: f32,
        dose_a_ml_reported: f32,
    },
    DosingPH {
        next_toggle_time: u64,
        is_up: bool,
        dose_target_ml: f32,
        delivered_ml_est: f32,
        pulse_on: bool,
        pulse_count: u32,
        max_pulse_count: u32,
        pulse_on_ms: u64,
        pulse_off_ms: u64,
        pwm_percent: u32,
        active_capacity_ml_per_sec: f32,
        target_ph: f32,
        start_ec: f32,
        start_ph: f32,
    },
    ActiveMixing {
        finish_time: u64,
    },
    Stabilizing {
        finish_time: u64,
    },
}

impl SystemState {
    pub fn to_payload_string(&self) -> String {
        match self {
            SystemState::SystemBooting => "SystemBooting".to_string(),
            SystemState::ManualMode => "ManualMode".to_string(),
            SystemState::DosingCycleComplete => "DosingCycleComplete".to_string(),
            SystemState::Monitoring => "Monitoring".to_string(),
            SystemState::EmergencyStop(reason) => format!("EmergencyStop:{}", reason),
            SystemState::SystemFault(reason) => format!("SystemFault:{}", reason),
            SystemState::SensorCalibration { step, .. } => format!("SensorCalibration:{}", step),
            SystemState::WaterRefilling { .. } => "WaterRefilling".to_string(),
            SystemState::WaterDraining { .. } => "WaterDraining".to_string(),
            SystemState::DosingPumpA { .. } => "DosingPumpA".to_string(),
            SystemState::WaitingBetweenDose { .. } => "WaitingBetweenDose".to_string(),
            SystemState::DosingPumpB { .. } => "DosingPumpB".to_string(),
            SystemState::DosingPH { .. } => "DosingPH".to_string(),
            SystemState::StartingOsakaPump { .. } => "StartingOsakaPump".to_string(),
            SystemState::ActiveMixing { .. } => "ActiveMixing".to_string(),
            SystemState::Stabilizing { .. } => "Stabilizing".to_string(),
        }
    }
}

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
    pub hourly_dose_history_ml_by_pump: HashMap<String, Vec<(u64, f32)>>,
    pub pending_calibration_sample: Option<PendingCalibrationSample>,
    pub calibration_pending_publish_count: u32,
}

#[derive(Debug, Clone)]
pub struct PendingCalibrationSample {
    pub start_ec: f32,
    pub start_ph: f32,
    pub pump_a_ml: f32,
    pub pump_b_ml: f32,
    pub ph_up_ml: f32,
    pub ph_down_ml: f32,
    pub active_mixing_start_ms: u64,
    pub active_mixing_finish_ms: u64,
    pub stabilizing_start_ms: Option<u64>,
    pub stabilizing_finish_ms: Option<u64>,
    pub invalid_by_noise: bool,
    pub invalid_by_water_change: bool,
}

#[derive(Debug, Clone, Copy)]
enum DosePumpKind {
    PumpA,
    PumpB,
    PhUp,
    PhDown,
}

fn effective_flow_ml_per_sec(
    pump: DosePumpKind,
    pwm_percent: u32,
    config: &DeviceConfig,
) -> Option<f32> {
    let (capacity, min_pwm) = match pump {
        DosePumpKind::PumpA => (
            config.pump_a_capacity_ml_per_sec,
            config
                .pump_a_min_pwm_percent
                .unwrap_or(config.dosing_min_pwm_percent),
        ),
        DosePumpKind::PumpB => (
            config.pump_b_capacity_ml_per_sec,
            config
                .pump_b_min_pwm_percent
                .unwrap_or(config.dosing_min_pwm_percent),
        ),
        DosePumpKind::PhUp => (
            config.pump_ph_up_capacity_ml_per_sec,
            config
                .pump_ph_up_min_pwm_percent
                .unwrap_or(config.dosing_min_pwm_percent),
        ),
        DosePumpKind::PhDown => (
            config.pump_ph_down_capacity_ml_per_sec,
            config
                .pump_ph_down_min_pwm_percent
                .unwrap_or(config.dosing_min_pwm_percent),
        ),
    };

    let safe_pwm = pwm_percent.clamp(1, 100);
    let safe_min_pwm = min_pwm.clamp(1, 100);
    if capacity <= 0.0 || safe_pwm < safe_min_pwm {
        return None;
    }

    Some(capacity * (safe_pwm as f32 / 100.0))
}

impl Default for ControlContext {
    fn default() -> Self {
        Self {
            current_state: SystemState::SystemBooting, // 🟢 KHỞI ĐỘNG VÀO STATE NÀY
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
            pending_calibration_sample: None,
            calibration_pending_publish_count: 0,
        }
    }
}

impl ControlContext {
    fn stop_all_pumps(&mut self, pump_ctrl: &mut PumpController) {
        let _ = pump_ctrl.stop_all();
        self.pump_status = PumpStatus::default();
        self.is_misting_active = false;
        self.is_scheduled_mixing_active = false;
        self.fsm_osaka_active = false;
        self.current_osaka_pwm = 0;
        self.manual_timeouts.clear();
    }

    fn set_pulse_status(&mut self, active: bool, pulse_count: u32) {
        self.pump_status.dosing_pulse_active = active;
        self.pump_status.dosing_pulse_count = pulse_count;
    }

    fn reset_faults(&mut self) {
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

    fn check_and_update_noise(&mut self, sensors: &SensorData, config: &DeviceConfig) -> bool {
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

    fn mark_pending_sample_noise_violation(&mut self) {
        if let Some(sample) = self.pending_calibration_sample.as_mut() {
            sample.invalid_by_noise = true;
        }
    }

    fn mark_pending_sample_water_change_violation(&mut self) {
        if let Some(sample) = self.pending_calibration_sample.as_mut() {
            sample.invalid_by_water_change = true;
        }
    }

    fn verify_sensor_ack(&mut self, sensors: &SensorData, config: &DeviceConfig, now_sec: u64) {
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
                let is_ack_ok = response >= config.ph_ack_threshold;
                if is_ack_ok {
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

    fn sync_adaptive_ratios_from_config(&mut self, config: &DeviceConfig) {
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

    fn adjust_ec_step_ratio(&mut self, config: &DeviceConfig, now_sec: u64, requested_delta: f32) {
        self.ensure_tuning_windows(now_sec);
        let min_ratio = (config.ec_step_ratio * 0.4).max(0.05);
        let max_ratio = (config.ec_step_ratio * 1.8).min(2.5);
        let allowed_hour = (0.08 - self.tuning_hour_ec_delta.abs()).max(0.0);
        let allowed_day = (0.25 - self.tuning_day_ec_delta.abs()).max(0.0);
        let allowed = allowed_hour.min(allowed_day);
        let applied_delta = requested_delta.clamp(-allowed, allowed);
        self.adaptive_ec_step_ratio =
            (self.adaptive_ec_step_ratio + applied_delta).clamp(min_ratio, max_ratio);
        self.tuning_hour_ec_delta += applied_delta;
        self.tuning_day_ec_delta += applied_delta;
        self.tuning_last_update_sec = now_sec;
    }

    fn adjust_ph_step_ratio(&mut self, config: &DeviceConfig, now_sec: u64, requested_delta: f32) {
        self.ensure_tuning_windows(now_sec);
        let min_ratio = (config.ph_step_ratio * 0.4).max(0.05);
        let max_ratio = (config.ph_step_ratio * 1.8).min(2.5);
        let allowed_hour = (0.08 - self.tuning_hour_ph_delta.abs()).max(0.0);
        let allowed_day = (0.25 - self.tuning_day_ph_delta.abs()).max(0.0);
        let allowed = allowed_hour.min(allowed_day);
        let applied_delta = requested_delta.clamp(-allowed, allowed);
        self.adaptive_ph_step_ratio =
            (self.adaptive_ph_step_ratio + applied_delta).clamp(min_ratio, max_ratio);
        self.tuning_hour_ph_delta += applied_delta;
        self.tuning_day_ph_delta += applied_delta;
        self.tuning_last_update_sec = now_sec;
    }

    fn update_auto_tune_health(&mut self, abnormal_sample: bool) {
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

    fn get_hourly_total_dose_ml(&mut self, pump: &str, now_sec: u64) -> f32 {
        let history = self
            .hourly_dose_history_ml_by_pump
            .entry(pump.to_string())
            .or_default();
        history.retain(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600);
        history.iter().map(|(_, ml)| *ml).sum()
    }

    fn reserve_dose_if_within_hourly_limit(
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

    fn can_dose_within_hourly_limit(
        &mut self,
        pump: &str,
        now_sec: u64,
        dose_ml: f32,
        max_hourly_ml: f32,
    ) -> bool {
        let used = self.get_hourly_total_dose_ml(pump, now_sec);
        used + dose_ml <= max_hourly_ml
    }
}

fn soft_deadband_scale(error: f32, tolerance: f32) -> f32 {
    let soft_zone_end = (tolerance * 3.0).max(tolerance + 0.01);
    if error <= tolerance {
        0.0
    } else if error >= soft_zone_end {
        1.0
    } else {
        0.35 + 0.65 * ((error - tolerance) / (soft_zone_end - tolerance))
    }
}

const EMA_ALPHA: f32 = 0.1;
const MIN_TOTAL_EC_DOSE_ML: f32 = 0.05;
const MIN_PH_DOSE_ML: f32 = 0.05;
const MIN_ACTIVE_MIXING_SEC_FOR_CALIB: u64 = 3;
const MIN_STABILIZING_SEC_FOR_CALIB: u64 = 3;
const CALIBRATION_PERSIST_BATCH_SIZE: u32 = 3;

fn start_pending_calibration_sample(
    ctx: &mut ControlContext,
    start_ec: f32,
    start_ph: f32,
    pump_a_ml: f32,
    pump_b_ml: f32,
    ph_up_ml: f32,
    ph_down_ml: f32,
    current_time_ms: u64,
    config: &DeviceConfig,
) {
    ctx.pending_calibration_sample = Some(PendingCalibrationSample {
        start_ec,
        start_ph,
        pump_a_ml,
        pump_b_ml,
        ph_up_ml,
        ph_down_ml,
        active_mixing_start_ms: current_time_ms,
        active_mixing_finish_ms: current_time_ms + (config.active_mixing_sec as u64 * 1000),
        stabilizing_start_ms: None,
        stabilizing_finish_ms: None,
        invalid_by_noise: false,
        invalid_by_water_change: false,
    });
}

fn apply_runtime_calibration_ema(
    sensors: &SensorData,
    shared_config: &SharedConfig,
    ctx: &mut ControlContext,
    fsm_mqtt_tx: &Sender<String>,
) {
    let sample = match ctx.pending_calibration_sample.take() {
        Some(sample) => sample,
        None => return,
    };
    let stabilizing_start_ms = match sample.stabilizing_start_ms {
        Some(v) => v,
        None => return,
    };
    let stabilizing_finish_ms = match sample.stabilizing_finish_ms {
        Some(v) => v,
        None => return,
    };

    let active_mixing_elapsed_ms = sample
        .active_mixing_finish_ms
        .saturating_sub(sample.active_mixing_start_ms);
    let stabilizing_elapsed_ms = stabilizing_finish_ms.saturating_sub(stabilizing_start_ms);
    let mixing_ok = active_mixing_elapsed_ms >= MIN_ACTIVE_MIXING_SEC_FOR_CALIB * 1000;
    let stabilizing_ok = stabilizing_elapsed_ms >= MIN_STABILIZING_SEC_FOR_CALIB * 1000;

    if sample.invalid_by_noise
        || sample.invalid_by_water_change
        || !mixing_ok
        || !stabilizing_ok
        || sensors.err_ec
        || sensors.err_ph
    {
        info!(
            "⏭️ Bỏ qua EMA sample (noise={}, water_change={}, mixing_ok={}, stabilizing_ok={}, err_ec={}, err_ph={})",
            sample.invalid_by_noise,
            sample.invalid_by_water_change,
            mixing_ok,
            stabilizing_ok,
            sensors.err_ec,
            sensors.err_ph
        );
        return;
    }

    let ec_after = sensors.ec;
    let ph_after = sensors.ph;
    let total_ec_ml = sample.pump_a_ml + sample.pump_b_ml;

    let observed_ec_gain_per_ml = if total_ec_ml > MIN_TOTAL_EC_DOSE_ML {
        Some((ec_after - sample.start_ec) / total_ec_ml)
    } else {
        None
    };
    let observed_ph_up_per_ml = if sample.ph_up_ml > MIN_PH_DOSE_ML {
        Some((ph_after - sample.start_ph) / sample.ph_up_ml)
    } else {
        None
    };
    let observed_ph_down_per_ml = if sample.ph_down_ml > MIN_PH_DOSE_ML {
        Some((sample.start_ph - ph_after) / sample.ph_down_ml)
    } else {
        None
    };

    let mut updated = false;
    let mut applied_ec_gain = None;
    let mut applied_ph_up = None;
    let mut applied_ph_down = None;
    if let Ok(mut cfg) = shared_config.write() {
        if let Some(observed) = observed_ec_gain_per_ml {
            if observed.is_finite() && observed > 0.0 {
                cfg.ec_gain_per_ml = cfg.ec_gain_per_ml * (1.0 - EMA_ALPHA) + observed * EMA_ALPHA;
                applied_ec_gain = Some(cfg.ec_gain_per_ml);
                updated = true;
            }
        }

        if let Some(observed) = observed_ph_up_per_ml {
            if observed.is_finite() && observed > 0.0 {
                cfg.ph_shift_up_per_ml =
                    cfg.ph_shift_up_per_ml * (1.0 - EMA_ALPHA) + observed * EMA_ALPHA;
                applied_ph_up = Some(cfg.ph_shift_up_per_ml);
                updated = true;
            }
        }

        if let Some(observed) = observed_ph_down_per_ml {
            if observed.is_finite() && observed > 0.0 {
                cfg.ph_shift_down_per_ml =
                    cfg.ph_shift_down_per_ml * (1.0 - EMA_ALPHA) + observed * EMA_ALPHA;
                applied_ph_down = Some(cfg.ph_shift_down_per_ml);
                updated = true;
            }
        }
    }

    if !updated {
        return;
    }

    ctx.calibration_pending_publish_count += 1;
    if ctx.calibration_pending_publish_count >= CALIBRATION_PERSIST_BATCH_SIZE {
        ctx.calibration_pending_publish_count = 0;
        let payload = serde_json::json!({
            "type": "runtime_calibration_update",
            "alpha": EMA_ALPHA,
            "persist": true,
            "persist_target": "backend_api",
            "start_ec": sample.start_ec,
            "start_ph": sample.start_ph,
            "ec_after": ec_after,
            "ph_after": ph_after,
            "pump_a_ml": sample.pump_a_ml,
            "pump_b_ml": sample.pump_b_ml,
            "ph_up_ml": sample.ph_up_ml,
            "ph_down_ml": sample.ph_down_ml,
            "observed_ec_gain_per_ml": observed_ec_gain_per_ml,
            "observed_ph_up_per_ml": observed_ph_up_per_ml,
            "observed_ph_down_per_ml": observed_ph_down_per_ml,
            "runtime_coefficients": {
                "ec_gain_per_ml": applied_ec_gain,
                "ph_shift_up_per_ml": applied_ph_up,
                "ph_shift_down_per_ml": applied_ph_down
            }
        });
        let _ = fsm_mqtt_tx.send(payload.to_string());
    }
}

fn get_current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

fn get_current_time_sec() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

pub fn start_fsm_control_loop(
    shared_config: SharedConfig,
    shared_sensors: SharedSensorData,
    mut pump_ctrl: PumpController,
    nvs_partition: EspDefaultNvsPartition,
    cmd_rx: Receiver<MqttCommandPayload>,
    fsm_mqtt_tx: Sender<String>,
    dosing_report_tx: Sender<String>,
    sensor_cmd_tx: Sender<String>,
) {
    let mut ctx = ControlContext::default();
    let mut last_reported_state = "".to_string();

    let mut nvs = EspNvs::new(nvs_partition, "agitech", true).ok();
    let current_time_on_boot = get_current_time_sec();

    ctx.last_water_change_time = nvs
        .as_mut()
        .and_then(|flash| flash.get_u64("last_w_change").unwrap_or(None))
        .unwrap_or_else(|| {
            if let Some(flash) = nvs.as_mut() {
                let _ = flash.set_u64("last_w_change", current_time_on_boot);
            }
            current_time_on_boot
        });

    ctx.last_scheduled_dose_time_sec = nvs
        .as_mut()
        .and_then(|flash| flash.get_u64("last_sched_dose").unwrap_or(None))
        .unwrap_or_else(|| {
            if let Some(flash) = nvs.as_mut() {
                let _ = flash.set_u64("last_sched_dose", current_time_on_boot);
            }
            current_time_on_boot
        });

    ctx.last_mixing_start_sec = current_time_on_boot;

    info!("🚀 Bắt đầu chạy Máy trạng thái (FSM) Đa luồng Hợp nhất...");

    // Khởi động ở trạng thái SystemBooting trong 3 giây
    let boot_start_ms = get_current_time_ms();
    loop {
        if get_current_time_ms() - boot_start_ms > 3000 {
            // Chuyển từ SystemBooting sang Monitoring sau 3 giây
            ctx.current_state = SystemState::Monitoring;
            break;
        }

        let state_changed = report_state_if_changed(&ctx.current_state, &mut last_reported_state);
        if state_changed {
            let status_msg = serde_json::json!({
                "online": true,
                "current_state": ctx.current_state.to_payload_string(),
                "pump_status": ctx.pump_status
            })
            .to_string();
            let _ = fsm_mqtt_tx.send(status_msg);
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    loop {
        let config = shared_config.read().unwrap().clone();
        let sensors = shared_sensors.read().unwrap().clone();
        let current_time_ms = get_current_time_ms();
        let current_time_sec = current_time_ms / 1000;
        ctx.sync_adaptive_ratios_from_config(&config);

        let force_sync =
            process_mqtt_commands(&cmd_rx, &config, &mut pump_ctrl, &mut ctx, current_time_ms);

        let mut expired_pumps = Vec::new();
        for (pump, &finish_time) in &ctx.manual_timeouts {
            if current_time_ms >= finish_time {
                expired_pumps.push(pump.clone());
            }
        }
        for pump in expired_pumps {
            ctx.manual_timeouts.remove(&pump);
            info!("⏱️ HẾT GIỜ (SAFE TIMEOUT): Tự động tắt bơm {}!", pump);
            ctx.turn_off_pump(&pump, &mut pump_ctrl);
        }

        let is_safety_overridden = current_time_ms < ctx.safety_override_until;

        if !is_safety_overridden {
            let is_noisy_sample = ctx.check_and_update_noise(&sensors, &config);
            let has_sensor_fault = (config.enable_water_level_sensor && sensors.err_water)
                || (config.enable_ec_sensor && sensors.err_ec)
                || (config.enable_ph_sensor && sensors.err_ph)
                || (config.enable_temp_sensor && sensors.err_temp);
            ctx.update_auto_tune_health(is_noisy_sample || has_sensor_fault);

            if is_noisy_sample {
                ctx.mark_pending_sample_noise_violation();
            }

            if is_noisy_sample && config.control_mode == ControlMode::Auto {
                // Skip logic auto nếu nhiễu
            } else {
                let is_water_critical = config.enable_water_level_sensor
                    && (sensors.water_level < config.water_level_critical_min);
                let is_ec_out_of_bounds = config.enable_ec_sensor
                    && (sensors.ec < config.min_ec_limit || sensors.ec > config.max_ec_limit);
                let is_ph_out_of_bounds = config.enable_ph_sensor
                    && (sensors.ph < config.min_ph_limit || sensors.ph > config.max_ph_limit);

                let mut emergency_reason = String::new();
                if config.emergency_shutdown {
                    emergency_reason = "MANUAL_STOP".to_string();
                } else if config.enable_water_level_sensor && sensors.err_water {
                    emergency_reason = "SENSOR_FAULT_WATER".to_string();
                } else if config.enable_ec_sensor && sensors.err_ec {
                    emergency_reason = "SENSOR_FAULT_EC".to_string();
                } else if config.enable_ph_sensor && sensors.err_ph {
                    emergency_reason = "SENSOR_FAULT_PH".to_string();
                } else if config.enable_temp_sensor && sensors.err_temp {
                    emergency_reason = "SENSOR_FAULT_TEMP".to_string();
                } else if is_water_critical {
                    emergency_reason = "WATER_CRITICAL".to_string();
                } else if is_ec_out_of_bounds {
                    emergency_reason = "EC_OUT_OF_BOUNDS".to_string();
                } else if is_ph_out_of_bounds {
                    emergency_reason = "PH_OUT_OF_BOUNDS".to_string();
                }

                let should_emergency_stop = !emergency_reason.is_empty();

                if should_emergency_stop {
                    if !matches!(ctx.current_state, SystemState::EmergencyStop(_)) {
                        error!(
                            "⚠️ DỪNG KHẨN CẤP Toàn bộ hệ thống! Lý do: {}",
                            emergency_reason
                        );
                        ctx.stop_all_pumps(&mut pump_ctrl);
                        ctx.current_state = SystemState::EmergencyStop(emergency_reason);
                    }
                } else if !config.is_enabled {
                    // 🟢 NẾU HỆ THỐNG TẮT, ĐƯA VỀ MONITORING BÌNH THƯỜNG
                    if ctx.current_state != SystemState::Monitoring {
                        ctx.stop_all_pumps(&mut pump_ctrl);
                        ctx.current_state = SystemState::Monitoring;
                    }
                } else if matches!(ctx.current_state, SystemState::EmergencyStop(_)) {
                    if !should_emergency_stop {
                        info!("✅ Hệ thống an toàn trở lại (hoặc đang Cưỡng chế).");
                        ctx.current_state = SystemState::Monitoring;
                    }
                } else if config.control_mode == ControlMode::Auto {
                    let is_hot = config.enable_temp_sensor
                        && (sensors.temp >= config.misting_temp_threshold);
                    let on_duration = if is_hot {
                        config.high_temp_misting_on_duration_ms
                    } else {
                        config.misting_on_duration_ms
                    };
                    let off_duration = if is_hot {
                        config.high_temp_misting_off_duration_ms
                    } else {
                        config.misting_off_duration_ms
                    };

                    if ctx.is_misting_active {
                        if current_time_ms >= ctx.last_mist_toggle_time + on_duration {
                            let _ = pump_ctrl.set_mist_valve(false);
                            ctx.is_misting_active = false;
                            ctx.last_mist_toggle_time = current_time_ms;
                            ctx.pump_status.mist_valve = false;
                        }
                    } else {
                        if current_time_ms >= ctx.last_mist_toggle_time + off_duration {
                            let _ = pump_ctrl.set_mist_valve(true);
                            ctx.is_misting_active = true;
                            ctx.last_mist_toggle_time = current_time_ms;
                            ctx.pump_status.mist_valve = true;
                        }
                    }

                    if config.scheduled_mixing_interval_sec > 0
                        && config.scheduled_mixing_duration_sec > 0
                    {
                        if ctx.is_scheduled_mixing_active {
                            if current_time_sec
                                >= ctx.last_mixing_start_sec + config.scheduled_mixing_duration_sec
                            {
                                ctx.is_scheduled_mixing_active = false;
                            }
                        } else {
                            if current_time_sec
                                >= ctx.last_mixing_start_sec + config.scheduled_mixing_interval_sec
                            {
                                ctx.is_scheduled_mixing_active = true;
                                ctx.last_mixing_start_sec = current_time_sec;
                            }
                        }
                    } else {
                        ctx.is_scheduled_mixing_active = false;
                    }

                    if !matches!(ctx.current_state, SystemState::SystemFault(_)) {
                        run_auto_fsm(
                            current_time_ms,
                            &config,
                            &sensors,
                            &mut ctx,
                            &mut pump_ctrl,
                            &shared_config,
                            &mut nvs,
                            &dosing_report_tx,
                            &fsm_mqtt_tx,
                        );
                    }

                    let needs_osaka = ctx.fsm_osaka_active
                        || ctx.is_misting_active
                        || ctx.is_scheduled_mixing_active;
                    if needs_osaka {
                        let target_pwm = if ctx.is_misting_active {
                            config.osaka_misting_pwm_percent
                        } else {
                            config.osaka_mixing_pwm_percent
                        };
                        if !ctx.pump_status.osaka_pump {
                            let _ = pump_ctrl.start_osaka_pump_soft(target_pwm);
                            ctx.pump_status.osaka_pump = true;
                            ctx.pump_status.osaka_pwm = Some(target_pwm);
                            ctx.current_osaka_pwm = target_pwm;
                        } else if ctx.current_osaka_pwm != target_pwm {
                            let _ = pump_ctrl.set_osaka_pump_pwm(target_pwm);
                            ctx.pump_status.osaka_pwm = Some(target_pwm);
                            ctx.current_osaka_pwm = target_pwm;
                        }
                    } else {
                        if ctx.pump_status.osaka_pump {
                            let _ = pump_ctrl.set_osaka_pump_pwm(0);
                            ctx.pump_status.osaka_pump = false;
                            ctx.pump_status.osaka_pwm = Some(0);
                            ctx.current_osaka_pwm = 0;
                        }
                    }
                } else if !matches!(
                    ctx.current_state,
                    SystemState::Monitoring
                        | SystemState::SystemFault(_)
                        | SystemState::EmergencyStop(_)
                        | SystemState::SensorCalibration { .. }
                        | SystemState::ManualMode
                        | SystemState::DosingCycleComplete
                ) {
                    info!("Chuyển sang chế độ MANUAL.");
                    ctx.stop_all_pumps(&mut pump_ctrl);
                    ctx.current_state = SystemState::ManualMode;
                }
            }
        }

        // DosingCycleComplete: Tự động về Monitoring sau 2 giây
        if let SystemState::DosingCycleComplete = ctx.current_state {
            std::thread::sleep(Duration::from_secs(2));
            ctx.current_state = SystemState::Monitoring;
        }

        let needs_continuous = matches!(
            ctx.current_state,
            SystemState::WaterRefilling { .. } | SystemState::WaterDraining { .. }
        );
        if needs_continuous != ctx.last_continuous_level {
            let payload = format!(
                r#"{{"target":"sensor","action":"set_continuous","params":{{"state":{}}}}}"#,
                needs_continuous
            );
            let _ = sensor_cmd_tx.send(payload);
            ctx.last_continuous_level = needs_continuous;
        }

        if let Ok(mut sensors_lock) = shared_sensors.write() {
            sensors_lock.pump_status = ctx.pump_status.clone();
        }

        let state_changed = report_state_if_changed(&ctx.current_state, &mut last_reported_state);

        if state_changed || force_sync {
            let status_msg = serde_json::json!({
                "online": true,
                "current_state": ctx.current_state.to_payload_string(),
                "pump_status": ctx.pump_status
            })
            .to_string();

            let _ = fsm_mqtt_tx.send(status_msg);

            if force_sync {
                last_reported_state = "".to_string();
                let _ = sensor_cmd_tx.send(
                    r#"{"target":"sensor","action":"force_publish","params":{}}"#.to_string(),
                );
                info!("⚡ Đã ép luồng chính Publish trạng thái bơm mới nhất lên App!");
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    fn report_state_if_changed(
        current_state: &SystemState,
        last_reported_state: &mut String,
    ) -> bool {
        let current_state_str = current_state.to_payload_string();
        if current_state_str != *last_reported_state {
            info!("📡 Trạng thái FSM: [{}]", current_state_str);
            *last_reported_state = current_state_str;
            true
        } else {
            false
        }
    }

    fn process_mqtt_commands(
        cmd_rx: &Receiver<MqttCommandPayload>,
        config: &DeviceConfig,
        pump_ctrl: &mut PumpController,
        ctx: &mut ControlContext,
        current_time_ms: u64,
    ) -> bool {
        let mut force_sync = false;

        let is_emergency_state = matches!(
            ctx.current_state,
            SystemState::EmergencyStop(_)
                | SystemState::SystemFault(_)
                | SystemState::SensorCalibration { .. }
        );

        while let Ok(cmd) = cmd_rx.try_recv() {
            let action_lower = cmd.action.to_lowercase();

            if action_lower == "enter_calibration" {
                info!("🛠️ Bắt đầu chế độ Hiệu chuẩn Cảm biến! Khóa chéo an toàn.");
                ctx.stop_all_pumps(pump_ctrl);
                let step = cmd.target.clone().unwrap_or_else(|| "IDLE".to_string());
                ctx.current_state = SystemState::SensorCalibration {
                    step,
                    finish_time: current_time_ms + 3600_000, // 1 hour timeout
                };
                force_sync = true;
                continue;
            }

            if action_lower == "exit_calibration" {
                if matches!(ctx.current_state, SystemState::SensorCalibration { .. }) {
                    info!("✅ Thoát chế độ Hiệu chuẩn, quay về Monitoring.");
                    ctx.current_state = SystemState::Monitoring;
                    force_sync = true;
                }
                continue;
            }

            if action_lower == "sync_status" {
                force_sync = true;
                continue;
            }

            if action_lower == "reset_fault" {
                info!("🔄 Nhận lệnh Reset. Khôi phục hệ thống...");
                ctx.stop_all_pumps(pump_ctrl);
                ctx.reset_faults();
                force_sync = true;
                continue;
            }

            if config.control_mode == ControlMode::Auto {
                warn!("Bỏ qua lệnh thủ công vì đang ở AUTO.");
                continue;
            }

            if let Some(target) = &cmd.target {
                let target_lower = target.to_lowercase();
                if target_lower != "pump" && target_lower != "all" {
                    continue;
                }
            }

            let pump_name = cmd
                .params
                .as_ref()
                .and_then(|p| p.pump_id.as_ref())
                .map(|p| p.to_uppercase())
                .or_else(|| cmd.pump.as_ref().map(|p| p.to_uppercase()))
                .unwrap_or_else(|| "ALL".to_string());

            let is_force_on = action_lower == "force_on";
            let is_set_pwm = action_lower == "set_pwm";
            let pwm = cmd.params.as_ref().and_then(|p| p.pwm).or(cmd.pwm);
            let duration_sec = cmd
                .params
                .as_ref()
                .and_then(|p| p.duration_sec)
                .or(cmd.duration_sec);
            let explicit_state = cmd.params.as_ref().and_then(|p| p.state);

            let mut is_on = is_force_on
                || action_lower == "pump_on"
                || action_lower == "on"
                || action_lower == "true"
                || action_lower == "1"
                || (is_set_pwm && pwm.unwrap_or(0) > 0);

            if let Some(state) = explicit_state {
                is_on = state;
            }

            if is_emergency_state && is_on && !is_force_on {
                warn!("❌ BLOCKED: Không thể điều khiển {} bình thường vì hệ thống đang Lỗi / Hiệu chuẩn / EmergencyStop. Vui lòng dùng FORCE.", pump_name);
                continue;
            }

            if is_force_on {
                info!("⚠️ NGƯỜI DÙNG CƯỠNG CHẾ BẬT {}!", pump_name);
                let duration = duration_sec.unwrap_or(120);
                ctx.safety_override_until = current_time_ms + (duration as u64 * 1000);
            }

            if is_on {
                if let Some(duration) = duration_sec {
                    if duration > 0 {
                        let finish_time = current_time_ms + (duration as u64 * 1000);
                        ctx.manual_timeouts.insert(pump_name.clone(), finish_time);
                    }
                }
            } else {
                ctx.manual_timeouts.remove(&pump_name);
            }

            let pwm_val = if let Some(p) = pwm {
                p
            } else {
                if is_on {
                    100
                } else {
                    0
                }
            };

            let _ = match pump_name.as_str() {
                "A" | "PUMP_A" => {
                    ctx.pump_status.pump_a = is_on;
                    ctx.pump_status.pump_a_pwm = Some(if is_on { pwm_val } else { 0 });
                    if pwm.is_some() || is_set_pwm {
                        pump_ctrl.set_dosing_pump_pwm(PumpType::NutrientA, is_on, pwm_val)
                    } else {
                        pump_ctrl.set_pump_state(PumpType::NutrientA, is_on)
                    }
                }
                "B" | "PUMP_B" => {
                    ctx.pump_status.pump_b = is_on;
                    ctx.pump_status.pump_b_pwm = Some(if is_on { pwm_val } else { 0 });
                    if pwm.is_some() || is_set_pwm {
                        pump_ctrl.set_dosing_pump_pwm(PumpType::NutrientB, is_on, pwm_val)
                    } else {
                        pump_ctrl.set_pump_state(PumpType::NutrientB, is_on)
                    }
                }
                "PH_UP" | "PUMP_PH_UP" => {
                    ctx.pump_status.ph_up = is_on;
                    ctx.pump_status.ph_up_pwm = Some(if is_on { pwm_val } else { 0 });
                    if pwm.is_some() || is_set_pwm {
                        pump_ctrl.set_dosing_pump_pwm(PumpType::PhUp, is_on, pwm_val)
                    } else {
                        pump_ctrl.set_pump_state(PumpType::PhUp, is_on)
                    }
                }
                "PH_DOWN" | "PUMP_PH_DOWN" => {
                    ctx.pump_status.ph_down = is_on;
                    ctx.pump_status.ph_down_pwm = Some(if is_on { pwm_val } else { 0 });
                    if pwm.is_some() || is_set_pwm {
                        pump_ctrl.set_dosing_pump_pwm(PumpType::PhDown, is_on, pwm_val)
                    } else {
                        pump_ctrl.set_pump_state(PumpType::PhDown, is_on)
                    }
                }
                "OSAKA_PUMP" | "OSAKA" => {
                    ctx.pump_status.osaka_pump = is_on;
                    ctx.pump_status.osaka_pwm = Some(if is_on { pwm_val } else { 0 });
                    if pwm.is_some() || is_set_pwm {
                        pump_ctrl.set_osaka_pump_pwm(pwm_val)
                    } else {
                        pump_ctrl.set_osaka_pump(is_on)
                    }
                }
                "MIST_VALVE" | "MIST" => {
                    ctx.pump_status.mist_valve = is_on;
                    ctx.is_misting_active = is_on;
                    if is_on {
                        ctx.last_mist_toggle_time = current_time_ms;
                    }
                    pump_ctrl.set_mist_valve(is_on)
                }
                "WATER_PUMP" | "WATER_PUMP_IN" | "PUMP_IN" => {
                    ctx.pump_status.water_pump_in = is_on;
                    if is_on {
                        ctx.pump_status.water_pump_out = false;
                    }
                    pump_ctrl.set_water_pump(if is_on {
                        WaterDirection::In
                    } else {
                        WaterDirection::Stop
                    })
                }
                "DRAIN_PUMP" | "WATER_PUMP_OUT" | "PUMP_OUT" => {
                    ctx.pump_status.water_pump_out = is_on;
                    if is_on {
                        ctx.pump_status.water_pump_in = false;
                    }
                    pump_ctrl.set_water_pump(if is_on {
                        WaterDirection::Out
                    } else {
                        WaterDirection::Stop
                    })
                }
                _ => Ok(()),
            };

            force_sync = true;
        }

        force_sync
    }

    fn run_auto_fsm(
        current_time_ms: u64,
        config: &DeviceConfig,
        sensors: &SensorData,
        ctx: &mut ControlContext,
        pump_ctrl: &mut PumpController,
        shared_config: &SharedConfig,
        nvs: &mut Option<EspDefaultNvs>,
        dosing_report_tx: &Sender<String>,
        fsm_mqtt_tx: &Sender<String>,
    ) {
        let current_time_sec = current_time_ms / 1000;
        const MAX_TOTAL_DOSE_ML_PER_HOUR_BY_PUMP: f32 = 30.0;

        match ctx.current_state {
            SystemState::SystemBooting
            | SystemState::ManualMode
            | SystemState::DosingCycleComplete => {
                // Các state này do luồng ngoài xử lý, auto fsm không làm gì cả
            }

            SystemState::SensorCalibration { finish_time, .. } => {
                if current_time_ms >= finish_time {
                    warn!("⏱️ Timeout Hiệu chuẩn. Hệ thống sẽ trở lại Monitoring.");
                    ctx.current_state = SystemState::Monitoring;
                }
            }

            SystemState::SystemFault(ref reason) => {
                warn!("🚨 BÁO LỖI: [{}]. Chờ reset...", reason);
            }

            SystemState::Monitoring => {
                ctx.verify_sensor_ack(sensors, config, current_time_sec);

                // 🟢 THAY NƯỚC ĐỊNH KỲ THEO LỊCH CRON
                if config.enable_water_level_sensor
                    && config.scheduled_water_change_enabled
                    && !config.water_change_cron.is_empty()
                {
                    if ctx.current_water_change_cron_expr != config.water_change_cron {
                        ctx.current_water_change_cron_expr = config.water_change_cron.clone();
                        if let Ok(schedule) =
                            Schedule::from_str(&ctx.current_water_change_cron_expr)
                        {
                            if let Some(next) = schedule.upcoming(Local).next() {
                                ctx.next_water_change_trigger_sec = Some(next.timestamp() as u64);
                                info!("⏰ Cập nhật lịch Thay nước Cron: {}", next);
                            }
                        } else {
                            warn!("⚠️ Lỗi cú pháp Cron Thay nước!");
                            ctx.next_water_change_trigger_sec = None;
                        }
                    }

                    if let Some(next_trigger) = ctx.next_water_change_trigger_sec {
                        if current_time_sec >= next_trigger {
                            info!("⏰ Đã đến giờ THAY NƯỚC ĐỊNH KỲ theo lịch CRON!");

                            if let Ok(schedule) =
                                Schedule::from_str(&ctx.current_water_change_cron_expr)
                            {
                                let future = Local::now() + chrono::Duration::seconds(1);
                                if let Some(next) = schedule.after(&future).next() {
                                    ctx.next_water_change_trigger_sec =
                                        Some(next.timestamp() as u64);
                                }
                            }

                            let target = (sensors.water_level - config.scheduled_drain_amount_cm)
                                .max(config.water_level_min);
                            ctx.last_water_change_time = current_time_sec;

                            if let Some(flash) = nvs.as_mut() {
                                let _ = flash.set_u64("last_w_change", current_time_sec);
                            }

                            ctx.mark_pending_sample_water_change_violation();
                            ctx.current_state = SystemState::WaterDraining {
                                target_level: target,
                                start_time: current_time_ms,
                            };
                            let _ = pump_ctrl.set_water_pump(WaterDirection::Out);
                            ctx.pump_status.water_pump_out = true;
                            ctx.pump_status.water_pump_in = false;
                            ctx.fsm_osaka_active = false;

                            return;
                        }
                    }
                }

                if config.enable_water_level_sensor
                    && config.auto_refill_enabled
                    && sensors.water_level
                        < (config.water_level_target - config.water_level_tolerance)
                {
                    if ctx.water_refill_retry_count >= 3 {
                        ctx.stop_all_pumps(pump_ctrl);
                        ctx.current_state =
                            SystemState::SystemFault("WATER_REFILL_FAILED".to_string());
                    } else {
                        ctx.last_water_before_refill = Some(sensors.water_level);
                        ctx.mark_pending_sample_water_change_violation();
                        ctx.current_state = SystemState::WaterRefilling {
                            target_level: config.water_level_target,
                            start_time: current_time_ms,
                        };
                        let _ = pump_ctrl.set_water_pump(WaterDirection::In);
                        ctx.pump_status.water_pump_in = true;
                        ctx.pump_status.water_pump_out = false;
                        ctx.fsm_osaka_active = false;
                    }
                } else if config.enable_water_level_sensor
                    && config.auto_drain_overflow
                    && sensors.water_level > config.water_level_max
                {
                    ctx.mark_pending_sample_water_change_violation();
                    ctx.current_state = SystemState::WaterDraining {
                        target_level: config.water_level_target,
                        start_time: current_time_ms,
                    };
                    let _ = pump_ctrl.set_water_pump(WaterDirection::Out);
                    ctx.pump_status.water_pump_out = true;
                    ctx.pump_status.water_pump_in = false;
                    ctx.fsm_osaka_active = false;
                } else if config.enable_ec_sensor
                    && config.enable_water_level_sensor
                    && config.auto_dilute_enabled
                    && sensors.ec > (config.ec_target + config.ec_tolerance)
                {
                    let target = (sensors.water_level - config.dilute_drain_amount_cm)
                        .max(config.water_level_min);
                    ctx.mark_pending_sample_water_change_violation();
                    ctx.current_state = SystemState::WaterDraining {
                        target_level: target,
                        start_time: current_time_ms,
                    };
                    let _ = pump_ctrl.set_water_pump(WaterDirection::Out);
                    ctx.pump_status.water_pump_out = true;
                    ctx.pump_status.water_pump_in = false;
                    ctx.fsm_osaka_active = false;
                } else {
                    let mut is_dosing_active = false;

                    // 🟢 BƠM ĐỊNH KỲ THEO LỊCH CRON
                    if config.scheduled_dosing_enabled && !config.scheduled_dosing_cron.is_empty() {
                        if ctx.current_cron_expr != config.scheduled_dosing_cron {
                            ctx.current_cron_expr = config.scheduled_dosing_cron.clone();
                            if let Ok(schedule) = Schedule::from_str(&ctx.current_cron_expr) {
                                if let Some(next) = schedule.upcoming(Local).next() {
                                    ctx.next_cron_trigger_sec = Some(next.timestamp() as u64);
                                    info!("⏰ Cập nhật lịch Dosing Cron: {}", next);
                                }
                            } else {
                                warn!("⚠️ Biểu thức Cron Dosing không hợp lệ!");
                                ctx.next_cron_trigger_sec = None;
                            }
                        }

                        if let Some(next_trigger) = ctx.next_cron_trigger_sec {
                            if current_time_sec >= next_trigger {
                                info!("⏰ Đã đến giờ BƠM DINH DƯỠNG theo lịch CRON!");

                                if let Ok(schedule) = Schedule::from_str(&ctx.current_cron_expr) {
                                    let future = Local::now() + chrono::Duration::seconds(1);
                                    if let Some(next) = schedule.after(&future).next() {
                                        ctx.next_cron_trigger_sec = Some(next.timestamp() as u64);
                                    }
                                }

                                ctx.last_scheduled_dose_time_sec = current_time_sec;
                                if let Some(flash) = nvs.as_mut() {
                                    let _ = flash.set_u64("last_sched_dose", current_time_sec);
                                }

                                let safe_pwm = config.dosing_pwm_percent.clamp(1, 100);
                                if config.scheduled_dose_a_ml > 0.0
                                    || config.scheduled_dose_b_ml > 0.0
                                {
                                    let allow_a = config.scheduled_dose_a_ml <= 0.0
                                        || ctx.can_dose_within_hourly_limit(
                                            "NutrientA",
                                            current_time_sec,
                                            config.scheduled_dose_a_ml,
                                            MAX_TOTAL_DOSE_ML_PER_HOUR_BY_PUMP,
                                        );
                                    let allow_b = config.scheduled_dose_b_ml <= 0.0
                                        || ctx.can_dose_within_hourly_limit(
                                            "NutrientB",
                                            current_time_sec,
                                            config.scheduled_dose_b_ml,
                                            MAX_TOTAL_DOSE_ML_PER_HOUR_BY_PUMP,
                                        );
                                    if allow_a && allow_b {
                                        if config.scheduled_dose_a_ml > 0.0 {
                                            let _ = ctx.reserve_dose_if_within_hourly_limit(
                                                "NutrientA",
                                                current_time_sec,
                                                config.scheduled_dose_a_ml,
                                                MAX_TOTAL_DOSE_ML_PER_HOUR_BY_PUMP,
                                            );
                                        }
                                        if config.scheduled_dose_b_ml > 0.0 {
                                            let _ = ctx.reserve_dose_if_within_hourly_limit(
                                                "NutrientB",
                                                current_time_sec,
                                                config.scheduled_dose_b_ml,
                                                MAX_TOTAL_DOSE_ML_PER_HOUR_BY_PUMP,
                                            );
                                        }
                                        ctx.current_state = SystemState::StartingOsakaPump {
                                            finish_time: current_time_ms
                                                + config.soft_start_duration,
                                            pending_action: PendingDose::ScheduledDose {
                                                dose_a_ml: config.scheduled_dose_a_ml,
                                                dose_b_ml: config.scheduled_dose_b_ml,
                                                pwm_percent: safe_pwm,
                                            },
                                        };
                                        ctx.fsm_osaka_active = true;
                                        is_dosing_active = true;
                                    }
                                }
                            }
                        }
                    }

                    // 🟢 BÙ EC TỰ ĐỘNG
                    if config.enable_ec_sensor
                        && !is_dosing_active
                        && sensors.ec < (config.ec_target - config.ec_tolerance)
                    {
                        if ctx.ec_retry_count >= 3 {
                            ctx.stop_all_pumps(pump_ctrl);
                            ctx.current_state =
                                SystemState::SystemFault("EC_DOSING_FAILED".to_string());
                            is_dosing_active = true;
                        } else {
                            let safe_pwm = config.dosing_pwm_percent.clamp(1, 100);
                            let ec_error = config.ec_target - sensors.ec;
                            let deadband_scale = soft_deadband_scale(ec_error, config.ec_tolerance);
                            let active_ec_step_ratio = if ctx.auto_tune_locked {
                                ctx.best_known_ec_step_ratio
                            } else {
                                ctx.adaptive_ec_step_ratio
                            };
                            let dose_ml = (ec_error / config.ec_gain_per_ml
                                * active_ec_step_ratio
                                * deadband_scale)
                                .clamp(0.0, config.max_dose_per_cycle);

                            let can_dose_ec_a = ctx.can_dose_within_hourly_limit(
                                "NutrientA",
                                current_time_sec,
                                dose_ml,
                                MAX_TOTAL_DOSE_ML_PER_HOUR_BY_PUMP,
                            );
                            let can_dose_ec_b = ctx.can_dose_within_hourly_limit(
                                "NutrientB",
                                current_time_sec,
                                dose_ml,
                                MAX_TOTAL_DOSE_ML_PER_HOUR_BY_PUMP,
                            );
                            if dose_ml > 0.0 && can_dose_ec_a && can_dose_ec_b {
                                let _ = ctx.reserve_dose_if_within_hourly_limit(
                                    "NutrientA",
                                    current_time_sec,
                                    dose_ml,
                                    MAX_TOTAL_DOSE_ML_PER_HOUR_BY_PUMP,
                                );
                                let _ = ctx.reserve_dose_if_within_hourly_limit(
                                    "NutrientB",
                                    current_time_sec,
                                    dose_ml,
                                    MAX_TOTAL_DOSE_ML_PER_HOUR_BY_PUMP,
                                );
                                ctx.last_ec_before_dosing = Some(sensors.ec);
                                ctx.current_state = SystemState::StartingOsakaPump {
                                    finish_time: current_time_ms + config.soft_start_duration,
                                    pending_action: PendingDose::EC {
                                        dose_ml,
                                        target_ec: config.ec_target,
                                        pwm_percent: safe_pwm,
                                    },
                                };
                                ctx.fsm_osaka_active = true;
                                is_dosing_active = true;
                            }
                        }
                    }

                    // 🟢 BÙ PH TỰ ĐỘNG
                    if config.enable_ph_sensor
                        && !is_dosing_active
                        && (sensors.ph - config.ph_target).abs() > config.ph_tolerance
                    {
                        if ctx.ph_retry_count >= 3 {
                            ctx.stop_all_pumps(pump_ctrl);
                            ctx.current_state =
                                SystemState::SystemFault("PH_DOSING_FAILED".to_string());
                            is_dosing_active = true;
                        } else {
                            let is_ph_up = sensors.ph < config.ph_target;
                            let diff = (sensors.ph - config.ph_target).abs();
                            let ratio = if is_ph_up {
                                config.ph_shift_up_per_ml
                            } else {
                                config.ph_shift_down_per_ml
                            };
                            let safe_pwm = config.dosing_pwm_percent.clamp(1, 100);

                            let pump_kind = if is_ph_up {
                                DosePumpKind::PhUp
                            } else {
                                DosePumpKind::PhDown
                            };

                            let active_capacity =
                                effective_flow_ml_per_sec(pump_kind, safe_pwm, config);

                            let deadband_scale = soft_deadband_scale(diff, config.ph_tolerance);
                            let active_ph_step_ratio = if ctx.auto_tune_locked {
                                ctx.best_known_ph_step_ratio
                            } else {
                                ctx.adaptive_ph_step_ratio
                            };
                            let dose_ml = (diff / ratio * active_ph_step_ratio * deadband_scale)
                                .clamp(0.0, config.max_dose_per_cycle);
                            let ph_pump_name = if is_ph_up { "PhUp" } else { "PhDown" };

                            if let Some(cap) = active_capacity {
                                let duration_ms = ((dose_ml / cap) * 1000.0) as u64;
                                if duration_ms > 0
                                    && ctx.reserve_dose_if_within_hourly_limit(
                                        ph_pump_name,
                                        current_time_sec,
                                        dose_ml,
                                        MAX_TOTAL_DOSE_ML_PER_HOUR_BY_PUMP,
                                    )
                                {
                                    let final_dose_ml = (diff / ratio * config.ph_step_ratio)
                                        .clamp(0.0, config.max_dose_per_cycle);
                                    if final_dose_ml > 0.0 {
                                        ctx.last_ph_before_dosing = Some(sensors.ph);
                                        ctx.last_ph_dosing_is_up = Some(is_ph_up);

                                        ctx.current_state = SystemState::StartingOsakaPump {
                                            finish_time: current_time_ms
                                                + config.soft_start_duration,
                                            pending_action: PendingDose::PH {
                                                is_up: is_ph_up,
                                                dose_ml: final_dose_ml,
                                                target_ph: config.ph_target,
                                                pwm_percent: safe_pwm,
                                            },
                                        };
                                        ctx.fsm_osaka_active = true;
                                        is_dosing_active = true;
                                    }
                                }
                            } else {
                                warn!(
                                "Skip PH dosing: invalid pump config or PWM below min (pump={}, pwm={}%)",
                                ph_pump_name, safe_pwm
                            );
                                ctx.stop_all_pumps(pump_ctrl);
                                ctx.current_state = SystemState::Monitoring;
                            }
                        }
                    }

                    if !is_dosing_active {
                        ctx.fsm_osaka_active = false;
                    }
                }
            }

            SystemState::WaterRefilling {
                target_level,
                start_time,
            } => {
                if sensors.water_level >= target_level
                    || current_time_ms.saturating_sub(start_time)
                        > (config.max_refill_duration_sec as u64 * 1000)
                {
                    let _ = pump_ctrl.set_water_pump(WaterDirection::Stop);
                    ctx.pump_status.water_pump_in = false;
                    ctx.pump_status.water_pump_out = false;
                    ctx.fsm_osaka_active = true;
                    ctx.current_state = SystemState::ActiveMixing {
                        finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
                    };
                }
            }

            SystemState::WaterDraining {
                target_level,
                start_time,
            } => {
                if sensors.water_level <= target_level
                    || current_time_ms.saturating_sub(start_time)
                        > (config.max_drain_duration_sec as u64 * 1000)
                {
                    let _ = pump_ctrl.set_water_pump(WaterDirection::Stop);
                    ctx.pump_status.water_pump_in = false;
                    ctx.pump_status.water_pump_out = false;
                    ctx.fsm_osaka_active = false;
                    ctx.current_state = SystemState::Stabilizing {
                        finish_time: current_time_ms + (config.sensor_stabilize_sec as u64 * 1000),
                    };
                }
            }

            SystemState::StartingOsakaPump {
                finish_time,
                ref pending_action,
            } => {
                if current_time_ms >= finish_time {
                    let action = pending_action.clone();
                    match action {
                        PendingDose::ScheduledDose {
                            dose_a_ml,
                            dose_b_ml,
                            pwm_percent,
                        } => {
                            if dose_a_ml > 0.0 {
                                let dose_pwm = pwm_percent.clamp(1, 100);
                                let active_capacity_a = if let Some(cap) =
                                    effective_flow_ml_per_sec(DosePumpKind::PumpA, dose_pwm, config)
                                {
                                    cap
                                } else {
                                    warn!(
                                        "Skip scheduled dose pump A: invalid config/pwm (pwm={}%)",
                                        dose_pwm
                                    );
                                    ctx.stop_all_pumps(pump_ctrl);
                                    ctx.current_state = SystemState::Monitoring;
                                    return;
                                };
                                let is_pulse_mode = dose_a_ml < config.dosing_min_dose_ml;
                                let pulse_on_ms = if is_pulse_mode {
                                    config.dosing_pulse_on_ms.max(1)
                                } else {
                                    ((dose_a_ml / active_capacity_a) * 1000.0) as u64
                                };
                                let pulse_off_ms = if is_pulse_mode {
                                    config.dosing_pulse_off_ms
                                } else {
                                    0
                                };
                                let max_pulse_count = if is_pulse_mode {
                                    config.dosing_max_pulse_count_per_cycle.max(1)
                                } else {
                                    1
                                };
                                let _ = pump_ctrl.set_dosing_pump_pulse(
                                    PumpType::NutrientA,
                                    true,
                                    dose_pwm,
                                );
                                ctx.pump_status.pump_a = true;
                                ctx.pump_status.pump_a_pwm = Some(dose_pwm);
                                ctx.set_pulse_status(
                                    is_pulse_mode,
                                    if is_pulse_mode { 1 } else { 0 },
                                );
                                let delivered_ml_est =
                                    active_capacity_a * (pulse_on_ms as f32 / 1000.0);

                                ctx.current_state = SystemState::DosingPumpA {
                                    next_toggle_time: current_time_ms + pulse_on_ms,
                                    dose_target_ml: dose_a_ml,
                                    delivered_ml_est,
                                    dose_b_ml,
                                    pulse_on: true,
                                    pulse_count: 1,
                                    max_pulse_count,
                                    pulse_on_ms,
                                    pulse_off_ms,
                                    pwm_percent: dose_pwm,
                                    active_capacity_ml_per_sec: active_capacity_a,
                                    target_ec: sensors.ec,
                                    start_ec: sensors.ec,
                                    start_ph: sensors.ph,
                                };
                            } else if dose_b_ml > 0.0 {
                                ctx.current_state = SystemState::WaitingBetweenDose {
                                    finish_time: current_time_ms,
                                    dose_b_ml,
                                    target_ec: sensors.ec,
                                    start_ec: sensors.ec,
                                    start_ph: sensors.ph,
                                    dose_a_ml_reported: 0.0,
                                };
                            } else {
                                ctx.current_state = SystemState::ActiveMixing {
                                    finish_time: current_time_ms
                                        + (config.active_mixing_sec as u64 * 1000),
                                };
                            }
                        }
                        PendingDose::EC {
                            dose_ml,
                            target_ec,
                            pwm_percent,
                        } => {
                            let dose_pwm = pwm_percent.clamp(1, 100);
                            let active_capacity_a = if let Some(cap) =
                                effective_flow_ml_per_sec(DosePumpKind::PumpA, dose_pwm, config)
                            {
                                cap
                            } else {
                                warn!(
                                    "Skip EC dosing pump A: invalid config/pwm (pwm={}%)",
                                    dose_pwm
                                );
                                ctx.stop_all_pumps(pump_ctrl);
                                ctx.current_state = SystemState::Monitoring;
                                return;
                            };
                            let is_pulse_mode = dose_ml < config.dosing_min_dose_ml;
                            let pulse_on_ms = if is_pulse_mode {
                                config.dosing_pulse_on_ms.max(1)
                            } else {
                                ((dose_ml / active_capacity_a) * 1000.0) as u64
                            };
                            let pulse_off_ms = if is_pulse_mode {
                                config.dosing_pulse_off_ms
                            } else {
                                0
                            };
                            let max_pulse_count = if is_pulse_mode {
                                config.dosing_max_pulse_count_per_cycle.max(1)
                            } else {
                                1
                            };
                            let _ = pump_ctrl.set_dosing_pump_pulse(
                                PumpType::NutrientA,
                                true,
                                dose_pwm,
                            );
                            ctx.pump_status.pump_a = true;
                            ctx.pump_status.pump_a_pwm = Some(dose_pwm);
                            ctx.set_pulse_status(is_pulse_mode, if is_pulse_mode { 1 } else { 0 });
                            let delivered_ml_est =
                                active_capacity_a * (pulse_on_ms as f32 / 1000.0);

                            ctx.current_state = SystemState::DosingPumpA {
                                next_toggle_time: current_time_ms + pulse_on_ms,
                                dose_target_ml: dose_ml,
                                delivered_ml_est,
                                dose_b_ml: dose_ml,
                                pulse_on: true,
                                pulse_count: 1,
                                max_pulse_count,
                                pulse_on_ms,
                                pulse_off_ms,
                                pwm_percent: dose_pwm,
                                active_capacity_ml_per_sec: active_capacity_a,
                                target_ec,
                                start_ec: sensors.ec,
                                start_ph: sensors.ph,
                            };
                        }
                        PendingDose::PH {
                            is_up,
                            dose_ml,
                            target_ph,
                            pwm_percent,
                        } => {
                            let dose_pwm = pwm_percent.clamp(1, 100);
                            let pump_kind = if is_up {
                                DosePumpKind::PhUp
                            } else {
                                DosePumpKind::PhDown
                            };

                            let active_capacity = if let Some(cap) =
                                effective_flow_ml_per_sec(pump_kind, dose_pwm, config)
                            {
                                cap
                            } else {
                                warn!(
                                    "Skip PH dosing: invalid config/pwm (is_up={}, pwm={}%)",
                                    is_up, dose_pwm
                                );
                                ctx.stop_all_pumps(pump_ctrl);
                                ctx.current_state = SystemState::Monitoring;
                                return;
                            };

                            let is_pulse_mode = dose_ml < config.dosing_min_dose_ml;
                            let pulse_on_ms = if is_pulse_mode {
                                config.dosing_pulse_on_ms.max(1)
                            } else {
                                ((dose_ml / active_capacity) * 1000.0) as u64
                            };
                            let pulse_off_ms = if is_pulse_mode {
                                config.dosing_pulse_off_ms
                            } else {
                                0
                            };
                            let max_pulse_count = if is_pulse_mode {
                                config.dosing_max_pulse_count_per_cycle.max(1)
                            } else {
                                1
                            };
                            let _ = pump_ctrl.set_dosing_pump_pulse(
                                if is_up {
                                    PumpType::PhUp
                                } else {
                                    PumpType::PhDown
                                },
                                true,
                                dose_pwm,
                            );
                            if is_up {
                                ctx.pump_status.ph_up = true;
                                ctx.pump_status.ph_up_pwm = Some(dose_pwm);
                            } else {
                                ctx.pump_status.ph_down = true;
                                ctx.pump_status.ph_down_pwm = Some(dose_pwm);
                            }
                            ctx.set_pulse_status(is_pulse_mode, if is_pulse_mode { 1 } else { 0 });
                            let delivered_ml_est = active_capacity * (pulse_on_ms as f32 / 1000.0);

                            ctx.current_state = SystemState::DosingPH {
                                next_toggle_time: current_time_ms + pulse_on_ms,
                                is_up,
                                dose_target_ml: dose_ml,
                                delivered_ml_est,
                                pulse_on: true,
                                pulse_count: 1,
                                max_pulse_count,
                                pulse_on_ms,
                                pulse_off_ms,
                                pwm_percent: dose_pwm,
                                active_capacity_ml_per_sec: active_capacity,
                                target_ph,
                                start_ec: sensors.ec,
                                start_ph: sensors.ph,
                            };
                        }
                    }
                }
            }

            SystemState::DosingPumpA {
                next_toggle_time,
                dose_target_ml,
                delivered_ml_est,
                dose_b_ml,
                pulse_on,
                pulse_count,
                max_pulse_count,
                pulse_on_ms,
                pulse_off_ms,
                pwm_percent,
                active_capacity_ml_per_sec,
                target_ec,
                start_ec,
                start_ph,
            } => {
                if current_time_ms >= next_toggle_time {
                    if pulse_on {
                        let _ = pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientA, false, 0);
                        ctx.pump_status.pump_a = false;
                        ctx.pump_status.pump_a_pwm = Some(0);
                        let hit_target = delivered_ml_est >= dose_target_ml;
                        let hit_limit = pulse_count >= max_pulse_count;
                        if hit_target || hit_limit {
                            ctx.set_pulse_status(false, pulse_count);
                            ctx.current_state = SystemState::WaitingBetweenDose {
                                finish_time: current_time_ms
                                    + (config.delay_between_a_and_b_sec as u64 * 1000),
                                dose_b_ml,
                                target_ec,
                                start_ec,
                                start_ph,
                                dose_a_ml_reported: delivered_ml_est.min(dose_target_ml),
                            };
                        } else {
                            ctx.set_pulse_status(true, pulse_count);
                            ctx.current_state = SystemState::DosingPumpA {
                                next_toggle_time: current_time_ms + pulse_off_ms,
                                dose_target_ml,
                                delivered_ml_est,
                                dose_b_ml,
                                pulse_on: false,
                                pulse_count,
                                max_pulse_count,
                                pulse_on_ms,
                                pulse_off_ms,
                                pwm_percent,
                                active_capacity_ml_per_sec,
                                target_ec,
                                start_ec,
                                start_ph,
                            };
                        }
                    } else {
                        let _ =
                            pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientA, true, pwm_percent);
                        ctx.pump_status.pump_a = true;
                        ctx.pump_status.pump_a_pwm = Some(pwm_percent);
                        let next_count = pulse_count + 1;
                        let next_delivered = delivered_ml_est
                            + active_capacity_ml_per_sec * (pulse_on_ms as f32 / 1000.0);
                        ctx.set_pulse_status(true, next_count);
                        ctx.current_state = SystemState::DosingPumpA {
                            next_toggle_time: current_time_ms + pulse_on_ms,
                            dose_target_ml,
                            delivered_ml_est: next_delivered,
                            dose_b_ml,
                            pulse_on: true,
                            pulse_count: next_count,
                            max_pulse_count,
                            pulse_on_ms,
                            pulse_off_ms,
                            pwm_percent,
                            active_capacity_ml_per_sec,
                            target_ec,
                            start_ec,
                            start_ph,
                        };
                    }
                }
            }

            SystemState::WaitingBetweenDose {
                finish_time,
                dose_b_ml,
                target_ec,
                start_ec,
                start_ph,
                dose_a_ml_reported,
            } => {
                if current_time_ms >= finish_time {
                    if dose_b_ml > 0.0 {
                        let dose_pwm = config.dosing_pwm_percent.clamp(1, 100);
                        let is_pulse_mode = dose_b_ml < config.dosing_min_dose_ml;
                        let active_capacity_b = if let Some(cap) =
                            effective_flow_ml_per_sec(DosePumpKind::PumpB, dose_pwm, config)
                        {
                            cap
                        } else {
                            warn!("Skip dose pump B: invalid config/pwm (pwm={}%)", dose_pwm);
                            ctx.stop_all_pumps(pump_ctrl);
                            ctx.current_state = SystemState::Monitoring;
                            return;
                        };

                        let pulse_on_ms = if is_pulse_mode {
                            config.dosing_pulse_on_ms.max(1)
                        } else {
                            ((dose_b_ml / active_capacity_b) * 1000.0) as u64
                        };
                        let pulse_off_ms = if is_pulse_mode {
                            config.dosing_pulse_off_ms
                        } else {
                            0
                        };
                        let max_pulse_count = if is_pulse_mode {
                            config.dosing_max_pulse_count_per_cycle.max(1)
                        } else {
                            1
                        };
                        let _ =
                            pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientB, true, dose_pwm);
                        ctx.pump_status.pump_b = true;
                        ctx.pump_status.pump_b_pwm = Some(dose_pwm);
                        ctx.set_pulse_status(is_pulse_mode, if is_pulse_mode { 1 } else { 0 });
                        let delivered_ml_est = active_capacity_b * (pulse_on_ms as f32 / 1000.0);

                        ctx.current_state = SystemState::DosingPumpB {
                            next_toggle_time: current_time_ms + pulse_on_ms,
                            dose_target_ml: dose_b_ml,
                            delivered_ml_est,
                            pulse_on: true,
                            pulse_count: 1,
                            max_pulse_count,
                            pulse_on_ms,
                            pulse_off_ms,
                            pwm_percent: dose_pwm,
                            active_capacity_ml_per_sec: active_capacity_b,
                            target_ec,
                            start_ec,
                            start_ph,
                            dose_a_ml_reported,
                        };
                    } else {
                        let report_json = format!(
                            r#"{{"start_ec":{:.2},"start_ph":{:.2},"pump_a_ml":{:.2},"pump_b_ml":0.0,"ph_up_ml":0.0,"ph_down_ml":0.0,"target_ec":{:.2},"target_ph":{:.2}}}"#,
                            start_ec, start_ph, dose_a_ml_reported, target_ec, config.ph_target
                        );
                        let _ = dosing_report_tx.send(report_json);
                        start_pending_calibration_sample(
                            ctx,
                            start_ec,
                            start_ph,
                            dose_a_ml_reported,
                            0.0,
                            0.0,
                            0.0,
                            current_time_ms,
                            config,
                        );
                        ctx.current_state = SystemState::ActiveMixing {
                            finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
                        };
                    }
                }
            }

            SystemState::DosingPumpB {
                next_toggle_time,
                dose_target_ml,
                delivered_ml_est,
                pulse_on,
                pulse_count,
                max_pulse_count,
                pulse_on_ms,
                pulse_off_ms,
                pwm_percent,
                active_capacity_ml_per_sec,
                target_ec,
                start_ec,
                start_ph,
                dose_a_ml_reported,
            } => {
                if current_time_ms >= next_toggle_time {
                    if pulse_on {
                        let _ = pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientB, false, 0);
                        ctx.pump_status.pump_b = false;
                        ctx.pump_status.pump_b_pwm = Some(0);
                        let hit_target = delivered_ml_est >= dose_target_ml;
                        let hit_limit = pulse_count >= max_pulse_count;
                        if hit_target || hit_limit {
                            ctx.set_pulse_status(false, pulse_count);
                            let report_json = format!(
                                r#"{{"start_ec":{:.2},"start_ph":{:.2},"pump_a_ml":{:.2},"pump_b_ml":{:.2},"ph_up_ml":0.0,"ph_down_ml":0.0,"target_ec":{:.2},"target_ph":{:.2}}}"#,
                                start_ec,
                                start_ph,
                                dose_a_ml_reported,
                                delivered_ml_est.min(dose_target_ml),
                                target_ec,
                                config.ph_target
                            );
                            let _ = dosing_report_tx.send(report_json);
                            ctx.current_state = SystemState::ActiveMixing {
                                finish_time: current_time_ms
                                    + (config.active_mixing_sec as u64 * 1000),
                            };
                        } else {
                            ctx.set_pulse_status(true, pulse_count);
                            ctx.current_state = SystemState::DosingPumpB {
                                next_toggle_time: current_time_ms + pulse_off_ms,
                                dose_target_ml,
                                delivered_ml_est,
                                pulse_on: false,
                                pulse_count,
                                max_pulse_count,
                                pulse_on_ms,
                                pulse_off_ms,
                                pwm_percent,
                                active_capacity_ml_per_sec,
                                target_ec,
                                start_ec,
                                start_ph,
                                dose_a_ml_reported,
                            };
                        }
                    } else {
                        let _ =
                            pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientB, true, pwm_percent);
                        ctx.pump_status.pump_b = true;
                        ctx.pump_status.pump_b_pwm = Some(pwm_percent);
                        let next_count = pulse_count + 1;
                        let next_delivered = delivered_ml_est
                            + active_capacity_ml_per_sec * (pulse_on_ms as f32 / 1000.0);
                        ctx.set_pulse_status(true, next_count);
                        ctx.current_state = SystemState::DosingPumpB {
                            next_toggle_time: current_time_ms + pulse_on_ms,
                            dose_target_ml,
                            delivered_ml_est: next_delivered,
                            pulse_on: true,
                            pulse_count: next_count,
                            max_pulse_count,
                            pulse_on_ms,
                            pulse_off_ms,
                            pwm_percent,
                            active_capacity_ml_per_sec,
                            target_ec,
                            start_ec,
                            start_ph,
                            dose_a_ml_reported,
                        };
                    }
                }
            }

            SystemState::DosingPH {
                next_toggle_time,
                is_up,
                dose_target_ml,
                delivered_ml_est,
                pulse_on,
                pulse_count,
                max_pulse_count,
                pulse_on_ms,
                pulse_off_ms,
                pwm_percent,
                active_capacity_ml_per_sec,
                target_ph,
                start_ec,
                start_ph,
            } => {
                if current_time_ms >= next_toggle_time {
                    let pump_type = if is_up {
                        PumpType::PhUp
                    } else {
                        PumpType::PhDown
                    };
                    if pulse_on {
                        let _ = pump_ctrl.set_dosing_pump_pulse(pump_type, false, 0);
                        if is_up {
                            ctx.pump_status.ph_up = false;
                            ctx.pump_status.ph_up_pwm = Some(0);
                        } else {
                            ctx.pump_status.ph_down = false;
                            ctx.pump_status.ph_down_pwm = Some(0);
                        }
                        let hit_target = delivered_ml_est >= dose_target_ml;
                        let hit_limit = pulse_count >= max_pulse_count;
                        if hit_target || hit_limit {
                            ctx.set_pulse_status(false, pulse_count);
                            let ph_up_ml = if is_up {
                                delivered_ml_est.min(dose_target_ml)
                            } else {
                                0.0
                            };
                            let ph_down_ml = if !is_up {
                                delivered_ml_est.min(dose_target_ml)
                            } else {
                                0.0
                            };
                            let report_json = format!(
                                r#"{{"start_ec":{:.2},"start_ph":{:.2},"pump_a_ml":0.0,"pump_b_ml":0.0,"ph_up_ml":{:.2},"ph_down_ml":{:.2},"target_ec":{:.2},"target_ph":{:.2}}}"#,
                                start_ec,
                                start_ph,
                                ph_up_ml,
                                ph_down_ml,
                                config.ec_target,
                                target_ph
                            );
                            let _ = dosing_report_tx.send(report_json);
                            ctx.current_state = SystemState::ActiveMixing {
                                finish_time: current_time_ms
                                    + (config.active_mixing_sec as u64 * 1000),
                            };
                        } else {
                            ctx.set_pulse_status(true, pulse_count);
                            ctx.current_state = SystemState::DosingPH {
                                next_toggle_time: current_time_ms + pulse_off_ms,
                                is_up,
                                dose_target_ml,
                                delivered_ml_est,
                                pulse_on: false,
                                pulse_count,
                                max_pulse_count,
                                pulse_on_ms,
                                pulse_off_ms,
                                pwm_percent,
                                active_capacity_ml_per_sec,
                                target_ph,
                                start_ec,
                                start_ph,
                            };
                        }
                    } else {
                        let _ = pump_ctrl.set_dosing_pump_pulse(pump_type, true, pwm_percent);
                        if is_up {
                            ctx.pump_status.ph_up = true;
                            ctx.pump_status.ph_up_pwm = Some(pwm_percent);
                        } else {
                            ctx.pump_status.ph_down = true;
                            ctx.pump_status.ph_down_pwm = Some(pwm_percent);
                        }
                        let next_count = pulse_count + 1;
                        let next_delivered = delivered_ml_est
                            + active_capacity_ml_per_sec * (pulse_on_ms as f32 / 1000.0);
                        ctx.set_pulse_status(true, next_count);
                        ctx.current_state = SystemState::DosingPH {
                            next_toggle_time: current_time_ms + pulse_on_ms,
                            is_up,
                            dose_target_ml,
                            delivered_ml_est: next_delivered,
                            pulse_on: true,
                            pulse_count: next_count,
                            max_pulse_count,
                            pulse_on_ms,
                            pulse_off_ms,
                            pwm_percent,
                            active_capacity_ml_per_sec,
                            target_ph,
                            start_ec,
                            start_ph,
                        };
                    }
                }
            }

            SystemState::ActiveMixing { finish_time } => {
                if current_time_ms >= finish_time {
                    ctx.fsm_osaka_active = false;
                    if let Some(sample) = ctx.pending_calibration_sample.as_mut() {
                        sample.active_mixing_finish_ms = current_time_ms;
                        sample.stabilizing_start_ms = Some(current_time_ms);
                        sample.stabilizing_finish_ms =
                            Some(current_time_ms + (config.sensor_stabilize_sec as u64 * 1000));
                    }
                    ctx.current_state = SystemState::Stabilizing {
                        finish_time: current_time_ms + (config.sensor_stabilize_sec as u64 * 1000),
                    };
                }
            }

            SystemState::Stabilizing { finish_time } => {
                if current_time_ms >= finish_time {
                    if let Some(sample) = ctx.pending_calibration_sample.as_mut() {
                        sample.stabilizing_finish_ms = Some(current_time_ms);
                    }
                    apply_runtime_calibration_ema(sensors, shared_config, ctx, fsm_mqtt_tx);

                    ctx.current_state = SystemState::DosingCycleComplete;
                }
            }

            SystemState::EmergencyStop(_) => {
                //Không làm gì cả - tránh spam khi EmergencyStop
            }
        }
    }
}
