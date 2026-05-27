// src/fsm/phase_tick.rs
//! PhaseTick — Interface chung cho tất cả Phase trong FSM.
//! Mỗi Phase nhận input read-only, trả về TickResult.
//! KHÔNG được mutate bất cứ thứ gì bên ngoài TickResult.

use crate::fsm::system_context::SystemContext;
use crate::fsm::tick_result::TickResult;
use hydragrow_shared::{ControllerConfig, SensorData};

/// Trait buộc mỗi Phase phải pure: chỉ nhận input, chỉ trả TickResult.
pub trait PhaseTick {
    fn tick(
        &self,
        now_ms: u64,
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult;
}
