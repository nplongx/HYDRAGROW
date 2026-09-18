use crate::actuators::virtual_hw::VirtualHardwareState;
use crate::clock::VirtualClock;
use crate::controller::VirtualController;
use crate::dispatcher::SimDispatcher;
use crate::faults::injector::Injector;
use crate::plant::tank::Tank;
use crate::scenario::engine::ScenarioEngine;
use crate::scenario::format::{Scenario, load_scenario};
use crate::sensors::sensor_model::{NoiseConfig, read_sensor};
use crate::telemetry::mqtt_bridge::{InboundMqttMessage, MqttBridge};
use crate::telemetry::recorder::Recorder;
use anyhow::{Context, Result};
use hydragrow_controller_core::{
    core::fsm::tick_result::TickResult,
    core::fsm::{context::SystemContext, orchestrator},
};
use hydragrow_shared::telemetry::DeviceHealthSnapshot;
use hydragrow_shared::{ControllerConfig, SensorData};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

pub type SimClock = VirtualClock;

pub struct HarnessOutputs {
    pub recorder: Option<Recorder>,
    pub mqtt: Option<MqttBridge>,
}

pub struct Harness {
    pub controller: VirtualController,
    pub config: ControllerConfig,
    pub ctx: SystemContext,
    pub hw: VirtualHardwareState,
    pub dispatcher: SimDispatcher,
    pub tank: Tank,
    pub noise: NoiseConfig,
    pub rng: StdRng,
    pub injector: Injector,
    pub clock: SimClock,
    pub sensor_last_update_ms: u64,
    pub last_sensor: SensorData,
    pub scenario_engine: Option<ScenarioEngine>,
    pub outputs: Option<HarnessOutputs>,
    pub device_id: String,
    pub configuration_available: bool,
    pub applied_config_version: i64,
    pending_telemetry: Vec<(u64, SensorData)>,
    pending_inbound_mqtt: Vec<(u64, InboundMqttMessage)>,
    last_published_telemetry: Option<SensorData>,
    last_health_publish_ms: u64,
    processed_command_ids: HashSet<String>,
    processed_command_order: VecDeque<String>,
}

pub struct HarnessBuilder {
    config: ControllerConfig,
    tank: Tank,
    noise: NoiseConfig,
    device_id: String,
    mqtt_broker: Option<String>,
    record_path: Option<PathBuf>,
    scenario: Option<Scenario>,
    wall_clock_ms: Option<u64>,
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
            wall_clock_ms: None,
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

    pub fn wall_clock_ms(mut self, now_ms: u64) -> Self {
        self.wall_clock_ms = Some(now_ms);
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
        let clock = self.wall_clock_ms.map(SimClock::new).unwrap_or_default();
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
            controller_received_ms: Some(clock.now_ms()),
            rssi: None,
            free_heap: None,
            uptime: Some(clock.uptime_ms() as u32),
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

        let rng = StdRng::seed_from_u64(self.noise.seed);

        Ok(Harness {
            controller: VirtualController::new(),
            config: self.config,
            ctx,
            hw: VirtualHardwareState::default(),
            dispatcher: SimDispatcher::new(),
            tank: self.tank,
            noise: self.noise,
            rng,
            injector: Injector::new(),
            clock,
            sensor_last_update_ms: clock.now_ms(),
            last_sensor: initial_sensor,
            scenario_engine,
            outputs,
            device_id: self.device_id,
            configuration_available: true,
            applied_config_version: 0,
            pending_telemetry: Vec::new(),
            pending_inbound_mqtt: Vec::new(),
            last_published_telemetry: None,
            last_health_publish_ms: 0,
            processed_command_ids: HashSet::new(),
            processed_command_order: VecDeque::new(),
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
        self.clock.uptime_ms()
    }

    pub fn controller_restart(&mut self) {
        self.controller.restart();
        self.ctx = SystemContext::default();
        self.ctx.phase = hydragrow_shared::fsm::SystemPhase::Monitoring;
        self.hw = VirtualHardwareState::default();
        self.clock.restart();
        self.sensor_last_update_ms = self.clock.now_ms();
        self.configuration_available = true;
        self.last_health_publish_ms = 0;
    }

    fn process_inbound_mqtt(&mut self) {
        if self.injector.mqtt_disconnected() || self.injector.mqtt_drop() {
            return;
        }
        let mut inbound = Vec::new();
        if let Some(outputs) = self.outputs.as_ref()
            && let Some(mqtt) = outputs.mqtt.as_ref()
        {
            while let Ok(message) = mqtt.try_recv() {
                if let Some(delay_ms) = self.injector.mqtt_delay_ms() {
                    self.pending_inbound_mqtt.push((
                        self.clock.uptime_ms().saturating_add(delay_ms),
                        message.clone(),
                    ));
                    if self.injector.mqtt_duplicate() {
                        self.pending_inbound_mqtt
                            .push((self.clock.uptime_ms().saturating_add(delay_ms), message));
                    }
                } else {
                    inbound.push(message.clone());
                    if self.injector.mqtt_duplicate() {
                        inbound.push(message);
                    }
                }
            }
        }
        let now = self.clock.uptime_ms();
        self.pending_inbound_mqtt.retain(|(due, message)| {
            if *due <= now {
                inbound.push(message.clone());
                false
            } else {
                true
            }
        });

        let command_topic = hydragrow_shared::topics::topic_controller_command(&self.device_id);
        let config_topic = hydragrow_shared::topics::topic_controller_config(&self.device_id);
        let (cmd_tx, cmd_rx) = channel();
        let (fsm_mqtt_tx, _fsm_mqtt_rx) = channel::<String>();

        for message in inbound {
            if message.topic == command_topic {
                if let Some(mqtt) = self.outputs.as_ref().and_then(|o| o.mqtt.as_ref()) {
                    match mqtt.decode_command(&message.payload) {
                        Ok(command) => {
                            tracing::debug!(
                                device_id = %self.device_id,
                                action = %command.action,
                                command_id = ?command.metadata.as_ref().and_then(|m| m.command_id.as_deref()),
                                "Twin decoded MQTT command"
                            );
                            let command_id = command
                                .metadata
                                .as_ref()
                                .and_then(|metadata| metadata.command_id.clone());
                            let duplicate = command_id
                                .as_ref()
                                .is_some_and(|id| self.processed_command_ids.contains(id));
                            if !duplicate {
                                if let Some(id) = command_id {
                                    self.processed_command_ids.insert(id.clone());
                                    self.processed_command_order.push_back(id);
                                    while self.processed_command_order.len() > 32 {
                                        if let Some(old) = self.processed_command_order.pop_front()
                                        {
                                            self.processed_command_ids.remove(&old);
                                        }
                                    }
                                }
                                let _ = cmd_tx.send(command);
                            }
                        }
                        Err(error) => {
                            tracing::warn!(device_id = %self.device_id, ?error, "Twin rejected invalid MQTT command");
                        }
                    }
                }
            } else if message.topic == config_topic
                && let Some(mqtt) = self.outputs.as_ref().and_then(|o| o.mqtt.as_ref())
            {
                match mqtt.decode_config(&message.payload) {
                    Ok(config) => match config.validate() {
                        Ok(()) => {
                            if let Ok(value) =
                                serde_json::from_slice::<serde_json::Value>(&message.payload)
                                && let Some(version) =
                                    value.get("config_version").and_then(|v| v.as_i64())
                            {
                                self.applied_config_version = version;
                            }
                            self.config = config;
                        }
                        Err(errors) => {
                            tracing::warn!(?errors, "Twin rejected invalid controller config")
                        }
                    },
                    Err(error) => {
                        tracing::warn!(?error, "Twin rejected invalid controller config JSON")
                    }
                }
            }
        }

        let (delta, events) =
            hydragrow_controller_core::runtime::command_handler::process_mqtt_commands(
                &cmd_rx,
                &self.config,
                &self.ctx,
                self.clock.uptime_ms(),
                self.clock.now_ms(),
                &fsm_mqtt_tx,
            );
        if !events.is_empty() {
            tracing::debug!(device_id = %self.device_id, event_count = events.len(), "Twin processed controller-core events");
        }
        let mut delta = delta;
        self.ctx.apply_delta(&mut delta);
        for event in &events {
            self.dispatcher.dispatch(event, &mut self.hw);
        }
        if let Some(outputs) = self.outputs.as_mut()
            && let Some(mqtt) = outputs.mqtt.as_mut()
        {
            for event in &events {
                if let hydragrow_controller_core::core::fsm::events::OrchestratorEvent::PublishCommandLifecycle {
                        command_id,
                        lifecycle,
                        reason,
                    } = event
                    {
                        mqtt.publish_command_lifecycle(
                            command_id.clone(),
                            *lifecycle,
                            reason.clone(),
                            self.clock.now_ms(),
                        );
                        if self.injector.mqtt_duplicate() {
                            mqtt.publish_command_lifecycle(
                                command_id.clone(),
                                *lifecycle,
                                reason.clone(),
                                self.clock.now_ms(),
                            );
                        }
                    }
                mqtt.publish_event(event, &self.ctx, self.clock.uptime_ms());
                if self.injector.mqtt_duplicate() {
                    mqtt.publish_event(event, &self.ctx, self.clock.uptime_ms());
                }
            }
            // Publish the canonical controller snapshot after command processing so
            // backend confirmation observes actual controller-core state. No Twin-only
            // topic or confirmation path is introduced.
            mqtt.publish_event(
                &hydragrow_controller_core::core::fsm::events::OrchestratorEvent::PublishFsmState,
                &self.ctx,
                self.clock.uptime_ms(),
            );
            if self.injector.mqtt_duplicate() {
                mqtt.publish_event(
                        &hydragrow_controller_core::core::fsm::events::OrchestratorEvent::PublishFsmState,
                        &self.ctx,
                        self.clock.uptime_ms(),
                    );
            }

            // Mirror the physical controller's authoritative health channel.
            // Keep the 10s cadence, but include the current pump_status as an
            // additive field because the backend uses it for command confirmation.
            let now = self.clock.uptime_ms();
            if now.saturating_sub(self.last_health_publish_ms) >= 10_000 {
                let health = DeviceHealthSnapshot {
                    device_id: self.device_id.clone(),
                    free_heap: 0,
                    uptime_sec: now / 1000,
                    rssi: 0,
                    health_score_percent: 100,
                    fsm_state_display: format!("{:?}", self.ctx.phase),
                    log_drop_count: 0,
                    firmware_version: "digital-twin".to_string(),
                    kalman_confidence: None,
                    matrix_update_count: 0,
                    matrix_is_warm: false,
                    hestia: None,
                    timestamp_ms: self.clock.now_ms(),
                };
                if let Ok(mut payload) = serde_json::to_value(health) {
                    payload["config_version"] = serde_json::json!(self.applied_config_version);
                    payload["pump_status"] =
                        serde_json::to_value(&self.ctx.peripherals.pump_status)
                            .unwrap_or(serde_json::Value::Null);
                    if let Ok(bytes) = serde_json::to_vec(&payload) {
                        mqtt.publish_raw_controller_status(bytes);
                    }
                }
                self.last_health_publish_ms = now;
            }
        }
    }

    pub fn tick(&mut self, dt_ms: u64) -> Result<TickResult> {
        let previous_ms = self.clock.uptime_ms();
        self.clock.advance(dt_ms);

        let activated_faults = if let Some(engine) = self.scenario_engine.as_mut() {
            engine.activate_between(previous_ms, self.clock.uptime_ms())
        } else {
            Vec::new()
        };
        for fault in activated_faults {
            let is_restart = matches!(
                fault,
                crate::scenario::format::FaultEventKind::ControllerRestart
            );
            let is_clock_jump = matches!(
                fault,
                crate::scenario::format::FaultEventKind::ClockJump { .. }
            );
            let clock_jump = match &fault {
                crate::scenario::format::FaultEventKind::ClockJump { delta_ms } => Some(*delta_ms),
                _ => None,
            };
            self.injector
                .add_active_fault_at(fault, self.clock.uptime_ms());
            if is_restart {
                self.controller_restart();
                self.applied_config_version = 0;
            }
            if is_clock_jump && let Some(delta_ms) = clock_jump {
                self.clock.jump_wall_clock(delta_ms);
            }
        }

        if self.injector.configuration_lost() {
            self.configuration_available = false;
        }

        // Publish one authoritative status before consuming retained config.
        // After restart this exposes runtime config loss (revision 0), allowing
        // the backend ConfigurationSync worker to reconcile desired state.
        if self.last_health_publish_ms == 0 {
            if let Some(mqtt) = self.outputs.as_mut().and_then(|o| o.mqtt.as_mut()) {
                let health = DeviceHealthSnapshot {
                    device_id: self.device_id.clone(),
                    free_heap: 0,
                    uptime_sec: self.clock.uptime_ms() / 1000,
                    rssi: 0,
                    health_score_percent: 100,
                    fsm_state_display: format!("{:?}", self.ctx.phase),
                    log_drop_count: 0,
                    firmware_version: "digital-twin".to_string(),
                    kalman_confidence: None,
                    matrix_update_count: 0,
                    matrix_is_warm: false,
                    hestia: None,
                    timestamp_ms: self.clock.now_ms(),
                };
                if let Ok(mut payload) = serde_json::to_value(health) {
                    payload["config_version"] = serde_json::json!(self.applied_config_version);
                    payload["pump_status"] =
                        serde_json::to_value(&self.ctx.peripherals.pump_status)
                            .unwrap_or(serde_json::Value::Null);
                    if let Ok(bytes) = serde_json::to_vec(&payload) {
                        mqtt.publish_raw_controller_status(bytes);
                    }
                }
            }
            self.last_health_publish_ms = self.clock.uptime_ms();
        }

        self.process_inbound_mqtt();
        if let Some(mqtt) = self.outputs.as_ref().and_then(|o| o.mqtt.as_ref()) {
            if mqtt.is_connected() && !self.injector.mqtt_disconnected() {
                self.controller.connecting();
                self.controller.connect();
                self.controller.running();
            } else if !matches!(
                self.controller.lifecycle,
                crate::controller::ControllerLifecycle::Boot
            ) {
                self.controller.disconnect();
            }
        }

        self.injector
            .apply_hardware_faults_at(&mut self.hw, self.clock.uptime_ms());
        if self.injector.boot_loop() || !self.configuration_available {
            self.controller.degrade();
            self.hw = VirtualHardwareState::default();
        }
        self.tank.step(dt_ms, &self.hw, &self.config);
        let mut sensor = read_sensor(&self.tank, &self.noise, &mut self.rng);
        sensor.device_id = self.device_id.clone();
        sensor.time = chrono::DateTime::from_timestamp_millis(self.clock.now_ms() as i64)
            .map(|ts| ts.to_rfc3339())
            .unwrap_or_else(|| "1970-01-01T00:00:00+00:00".to_string());
        sensor.uptime = Some(self.clock.uptime_ms() as u32);
        sensor.controller_received_ms = Some(self.clock.uptime_ms());
        sensor.pump_status = hydragrow_shared::PumpStatus {
            pump_a: self.hw.pump_a.on,
            pump_b: self.hw.pump_b.on,
            ph_up: self.hw.pump_ph_up.on,
            ph_down: self.hw.pump_ph_down.on,
            osaka_pump: self.hw.osaka_pwm_percent > 0,
            mist_valve: self.hw.mist_valve,
            mix_valve: self.hw.mix_valve,
            water_pump_in: self.hw.water_pump_in.on,
            water_pump_out: self.hw.water_pump_out.on,
            pump_a_pwm: Some(self.hw.pump_a.pwm_percent as u32),
            pump_b_pwm: Some(self.hw.pump_b.pwm_percent as u32),
            ph_up_pwm: Some(self.hw.pump_ph_up.pwm_percent as u32),
            ph_down_pwm: Some(self.hw.pump_ph_down.pwm_percent as u32),
            osaka_pwm: Some(self.hw.osaka_pwm_percent as u32),
            dosing_pulse_active: None,
            dosing_pulse_count: None,
        };
        self.injector.apply_sensor_faults(&mut sensor);

        self.sensor_last_update_ms = self.clock.now_ms();

        let mut result = orchestrator::tick(
            self.clock.now_ms(),
            self.clock.uptime_ms(),
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
                let telemetry_blocked = self.injector.mqtt_disconnected()
                    || self.injector.mqtt_drop()
                    || self.injector.telemetry_paused();
                if !telemetry_blocked {
                    if let Some(delay_ms) = self
                        .injector
                        .telemetry_delay_ms()
                        .or_else(|| self.injector.mqtt_delay_ms())
                    {
                        self.pending_telemetry.push((
                            self.clock.uptime_ms().saturating_add(delay_ms),
                            sensor.clone(),
                        ));
                    } else if self.injector.telemetry_out_of_order() {
                        mqtt.publish_sensors(&sensor);
                        if let Some(previous) = self.last_published_telemetry.as_ref() {
                            mqtt.publish_sensors(previous);
                        }
                    } else {
                        mqtt.publish_sensors(&sensor);
                        if self.injector.mqtt_duplicate() {
                            mqtt.publish_sensors(&sensor);
                        }
                    }

                    let now = self.clock.uptime_ms();
                    let mut ready = Vec::new();
                    self.pending_telemetry.retain(|(due, sample)| {
                        if *due <= now {
                            ready.push(sample.clone());
                            false
                        } else {
                            true
                        }
                    });
                    for sample in ready {
                        mqtt.publish_sensors(&sample);
                    }
                    self.last_published_telemetry = Some(sensor.clone());
                }
                for event in &result.events {
                    if !telemetry_blocked {
                        mqtt.publish_event(event, &self.ctx, self.clock.uptime_ms());
                        if self.injector.mqtt_duplicate() {
                            mqtt.publish_event(event, &self.ctx, self.clock.uptime_ms());
                        }
                    }
                }
            }
            if let Some(recorder) = outputs.recorder.as_mut() {
                recorder.record(
                    self.clock.uptime_ms(),
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
