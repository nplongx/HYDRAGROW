// src/fsm/phase_tick.rs
//! PhaseTick — Interface chung cho tất cả Phase trong FSM.
//! Mỗi Phase nhận input read-only, trả về TickResult.
//! KHÔNG được mutate bất cứ thứ gì bên ngoài TickResult.

use hydragrow_shared::{ControllerConfig, SensorData};

use crate::core::fsm::{SystemContext, TickResult};

/// Trait buộc mỗi Phase phải pure: chỉ nhận input, chỉ trả TickResult.
pub trait PhaseTick {
    fn tick(
        &self,
        now_ms: u64,
        uptime_ms: u64,
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult;
}
