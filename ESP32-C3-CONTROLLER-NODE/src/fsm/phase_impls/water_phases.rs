use hydragrow_shared::fsm::SystemPhase;
// src/fsm/phase_impls/water_phases.rs
use hydragrow_shared::{ControllerConfig, SensorData};

use crate::fsm::actors::water_actor::{WaterEvent, WaterSubState};
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

        // Khởi động WaterActor nếu chưa bắt đầu (chỉ gọi một lần khi vào phase)
        match ctx.water.sub_state {
            WaterSubState::Idle => {
                // Mark as starting before calling start_fill to prevent re-entry
                ctx.water.sub_state = WaterSubState::Starting;
                ctx.water.start_fill(
                    now_ms,
                    config.water_level_target,
                    sensors,
                    "water_refill_phase",
                );
            }
            WaterSubState::Starting => {
                // start_fill sets sub_state to Filling immediately, so this should never be reached
                // but guard against it anyway
                log::warn!("[WATER] Unexpected Starting state — start_fill was not called?");
            }
            _ => {} // Filling or Draining — normal tick below
        }

        let (event, hw_events, sys_log) = ctx.water.tick(now_ms, sensors, config);
        result.events.extend(hw_events);

        if let WaterEvent::Done {
            success,
            duration_sec,
        } = event
        {
            if !success {
                log::warn!("⚠️ WaterRefilling: timeout sau {}s", duration_sec);
            }
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

        match ctx.water.sub_state {
            WaterSubState::Idle => {
                // Mark as starting before calling start_fill to prevent re-entry
                ctx.water.sub_state = WaterSubState::Starting;
                ctx.water.start_drain(
                    now_ms,
                    config.water_level_target,
                    sensors,
                    "water_drain_phase",
                );
            }
            WaterSubState::Starting => {
                // start_fill sets sub_state to Filling immediately, so this should never be reached
                // but guard against it anyway
                log::warn!("[WATER] Unexpected Starting state — start_drain was not called?");
            }
            _ => {} // Filling or Draining — normal tick below
        }

        let (event, hw_events, sys_log) = ctx.water.tick(now_ms, sensors, config);
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
