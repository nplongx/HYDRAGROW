use hydragrow_shared::ControllerConfig;

use crate::pump::PumpController;

use super::super::phases::FaultCode;

#[derive(Debug, Clone)]
pub enum DosingSubState {
    Idle,
    SoftStarting { finish_ms: u64 },
    PumpingA(PulseJob),
    WaitingAtoB { finish_ms: u64, b_job: PulseJob },
    PumpingB(PulseJob),
    PumpingPH(PulseJob),
}

#[derive(Debug, Clone)]
pub struct PulseJob {
    pub pump: PumpTarget,
    pub target_ml: f32,
    pub delivered_ml: f32,
    pub pulse_on: bool,
    pub pulse_count: u32,
    pub max_pulses: u32,
    pub on_ms: u64,
    pub off_ms: u64,
    pub pwm: u32,
    pub ml_per_sec: f32,
    pub next_toggle_ms: u64,
}

#[derive(Debug, Clone)]
pub enum PumpTarget {
    NutrientA { dose_b_ml: f32 },
    NutrientB,
    PhUp,
    PhDown,
}

#[derive(Debug, Clone)]
pub struct DosingCycleCtx {
    pub cycle_id: String,
    pub trigger: String,
    pub start_ec: f32,
    pub start_ph: f32,
    pub target_ec: f32,
    pub target_ph: f32,
    pub start_water_level: f32,
    pub start_ms: u64,
    pub post_mixing_ec: f32,
    pub post_mixing_ph: f32,
}

#[must_use]
pub enum DosingEvent {
    Pending,
    PulseComplete { pump: PumpTarget, delivered_ml: f32 },
    CycleComplete,
    Failed(FaultCode),
}

pub struct DosingActor {
    pub sub_state: DosingSubState,
    pub cycle_ctx: Option<DosingCycleCtx>,
    pub retry_ec: u8,
    pub retry_ph: u8,
}

impl DosingActor {
    pub fn new() -> Self {
        Self { sub_state: DosingSubState::Idle, cycle_ctx: None, retry_ec: 0, retry_ph: 0 }
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.sub_state, DosingSubState::Idle)
    }

    pub fn tick(&mut self, now_ms: u64, _config: &ControllerConfig, _pumps: &mut PumpController) -> DosingEvent {
        match &self.sub_state {
            DosingSubState::WaitingAtoB { finish_ms, b_job } if now_ms >= *finish_ms => {
                self.sub_state = DosingSubState::PumpingB(b_job.clone());
                DosingEvent::Pending
            }
            _ => DosingEvent::Pending,
        }
    }
}
