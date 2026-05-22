use hydragrow_shared::{ControllerConfig, SensorData};

use crate::{
    fsm::OrchestratorEvent,
    pump::{PumpController, WaterDirection},
};

#[derive(Debug, Clone)]
pub enum WaterSubState {
    Idle,
    Filling { job: WaterJob },
    Draining { job: WaterJob },
}

#[derive(Debug, Clone)]
pub struct WaterJob {
    pub trigger: String,
    pub target_level: f32,
    pub start_level: f32,
    pub start_ec: f32,
    pub start_ms: u64,
}

#[must_use]
pub enum WaterEvent {
    Pending,
    Done { success: bool, duration_sec: u64 },
}

pub struct WaterActor {
    pub sub_state: WaterSubState,
    pub retry_refill: u32,
}

impl WaterActor {
    pub fn new() -> Self {
        Self {
            sub_state: WaterSubState::Idle,
            retry_refill: 0,
        }
    }

    pub fn start_fill(&mut self, now_ms: u64, target: f32, sensors: &SensorData, trigger: &str) {
        self.sub_state = WaterSubState::Filling {
            job: WaterJob {
                trigger: trigger.into(),
                target_level: target,
                start_level: sensors.water_level,
                start_ec: sensors.ec,
                start_ms: now_ms,
            },
        };
        self.retry_refill = 0;
    }

    pub fn start_drain(&mut self, now_ms: u64, target: f32, sensors: &SensorData, trigger: &str) {
        self.sub_state = WaterSubState::Draining {
            job: WaterJob {
                trigger: trigger.into(),
                target_level: target,
                start_level: sensors.water_level,
                start_ec: sensors.ec,
                start_ms: now_ms,
            },
        };
    }

    pub fn tick(
        &mut self,
        now_ms: u64,
        sensors: &SensorData,
        config: &ControllerConfig,
    ) -> (WaterEvent, Vec<OrchestratorEvent>) {
        match &self.sub_state.clone() {
            WaterSubState::Filling { job } => {
                let elapsed = now_ms.saturating_sub(job.start_ms) / 1000;
                let reached = sensors.water_level >= job.target_level;
                let timeout = elapsed > config.max_refill_duration_sec as u64;

                if reached || timeout {
                    self.sub_state = WaterSubState::Idle;
                    return (
                        WaterEvent::Done {
                            success: reached,
                            duration_sec: elapsed,
                        },
                        vec![OrchestratorEvent::SetWaterPump {
                            direction: crate::pump::WaterDirection::Stop,
                        }],
                    );
                }

                (
                    WaterEvent::Pending,
                    vec![OrchestratorEvent::SetWaterPump {
                        direction: crate::pump::WaterDirection::In,
                    }],
                )
            }
            WaterSubState::Draining { job } => {
                let elapsed = now_ms.saturating_sub(job.start_ms) / 1000;
                let reached = sensors.water_level <= job.target_level;
                let timeout = elapsed > config.max_drain_duration_sec as u64;

                if reached || timeout {
                    self.sub_state = WaterSubState::Idle;
                    return (
                        WaterEvent::Done {
                            success: reached,
                            duration_sec: elapsed,
                        },
                        vec![OrchestratorEvent::SetWaterPump {
                            direction: crate::pump::WaterDirection::Stop,
                        }],
                    );
                }

                (
                    WaterEvent::Pending,
                    vec![OrchestratorEvent::SetWaterPump {
                        direction: crate::pump::WaterDirection::Out,
                    }],
                )
            }
            WaterSubState::Idle => (
                WaterEvent::Pending,
                vec![OrchestratorEvent::SetWaterPump {
                    direction: crate::pump::WaterDirection::Stop,
                }],
            ),
        }
    }
}
