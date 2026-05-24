// hydragrow-shared/src/telemetry/transition.rs
use crate::fsm::{FaultCode, SystemPhase};
use serde::{Deserialize, Serialize};

/// Lý do FSM chuyển phase — typed, không phải string tự do
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TransitionReason {
    /// Khởi động xong, chuyển vào Monitoring
    BootComplete,

    /// Chu kỳ MIMO kết thúc, chuyển sang ActiveMixing
    DosingComplete {
        dose_a_ml: f32,
        dose_b_ml: f32,
        ph_up_ml: f32,
        ph_down_ml: f32,
    },

    /// ActiveMixing xong (EC/pH đã flat), chuyển sang Stabilizing
    MixingComplete {
        /// Thời gian thực tế khuấy (ms)
        actual_mixing_ms: u64,
    },

    /// Stabilizing xong, chuyển sang Cooldown
    StabilizingComplete {
        final_ec: f32,
        final_ph: f32,
        actual_stabilize_ms: u64,
    },

    /// Cooldown timer hết, quay về Monitoring
    CooldownExpired,

    /// Cấp nước xong (đạt target hoặc timeout)
    WaterRefillComplete {
        success: bool,
        duration_sec: u64,
        final_level: f32,
    },

    /// Xả nước xong
    WaterDrainComplete {
        success: bool,
        duration_sec: u64,
        final_level: f32,
    },

    /// Sensor timeout — không nhận được data > 90s
    SensorTimeout { last_seen_ms_ago: u64 },

    /// Phát hiện lỗi phần cứng, chuyển vào Fault
    FaultDetected {
        fault_code: FaultCode,
        consecutive_failures: u32,
    },

    /// Người dùng reset fault, quay về Monitoring
    FaultReset,

    /// Người dùng vào chế độ Calibration
    EnterCalibration,

    /// Thoát chế độ Calibration
    ExitCalibration,

    /// Lý do khác (emergency stop, force từ user,...)
    Manual { description: String },
}

/// Event FSM transition — gửi qua MQTT mỗi khi phase thay đổi
/// Topic: `AGITECH/{device_id}/fsm/transition`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsmTransitionEvent {
    pub device_id: String,
    pub from_phase: Option<SystemPhase>,
    pub to_phase: SystemPhase,
    pub reason: TransitionReason,
    /// Timestamp epoch milliseconds — phải được truyền từ firmware (không tự tính trong shared)
    pub timestamp_ms: u64,
    /// Duration thực tế ở phase trước (ms) — optional
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_duration_ms: Option<u64>,
}

/// Builder cho `FsmTransitionEvent` — enforce mandatory fields tại compile time
pub struct FsmTransitionEventBuilder {
    device_id: Option<String>,
    from_phase: Option<SystemPhase>,
    to_phase: Option<SystemPhase>,
    reason: Option<TransitionReason>,
    timestamp_ms: Option<u64>,
    phase_duration_ms: Option<u64>,
}

impl FsmTransitionEvent {
    pub fn builder() -> FsmTransitionEventBuilder {
        FsmTransitionEventBuilder {
            device_id: None,
            from_phase: None,
            to_phase: None,
            reason: None,
            timestamp_ms: None,
            phase_duration_ms: None,
        }
    }
}

impl FsmTransitionEventBuilder {
    pub fn device_id(mut self, id: impl Into<String>) -> Self {
        self.device_id = Some(id.into());
        self
    }

    pub fn from(mut self, phase: SystemPhase) -> Self {
        self.from_phase = Some(phase);
        self
    }

    pub fn to(mut self, phase: SystemPhase) -> Self {
        self.to_phase = Some(phase);
        self
    }

    pub fn reason(mut self, reason: TransitionReason) -> Self {
        self.reason = Some(reason);
        self
    }

    pub fn timestamp_ms(mut self, ts: u64) -> Self {
        self.timestamp_ms = Some(ts);
        self
    }

    pub fn phase_duration_ms(mut self, duration: u64) -> Self {
        self.phase_duration_ms = Some(duration);
        self
    }

    /// Panics nếu thiếu mandatory field — chỉ dùng trong firmware (embedded panic = reset)
    pub fn build(self) -> FsmTransitionEvent {
        FsmTransitionEvent {
            device_id: self.device_id.expect("device_id is required"),
            from_phase: self.from_phase.expect("from_phase is required"),
            to_phase: self.to_phase.expect("to_phase is required"),
            reason: self.reason.expect("reason is required"),
            timestamp_ms: self.timestamp_ms.expect("timestamp_ms is required"),
            phase_duration_ms: self.phase_duration_ms,
        }
    }

    /// Trả về Result — dùng trong backend/test khi không muốn panic
    pub fn try_build(self) -> Result<FsmTransitionEvent, &'static str> {
        Ok(FsmTransitionEvent {
            device_id: self.device_id.ok_or("device_id is required")?,
            from_phase: self.from_phase.ok_or("from_phase is required")?,
            to_phase: self.to_phase.ok_or("to_phase is required")?,
            reason: self.reason.ok_or("reason is required")?,
            timestamp_ms: self.timestamp_ms.ok_or("timestamp_ms is required")?,
            phase_duration_ms: self.phase_duration_ms,
        })
    }
}
