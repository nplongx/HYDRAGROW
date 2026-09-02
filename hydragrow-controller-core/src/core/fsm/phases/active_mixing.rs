use hydragrow_shared::fsm::SystemPhase;
// src/fsm/phase_impls/active_mixing.rs
use hydragrow_shared::{ControllerConfig, SensorData};

use crate::core::fsm::tick_result::CalibrationDelta::UpdatePostMixing;
use crate::core::fsm::{PhaseTick, SystemContext, TickResult};

pub struct ActiveMixingPhase;

impl PhaseTick for ActiveMixingPhase {
    fn tick(
        &self,
        _now_ms: u64, // Không dùng now_ms nữa
        uptime: u64,  // [VÁ BUG]: Dùng uptime để tính toán
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();

        // 1. Tính toán mốc thời gian an toàn
        let elapsed_ms = uptime.saturating_sub(ctx.phase_start_ms.unwrap_or(uptime));
        let max_mixing_timeout = uptime >= ctx.phase_finish_ms.unwrap_or(u64::MAX);

        if (elapsed_ms >= 15_000 && ctx.stabilizer_tracker.is_stable(config)) || max_mixing_timeout
        {
            if ctx.peripherals.mix_valve_started_by_dosing {
                result
                    .events
                    .push(crate::core::fsm::events::OrchestratorEvent::SetMixValve { on: false });
                let mut peri_delta = result.delta.peripherals.take().unwrap_or_default();
                peri_delta.mix_valve = Some(false);
                peri_delta.mix_valve_started_by_dosing = Some(false);
                result.delta.peripherals = Some(peri_delta);
            }

            result.delta.phase = Some(SystemPhase::Stabilizing);

            // 2. [VÁ BUG] Thiết lập mốc thời gian tương lai bằng UPTIME
            result.delta.phase_start_ms = Some(Some(uptime));
            result.delta.phase_finish_ms = Some(Some(
                uptime + ctx.diagnostic.adaptive_stabilize_sec as u64 * 1000,
            ));
            result.delta.reset_stabilizer = true;

            result.delta.calibration = Some(UpdatePostMixing {
                ec: sensors.ec,
                ph: sensors.ph,
                finish_ms: uptime, // Phải truyền uptime để StabilizingPhase tính toán delta chính xác
            });
        }

        result
    }
}
