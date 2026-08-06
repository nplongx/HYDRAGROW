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

    /// Xóa các budget/history an toàn khi reset lỗi thủ công
    pub reset_safety_budget: bool,

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
    pub last_mist_toggle_time: Option<u64>,
    pub last_mixing_start_sec: Option<u64>,
    pub last_ec_before_dose: Option<Option<f32>>,
    pub last_ph_before_dose: Option<Option<f32>>,
    pub previous_ec: Option<Option<f32>>,
    pub previous_ph: Option<Option<f32>>,
    pub last_continuous_level: Option<bool>,
}

#[derive(Debug, Clone)]
pub enum CalibrationDelta {
    /// Bắt đầu thu thập sample mới
    Start(PendingCalibrationSample),
    /// Xóa sample hiện tại (invalid)
    Invalidate,
    /// Cập nhật post-mixing EC/pH khi chuyển từ ActiveMixing → Stabilizing
    UpdatePostMixing { ec: f32, ph: f32, finish_ms: u64 },
}

/// Output của một tick Pure Decision Engine.
#[derive(Debug, Default)]
pub struct TickResult {
    /// Những thay đổi cần áp dụng vào SystemContext
    pub delta: ContextDelta,

    /// Các side effect cần thực thi (hardware, MQTT, NVS...)
    pub events: Vec<OrchestratorEvent>,
}
