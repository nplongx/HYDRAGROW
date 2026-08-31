use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::dispatcher::SimDispatcher;
use crate::faults::injector::Injector;
use crate::plant::tank::Tank;
use crate::sensors::sensor_model::{read_sensor, NoiseConfig};
use hydragrow_controller_core::{
    core::fsm::tick_result::TickResult,
    core::fsm::{context::SystemContext, orchestrator},
};
use hydragrow_shared::ControllerConfig;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Harness {
    pub config: ControllerConfig,
    pub ctx: SystemContext,
    pub hw: VirtualHardwareState,
    pub dispatcher: SimDispatcher,
    pub tank: Tank,
    pub noise: NoiseConfig,
    pub injector: Injector,
    uptime_ms: u64,
}

impl Harness {
    pub fn new(config: ControllerConfig, tank: Tank, noise: NoiseConfig) -> Self {
        Self {
            config,
            ctx: SystemContext::default(),
            hw: VirtualHardwareState::default(),
            dispatcher: SimDispatcher::new(),
            tank,
            noise,
            injector: Injector::new(),
            uptime_ms: 0,
        }
    }

    pub fn uptime_ms(&self) -> u64 {
        self.uptime_ms
    }

    pub fn tick(&mut self, dt_ms: u64) -> TickResult {
        self.injector.apply_hardware_faults(&mut self.hw);
        self.tank.step(dt_ms, &self.hw, &self.config);
        let mut sensor = read_sensor(&self.tank, &self.noise);
        self.injector.apply_sensor_faults(&mut sensor);

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.uptime_ms += dt_ms;

        let result = orchestrator::tick(
            now_ms,
            self.uptime_ms,
            &self.config,
            &sensor,
            now_ms,
            &mut self.ctx,
        );

        for event in &result.events {
            self.dispatcher.dispatch(event);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plant::tank::Tank;
    use crate::sensors::sensor_model::NoiseConfig;
    use hydragrow_shared::ControllerConfig;

    #[test]
    fn test_harness_single_tick() {
        let config = ControllerConfig::default();
        let tank = Tank::default();
        let noise = NoiseConfig::default();

        let mut harness = Harness::new(config, tank, noise);

        let delta_ms = 100;
        harness.tick(delta_ms);

        assert_eq!(harness.uptime_ms(), 100);
    }
}
