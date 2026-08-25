//! Test utility functions

pub mod fixtures;

use hydragrow_controller_core::core::fsm::{context::SystemContext, events::OrchestratorEvent};
use hydragrow_shared::fsm::SystemPhase;

/// Kiểm tra trong danh sách events có event nào match predicate không
pub fn has_event<F>(events: &[OrchestratorEvent], predicate: F) -> bool
where
    F: Fn(&OrchestratorEvent) -> bool,
{
    events.iter().any(predicate)
}

/// Advance FSM nhiều tick với sensor data cố định, trả về events cuối cùng
pub fn tick_n_times(
    ctx: &mut SystemContext,
    config: &hydragrow_shared::ControllerConfig,
    sensors: &hydragrow_shared::SensorData,
    n: usize,
    start_uptime_ms: u64,
    tick_interval_ms: u64,
) -> Vec<OrchestratorEvent> {
    let mut last_events = vec![];
    for i in 0..n {
        let uptime_ms = start_uptime_ms + (i as u64 * tick_interval_ms);
        let now_ms = uptime_ms + 1_700_000_000_000; // Wall clock offset
        let result = hydragrow_controller_core::core::fsm::orchestrator::tick(
            now_ms,
            uptime_ms,
            config,
            sensors,
            uptime_ms,
            ctx,
        );
        ctx.apply_delta(&mut result.delta.clone());
        last_events = result.events;
    }
    last_events
}

/// Assert phase của context
pub fn assert_phase(ctx: &SystemContext, expected: &SystemPhase) {
    assert_eq!(
        &ctx.phase, expected,
        "Expected phase {:?} but got {:?}",
        expected, ctx.phase
    );
}
