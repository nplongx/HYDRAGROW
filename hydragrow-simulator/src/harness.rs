use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::dispatcher::SimDispatcher;
use crate::faults::injector::Injector;
use crate::plant::tank::Tank;
use crate::scenario::engine::ScenarioEngine;
use crate::scenario::format::{Scenario, load_scenario};
use crate::sensors::sensor_model::{NoiseConfig, read_sensor};
use crate::telemetry::mqtt_bridge::MqttBridge;
use crate::telemetry::recorder::Recorder;
use anyhow::{Context, Result};
use hydragrow_controller_core::{
    core::fsm::tick_result::TickResult,
    core::fsm::{context::SystemContext, orchestrator},
};
use hydragrow_shared::{ControllerConfig, SensorData};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimClock {
    pub now_ms: u64,
    pub uptime_ms: u64,
}

impl Default for SimClock {
    fn default() -> Self {
        Self {
            now_ms: 1_700_000_000_000,
            uptime_ms: 0,
        }
    }
}

impl SimClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&mut self, dt_ms: u64) {
        self.now_ms = self.now_ms.saturating_add(dt_ms);
        self.uptime_ms = self.uptime_ms.saturating_add(dt_ms);
    }
}

pub struct HarnessOutputs {
    pub recorder: Option<Recorder>,
    pub mqtt: Option<MqttBridge>,
}

pub struct Harness {
    pub config: ControllerConfig,
    pub ctx: SystemContext,
    pub hw: VirtualHardwareState,
    pub dispatcher: SimDispatcher,
    pub tank: Tank,
    pub noise: NoiseConfig,
    pub injector: Injector,
    pub clock: SimClock,
    pub sensor_last_update_ms: u64,
    pub last_sensor: SensorData,
    pub scenario_engine: Option<ScenarioEngine>,
    pub outputs: Option<HarnessOutputs>,
    pub device_id: String,
}

pub struct HarnessBuilder {
    config: ControllerConfig,
    tank: Tank,
    noise: NoiseConfig,
    device_id: String,
    mqtt_broker: Option<String>,
    record_path: Option<PathBuf>,
    scenario: Option<Scenario>,
}

impl HarnessBuilder {
    pub fn new(config: ControllerConfig, tank: Tank) -> Self {
        Self {
            config,
            tank,
            noise: NoiseConfig::none(),
            device_id: "sim-dev".to_string(),
            mqtt_broker: None,
            record_path: None,
            scenario: None,
        }
    }

    pub fn noise(mut self, noise: NoiseConfig) -> Self {
        self.noise = noise;
        self
    }

    pub fn device_id(mut self, device_id: impl Into<String>) -> Self {
        self.device_id = device_id.into();
        self
    }

    pub fn mqtt(mut self, broker: Option<String>) -> Self {
        self.mqtt_broker = broker;
        self
    }

    pub fn record(mut self, path: Option<PathBuf>) -> Self {
        self.record_path = path;
        self
    }

    pub fn scenario(mut self, scenario: Scenario) -> Self {
        self.scenario = Some(scenario);
        self
    }

    pub fn build(self) -> Result<Harness> {
        let clock = SimClock::default();
        let recorder = if let Some(path) = &self.record_path {
            Some(Recorder::new(path.to_str().with_context(|| {
                format!("invalid record file path: {}", path.display())
            })?)?)
        } else {
            None
        };

        let mqtt = self
            .mqtt_broker
            .as_deref()
            .map(|broker| MqttBridge::new(&self.device_id, broker));

        let outputs = if recorder.is_some() || mqtt.is_some() {
            Some(HarnessOutputs { recorder, mqtt })
        } else {
            None
        };

        let scenario_engine = self.scenario.map(ScenarioEngine::new);

        let initial_sensor = SensorData {
            device_id: self.device_id.clone(),
            ec: self.tank.ec,
            ph: self.tank.ph,
            temp: self.tank.temp,
            water_level: self.tank.water_level,
            pump_status: Default::default(),
            time: "2026-01-01T00:00:00Z".to_string(),
            controller_received_ms: Some(clock.now_ms),
            rssi: None,
            free_heap: None,
            uptime: Some(clock.uptime_ms as u32),
            err_water: None,
            err_temp: None,
            err_ec: None,
            err_ph: None,
            is_continuous: None,
            ph_voltage_mv: None,
            ec_received_ms: None,
            ph_received_ms: None,
            temp_received_ms: None,
            water_received_ms: None,
        };

        let ctx = SystemContext {
            phase: hydragrow_shared::fsm::SystemPhase::Monitoring,
            ..Default::default()
        };

        Ok(Harness {
            config: self.config,
            ctx,
            hw: VirtualHardwareState::default(),
            dispatcher: SimDispatcher::new(),
            tank: self.tank,
            noise: self.noise,
            injector: Injector::new(),
            clock: clock.clone(),
            sensor_last_update_ms: clock.now_ms,
            last_sensor: initial_sensor,
            scenario_engine,
            outputs,
            device_id: self.device_id,
        })
    }
}

impl Harness {
    pub fn new(config: ControllerConfig, tank: Tank, noise: NoiseConfig) -> Self {
        Self::builder(config, tank).noise(noise).build().unwrap()
    }

    pub fn builder(config: ControllerConfig, tank: Tank) -> HarnessBuilder {
        HarnessBuilder::new(config, tank)
    }

    pub fn from_scenario<P: AsRef<Path>>(
        config: ControllerConfig,
        scenario_path: P,
    ) -> Result<Self> {
        let scenario = load_scenario(scenario_path.as_ref())?;
        let tank = Tank::from_initial(&scenario.initial_tank);
        Self::builder(config, tank).scenario(scenario).build()
    }

    pub fn uptime_ms(&self) -> u64 {
        self.clock.uptime_ms
    }

    pub fn tick(&mut self, dt_ms: u64) -> Result<TickResult> {
        let previous_ms = self.clock.uptime_ms;
        self.clock.advance(dt_ms);

        if let Some(engine) = self.scenario_engine.as_mut() {
            for fault in engine.activate_between(previous_ms, self.clock.uptime_ms) {
                self.injector.add_active_fault(fault);
            }
        }

        self.injector.apply_hardware_faults(&mut self.hw);
        self.tank.step(dt_ms, &self.hw, &self.config);
        let mut sensor = read_sensor(&self.tank, &self.noise);
        sensor.device_id = self.device_id.clone();
        self.injector.apply_sensor_faults(&mut sensor);

        self.sensor_last_update_ms = self.clock.now_ms;

        let mut result = orchestrator::tick(
            self.clock.now_ms,
            self.clock.uptime_ms,
            &self.config,
            &sensor,
            self.sensor_last_update_ms,
            &mut self.ctx,
        );
        self.ctx.apply_delta(&mut result.delta);

        for event in &result.events {
            self.dispatcher.dispatch(event, &mut self.hw);
        }
        if let Some(tx) = &result.safety_transaction {
            self.ctx.commit_safety_transaction(tx);
        }
        self.last_sensor = sensor.clone();

        if let Some(outputs) = self.outputs.as_mut() {
            if let Some(mqtt) = outputs.mqtt.as_mut() {
                mqtt.publish_sensors(&sensor);
                for event in &result.events {
                    mqtt.publish_event(event);
                }
            }
            if let Some(recorder) = outputs.recorder.as_mut() {
                recorder.record(
                    self.clock.uptime_ms,
                    &format!("{:?}", self.ctx.phase),
                    sensor.ec,
                    sensor.ph,
                    sensor.temp,
                    sensor.water_level,
                    self.hw.pump_a.on,
                    self.hw.pump_b.on,
                )?;
            }
        }

        Ok(result)
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
        harness.tick(delta_ms).unwrap();

        assert_eq!(harness.uptime_ms(), 100);
    }
}
