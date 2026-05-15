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

pub struct WaterActor {
    pub sub_state: WaterSubState,
    pub retry_refill: u8,
}

impl WaterActor {
    pub fn new() -> Self {
        Self { sub_state: WaterSubState::Idle, retry_refill: 0 }
    }
}
