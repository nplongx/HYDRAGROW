use hydragrow_shared::fsm::SystemPhase;
// src/fsm/phases/cooldown.rs
use hydragrow_shared::{ControllerConfig, SensorData};

use crate::fsm::phase_tick::PhaseTick;
use crate::fsm::system_context::SystemContext;
use crate::fsm::tick_result::TickResult;

pub struct CooldownPhase;

impl PhaseTick for CooldownPhase {
    fn tick(
        &self,
        now_ms: u64,
        _config: &ControllerConfig,
        _sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();

        if now_ms >= ctx.phase_finish_ms.unwrap_or(0) {
            result.delta.phase = Some(SystemPhase::Monitoring);
            result.delta.phase_start_ms = Some(None);
            result.delta.phase_finish_ms = Some(None);
        }

        result
    }
}
