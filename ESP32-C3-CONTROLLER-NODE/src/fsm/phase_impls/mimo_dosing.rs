// src/fsm/phase_impls/mimo_dosing.rs
use hydragrow_shared::{ControllerConfig, SensorData};
use log::warn;

use crate::fsm::actors::dosing_actor::DosingEvent;
use crate::fsm::events::{DosingPumpTarget, OrchestratorEvent};
use crate::fsm::phase_impls::SystemPhase;
use crate::fsm::phase_tick::PhaseTick;
use crate::fsm::system_context::SystemContext;
use crate::fsm::tick_result::{CalibrationDelta, ContextDelta, PeripheralDelta, TickResult};
use crate::fsm::types::PendingCalibrationSample;
use crate::pump::WaterDirection;

pub struct MimoDosingPhase;

impl PhaseTick for MimoDosingPhase {
    fn tick(
        &self,
        now_ms: u64,
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();
        let mut peri_delta = PeripheralDelta::default();
        let elapsed_ms = now_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(now_ms));

        // Timeout bơm nước
        if ctx.peripherals.pump_status.water_pump_in
            && elapsed_ms >= (config.max_refill_duration_sec as u64 * 1000)
        {
            result.events.push(OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop,
            });
            peri_delta.water_pump_in = Some(false);
        }
        if ctx.peripherals.pump_status.water_pump_out
            && elapsed_ms >= (config.max_drain_duration_sec as u64 * 1000)
        {
            result.events.push(OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop,
            });
            peri_delta.water_pump_out = Some(false);
        }

        // Hard timeout toàn phase
        if now_ms >= ctx.phase_finish_ms.unwrap_or(u64::MAX) + 5_000 {
            warn!("⚠️ [FSM] Dosing phase timeout cứng! Chuyển về Cooldown.");
            result.events.push(OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop,
            });
            if ctx.peripherals.misting_started_by_dosing {
                result
                    .events
                    .push(OrchestratorEvent::SetMistValve { on: false });
                peri_delta.mist_valve = Some(false);
                peri_delta.is_misting_active = Some(false);
                peri_delta.misting_started_by_dosing = Some(false);
            }
            result.delta.phase = Some(SystemPhase::Cooldown);
            result.delta.phase_finish_ms =
                Some(Some(now_ms + config.cooldown_sec.max(30) as u64 * 1000));
            result.delta.peripherals = Some(peri_delta);
            return result;
        }

        // Tick DosingActor
        let (dosing_event, hardware_events) = ctx.dosing.tick(now_ms, config);
        result.events.extend(hardware_events);

        match dosing_event {
            DosingEvent::Pending => {
                if ctx.dosing.is_idle() && elapsed_ms >= 500 {
                    // Water-only cycle complete
                    let water_in_spent = if ctx.peripherals.pump_status.water_pump_in {
                        elapsed_ms.min(config.max_refill_duration_sec as u64 * 1000) as f32 / 1000.0
                    } else {
                        0.0
                    };
                    let water_out_spent = if ctx.peripherals.pump_status.water_pump_out {
                        elapsed_ms.min(config.max_drain_duration_sec as u64 * 1000) as f32 / 1000.0
                    } else {
                        0.0
                    };

                    result.events.push(OrchestratorEvent::SetWaterPump {
                        direction: WaterDirection::Stop,
                    });
                    if ctx.peripherals.misting_started_by_dosing {
                        result
                            .events
                            .push(OrchestratorEvent::SetMistValve { on: false });
                        peri_delta.mist_valve = Some(false);
                        peri_delta.is_misting_active = Some(false);
                        peri_delta.misting_started_by_dosing = Some(false);
                    }
                    peri_delta.water_pump_in = Some(false);
                    peri_delta.water_pump_out = Some(false);

                    result.delta.calibration =
                        Some(CalibrationDelta::Start(PendingCalibrationSample {
                            cycle_id: format!("water-{now_ms}"),
                            trigger: "water_only_cycle".to_string(),
                            start_ec: ctx.safety.last_ec_before_dose.unwrap_or(sensors.ec),
                            start_ph: ctx.safety.last_ph_before_dose.unwrap_or(sensors.ph),
                            start_water_level: sensors.water_level,
                            start_temp: sensors.temp,
                            target_ec: config.ec_target,
                            target_ph: config.ph_target,
                            dose_a_ml: 0.0,
                            dose_b_ml: 0.0,
                            dose_ph_up_ml: 0.0,
                            dose_ph_down_ml: 0.0,
                            water_in_sec: water_in_spent,
                            water_out_sec: water_out_spent,
                            post_mixing_ec: 0.0,
                            post_mixing_ph: 0.0,
                            start_ms: ctx.phase_start_ms.unwrap_or(now_ms),
                            active_mixing_finish_ms: now_ms
                                + (ctx.diagnostic.adaptive_mixing_sec as u64 * 1000),
                            stabilizing_start_ms: None,
                            stabilizing_finish_ms: None,
                            invalid_by_noise: false,
                            invalid_by_water_change: false,
                        }));

                    result.delta.phase = Some(SystemPhase::ActiveMixing);
                    result.delta.phase_start_ms = Some(Some(now_ms));
                    result.delta.phase_finish_ms = Some(Some(
                        now_ms + ctx.diagnostic.adaptive_mixing_sec as u64 * 1000,
                    ));
                    result.delta.reset_stabilizer = true;
                }
            }
            DosingEvent::SoftStartDone | DosingEvent::PhaseTransition => {}
            DosingEvent::PulseToggle { pump, pulse_on } => {
                let target_pump = match pump {
                    crate::fsm::actors::dosing_actor::PumpTarget::NutrientA { .. } => {
                        DosingPumpTarget::NutrientA
                    }
                    crate::fsm::actors::dosing_actor::PumpTarget::NutrientB => {
                        DosingPumpTarget::NutrientB
                    }
                    crate::fsm::actors::dosing_actor::PumpTarget::PhUp => DosingPumpTarget::PhUp,
                    crate::fsm::actors::dosing_actor::PumpTarget::PhDown => {
                        DosingPumpTarget::PhDown
                    }
                };
                result.events.push(OrchestratorEvent::SetDosingPump {
                    pump: target_pump,
                    on: pulse_on,
                    pwm_percent: if pulse_on {
                        config.dosing_pwm_percent as u32
                    } else {
                        0
                    },
                });
            }
            DosingEvent::CycleComplete {
                dose_a_ml,
                dose_b_ml,
                ph_up_ml,
                ph_down_ml,
            } => {
                let water_in_spent = if ctx.peripherals.pump_status.water_pump_in {
                    config.max_refill_duration_sec as f32
                } else {
                    0.0
                };
                let water_out_spent = if ctx.peripherals.pump_status.water_pump_out {
                    config.max_drain_duration_sec as f32
                } else {
                    0.0
                };

                result.events.push(OrchestratorEvent::SetWaterPump {
                    direction: WaterDirection::Stop,
                });
                if ctx.peripherals.misting_started_by_dosing {
                    result
                        .events
                        .push(OrchestratorEvent::SetMistValve { on: false });
                    peri_delta.mist_valve = Some(false);
                    peri_delta.is_misting_active = Some(false);
                    peri_delta.misting_started_by_dosing = Some(false);
                }
                peri_delta.water_pump_in = Some(false);
                peri_delta.water_pump_out = Some(false);

                result.delta.calibration =
                    Some(CalibrationDelta::Start(PendingCalibrationSample {
                        cycle_id: format!("mimo-{now_ms}"),
                        trigger: "mimo_matrix_control".to_string(),
                        start_ec: ctx.safety.last_ec_before_dose.unwrap_or(sensors.ec),
                        start_ph: ctx.safety.last_ph_before_dose.unwrap_or(sensors.ph),
                        start_water_level: sensors.water_level,
                        start_temp: sensors.temp,
                        target_ec: config.ec_target,
                        target_ph: config.ph_target,
                        dose_a_ml,
                        dose_b_ml,
                        dose_ph_up_ml: ph_up_ml,
                        dose_ph_down_ml: ph_down_ml,
                        water_in_sec: water_in_spent,
                        water_out_sec: water_out_spent,
                        post_mixing_ec: 0.0,
                        post_mixing_ph: 0.0,
                        start_ms: ctx.phase_start_ms.unwrap_or(now_ms),
                        active_mixing_finish_ms: now_ms
                            + (ctx.diagnostic.adaptive_mixing_sec as u64 * 1000),
                        stabilizing_start_ms: None,
                        stabilizing_finish_ms: None,
                        invalid_by_noise: false,
                        invalid_by_water_change: false,
                    }));

                result.delta.phase = Some(SystemPhase::ActiveMixing);
                result.delta.phase_start_ms = Some(Some(now_ms));
                result.delta.phase_finish_ms = Some(Some(
                    now_ms + ctx.diagnostic.adaptive_mixing_sec as u64 * 1000,
                ));
                result.delta.reset_stabilizer = true;
            }
            DosingEvent::Failed(code) => {
                result.delta.phase = Some(SystemPhase::Fault(code));
            }
        }

        result.delta.peripherals = Some(peri_delta);
        result
    }
}
