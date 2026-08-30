use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::dispatcher::SimDispatcher;
use hydragrow_controller_core::core::fsm::{SystemContext, TickResult, orchestrator};
use hydragrow_shared::{ControllerConfig, SensorData};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Harness {
    pub config: ControllerConfig,
    pub ctx: SystemContext,
    pub hw: VirtualHardwareState,
    pub dispatcher: SimDispatcher,
    uptime_ms: u64,
}

impl Harness {
    pub fn new(config: ControllerConfig) -> Self {
        Self {
            config,
            ctx: SystemContext::default(),
            hw: VirtualHardwareState::default(),
            dispatcher: SimDispatcher::new(),
            uptime_ms: 0,
        }
    }

    pub fn uptime_ms(&self) -> u64 {
        self.uptime_ms
    }

    pub fn tick(&mut self, dt_ms: u64, sensor: SensorData) -> TickResult {
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
            self.uptime_ms, // pass uptime_ms as sensor_last_update_ms
            &mut self.ctx,
        );

        for event in &result.events {
            self.dispatcher.dispatch(event, &mut self.hw);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_shared::{ControllerConfig, SensorData};

    #[test]
    fn test_harness_single_tick() {
        let config = ControllerConfig::default();
        let sensor = SensorData {
            device_id: "test_dev".to_string(),
            ec: 1.0,
            ph: 6.0,
            temp: 25.0,
            water_level: 50.0,
            pump_status: Default::default(),
            time: "".to_string(),
            controller_received_ms: None,
            rssi: None,
            free_heap: None,
            uptime: None,
            err_water: None,
            err_temp: None,
            err_ph: None,
            err_ec: None,
            is_continuous: None,
            ph_voltage_mv: None,
        };
        let mut harness = Harness::new(config);

        let delta_ms = 100;
        harness.tick(delta_ms, sensor);
        assert_eq!(harness.uptime_ms(), 100);
    }
}
