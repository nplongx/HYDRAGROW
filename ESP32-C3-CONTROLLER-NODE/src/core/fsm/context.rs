// src/core/fsm/context.rs
//! SystemContext & State DTOs — Lưu trữ toàn bộ trạng thái Runtime của FSM.
//! Thuộc tầng Pure Core: Không chứa side-effects hay driver phần cứng.

use hydragrow_shared::fsm::{FsmDiagnostics, SystemPhase};
use hydragrow_shared::{ControllerConfig, PumpStatus};
use serde::{Deserialize, Serialize};

use crate::core::actors::dosing_actor::DosingActor;
use crate::core::actors::safety_guard::SafetyGuard;
use crate::core::actors::water_actor::WaterActor;
use crate::core::adaptive::tuner::{AutoTuner, TunerState};
use crate::core::fsm::tick_result::{CalibrationDelta, ContextDelta};
use crate::core::fsm::types::PendingCalibrationSample;

const DEVICE_ID: &str = "device_001";
pub type CronSchedule = String;

// ============================================================================
// 1. SENSOR STABILIZER TRACKER
// ============================================================================

/// Bộ lọc theo dõi 5 mẫu EC/pH gần nhất để xác định độ ổn định dung dịch
#[derive(Debug, Clone)]
pub struct SensorStabilizerTracker {
    pub history_ec: [f32; 5],
    pub history_ph: [f32; 5],
    pub count: usize,
    pub head: usize,
}

impl Default for SensorStabilizerTracker {
    fn default() -> Self {
        Self {
            history_ec: [0.0; 5],
            history_ph: [0.0; 5],
            count: 0,
            head: 0,
        }
    }
}

impl SensorStabilizerTracker {
    pub fn push(&mut self, ec: f32, ph: f32) {
        self.history_ec[self.head] = ec;
        self.history_ph[self.head] = ph;
        self.head = (self.head + 1) % 5;
        if self.count < 5 {
            self.count += 1;
        }
    }

    pub fn is_stable(&self, config: &ControllerConfig) -> bool {
        if self.count < 5 {
            return false; // Chưa đủ 5 mẫu
        }

        // Kiểm tra ổn định EC
        let ec_is_stable = if config.enable_ec_sensor {
            let max_ec = self.history_ec.iter().fold(f32::MIN, |a, &b| a.max(b));
            let min_ec = self.history_ec.iter().fold(f32::MAX, |a, &b| a.min(b));
            (max_ec - min_ec) < 0.05
        } else {
            true
        };

        // Kiểm tra ổn định pH
        let ph_is_stable = if config.enable_ph_sensor {
            let max_ph = self.history_ph.iter().fold(f32::MIN, |a, &b| a.max(b));
            let min_ph = self.history_ph.iter().fold(f32::MAX, |a, &b| a.min(b));
            (max_ph - min_ph) < 0.05
        } else {
            true
        };

        ec_is_stable && ph_is_stable
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

// ============================================================================
// 2. PERIPHERAL STATE & CALIBRATION SAMPLER
// ============================================================================

/// Trạng thái ngoại vi (Bơm, Van, Phun sương, Osaka)
#[derive(Debug, Clone, Default)]
pub struct PeripheralState {
    pub pump_status: PumpStatus, // Ý định phần cứng
    pub osaka_pwm: u32,
    pub is_misting_active: bool, // Ý định logic
    pub last_mist_toggle_time: u64,
    pub is_scheduled_mixing_active: bool,
    pub last_mixing_start_sec: u64,
    pub last_continuous_level: bool,
    pub previous_ec: Option<f32>,
    pub previous_ph: Option<f32>,
    pub misting_started_by_dosing: bool,
}

/// Quản lý mẫu thu thập dữ liệu calibration đang chờ xử lý
#[derive(Debug, Clone, Default)]
pub struct CalibrationSampler {
    pub pending_sample: Option<PendingCalibrationSample>,
}

impl CalibrationSampler {
    pub fn start_sample(&mut self, sample: PendingCalibrationSample) {
        self.pending_sample = Some(sample);
    }

    pub fn finalize(&mut self) -> Option<PendingCalibrationSample> {
        self.pending_sample.take()
    }
}

// ============================================================================
// 3. SYSTEM CONTEXT (CORE FSM STATE)
// ============================================================================

/// Context duy nhất chứa toàn bộ State của FSM
pub struct SystemContext {
    pub dosing_cycle_count: u64,
    pub phase: SystemPhase,
    pub previous_phase: Option<SystemPhase>,
    pub phase_start_ms: Option<u64>,
    pub phase_finish_ms: Option<u64>,

    // Sub-Actors
    pub dosing: DosingActor,
    pub water: WaterActor,
    pub safety: SafetyGuard,

    // Adaptive & Diagnostics Engine
    pub calibration: CalibrationSampler,
    pub tuner: AutoTuner,
    pub peripherals: PeripheralState,
    pub stabilizer_tracker: SensorStabilizerTracker,
    pub diagnostic: FsmDiagnostics,

    // Water Change Schedule
    pub water_change_cron: CronSchedule,
    pub last_water_change_sec: u64,
    pub next_water_change_trigger_sec: Option<u64>,

    // Recipe Engine
    pub current_stage_index: Option<usize>,
    pub recipe_completed: bool,
    pub last_recipe_check_sec: u64,
}

impl Default for SystemContext {
    fn default() -> Self {
        Self {
            phase: SystemPhase::Booting,
            previous_phase: None,
            dosing_cycle_count: 0,
            phase_start_ms: None,
            phase_finish_ms: None,
            dosing: DosingActor::new(),
            water: WaterActor::new(DEVICE_ID),
            safety: SafetyGuard::new(),
            calibration: CalibrationSampler::default(),
            tuner: AutoTuner::default(),
            peripherals: PeripheralState::default(),
            stabilizer_tracker: SensorStabilizerTracker::default(),
            diagnostic: FsmDiagnostics::default(),
            water_change_cron: String::new(),
            last_water_change_sec: 0,
            next_water_change_trigger_sec: None,
            current_stage_index: None,
            recipe_completed: false,
            last_recipe_check_sec: 0,
        }
    }
}

impl SystemContext {
    /// Áp dụng ContextDelta vào SystemContext (Nơi DUY NHẤT được phép mutate state của Context).
    pub fn apply_delta(&mut self, delta: &mut ContextDelta) {
        // --- 1. Phase Transition Tracking ---
        if let Some(ref new_phase) = delta.phase {
            if *new_phase != self.phase {
                delta.previous_phase = Some(self.phase.clone());
                delta.phase_start_before = self.phase_start_ms;
            }
        }

        if let Some(ref prev) = delta.previous_phase {
            self.previous_phase = Some(prev.clone());
        }

        if let Some(phase) = delta.phase.as_ref() {
            self.phase = phase.clone();
        }

        if let Some(v) = delta.phase_start_ms {
            self.phase_start_ms = v;
        }

        if let Some(v) = delta.phase_finish_ms {
            self.phase_finish_ms = v;
        }

        // --- 2. Counters & Timers ---
        if delta.dosing_cycle_count_increment {
            self.dosing_cycle_count = self.dosing_cycle_count.saturating_add(1);
        }

        if delta.reset_stabilizer {
            self.stabilizer_tracker.reset();
        }

        if let Some(sec) = delta.last_water_change_sec {
            self.last_water_change_sec = sec;
        }

        if let Some(v) = delta.next_water_change_trigger_sec {
            self.next_water_change_trigger_sec = v;
        }

        if let Some(cron) = delta.water_change_cron.clone() {
            self.water_change_cron = cron;
        }

        if let Some(v) = delta.current_stage_index {
            self.current_stage_index = v;
        }

        if let Some(v) = delta.recipe_completed {
            self.recipe_completed = v;
        }

        if let Some(v) = delta.last_recipe_check_sec {
            self.last_recipe_check_sec = v;
        }

        // --- 3. Safety & Budget Reset ---
        if delta.reset_safety_budget {
            self.safety.flush_for_reset();
            self.tuner.on_manual_reset();
        }

        if let Some(until) = delta.safety_override_until {
            self.safety.safety_override_until = until;
        }

        if let Some((pump, finish_ms)) = delta.manual_pump_timeout.clone() {
            self.safety.manual_timeouts.insert(pump, finish_ms);
        }

        if let Some(pump) = delta.manual_pump_timeout_clear.clone() {
            self.safety.manual_timeouts.remove(&pump);
        }

        // --- 4. Peripherals Update ---
        if let Some(pd) = delta.peripherals.clone() {
            let p = &mut self.peripherals;
            if let Some(v) = pd.pump_a {
                p.pump_status.pump_a = v;
            }
            if let Some(v) = pd.pump_b {
                p.pump_status.pump_b = v;
            }
            if let Some(v) = pd.ph_up {
                p.pump_status.ph_up = v;
            }
            if let Some(v) = pd.ph_down {
                p.pump_status.ph_down = v;
            }
            if let Some(v) = pd.water_pump_in {
                p.pump_status.water_pump_in = v;
            }
            if let Some(v) = pd.water_pump_out {
                p.pump_status.water_pump_out = v;
            }
            if let Some(v) = pd.mist_valve {
                p.pump_status.mist_valve = v;
            }
            if let Some(v) = pd.mix_valve {
                p.pump_status.mix_valve = v;
            }
            if let Some(v) = pd.osaka_pump {
                p.pump_status.osaka_pump = v;
            }
            if let Some(v) = pd.osaka_pwm {
                p.pump_status.osaka_pwm = Some(v);
                p.osaka_pwm = v;
            }
            if let Some(v) = pd.is_misting_active {
                p.is_misting_active = v;
            }
            if let Some(v) = pd.is_scheduled_mixing_active {
                p.is_scheduled_mixing_active = v;
            }
            if let Some(v) = pd.misting_started_by_dosing {
                p.misting_started_by_dosing = v;
            }
            if let Some(v) = pd.last_mist_toggle_time {
                p.last_mist_toggle_time = v;
            }
            if let Some(v) = pd.last_mixing_start_sec {
                p.last_mixing_start_sec = v;
            }
            if let Some(v) = pd.last_ec_before_dose {
                self.safety.last_ec_before_dose = v;
            }
            if let Some(v) = pd.last_ph_before_dose {
                self.safety.last_ph_before_dose = v;
            }
            if let Some(v) = pd.previous_ec {
                p.previous_ec = v;
            }
            if let Some(v) = pd.previous_ph {
                p.previous_ph = v;
            }
            if let Some(v) = pd.last_continuous_level {
                p.last_continuous_level = v;
            }
        }

        // --- 5. Calibration Sample Updates ---
        if let Some(cal) = delta.calibration.clone() {
            match cal {
                CalibrationDelta::Start(sample) => {
                    self.calibration.start_sample(sample);
                }
                CalibrationDelta::Invalidate => {
                    if let Some(s) = self.calibration.pending_sample.as_mut() {
                        s.invalid_by_noise = true;
                    }
                }
                CalibrationDelta::UpdatePostMixing { ec, ph, finish_ms } => {
                    if let Some(s) = self.calibration.pending_sample.as_mut() {
                        s.post_mixing_ec = ec;
                        s.post_mixing_ph = ph;
                        s.stabilizing_start_ms = Some(finish_ms);
                        s.active_mixing_finish_ms = finish_ms;
                    }
                }
            }
        }
    }
}

// ============================================================================
// 4. NVS SNAPSHOT DTO (PERISTENCE MAPPING)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NvsSnapshot {
    // 🟢 Tách riêng 4 kênh
    #[serde(default)]
    pub step_ratio_ec_a: f32,
    #[serde(default)]
    pub step_ratio_ec_b: f32,
    #[serde(default)]
    pub step_ratio_ph_up: f32,
    #[serde(default)]
    pub step_ratio_ph_down: f32,

    #[serde(default)]
    pub best_ec_a_ratio: f32,
    #[serde(default)]
    pub best_ec_b_ratio: f32,
    #[serde(default)]
    pub best_ph_up_ratio: f32,
    #[serde(default)]
    pub best_ph_down_ratio: f32,

    // 🟢 Chừa trường cũ để tương thích ngược JSON cũ (tránh crash lúc parse)
    #[serde(default)]
    pub step_ratio_ec: f32,
    #[serde(default)]
    pub step_ratio_ph: f32,
    #[serde(default)]
    pub best_ec_ratio: f32,
    #[serde(default)]
    pub best_ph_ratio: f32,

    pub last_water_change_sec: u64,
    pub hourly_dose_ec_ml: f32,
    pub hourly_dose_ph_ml: f32,
    pub hourly_window_start_sec: u64,
    pub retry_ec: u8,
    pub retry_ph: u8,
    pub dosing_cycle_count: u64,

    pub ema_ec_gain: f32,
    pub ema_ph_up_gain: f32,
    pub ema_ph_down_gain: f32,
    pub ec_sample_count: u32,
    pub ph_sample_count: u32,
    #[serde(default)]
    pub ph_up_sample_count: u32,
    #[serde(default)]
    pub ph_down_sample_count: u32,

    #[serde(default)]
    pub tuner_state: u8,

    #[serde(default)]
    pub ec_a_variance_baseline: f32,
    #[serde(default)]
    pub ec_b_variance_baseline: f32,
    #[serde(default)]
    pub ph_variance_baseline: f32,

    #[serde(default)]
    pub interaction_matrix: Option<[f32; 32]>,
    #[serde(default)]
    pub matrix_update_count: u32,
    #[serde(default)]
    pub matrix_is_warm: bool,

    #[serde(default)]
    pub current_stage_index: Option<usize>,
}

impl NvsSnapshot {
    pub fn from_context(ctx: &SystemContext, now_sec: u64) -> Self {
        let hourly_dose_ec_ml = ctx
            .safety
            .hourly_doses()
            .iter()
            .filter(|(p, _)| p.as_str() == "NutrientA" || p.as_str() == "NutrientB")
            .map(|(_, h)| {
                h.iter()
                    .filter(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600)
                    .map(|(_, ml)| ml)
                    .sum::<f32>()
            })
            .sum();

        let hourly_dose_ph_ml = ctx
            .safety
            .hourly_doses()
            .iter()
            .filter(|(p, _)| p.as_str() == "PhUp" || p.as_str() == "PhDown")
            .map(|(_, h)| {
                h.iter()
                    .filter(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600)
                    .map(|(_, ml)| ml)
                    .sum::<f32>()
            })
            .sum();

        Self {
            step_ratio_ec_a: ctx.tuner.adaptive_ec_a_ratio,
            step_ratio_ec_b: ctx.tuner.adaptive_ec_b_ratio,
            step_ratio_ph_up: ctx.tuner.adaptive_ph_up_ratio,
            step_ratio_ph_down: ctx.tuner.adaptive_ph_down_ratio,

            best_ec_a_ratio: ctx.tuner.best_ec_a_ratio,
            best_ec_b_ratio: ctx.tuner.best_ec_b_ratio,
            best_ph_up_ratio: ctx.tuner.best_ph_up_ratio,
            best_ph_down_ratio: ctx.tuner.best_ph_down_ratio,

            step_ratio_ec: ctx.tuner.adaptive_ec_ratio(),
            step_ratio_ph: ctx.tuner.adaptive_ph_ratio(),
            best_ec_ratio: ctx.tuner.best_ec_ratio(),
            best_ph_ratio: ctx.tuner.best_ph_ratio(),

            last_water_change_sec: ctx.last_water_change_sec,
            hourly_dose_ec_ml,
            hourly_dose_ph_ml,
            hourly_window_start_sec: now_sec.saturating_sub(3600),
            retry_ec: ctx.dosing.retry_ec,
            retry_ph: ctx.dosing.retry_ph,
            dosing_cycle_count: ctx.dosing_cycle_count,

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
            ec_a_variance_baseline: ctx.tuner.ec_a_variance_baseline,
            ec_b_variance_baseline: ctx.tuner.ec_b_variance_baseline,
            ph_variance_baseline: ctx.tuner.ph_variance_baseline,
            interaction_matrix: Some(ctx.tuner.interaction_matrix.as_flat()),
            matrix_update_count: ctx.tuner.matrix_update_count,
            matrix_is_warm: ctx.tuner.matrix_is_warm,
            current_stage_index: ctx.current_stage_index,
        }
    }
}
