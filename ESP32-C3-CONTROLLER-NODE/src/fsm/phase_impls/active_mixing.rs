// src/fsm/phase_impls/active_mixing.rs
use hydragrow_shared::{ControllerConfig, SensorData};

use crate::fsm::phase_impls::SystemPhase;
use crate::fsm::phase_tick::PhaseTick;
use crate::fsm::system_context::SystemContext;
use crate::fsm::tick_result::TickResult;

pub struct ActiveMixingPhase;

impl PhaseTick for ActiveMixingPhase {
    fn tick(
        &self,
        now_ms: u64,
        config: &ControllerConfig,
        _sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();
        let elapsed_ms = now_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(now_ms));
        let max_mixing_timeout = now_ms >= ctx.phase_finish_ms.unwrap_or(0);

        if (elapsed_ms >= 15_000 && ctx.stabilizer_tracker.is_stable(config)) || max_mixing_timeout
        {
            result.delta.phase = Some(SystemPhase::Stabilizing);
            result.delta.phase_start_ms = Some(Some(now_ms));
            result.delta.phase_finish_ms = Some(Some(
                now_ms + ctx.diagnostic.adaptive_stabilize_sec as u64 * 1000,
            ));
            result.delta.reset_stabilizer = true;
        }

        result
    }
}
