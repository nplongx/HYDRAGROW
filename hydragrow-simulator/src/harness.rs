use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::dispatcher::SimDispatcher;
use crate::plant::tank::Tank;
use crate::sensors::sensor_model::{NoiseConfig, read_sensor};
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
            uptime_ms: 0,
        }
    }

    pub fn uptime_ms(&self) -> u64 {
        self.uptime_ms
    }

    pub fn tick(&mut self, dt_ms: u64) -> TickResult {
        self.tank.step(dt_ms, &self.hw, &self.config);
        let sensor = read_sensor(&self.tank, &self.noise);

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
            now_ms, // use now_ms as last update time to avoid sensor timeout
            &mut self.ctx,
        );

        for event in &result.events {
            self.dispatcher.dispatch(event, &mut self.hw);
        }

        result
    }
}
