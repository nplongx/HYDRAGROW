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
        uptime_ms: u64, // SỬA
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();

        match ctx.water.sub_state {
            WaterSubState::Idle => {
                ctx.water.sub_state = WaterSubState::Starting;
                ctx.water.start_fill(
                    uptime_ms, // SỬA: Đưa uptime vào để canh thời gian an toàn
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

        // SỬA: Dùng uptime để chạy tick
        let (event, hw_events, sys_log) = ctx.water.tick(uptime_ms, sensors, config);
        result.events.extend(hw_events);
        
        result.events.extend(sys_log.into_iter().filter_map(|mut log| {
            // THỦ THUẬT: Đè lại timestamp bằng giờ thực tế (now_ms) để Log hiển thị đúng ngày giờ
            log.timestamp_ms = now_ms;
            
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
        uptime_ms: u64, // SỬA
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();

        match ctx.water.sub_state {
            WaterSubState::Idle => {
                ctx.water.sub_state = WaterSubState::Starting;
                ctx.water.start_drain(
                    uptime_ms, // SỬA: Dùng uptime
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

        // SỬA: Dùng uptime để chạy tick
        let (event, hw_events, sys_log) = ctx.water.tick(uptime_ms, sensors, config);
        result.events.extend(hw_events);
        
        result.events.extend(sys_log.into_iter().filter_map(|mut log| {
            // THỦ THUẬT TƯƠNG TỰ: Vá lại timestamp trước khi đẩy lên MQTT
            log.timestamp_ms = now_ms;
            
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