use hydragrow_shared::fsm::SystemPhase;
// src/fsm/phase_impls/water_phases.rs
use hydragrow_shared::{ControllerConfig, SensorData};

use crate::fsm::actors::water_actor::WaterEvent;
use crate::fsm::events::OrchestratorEvent;
use crate::fsm::phase_tick::PhaseTick;
use crate::fsm::system_context::SystemContext;
use crate::fsm::tick_result::TickResult;

pub struct WaterRefillingPhase;

impl PhaseTick for WaterRefillingPhase {
    fn tick(
        &self,
        now_ms: u64,
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();
        let (event, hw_events) = ctx.water.tick(now_ms, sensors, config);
        result.events.extend(hw_events);

        if let WaterEvent::Done {
            success,
            duration_sec,
        } = event
        {
            if !success {
                log::warn!("⚠️ WaterRefilling: timeout sau {}s", duration_sec);
            }
            // Về Monitoring sau khi hoàn tất
            result.delta.phase = Some(SystemPhase::Monitoring);
            result.delta.phase_start_ms = Some(None);
            result.delta.phase_finish_ms = Some(None);
            result.events.push(OrchestratorEvent::SaveNvsSnapshot);
        }

        result
    }
}

pub struct WaterDrainingPhase;

impl PhaseTick for WaterDrainingPhase {
    fn tick(
        &self,
        now_ms: u64,
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();
        let (event, hw_events) = ctx.water.tick(now_ms, sensors, config);
        result.events.extend(hw_events);

        if let WaterEvent::Done {
            success,
            duration_sec,
        } = event
        {
            if !success {
                log::warn!("⚠️ WaterDraining: timeout sau {}s", duration_sec);
            }
            result.delta.phase = Some(SystemPhase::Monitoring);
            result.delta.phase_start_ms = Some(None);
            result.delta.phase_finish_ms = Some(None);
            result.events.push(OrchestratorEvent::SaveNvsSnapshot);
        }

        result
    }
}
