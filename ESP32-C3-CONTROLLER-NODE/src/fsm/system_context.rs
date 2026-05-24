use hydragrow_shared::fsm::{FsmDiagnostics, SystemPhase};
use hydragrow_shared::PumpStatus;
use serde::{Deserialize, Serialize};

use super::actors::{
    dosing_actor::DosingActor, safety_guard::SafetyGuard, water_actor::WaterActor,
};
use super::types::PendingCalibrationSample;
use crate::fsm::matrix::{InteractionMatrix, KalmanCovarianceDiag};

pub type CronSchedule = String;

// --- BỘ GIÁM SÁT ADAPTIVE TÍN HIỆU PHẲNG ---
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

    pub fn is_stable(&self, config: &hydragrow_shared::ControllerConfig) -> bool {
        if self.count < 5 {
            return false; // Chưa tích lũy đủ 5 mốc dữ liệu nền
        }

        // Trục kiểm tra Dinh dưỡng EC (Chỉ xét khi cảm biến bật)
        let ec_is_stable = if config.enable_ec_sensor {
            let max_ec = self.history_ec.iter().fold(f32::MIN, |a, &b| a.max(b));
            let min_ec = self.history_ec.iter().fold(f32::MAX, |a, &b| a.min(b));
            (max_ec - min_ec) < 0.05
        } else {
            true // Nếu cảm biến tắt, mặc định trục này bỏ qua, coi như đã phẳng an toàn
        };

        // Trục kiểm tra Độ kiềm pH (Chỉ xét khi cảm biến bật)
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

pub struct SystemContext {
    pub dosing_cycle_count: u64,
    pub phase: SystemPhase,
    pub previous_phase: Option<SystemPhase>,
    pub phase_start_ms: Option<u64>,
    pub phase_finish_ms: Option<u64>,
    pub dosing: DosingActor,
    pub water: WaterActor,
    pub safety: SafetyGuard,
    pub calibration: CalibrationSampler,
    pub tuner: AutoTuner,
    pub peripherals: PeripheralState,
    pub stabilizer_tracker: SensorStabilizerTracker,
    pub diagnostic: FsmDiagnostics,
    pub water_change_cron: CronSchedule,
    pub scheduled_dosing_cron: CronSchedule,
    pub last_water_change_sec: u64,
    pub next_water_change_trigger_sec: Option<u64>,
    pub next_scheduled_dosing_trigger_sec: Option<u64>,
}

impl SystemContext {
    pub fn apply_peripheral_delta(&mut self, pd: crate::fsm::tick_result::PeripheralDelta) {
        let p = &mut self.peripherals;
        if let Some(v) = pd.osaka_pump {
            p.pump_status.osaka_pump = v;
            // Đã xóa p.osaka_active = v; (Task 2)
        }
        if let Some(v) = pd.osaka_pwm {
            p.pump_status.osaka_pwm = Some(v);
            p.osaka_pwm = v;
        }
        if let Some(v) = pd.is_misting_active {
            p.is_misting_active = v;
        }
        if let Some(v) = pd.last_mist_toggle_time {
            p.last_mist_toggle_time = v;
        }
        if let Some(v) = pd.mist_valve {
            p.pump_status.mist_valve = v;
        }
        if let Some(v) = pd.is_scheduled_mixing_active {
            p.is_scheduled_mixing_active = v;
        }
        if let Some(v) = pd.last_mixing_start_sec {
            p.last_mixing_start_sec = v;
        }
    }
}

impl SystemContext {
    /// Áp dụng ContextDelta vào context — điểm duy nhất được phép mutate ctx.
    /// Gọi sau khi Pure Decision Engine trả về TickResult.
    pub fn apply_delta(&mut self, delta: &mut crate::fsm::tick_result::ContextDelta) {
        use crate::fsm::tick_result::CalibrationDelta;

        if let Some(ref new_phase) = delta.phase {
            if *new_phase != self.phase {
                // Phase đang thay đổi — lưu lại để observer có thể emit transition event
                delta.previous_phase = Some(self.phase.clone());
                delta.phase_start_before = self.phase_start_ms;
            }
        }

        // --- TASK 1 FIX ---
        // Ghi lại previous_phase vào context để MQTT payload build_status_msg có thể lấy được
        if let Some(ref prev) = delta.previous_phase {
            self.previous_phase = Some(prev.clone());
        }

        if let Some(phase) = delta.phase.as_ref() {
            self.phase = phase.clone();
        }

        // phase_start_ms: Some(Some(t)) = set t, Some(None) = clear
        if let Some(v) = delta.phase_start_ms {
            self.phase_start_ms = v;
        }
        if let Some(v) = delta.phase_finish_ms {
            self.phase_finish_ms = v;
        }

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

        if let Some(until) = delta.safety_override_until {
            self.safety.safety_override_until = until;
        }
        if let Some((pump, finish_ms)) = delta.manual_pump_timeout.clone() {
            self.safety.manual_timeouts.insert(pump, finish_ms);
        }
        if let Some(pump) = delta.manual_pump_timeout_clear.clone() {
            self.safety.manual_timeouts.remove(&pump);
        }

        // --- Peripherals ---
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

        // --- Calibration ---
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

// Cập nhật hàm impl Default của SystemContext để tránh lỗi khởi tạo rỗng:
impl Default for SystemContext {
    fn default() -> Self {
        Self {
            phase: SystemPhase::Booting,
            previous_phase: None,
            dosing_cycle_count: 0,
            phase_start_ms: None,
            phase_finish_ms: None,
            dosing: DosingActor::new(),
            water: WaterActor::new(),
            safety: SafetyGuard::new(),
            calibration: CalibrationSampler::default(),
            tuner: AutoTuner::default(),
            peripherals: PeripheralState::default(),
            stabilizer_tracker: SensorStabilizerTracker::default(),
            diagnostic: FsmDiagnostics::default(),
            water_change_cron: String::new(),
            scheduled_dosing_cron: String::new(),
            last_water_change_sec: 0,
            next_water_change_trigger_sec: None,
            next_scheduled_dosing_trigger_sec: None,
        }
    }
}

pub struct PeripheralState {
    pub pump_status: PumpStatus,
    pub osaka_pwm: u32,
    pub is_misting_active: bool,
    pub last_mist_toggle_time: u64,
    pub is_scheduled_mixing_active: bool,
    pub last_mixing_start_sec: u64,
    pub last_continuous_level: bool,
    pub previous_ec: Option<f32>,
    pub previous_ph: Option<f32>,
    pub misting_started_by_dosing: bool,
}

impl PeripheralState {
    pub fn reset(&mut self, now_sec: u64) {
        self.pump_status = PumpStatus::default();
        self.osaka_pwm = 0;
        self.is_misting_active = false;
        self.is_scheduled_mixing_active = false;
        self.last_mist_toggle_time = 0;
        self.misting_started_by_dosing = false;
        self.last_mixing_start_sec = now_sec;
    }
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
    pub interaction_matrix: Option<[f32; 32]>,

    #[serde(default)]
    pub matrix_update_count: u32,
    #[serde(default)]
    pub matrix_is_warm: bool,
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

impl Default for PeripheralState {
    fn default() -> Self {
        Self {
            pump_status: PumpStatus::default(),
            osaka_pwm: 0,
            is_misting_active: false,
            last_mist_toggle_time: 0,
            is_scheduled_mixing_active: false,
            last_mixing_start_sec: 0,
            last_continuous_level: false,
            previous_ec: None,
            previous_ph: None,
            misting_started_by_dosing: false,
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
            interaction_matrix: InteractionMatrix::from_scalar(0.015, 0.02, 0.025, 0.05, 0.04),
            kalman: KalmanCovarianceDiag::new(1.0, 0.001, 0.1),
            matrix_update_count: 0,
            matrix_is_warm: false,
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
        let flat = self.interaction_matrix.as_flat();

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

impl NvsSnapshot {
    pub fn from_context(ctx: &SystemContext, now_sec: u64) -> Self {
        let hourly_dose_ec_ml = ctx
            .safety
            .hourly_doses()
            .iter()
            .filter(|(pump, _)| pump.as_str() == "NutrientA" || pump.as_str() == "NutrientB")
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
            .filter(|(pump, _)| pump.as_str() == "PhUp" || pump.as_str() == "PhDown")
            .map(|(_, h)| {
                h.iter()
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
            matrix_is_warm: ctx.tuner.matrix_is_warm,
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

    /// Entry point duy nhất cho adaptive learning pipeline.
    /// Gọi sau mỗi dosing cycle hoàn tất (cuối StabilizingPhase).
    /// Điều phối: GainLearner → AutoTuner ACK → InteractionMatrix Kalman → warm tracking.
    ///
    /// Trả về true nếu learning đã được thực hiện (để caller quyết định có publish update không).
    pub fn learn_from_cycle(
        &mut self,
        sample: &crate::fsm::types::PendingCalibrationSample,
        post_ec: f32,
        post_ph: f32,
        post_water: f32,
        post_temp: f32,
        config: &hydragrow_shared::ControllerConfig,
        now_sec: u64,
    ) -> bool {
        // Bỏ qua nếu sample bị đánh dấu invalid
        if sample.invalid_by_noise || sample.invalid_by_water_change {
            return false;
        }

        let actual_delta_ec = post_ec - sample.start_ec;
        let actual_delta_ph = post_ph - sample.start_ph;
        let total_nutrient_ml = sample.dose_a_ml + sample.dose_b_ml;
        let ph_dose_ml = sample.dose_ph_up_ml + sample.dose_ph_down_ml;
        let is_ph_up = sample.dose_ph_up_ml > sample.dose_ph_down_ml;

        // Cập nhật GainLearner từ kết quả quan sát thực tế
        if total_nutrient_ml > 0.5 && actual_delta_ec > 0.01 {
            self.gain_learner
                .update_ec_gain(total_nutrient_ml, actual_delta_ec, config);
        }
        if ph_dose_ml > 0.1 && actual_delta_ph.abs() > 0.01 {
            self.gain_learner
                .update_ph_gain(ph_dose_ml, actual_delta_ph.abs(), is_ph_up, config);
        }

        // AutoTuner ACK để cập nhật adaptive step ratio
        // response = kết quả thực tế / kết quả kỳ vọng (theo gain hiện tại)
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

        // Cập nhật InteractionMatrix via Kalman filter
        self.interaction_matrix.update_matrix_adaptive(
            &mut self.kalman,
            sample,
            post_ec,
            post_ph,
            post_water,
            post_temp,
        );

        // Cập nhật tracking độ ấm của matrix
        self.matrix_update_count = self.matrix_update_count.saturating_add(1);
        if !self.matrix_is_warm && self.matrix_update_count >= 10 {
            self.matrix_is_warm = true;
            log::info!(
            "🔥 [ADAPTIVE] InteractionMatrix đã ĐỦ ẤM sau {} cycles! Chuyển sang WarmPathSolver.",
            self.matrix_update_count
        );
        }

        log::info!(
            "🧠 [ADAPTIVE] Cycle học #{}: ΔEC={:.3}, ΔpH={:.3}, Matrix warm={}, Updates={}",
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
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::fsm::tick_result::ContextDelta;
//     use hydragrow_shared::fsm::SystemPhase;
//
//     #[test]
//     fn apply_delta_sets_previous_phase_on_transition() {
//         let mut ctx = SystemContext::default();
//         assert_eq!(ctx.phase, SystemPhase::Booting);
//         assert!(ctx.previous_phase.is_none());
//
//         let mut delta = ContextDelta::default();
//         delta.phase = Some(SystemPhase::Monitoring);
//         ctx.apply_delta(&mut delta);
//
//         assert_eq!(ctx.phase, SystemPhase::Monitoring);
//         assert_eq!(ctx.previous_phase, Some(SystemPhase::Booting));
//     }
//
//     #[test]
//     fn apply_delta_does_not_overwrite_previous_phase_when_phase_unchanged() {
//         let mut ctx = SystemContext::default();
//
//         // First transition: Booting → Monitoring
//         let mut delta1 = ContextDelta::default();
//         delta1.phase = Some(SystemPhase::Monitoring);
//         ctx.apply_delta(&mut delta1);
//
//         // Second delta: no phase change
//         let mut delta2 = ContextDelta::default();
//         ctx.apply_delta(&mut delta2);
//
//         // previous_phase should still be Booting, not overwritten by Monitoring
//         assert_eq!(ctx.previous_phase, Some(SystemPhase::Booting));
//     }
// }
