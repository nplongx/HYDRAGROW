use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::dispatcher::SimDispatcher;
use crate::faults::injector::Injector;
use crate::telemetry::recorder::Recorder;
use hydragrow_controller_core::{
    core::fsm::{orchestrator, SystemContext, TickResult},
};
use hydragrow_shared::{ControllerConfig, SensorData};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Harness {
    pub config: ControllerConfig,
    pub ctx: SystemContext,
    pub hw: VirtualHardwareState,
    pub dispatcher: SimDispatcher,
    pub injector: Injector,
    pub recorder: Option<Recorder>,
    uptime_ms: u64,
}

impl Harness {
    pub fn new(config: ControllerConfig) -> Self {
        Self {
            config,
            ctx: SystemContext::default(),
            hw: VirtualHardwareState::default(),
            dispatcher: SimDispatcher::new(),
            injector: Injector::new(),
            recorder: None,
            uptime_ms: 0,
        }
    }

    pub fn uptime_ms(&self) -> u64 {
        self.uptime_ms
    }

    pub fn tick(&mut self, dt_ms: u64, mut sensor: SensorData) -> TickResult {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.uptime_ms += dt_ms;

        // 1. Apply sensor faults before feeding to orchestrator
        self.injector.apply_sensor_faults(&mut sensor);

        // 2. FSM Tick
        let mut result = orchestrator::tick(
            now_ms,
            self.uptime_ms,
            &self.config,
            &sensor,
            self.uptime_ms,
            &mut self.ctx,
        );

        // 3. Dispatch hardware events
        for event in &result.events {
            self.dispatcher.dispatch(event, &mut self.hw);
        }

        // 4. Apply hardware faults after dispatcher
        self.injector.apply_hardware_faults(&mut self.hw);

        // 5. Update FSM Context
        self.ctx.apply_delta(&mut result.delta);

        // 6. Record telemetry
        if let Some(recorder) = &mut self.recorder {
            recorder.record(
                self.uptime_ms,
                &format!("{:?}", self.ctx.phase),
                sensor.ec,
                sensor.ph,
                sensor.temp,
                sensor.water_level,
                self.hw.pump_a.on,
                self.hw.pump_b.on,
            );
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_shared::{ControllerConfig, SensorData, PumpStatus};

    #[test]
    fn test_harness_single_tick() {
        let config = ControllerConfig::default();
        let sensor = SensorData {
            device_id: "test".to_string(),
            temp: 25.0,
            water_level: 50.0,
            ec: 1.0,
            ph: 6.0,
            err_ec: Some(false),
            err_ph: Some(false),
            err_temp: Some(false),
            time: "".to_string(),
            pump_status: PumpStatus::default(),
            controller_received_ms: None,
            rssi: None,
            free_heap: None,
            uptime: None,
            err_water: None,
            is_continuous: None,
            ph_voltage_mv: None,
        };
        let mut harness = Harness::new(config);

        let delta_ms = 100;
        harness.tick(delta_ms, sensor);
        assert_eq!(harness.uptime_ms(), 100);
    }
}
