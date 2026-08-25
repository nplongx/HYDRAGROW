//! OrchestratorEvent — Toàn bộ side effect của FSM được biểu diễn qua enum này.
//! Không có logic nào ở đây. Đây là "ngôn ngữ" giao tiếp giữa Pure Logic và Hardware.

use crate::hw::{PumpType, WaterDirection};

/// Tất cả các hành động phần cứng và I/O mà FSM có thể yêu cầu.
/// Dispatcher sẽ translate enum này thành lệnh thực tế.
#[derive(Debug, Clone)]
#[must_use]
pub enum OrchestratorEvent {
    // --- HARDWARE: Bơm định lượng ---
    SetDosingPump {
        pump: DosingPumpTarget,
        on: bool,
        pwm_percent: u32,
    },

    // --- HARDWARE: Bơm nước ---
    SetWaterPump {
        direction: WaterDirection,
    },

    // --- HARDWARE: Van phun sương ---
    SetMistValve {
        on: bool,
    },

    // --- HARDWARE: Van trộn ---
    SetMixValve {
        on: bool,
    },

    // --- HARDWARE: Bơm Osaka (sục trộn) ---
    SetOsakaPump {
        pwm_percent: u32,
    },
    StartOsakaSoft {
        target_pwm_percent: u32,
    },

    // --- PERSISTENCE: NVS Flash ---
    SaveNvsSnapshot,
    SaveLastWaterChange {
        timestamp_sec: u64,
    },
    SaveCurrentStageIndex {
        stage_index: Option<usize>,
    },

    // --- MESSAGING: MQTT ---
    PublishFsmState,
    PublishCalibrationUpdate,
    PublishDosingReport {
        report_json: String,
    },
    PublishSystemLog {
        payload_json: String,
    },
    PublishRecipeStageChanged {
        payload_json: String,
    },

    // --- CONTROL FLOW: Sensor node ---
    RequestSensorForcePublish,
    SetSensorContinuousMode {
        enabled: bool,
    },
    PublishFsmTransition {
        from_phase: hydragrow_shared::fsm::SystemPhase,
        to_phase: hydragrow_shared::fsm::SystemPhase,
        reason: hydragrow_shared::telemetry::transition::TransitionReason,
        phase_duration_ms: Option<u64>,
    },
    PublishDosingCycle {
        cycle_json: String,
    },

    TriggerOtaUpdate,
    UpdateWifiList {
        list: hydragrow_shared::WifiCredentialList,
    },

    /// Reboot thiết bị ngay lập tức (sau khi dừng hardware).
    RebootDevice,

    /// Xoá toàn bộ NVS: recipe + wifi_list + safety_budget, sau đó reboot.
    FactoryReset,
}

/// Target bơm định lượng (tách riêng để tránh dùng PumpType từ pump.rs ở layer này)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DosingPumpTarget {
    NutrientA,
    NutrientB,
    PhUp,
    PhDown,
}

impl From<DosingPumpTarget> for PumpType {
    fn from(t: DosingPumpTarget) -> Self {
        match t {
            DosingPumpTarget::NutrientA => PumpType::NutrientA,
            DosingPumpTarget::NutrientB => PumpType::NutrientB,
            DosingPumpTarget::PhUp => PumpType::PhUp,
            DosingPumpTarget::PhDown => PumpType::PhDown,
        }
    }
}
