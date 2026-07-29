// src/core/fsm/phases/water_phases.rs
use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::{ControllerConfig, SensorData};

use crate::core::actors::water_actor::{WaterEvent, WaterSubState};
use crate::core::fsm::context::SystemContext;
use crate::core::fsm::events::OrchestratorEvent;
use crate::core::fsm::phase_tick::PhaseTick;
use crate::core::fsm::tick_result::TickResult;

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

        match ctx.water.sub_state {
            WaterSubState::Idle => {
                ctx.water.sub_state = WaterSubState::Starting;
                ctx.water.start_fill(
                    now_ms,
                    config.water_level_target,
                    sensors,
                    "water_refill_phase",
                );
            }
            WaterSubState::Starting => {
                log::warn!("[WATER] Trạng thái Starting bất thường — start_fill chưa được gọi?");
            }
            _ => {}
        }

        let (event, hw_events, sys_log) = ctx.water.tick(now_ms, sensors, config);
        result.events.extend(hw_events);
        result.events.extend(sys_log.into_iter().filter_map(|log| {
            serde_json::to_string(&log)
                .ok()
                .map(|payload_json| OrchestratorEvent::PublishSystemLog { payload_json })
        }));

        if let WaterEvent::Done { success, duration_sec } = event {
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
                ctx.water.sub_state = WaterSubState::Starting;
                ctx.water.start_drain(
                    now_ms,
                    config.water_level_target,
                    sensors,
                    "water_drain_phase",
                );
            }
            WaterSubState::Starting => {
                log::warn!("[WATER] Trạng thái Starting bất thường — start_drain chưa được gọi?");
            }
            _ => {}
        }

        let (event, hw_events, sys_log) = ctx.water.tick(now_ms, sensors, config);
        result.events.extend(hw_events);
        result.events.extend(sys_log.into_iter().filter_map(|log| {
            serde_json::to_string(&log)
                .ok()
                .map(|payload_json| OrchestratorEvent::PublishSystemLog { payload_json })
        }));

        if let WaterEvent::Done { success, duration_sec } = event {
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