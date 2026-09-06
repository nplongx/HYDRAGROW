//! TickResult — Output của một tick Pure Decision Engine.
//! Tách biệt hoàn toàn "quyết định gì" khỏi "thực thi gì".

use hydragrow_shared::fsm::SystemPhase;

use crate::core::fsm::types::PendingCalibrationSample;

use super::events::OrchestratorEvent;

/// Những thay đổi state muốn áp dụng vào SystemContext sau một tick.
/// Tất cả fields đều Optional — None nghĩa là "giữ nguyên".
#[derive(Debug, Default, Clone)]
pub struct ContextDelta {
    /// Chuyển phase FSM
    pub phase: Option<SystemPhase>,

    /// Cập nhật mốc thời gian phase
    pub phase_start_ms: Option<Option<u64>>, // Some(Some(t)) = set, Some(None) = clear
    pub phase_finish_ms: Option<Option<u64>>, // Thời gian hoàn thành phase lý thuyết

    /// Thay đổi trạng thái ngoại vi
    pub peripherals: Option<PeripheralDelta>,

    /// Thay đổi calibration sample
    pub calibration: Option<CalibrationDelta>,

    /// Cập nhật bộ đếm
    pub dosing_cycle_count_increment: bool,

    /// Reset stabilizer tracker
    pub reset_stabilizer: bool,

    /// Cập nhật last_water_change_sec
    pub last_water_change_sec: Option<u64>,

    /// Cập nhật next_water_change_trigger_sec
    pub next_water_change_trigger_sec: Option<Option<u64>>,

    /// Cập nhật water_change_cron
    pub water_change_cron: Option<String>,

    /// Cập nhật stage hiện tại của crop recipe.
    pub current_stage_index: Option<Option<usize>>,

    /// Đánh dấu recipe đã hoàn tất.
    pub recipe_completed: Option<bool>,

    /// Mốc lần cuối recipe engine được kiểm tra (wall-clock seconds).
    pub last_recipe_check_sec: Option<u64>,

    /// Recipe execution identity: ID
    pub active_recipe_id: Option<Option<String>>,

    /// Recipe execution identity: revision
    pub active_recipe_revision: Option<Option<u64>>,

    /// Xóa các budget/history an toàn khi reset lỗi thủ công
    pub reset_safety_budget: bool,

    /// Reset sub-actors (dosing, water) và xoá ownership/chiếm dụng ngoại vi
    pub reset_active_actors: bool,

    /// Cập nhật safety override timeout
    pub safety_override_until: Option<u64>,

    /// Set manual timeout cho một bơm cụ thể: (pump_name, finish_ms)
    pub manual_pump_timeout: Option<(String, u64)>,

    /// Xóa manual timeout cho một bơm
    pub manual_pump_timeout_clear: Option<String>,

    pub previous_phase: Option<hydragrow_shared::fsm::SystemPhase>,

    // Để tính duration THỰC TẾ
    pub phase_start_before: Option<u64>,
}

#[derive(Debug, Default, Clone)]
pub struct PeripheralDelta {
    pub pump_a: Option<bool>,
    pub pump_b: Option<bool>,
    pub ph_up: Option<bool>,
    pub ph_down: Option<bool>,
    pub water_pump_in: Option<bool>,
    pub water_pump_out: Option<bool>,
    pub mist_valve: Option<bool>,
    pub mix_valve: Option<bool>,
    pub osaka_pump: Option<bool>,
    pub osaka_pwm: Option<u32>,
    pub is_misting_active: Option<bool>,
    pub is_scheduled_mixing_active: Option<bool>,
    /// Do có 2 nguồn có thể kích hoạt phun sương (theo lịch trình, MIMO solver) \
    /// trường này biểu thị cho việc van phun sương đang bị chiếm dụng bời MimoDosingPhase (bao gồm cả việc điều khiển phun sương để giảm nhiệt độ) \
    /// TÓM LẠI: ĐANG DÙNG CHO CHẾ ĐỘ ĐẶT BIỆT, CẤM ĐỤNG!!!
    pub misting_started_by_dosing: Option<bool>,
    pub mix_valve_started_by_dosing: Option<bool>,
    pub last_mist_toggle_time: Option<u64>,
    pub last_mixing_start_sec: Option<u64>,
    pub last_ec_before_dose: Option<Option<f32>>,
    pub last_ph_before_dose: Option<Option<f32>>,
    pub previous_ec: Option<Option<f32>>,
    pub previous_ph: Option<Option<f32>>,
    pub last_continuous_level: Option<bool>,
    pub water_pump_started_uptime_ms: Option<Option<u64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CalibrationDelta {
    /// Bắt đầu thu thập sample mới
    Start(PendingCalibrationSample),
    /// Xóa sample hiện tại (invalid)
    Invalidate,
    /// Xóa toàn bộ pending sample
    Clear,
    /// Cập nhật post-mixing EC/pH khi chuyển từ ActiveMixing → Stabilizing
    UpdatePostMixing { ec: f32, ph: f32, finish_ms: u64 },
}

impl ContextDelta {
    pub fn merge_from(&mut self, addition: ContextDelta) {
        if addition.phase.is_some() {
            self.phase = addition.phase;
        }
        if addition.phase_start_ms.is_some() {
            self.phase_start_ms = addition.phase_start_ms;
        }
        if addition.phase_finish_ms.is_some() {
            self.phase_finish_ms = addition.phase_finish_ms;
        }
        if let Some(mut addition_peri) = addition.peripherals {
            if addition_peri.water_pump_in == Some(true)
                && addition_peri.water_pump_out == Some(true)
            {
                addition_peri.water_pump_in = Some(false);
                addition_peri.water_pump_out = Some(false);
            }
            match &mut self.peripherals {
                Some(base_peri) => base_peri.merge_from(addition_peri),
                None => self.peripherals = Some(addition_peri),
            }
        }
        if addition.calibration.is_some() {
            self.calibration = addition.calibration;
        }
        if addition.dosing_cycle_count_increment {
            self.dosing_cycle_count_increment = true;
        }
        if addition.reset_stabilizer {
            self.reset_stabilizer = true;
        }
        if addition.last_water_change_sec.is_some() {
            self.last_water_change_sec = addition.last_water_change_sec;
        }
        if addition.next_water_change_trigger_sec.is_some() {
            self.next_water_change_trigger_sec = addition.next_water_change_trigger_sec;
        }
        if addition.water_change_cron.is_some() {
            self.water_change_cron = addition.water_change_cron;
        }
        if addition.current_stage_index.is_some() {
            self.current_stage_index = addition.current_stage_index;
        }
        if addition.recipe_completed.is_some() {
            self.recipe_completed = addition.recipe_completed;
        }
        if addition.last_recipe_check_sec.is_some() {
            self.last_recipe_check_sec = addition.last_recipe_check_sec;
        }
        if addition.active_recipe_id.is_some() {
            self.active_recipe_id = addition.active_recipe_id;
        }
        if addition.active_recipe_revision.is_some() {
            self.active_recipe_revision = addition.active_recipe_revision;
        }
        if addition.reset_safety_budget {
            self.reset_safety_budget = true;
        }
        if addition.reset_active_actors {
            self.reset_active_actors = true;
        }
        if addition.safety_override_until.is_some() {
            self.safety_override_until = addition.safety_override_until;
        }
        if addition.manual_pump_timeout.is_some() {
            self.manual_pump_timeout = addition.manual_pump_timeout;
        }
        if addition.manual_pump_timeout_clear.is_some() {
            self.manual_pump_timeout_clear = addition.manual_pump_timeout_clear;
        }
        if addition.previous_phase.is_some() {
            self.previous_phase = addition.previous_phase;
        }
        if addition.phase_start_before.is_some() {
            self.phase_start_before = addition.phase_start_before;
        }
    }
}

impl PeripheralDelta {
    pub fn merge_from(&mut self, addition: PeripheralDelta) {
        if addition.pump_a.is_some() {
            self.pump_a = addition.pump_a;
        }
        if addition.pump_b.is_some() {
            self.pump_b = addition.pump_b;
        }
        if addition.ph_up.is_some() {
            self.ph_up = addition.ph_up;
        }
        if addition.ph_down.is_some() {
            self.ph_down = addition.ph_down;
        }
        if addition.water_pump_in == Some(true) && addition.water_pump_out == Some(true) {
            // Conflicting simultaneous assertion: reject both for safety
            self.water_pump_in = Some(false);
            self.water_pump_out = Some(false);
        } else if addition.water_pump_in == Some(true) {
            self.water_pump_in = Some(true);
            self.water_pump_out = Some(false);
        } else if addition.water_pump_out == Some(true) {
            self.water_pump_out = Some(true);
            self.water_pump_in = Some(false);
        } else {
            if addition.water_pump_in.is_some() {
                self.water_pump_in = addition.water_pump_in;
            }
            if addition.water_pump_out.is_some() {
                self.water_pump_out = addition.water_pump_out;
            }
        }
        if self.water_pump_in == Some(true) && self.water_pump_out == Some(true) {
            self.water_pump_in = Some(false);
            self.water_pump_out = Some(false);
        }
        if addition.osaka_pump.is_some() {
            self.osaka_pump = addition.osaka_pump;
        }
        if addition.osaka_pwm.is_some() {
            self.osaka_pwm = addition.osaka_pwm;
        }
        if addition.is_misting_active.is_some() {
            self.is_misting_active = addition.is_misting_active;
        }
        if addition.is_scheduled_mixing_active.is_some() {
            self.is_scheduled_mixing_active = addition.is_scheduled_mixing_active;
        }
        if addition.last_mist_toggle_time.is_some() {
            self.last_mist_toggle_time = addition.last_mist_toggle_time;
        }
        if addition.last_mixing_start_sec.is_some() {
            self.last_mixing_start_sec = addition.last_mixing_start_sec;
        }
        if addition.last_ec_before_dose.is_some() {
            self.last_ec_before_dose = addition.last_ec_before_dose;
        }
        if addition.last_ph_before_dose.is_some() {
            self.last_ph_before_dose = addition.last_ph_before_dose;
        }
        if addition.previous_ec.is_some() {
            self.previous_ec = addition.previous_ec;
        }
        if addition.previous_ph.is_some() {
            self.previous_ph = addition.previous_ph;
        }
        if addition.last_continuous_level.is_some() {
            self.last_continuous_level = addition.last_continuous_level;
        }
        if addition.water_pump_started_uptime_ms.is_some() {
            self.water_pump_started_uptime_ms = addition.water_pump_started_uptime_ms;
        }

        // Valve Ownership & Conflict Resolution:
        // Dosing ownership strictly wins over ambient/scheduled.
        match (
            self.misting_started_by_dosing,
            addition.misting_started_by_dosing,
        ) {
            (_, Some(true)) => {
                self.misting_started_by_dosing = Some(true);
                if addition.mist_valve.is_some() {
                    self.mist_valve = addition.mist_valve;
                }
            }
            (Some(true), Some(false)) => {
                // Base dosing ownership retains priority
            }
            _ => {
                if addition.mist_valve.is_some() {
                    self.mist_valve = addition.mist_valve;
                }
                if addition.misting_started_by_dosing.is_some() {
                    self.misting_started_by_dosing = addition.misting_started_by_dosing;
                }
            }
        }

        match (
            self.mix_valve_started_by_dosing,
            addition.mix_valve_started_by_dosing,
        ) {
            (_, Some(true)) => {
                self.mix_valve_started_by_dosing = Some(true);
                if addition.mix_valve.is_some() {
                    self.mix_valve = addition.mix_valve;
                }
            }
            (Some(true), Some(false)) => {
                // Base dosing ownership retains priority
            }
            _ => {
                if addition.mix_valve.is_some() {
                    self.mix_valve = addition.mix_valve;
                }
                if addition.mix_valve_started_by_dosing.is_some() {
                    self.mix_valve_started_by_dosing = addition.mix_valve_started_by_dosing;
                }
            }
        }
    }
}

/// Giao dịch an toàn cần commit sau khi phần cứng dispatch thành công.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SafetyTransaction {
    /// Cấp nước: (uptime_sec, max_refill_cycles_per_hour)
    pub refill: Option<(u64, u32)>,
    /// Xả nước: (uptime_sec, max_drain_cycles_per_hour)
    pub drain: Option<(u64, u32)>,
    /// Dosing: danh sách (pump_name, uptime_sec, amount_ml)
    pub hourly_doses: Vec<(String, u64, f32)>,
}

impl SafetyTransaction {
    pub fn is_empty(&self) -> bool {
        self.refill.is_none() && self.drain.is_none() && self.hourly_doses.is_empty()
    }

    pub fn commit(&self, safety: &mut crate::core::actors::safety_guard::SafetyGuard) {
        if let Some((uptime_sec, max)) = self.refill {
            safety.record_refill(uptime_sec, max);
        }
        if let Some((uptime_sec, max)) = self.drain {
            safety.record_drain(uptime_sec, max);
        }
        for (pump_name, uptime_sec, ml) in &self.hourly_doses {
            safety.commit_hourly_dose(pump_name, *uptime_sec, *ml);
        }
    }
}

/// Output của một tick Pure Decision Engine.
#[derive(Debug, Default)]
pub struct TickResult {
    /// Những thay đổi cần áp dụng vào SystemContext
    pub delta: ContextDelta,

    /// Các side effect cần thực thi (hardware, MQTT, NVS...)
    pub events: Vec<OrchestratorEvent>,

    /// Giao dịch an toàn (chỉ commit khi hardware dispatch thành công)
    pub safety_transaction: Option<SafetyTransaction>,
}
