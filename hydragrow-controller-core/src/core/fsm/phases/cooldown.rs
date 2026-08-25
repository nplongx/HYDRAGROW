// src/core/fsm/phases/cooldown.rs

use crate::core::fsm::{PhaseTick, SystemContext, TickResult};
use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::{ControllerConfig, SensorData};

pub struct CooldownPhase;

impl PhaseTick for CooldownPhase {
    fn tick(
        &self,
        _now_ms: u64,
        uptime: u64, // [VÁ BUG]: Dùng uptime để so sánh
        _config: &ControllerConfig,
        _sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();

        // Kiểm tra timeout bằng uptime
        if uptime >= ctx.phase_finish_ms.unwrap_or(u64::MAX) {
            result.delta.phase = Some(SystemPhase::Monitoring);
            result.delta.phase_start_ms = Some(None);
            result.delta.phase_finish_ms = Some(None);
        }

        result
    }
}
