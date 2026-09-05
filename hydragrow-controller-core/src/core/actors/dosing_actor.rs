use crate::core::adaptive::matrix::ControlVector;
use crate::core::fsm::{DosingPumpTarget, OrchestratorEvent};
use crate::utils::{DosePumpKind, effective_flow_ml_per_sec};
use hydragrow_shared::fsm::FaultCode;
use hydragrow_shared::{ControllerConfig, SensorData};

#[derive(Debug, Clone, PartialEq)]
pub enum DosingSubState {
    Idle,
    SoftStarting {
        finish_ms: u64,
        next_state: Box<DosingSubState>,
    },
    /// Châm dinh dưỡng A theo mạch xung Pulse
    PumpingA(PulseJob),
    /// Trễ hòa trộn an toàn chuyển tiếp giữa hai bình chứa đậm đặc
    WaitingAtoB {
        finish_ms: u64,
        b_job: PulseJob,
    },
    /// Châm dinh dưỡng B theo mạch xung Pulse
    PumpingB(PulseJob),
    /// Châm hóa chất hiệu chỉnh pH (Up hoặc Down độc lập) song song/kế tiếp
    PumpingPH(PulseJob),
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PumpTarget {
    NutrientA { dose_b_ml: f32 },
    NutrientB,
    PhUp,
    PhDown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DosingPlanRejection {
    ZeroOrEmpty,
    FlowUnavailable(DosePumpKind),
    ConflictingPh,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedDoseJob {
    pub pump: DosePumpKind,
    pub target_ml: f32,
    pub pwm: u32,
}

#[must_use]
#[derive(Debug, Clone, PartialEq)]
pub enum DosingPlanResult {
    Prepared(Vec<PreparedDoseJob>),
    Rejected(DosingPlanRejection),
}

#[derive(Debug, Clone)]
pub struct DosingCycleCtx {
    pub dose_a_delivered_ml: f32,
    pub dose_b_delivered_ml: f32,
    pub ph_up_delivered_ml: f32,
    pub ph_down_delivered_ml: f32,
}

#[must_use]
#[derive(Debug, Clone, PartialEq)]
pub enum DosingEvent {
    Pending,
    SoftStartDone,
    PulseToggle {
        pump: PumpTarget,
        pulse_on: bool,
    },
    PhaseTransition,
    CycleComplete {
        dose_a_ml: f32,
        dose_b_ml: f32,
        ph_up_ml: f32,
        ph_down_ml: f32,
    },
    Failed(FaultCode),
}

pub struct DosingActor {
    pub sub_state: DosingSubState,
    pub cycle_ctx: Option<DosingCycleCtx>,
    pub retry_ec: u8,
    pub retry_ph: u8,
    /// Hàng đợi lưu trữ lệnh châm pH kế tiếp nếu chu kỳ này chạy phối hợp cả EC và pH
    pub pending_ph_job: Option<PulseJob>,
}

impl Default for DosingActor {
    fn default() -> Self {
        Self::new()
    }
}

impl DosingActor {
    pub fn new() -> Self {
        Self {
            sub_state: DosingSubState::Idle,
            cycle_ctx: None,
            retry_ec: 0,
            retry_ph: 0,
            pending_ph_job: None,
        }
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.sub_state, DosingSubState::Idle)
    }

    /// Reset hoàn toàn trạng thái DosingActor về Idle, xoá toàn bộ job đang chờ,
    /// bộ đếm thử lại và tiến độ phân phối.
    pub fn reset(&mut self) {
        self.sub_state = DosingSubState::Idle;
        self.cycle_ctx = None;
        self.retry_ec = 0;
        self.retry_ph = 0;
        self.pending_ph_job = None;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_matrix_cycle(
        &mut self,
        now_ms: u64,
        control: &ControlVector,
        _target_ec: f32,
        _target_ph: f32,
        pwm: u32,
        config: &ControllerConfig,
        _sensors: &SensorData,
    ) -> DosingPlanResult {
        let ph_up_active = control.ph_up_ml > 1e-3;
        let ph_down_active = control.ph_down_ml > 1e-3;

        // Mutual exclusion: both pH Up and pH Down cannot be active in the same cycle
        if ph_up_active && ph_down_active {
            self.reset();
            return DosingPlanResult::Rejected(DosingPlanRejection::ConflictingPh);
        }

        let a_active = control.nutrient_a_ml > 1e-3;
        let b_active = control.nutrient_b_ml > 1e-3;

        if !a_active && !b_active && !ph_up_active && !ph_down_active {
            self.reset();
            return DosingPlanResult::Rejected(DosingPlanRejection::ZeroOrEmpty);
        }

        let safe_pwm = pwm.clamp(1, 100);

        // Verify flow availability for all requested channels
        let flow_a = if a_active {
            match effective_flow_ml_per_sec(DosePumpKind::PumpA, safe_pwm, config) {
                Some(f) => Some(f),
                None => {
                    self.reset();
                    return DosingPlanResult::Rejected(DosingPlanRejection::FlowUnavailable(
                        DosePumpKind::PumpA,
                    ));
                }
            }
        } else {
            None
        };

        let flow_b = if b_active {
            match effective_flow_ml_per_sec(DosePumpKind::PumpB, safe_pwm, config) {
                Some(f) => Some(f),
                None => {
                    self.reset();
                    return DosingPlanResult::Rejected(DosingPlanRejection::FlowUnavailable(
                        DosePumpKind::PumpB,
                    ));
                }
            }
        } else {
            None
        };

        let flow_up = if ph_up_active {
            match effective_flow_ml_per_sec(DosePumpKind::PhUp, safe_pwm, config) {
                Some(f) => Some(f),
                None => {
                    self.reset();
                    return DosingPlanResult::Rejected(DosingPlanRejection::FlowUnavailable(
                        DosePumpKind::PhUp,
                    ));
                }
            }
        } else {
            None
        };

        let flow_down = if ph_down_active {
            match effective_flow_ml_per_sec(DosePumpKind::PhDown, safe_pwm, config) {
                Some(f) => Some(f),
                None => {
                    self.reset();
                    return DosingPlanResult::Rejected(DosingPlanRejection::FlowUnavailable(
                        DosePumpKind::PhDown,
                    ));
                }
            }
        } else {
            None
        };

        let mut prepared_jobs = Vec::new();
        if a_active {
            prepared_jobs.push(PreparedDoseJob {
                pump: DosePumpKind::PumpA,
                target_ml: control.nutrient_a_ml,
                pwm: safe_pwm,
            });
        }
        if b_active {
            prepared_jobs.push(PreparedDoseJob {
                pump: DosePumpKind::PumpB,
                target_ml: control.nutrient_b_ml,
                pwm: safe_pwm,
            });
        }
        if ph_up_active {
            prepared_jobs.push(PreparedDoseJob {
                pump: DosePumpKind::PhUp,
                target_ml: control.ph_up_ml,
                pwm: safe_pwm,
            });
        }
        if ph_down_active {
            prepared_jobs.push(PreparedDoseJob {
                pump: DosePumpKind::PhDown,
                target_ml: control.ph_down_ml,
                pwm: safe_pwm,
            });
        }

        self.cycle_ctx = Some(DosingCycleCtx {
            dose_a_delivered_ml: 0.0,
            dose_b_delivered_ml: 0.0,
            ph_up_delivered_ml: 0.0,
            ph_down_delivered_ml: 0.0,
        });

        self.pending_ph_job = None;
        if ph_up_active {
            let f = flow_up.unwrap();
            let (on_ms, off_ms, max_pulses) = pulse_params(control.ph_up_ml, f, config);
            self.pending_ph_job = Some(PulseJob {
                pump: PumpTarget::PhUp,
                target_ml: control.ph_up_ml,
                delivered_ml: 0.0,
                pulse_on: false,
                pulse_count: 0,
                max_pulses,
                on_ms,
                off_ms,
                pwm: safe_pwm,
                ml_per_sec: f,
                next_toggle_ms: now_ms,
            });
        } else if ph_down_active {
            let f = flow_down.unwrap();
            let (on_ms, off_ms, max_pulses) = pulse_params(control.ph_down_ml, f, config);
            self.pending_ph_job = Some(PulseJob {
                pump: PumpTarget::PhDown,
                target_ml: control.ph_down_ml,
                delivered_ml: 0.0,
                pulse_on: false,
                pulse_count: 0,
                max_pulses,
                on_ms,
                off_ms,
                pwm: safe_pwm,
                ml_per_sec: f,
                next_toggle_ms: now_ms,
            });
        }

        if a_active {
            let f = flow_a.unwrap();
            let (on_ms, off_ms, max_pulses) = pulse_params(control.nutrient_a_ml, f, config);
            self.sub_state = DosingSubState::SoftStarting {
                finish_ms: now_ms + config.soft_start_duration as u64,
                next_state: Box::new(DosingSubState::PumpingA(PulseJob {
                    pump: PumpTarget::NutrientA {
                        dose_b_ml: if b_active {
                            control.nutrient_b_ml
                        } else {
                            0.0
                        },
                    },
                    target_ml: control.nutrient_a_ml,
                    delivered_ml: 0.0,
                    pulse_on: false,
                    pulse_count: 0,
                    max_pulses,
                    on_ms,
                    off_ms,
                    pwm: safe_pwm,
                    ml_per_sec: f,
                    next_toggle_ms: now_ms,
                })),
            };
        } else if b_active {
            let f = flow_b.unwrap();
            let (on_ms, off_ms, max_pulses) = pulse_params(control.nutrient_b_ml, f, config);
            self.sub_state = DosingSubState::SoftStarting {
                finish_ms: now_ms + config.soft_start_duration as u64,
                next_state: Box::new(DosingSubState::PumpingB(PulseJob {
                    pump: PumpTarget::NutrientB,
                    target_ml: control.nutrient_b_ml,
                    delivered_ml: 0.0,
                    pulse_on: false,
                    pulse_count: 0,
                    max_pulses,
                    on_ms,
                    off_ms,
                    pwm: safe_pwm,
                    ml_per_sec: f,
                    next_toggle_ms: now_ms,
                })),
            };
        } else if let Some(ph_job) = self.pending_ph_job.take() {
            self.sub_state = DosingSubState::SoftStarting {
                finish_ms: now_ms + config.soft_start_duration as u64,
                next_state: Box::new(DosingSubState::PumpingPH(ph_job)),
            };
        } else {
            self.sub_state = DosingSubState::Idle;
        }

        DosingPlanResult::Prepared(prepared_jobs)
    }

    pub fn tick(
        &mut self,
        now_ms: u64,
        config: &ControllerConfig,
    ) -> (DosingEvent, Vec<OrchestratorEvent>) {
        match &self.sub_state.clone() {
            DosingSubState::SoftStarting {
                finish_ms,
                next_state,
            } if now_ms >= *finish_ms => {
                self.sub_state = *next_state.clone();
                (DosingEvent::SoftStartDone, vec![])
            }
            DosingSubState::PumpingA(job) if now_ms >= job.next_toggle_ms => {
                self.tick_pump_a(now_ms, config)
            }
            DosingSubState::WaitingAtoB { finish_ms, b_job } if now_ms >= *finish_ms => {
                self.begin_pump_b(b_job.clone(), now_ms)
            }
            DosingSubState::PumpingB(job) if now_ms >= job.next_toggle_ms => {
                self.tick_pump_b(now_ms, config)
            }
            DosingSubState::PumpingPH(job) if now_ms >= job.next_toggle_ms => {
                self.tick_pump_ph(now_ms)
            }
            _ => (DosingEvent::Pending, vec![]),
        }
    }

    fn tick_pump_a(
        &mut self,
        now_ms: u64,
        config: &ControllerConfig,
    ) -> (DosingEvent, Vec<OrchestratorEvent>) {
        let DosingSubState::PumpingA(mut job) = self.sub_state.clone() else {
            return (DosingEvent::Pending, vec![]);
        };
        let mut hw_events = Vec::new();

        if job.pulse_on {
            hw_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: false,
                pwm_percent: 0,
            });

            if job.delivered_ml + 1e-3 >= job.target_ml {
                let PumpTarget::NutrientA { dose_b_ml } = job.pump else {
                    return (DosingEvent::Failed(FaultCode::EcDosingFailed), hw_events);
                };
                if let Some(ctx) = self.cycle_ctx.as_mut() {
                    ctx.dose_a_delivered_ml = job.delivered_ml;
                }

                // Chi cham A
                if dose_b_ml <= 1e-3 {
                    return self.transition_to_ph_or_idle(job.delivered_ml, 0.0, now_ms);
                }

                let safe_pwm = job.pwm.clamp(1, 100);
                let ml_per_sec =
                    match effective_flow_ml_per_sec(DosePumpKind::PumpB, safe_pwm, config) {
                        Some(v) => v,
                        None => return (DosingEvent::Failed(FaultCode::EcDosingFailed), hw_events),
                    };
                let (on_ms, off_ms, max_pulses) = pulse_params(dose_b_ml, ml_per_sec, config);
                let b_job = PulseJob {
                    pump: PumpTarget::NutrientB,
                    target_ml: dose_b_ml,
                    delivered_ml: 0.0,
                    pulse_on: false,
                    pulse_count: 0,
                    max_pulses,
                    on_ms,
                    off_ms,
                    pwm: safe_pwm,
                    ml_per_sec,
                    next_toggle_ms: now_ms,
                };
                self.sub_state = DosingSubState::WaitingAtoB {
                    finish_ms: now_ms + (config.delay_between_a_and_b_sec as u64 * 1000),
                    b_job,
                };
                (DosingEvent::PhaseTransition, hw_events)
            } else if job.pulse_count >= job.max_pulses {
                self.sub_state = DosingSubState::Idle;
                (DosingEvent::Failed(FaultCode::EcDosingFailed), hw_events)
            } else {
                job.pulse_on = false;
                job.next_toggle_ms = now_ms + job.off_ms;
                self.sub_state = DosingSubState::PumpingA(job.clone());
                (
                    DosingEvent::PulseToggle {
                        pump: job.pump,
                        pulse_on: false,
                    },
                    hw_events,
                )
            }
        } else {
            hw_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: true,
                pwm_percent: job.pwm,
            });

            job.pulse_on = true;
            job.pulse_count += 1;
            job.delivered_ml += job.ml_per_sec * (job.on_ms as f32 / 1000.0);
            job.next_toggle_ms = now_ms + job.on_ms;
            self.sub_state = DosingSubState::PumpingA(job.clone());
            (
                DosingEvent::PulseToggle {
                    pump: job.pump,
                    pulse_on: true,
                },
                hw_events,
            )
        }
    }

    fn begin_pump_b(
        &mut self,
        mut b_job: PulseJob,
        now_ms: u64,
    ) -> (DosingEvent, Vec<OrchestratorEvent>) {
        let hw_events = vec![OrchestratorEvent::SetDosingPump {
            pump: DosingPumpTarget::NutrientB,
            on: true,
            pwm_percent: b_job.pwm,
        }];

        b_job.pulse_on = true;
        b_job.pulse_count += 1;
        b_job.next_toggle_ms = now_ms + b_job.on_ms;
        self.sub_state = DosingSubState::PumpingB(b_job);
        (DosingEvent::PhaseTransition, hw_events)
    }

    fn tick_pump_b(
        &mut self,
        now_ms: u64,
        _config: &ControllerConfig,
    ) -> (DosingEvent, Vec<OrchestratorEvent>) {
        let DosingSubState::PumpingB(mut job) = self.sub_state.clone() else {
            return (DosingEvent::Pending, vec![]);
        };
        let mut hw_events = Vec::new();

        if job.pulse_on {
            hw_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientB,
                on: false,
                pwm_percent: 0,
            });

            job.delivered_ml += job.ml_per_sec * (job.on_ms as f32 / 1000.0);

            if job.delivered_ml + 1e-3 >= job.target_ml {
                let delivered_b = job.delivered_ml;
                if let Some(ctx) = self.cycle_ctx.as_mut() {
                    ctx.dose_b_delivered_ml = delivered_b;
                }
                let delivered_a = self
                    .cycle_ctx
                    .as_ref()
                    .map(|c| c.dose_a_delivered_ml)
                    .unwrap_or(0.0);

                let (ev, mut follow_events) =
                    self.transition_to_ph_or_idle(delivered_a, delivered_b, now_ms);
                hw_events.append(&mut follow_events);
                (ev, hw_events)
            } else if job.pulse_count >= job.max_pulses {
                self.sub_state = DosingSubState::Idle;
                (DosingEvent::Failed(FaultCode::EcDosingFailed), hw_events)
            } else {
                job.pulse_on = false;
                job.next_toggle_ms = now_ms + job.off_ms;
                self.sub_state = DosingSubState::PumpingB(job.clone());
                (
                    DosingEvent::PulseToggle {
                        pump: job.pump,
                        pulse_on: false,
                    },
                    hw_events,
                )
            }
        } else {
            hw_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientB,
                on: true,
                pwm_percent: job.pwm,
            });

            job.pulse_on = true;
            job.pulse_count += 1;
            job.next_toggle_ms = now_ms + job.on_ms;
            self.sub_state = DosingSubState::PumpingB(job.clone());
            (
                DosingEvent::PulseToggle {
                    pump: job.pump,
                    pulse_on: true,
                },
                hw_events,
            )
        }
    }

    fn tick_pump_ph(&mut self, now_ms: u64) -> (DosingEvent, Vec<OrchestratorEvent>) {
        let DosingSubState::PumpingPH(mut job) = self.sub_state.clone() else {
            return (DosingEvent::Pending, vec![]);
        };
        let mut hw_events = Vec::new();

        let target_pump = match job.pump {
            PumpTarget::PhUp => DosingPumpTarget::PhUp,
            PumpTarget::PhDown => DosingPumpTarget::PhDown,
            _ => return (DosingEvent::Failed(FaultCode::EcDosingFailed), hw_events),
        };

        if job.pulse_on {
            hw_events.push(OrchestratorEvent::SetDosingPump {
                pump: target_pump,
                on: false,
                pwm_percent: 0,
            });

            if job.delivered_ml + 1e-3 >= job.target_ml {
                if let Some(ctx) = self.cycle_ctx.as_mut() {
                    if matches!(job.pump, PumpTarget::PhUp) {
                        ctx.ph_up_delivered_ml = job.delivered_ml;
                    } else {
                        ctx.ph_down_delivered_ml = job.delivered_ml;
                    }
                }

                let final_a = self
                    .cycle_ctx
                    .as_ref()
                    .map(|c| c.dose_a_delivered_ml)
                    .unwrap_or(0.0);
                let final_b = self
                    .cycle_ctx
                    .as_ref()
                    .map(|c| c.dose_b_delivered_ml)
                    .unwrap_or(0.0);
                let final_up = self
                    .cycle_ctx
                    .as_ref()
                    .map(|c| c.ph_up_delivered_ml)
                    .unwrap_or(0.0);
                let final_down = self
                    .cycle_ctx
                    .as_ref()
                    .map(|c| c.ph_down_delivered_ml)
                    .unwrap_or(0.0);

                self.sub_state = DosingSubState::Idle;
                (
                    DosingEvent::CycleComplete {
                        dose_a_ml: final_a,
                        dose_b_ml: final_b,
                        ph_up_ml: final_up,
                        ph_down_ml: final_down,
                    },
                    hw_events,
                )
            } else if job.pulse_count >= job.max_pulses {
                self.sub_state = DosingSubState::Idle;
                (DosingEvent::Failed(FaultCode::PhDosingFailed), hw_events)
            } else {
                job.pulse_on = false;
                job.next_toggle_ms = now_ms + job.off_ms;
                self.sub_state = DosingSubState::PumpingPH(job.clone());
                (
                    DosingEvent::PulseToggle {
                        pump: job.pump,
                        pulse_on: false,
                    },
                    hw_events,
                )
            }
        } else {
            hw_events.push(OrchestratorEvent::SetDosingPump {
                pump: target_pump,
                on: true,
                pwm_percent: job.pwm,
            });

            job.pulse_on = true;
            job.pulse_count += 1;
            job.delivered_ml += job.ml_per_sec * (job.on_ms as f32 / 1000.0);
            job.next_toggle_ms = now_ms + job.on_ms;
            self.sub_state = DosingSubState::PumpingPH(job.clone());
            (
                DosingEvent::PulseToggle {
                    pump: job.pump,
                    pulse_on: true,
                },
                hw_events,
            )
        }
    }

    fn transition_to_ph_or_idle(
        &mut self,
        delivered_a: f32,
        delivered_b: f32,
        now_ms: u64,
    ) -> (DosingEvent, Vec<OrchestratorEvent>) {
        if let Some(mut ph_job) = self.pending_ph_job.take() {
            // Reset timing to NOW so the first pulse starts cleanly from current time
            ph_job.next_toggle_ms = now_ms;
            ph_job.pulse_on = false; // start in OFF state — tick_pump_ph will send ON
            ph_job.delivered_ml = 0.0;
            ph_job.pulse_count = 0;

            // Do NOT push a hardware event here — let tick_pump_ph handle it next tick
            self.sub_state = DosingSubState::PumpingPH(ph_job);
            (DosingEvent::PhaseTransition, vec![])
        } else {
            self.sub_state = DosingSubState::Idle;
            (
                DosingEvent::CycleComplete {
                    dose_a_ml: delivered_a,
                    dose_b_ml: delivered_b,
                    ph_up_ml: 0.0,
                    ph_down_ml: 0.0,
                },
                vec![],
            )
        }
    }
}

pub fn pulse_params(
    dose_ml: f32,
    capacity_ml_per_sec: f32,
    config: &ControllerConfig,
) -> (u64, u64, u32) {
    let is_pulse_mode = dose_ml < config.dosing_min_dose_ml;
    let pulse_on_ms = if is_pulse_mode {
        config.dosing_pulse_on_ms.max(1) as u64
    } else {
        ((dose_ml / capacity_ml_per_sec) * 1000.0).round().max(1.0) as u64
    };
    let pulse_off_ms = if is_pulse_mode {
        config.dosing_pulse_off_ms as u64
    } else {
        0
    };
    let max_pulse_count = if is_pulse_mode {
        config.dosing_max_pulse_count_per_cycle.max(1) as u32
    } else {
        1
    };
    (pulse_on_ms, pulse_off_ms, max_pulse_count)
}

pub fn calculate_channel_dosing_duration_ms(
    dose_ml: f32,
    capacity_ml_per_sec: f32,
    config: &ControllerConfig,
) -> u64 {
    if dose_ml <= 1e-3 || capacity_ml_per_sec <= 1e-4 {
        return 0;
    }
    let (on_ms, off_ms, max_pulse_count) = pulse_params(dose_ml, capacity_ml_per_sec, config);
    if dose_ml < config.dosing_min_dose_ml {
        let ml_per_pulse = capacity_ml_per_sec * (on_ms as f32 / 1000.0);
        if ml_per_pulse <= 1e-6 {
            return 0;
        }
        let pulses_needed = (dose_ml / ml_per_pulse).ceil() as u64;
        let actual_pulses = pulses_needed.min(max_pulse_count as u64);
        if actual_pulses == 0 {
            0
        } else {
            actual_pulses * on_ms + actual_pulses.saturating_sub(1) * off_ms
        }
    } else {
        on_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pump_b_delivered_ml_starts_at_zero() {
        let b_job = PulseJob {
            pump: PumpTarget::NutrientB,
            target_ml: 5.0,
            delivered_ml: 0.0,
            pulse_on: false,
            pulse_count: 0,
            max_pulses: 1,
            on_ms: 1000,
            off_ms: 0,
            pwm: 80,
            ml_per_sec: 5.0,
            next_toggle_ms: 0,
        };
        let mut actor = DosingActor::new();
        actor.cycle_ctx = Some(DosingCycleCtx {
            dose_a_delivered_ml: 5.0,
            dose_b_delivered_ml: 0.0,
            ph_up_delivered_ml: 0.0,
            ph_down_delivered_ml: 0.0,
        });
        let (_event, _hw) = actor.begin_pump_b(b_job, 1000);
        assert!(matches!(actor.sub_state, DosingSubState::PumpingB(_)));
        if let DosingSubState::PumpingB(job) = &actor.sub_state {
            assert_eq!(
                job.delivered_ml, 0.0,
                "delivered_ml phải là 0 khi vừa bắt đầu bơm B"
            );
        }
    }
}
