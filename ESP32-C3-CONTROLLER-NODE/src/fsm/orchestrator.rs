use std::str::FromStr;
use std::sync::mpsc::Sender;

use chrono::Local;
use cron::Schedule;

use esp_idf_svc::nvs::EspDefaultNvs;
use hydragrow_shared::ControllerConfig;

use crate::{config::SharedConfig, mqtt::SensorData, pump::PumpController};

use super::{
    actors::{dosing_actor::DosingEvent, water_actor::WaterEvent},
    peripheral::PeripheralController,
    phases::{FaultCode, SystemPhase},
    system_context::SystemContext,
    types::PendingCalibrationSample,
    utils::soft_deadband_scale,
};

enum OrchestratorDecision {
    StartEcDosing { dose_ml: f32, target_ec: f32, pwm: u32 },
    StartPhDosing {
        is_up: bool,
        dose_ml: f32,
        target_ph: f32,
        pwm: u32,
    },
    StartWaterFill { target: f32 },
    StartWaterDrain { target: f32, trigger: String },
    Idle,
    Fault(FaultCode),
}


fn check_scheduled_water_change(
    ctx: &mut SystemContext,
    config: &ControllerConfig,
    sensors: &SensorData,
    now_sec: u64,
    nvs: &mut Option<EspDefaultNvs>,
) -> Option<OrchestratorDecision> {
    if !(config.enable_water_level_sensor
        && config.scheduled_water_change_enabled
        && !config.water_change_cron.is_empty())
    {
        return None;
    }

    if ctx.water_change_cron != config.water_change_cron {
        ctx.water_change_cron = config.water_change_cron.clone();
        match Schedule::from_str(&ctx.water_change_cron) {
            Ok(schedule) => {
                if let Some(next) = schedule.upcoming(Local).next() {
                    ctx.next_water_change_trigger_sec = Some(next.timestamp() as u64);
                }
            }
            Err(_) => {
                ctx.next_water_change_trigger_sec = None;
                return None;
            }
        }
    }

    let next_trigger = ctx.next_water_change_trigger_sec?;
    if now_sec < next_trigger {
        return None;
    }

    if let Ok(schedule) = Schedule::from_str(&ctx.water_change_cron) {
        let future = Local::now() + chrono::Duration::seconds(1);
        if let Some(next) = schedule.after(&future).next() {
            ctx.next_water_change_trigger_sec = Some(next.timestamp() as u64);
        }
    }

    ctx.last_water_change_sec = now_sec;
    if let Some(flash) = nvs.as_mut() {
        let _ = flash.set_u64("last_w_change", now_sec);
    }

    let target = (sensors.water_level - config.scheduled_drain_amount_cm).max(config.water_level_min);
    Some(OrchestratorDecision::StartWaterDrain {
        target,
        trigger: "scheduled_change".to_string(),
    })
}

fn decide_monitoring(
    sensors: &SensorData,
    config: &ControllerConfig,
    ctx: &SystemContext,
    _now_ms: u64,
) -> OrchestratorDecision {

    if config.enable_water_level_sensor && config.auto_drain_overflow && sensors.water_level > config.water_level_max {
        return OrchestratorDecision::StartWaterDrain {
            target: config.water_level_target,
            trigger: "overflow".to_string(),
        };
    }

    if config.enable_water_level_sensor
        && config.auto_refill_enabled
        && sensors.water_level < (config.water_level_target - config.water_level_tolerance)
    {
        return OrchestratorDecision::StartWaterFill {
            target: config.water_level_target,
        };
    }

    if config.enable_ec_sensor && sensors.ec < (config.ec_target - config.ec_tolerance) {
        let delta = (config.ec_target - sensors.ec).max(0.0);
        let deadband_scale = soft_deadband_scale(delta, config.ec_tolerance);
        let step_ratio = if ctx.tuner.is_locked() { ctx.tuner.best_ec_ratio } else { ctx.tuner.active_ec_ratio() };
        let dose_ml = (delta / config.ec_gain_per_ml.max(0.0001) * step_ratio * deadband_scale)
            .clamp(0.0, config.max_dose_per_cycle);
        if dose_ml > 0.0 {
            return OrchestratorDecision::StartEcDosing {
                dose_ml,
                target_ec: config.ec_target,
                pwm: config.dosing_pwm_percent.clamp(1, 100) as u32,
            };
        }
    }

    if config.enable_ec_sensor
        && config.enable_water_level_sensor
        && config.auto_dilute_enabled
        && sensors.ec > (config.ec_target + config.ec_tolerance)
    {
        return OrchestratorDecision::StartWaterDrain {
            target: config.water_level_target,
            trigger: "auto_dilute".to_string(),
        };
    }

    if config.enable_ph_sensor {
        if sensors.ph > (config.ph_target + config.ph_tolerance) {
            let delta = (sensors.ph - config.ph_target).max(0.0);
            let deadband_scale = soft_deadband_scale(delta, config.ph_tolerance);
            let step_ratio = if ctx.tuner.is_locked() { ctx.tuner.best_ph_ratio } else { ctx.tuner.adaptive_ph_ratio };
            let dose_ml = (delta / config.ph_shift_down_per_ml.max(0.0001) * step_ratio * deadband_scale)
                .clamp(0.0, config.max_dose_per_cycle);
            if dose_ml > 0.0 {
                return OrchestratorDecision::StartPhDosing {
                    is_up: false,
                    dose_ml,
                    target_ph: config.ph_target,
                    pwm: config.dosing_pwm_percent.clamp(1, 100) as u32,
                };
            }
        } else if sensors.ph < (config.ph_target - config.ph_tolerance) {
            let delta = (config.ph_target - sensors.ph).max(0.0);
            let deadband_scale = soft_deadband_scale(delta, config.ph_tolerance);
            let step_ratio = if ctx.tuner.is_locked() { ctx.tuner.best_ph_ratio } else { ctx.tuner.adaptive_ph_ratio };
            let dose_ml = (delta / config.ph_shift_up_per_ml.max(0.0001) * step_ratio * deadband_scale)
                .clamp(0.0, config.max_dose_per_cycle);
            if dose_ml > 0.0 {
                return OrchestratorDecision::StartPhDosing {
                    is_up: true,
                    dose_ml,
                    target_ph: config.ph_target,
                    pwm: config.dosing_pwm_percent.clamp(1, 100) as u32,
                };
            }
        }
    }

    OrchestratorDecision::Idle
}

fn apply_decision(
    decision: OrchestratorDecision,
    ctx: &mut SystemContext,
    config: &ControllerConfig,
    sensors: &SensorData,
    now_ms: u64,
) {
    match decision {
        OrchestratorDecision::StartEcDosing {
            dose_ml,
            target_ec,
            pwm,
        } => {
            if !ctx
                .safety
                .check_hourly_dose("ec", now_ms / 1000, dose_ml, config.max_dose_per_hour)
            {
                ctx.phase = SystemPhase::Fault(FaultCode::MaxHourlyDoseEc);
                return;
            }
            ctx.dosing
                .start_ec_cycle(now_ms, dose_ml, target_ec, pwm, config);
            ctx.phase = SystemPhase::DosingEC;
            ctx.safety.last_ec_before_dose = Some(sensors.ec);
        }
        OrchestratorDecision::StartPhDosing { is_up, dose_ml, target_ph, pwm } => {
            if !ctx
                .safety
                .check_hourly_dose("ph", now_ms / 1000, dose_ml, config.max_dose_per_hour)
            {
                ctx.phase = SystemPhase::Fault(FaultCode::MaxHourlyDosePh);
                return;
            }
            ctx.dosing
                .start_ph_cycle(now_ms, is_up, dose_ml, target_ph, pwm, config);
            ctx.phase = SystemPhase::DosingPH;
            ctx.safety.last_ph_before_dose = Some(sensors.ph);
            ctx.safety.last_ph_dose_up = Some(is_up);
        }
        OrchestratorDecision::StartWaterFill { target } => {
            if !ctx
                .safety
                .record_refill(now_ms / 1000, config.max_refill_cycles_per_hour as u32)
            {
                ctx.phase = SystemPhase::Fault(FaultCode::TooManyRefills);
                return;
            }
            ctx.water.start_fill(now_ms, target, sensors, "auto_refill");
            ctx.phase = SystemPhase::WaterRefilling;
            ctx.safety.last_water_before_refill = Some(sensors.water_level);
        }
        OrchestratorDecision::StartWaterDrain { target, trigger } => {
            if !ctx
                .safety
                .record_drain(now_ms / 1000, config.max_drain_cycles_per_hour as u32)
            {
                ctx.phase = SystemPhase::Fault(FaultCode::TooManyDrains);
                return;
            }
            ctx.water.start_drain(now_ms, target, sensors, &trigger);
            ctx.phase = SystemPhase::WaterDraining;
        }
        OrchestratorDecision::Fault(code) => {
            ctx.phase = SystemPhase::Fault(code);
        }
        OrchestratorDecision::Idle => {}
    }
}

#[allow(clippy::too_many_arguments)]
pub fn tick(
    now_ms: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut SystemContext,
    pumps: &mut PumpController,
    shared_config: &SharedConfig,
    nvs: &mut Option<EspDefaultNvs>,
    dosing_report_tx: &Sender<String>,
    mqtt_tx: &Sender<String>,
) {
    let _ = (shared_config, dosing_report_tx, mqtt_tx);

    match &ctx.phase.clone() {
        SystemPhase::Monitoring => {
            let now_sec = now_ms / 1000;
            if let Some(decision) = check_scheduled_water_change(ctx, config, sensors, now_sec, nvs) {
                apply_decision(decision, ctx, config, sensors, now_ms);
            } else {
                let decision = decide_monitoring(sensors, config, ctx, now_ms);
                apply_decision(decision, ctx, config, sensors, now_ms);
            }
        }

        SystemPhase::DosingEC | SystemPhase::DosingPH => match ctx.dosing.tick(now_ms, config, pumps) {
            DosingEvent::CycleComplete { dose_a_ml, dose_b_ml, ph_up_ml, ph_down_ml } => {
                if let Some(c) = ctx.dosing.cycle_ctx.clone() {
                    ctx.calibration.start_sample(PendingCalibrationSample {
                        cycle_id: c.cycle_id,
                        trigger: c.trigger,
                        start_ec: sensors.ec,
                        start_ph: sensors.ph,
                        start_water_level: sensors.water_level,
                        target_ec: c.target_ec,
                        target_ph: c.target_ph,
                        dose_a_ml,
                        dose_b_ml,
                        dose_ph_up_ml: ph_up_ml,
                        dose_ph_down_ml: ph_down_ml,
                        post_mixing_ec: 0.0,
                        post_mixing_ph: 0.0,
                        start_ms: c.start_ms,
                        active_mixing_finish_ms: now_ms + config.active_mixing_sec as u64 * 1000,
                        stabilizing_start_ms: None,
                        stabilizing_finish_ms: None,
                        invalid_by_noise: false,
                        invalid_by_water_change: false,
                    });
                }
                let _ = dosing_report_tx.send(format!(
                    "{{\"type\":\"dosing_cycle_complete\",\"dose_a_ml\":{:.3},\"dose_b_ml\":{:.3},\"ph_up_ml\":{:.3},\"ph_down_ml\":{:.3}}}",
                    dose_a_ml, dose_b_ml, ph_up_ml, ph_down_ml
                ));
                ctx.phase = SystemPhase::ActiveMixing;
                ctx.phase_finish_ms = Some(now_ms + config.active_mixing_sec as u64 * 1000);
            }
            DosingEvent::Failed(code) => {
                let _ = mqtt_tx.send(format!("[ORCH] dosing_failed:{}", code.as_str()));
                ctx.phase = SystemPhase::Fault(code);
            }
            _ => {}
        },

        SystemPhase::WaterRefilling | SystemPhase::WaterDraining => {
            match ctx.water.tick(now_ms, sensors, config, pumps) {
                WaterEvent::Done { success, duration_sec } => {
                    let _ = duration_sec;
                    ctx.phase = if success {
                        SystemPhase::ActiveMixing
                    } else {
                        SystemPhase::Fault(FaultCode::WaterRefillFailed)
                    };
                    if matches!(ctx.phase, SystemPhase::ActiveMixing) {
                        ctx.phase_finish_ms = Some(now_ms + config.active_mixing_sec as u64 * 1000);
                    }
                }
                WaterEvent::Pending => {}
            }
        }

        SystemPhase::ActiveMixing => {
            if now_ms >= ctx.phase_finish_ms.unwrap_or(0) {
                ctx.phase = SystemPhase::Stabilizing;
                ctx.phase_finish_ms = Some(now_ms + config.sensor_stabilize_sec as u64 * 1000);
            }
        }

        SystemPhase::Stabilizing => {
            if let Some(sample) = ctx.calibration.pending_sample.as_mut() {
                sample.stabilizing_start_ms.get_or_insert(now_ms);
            }
            if now_ms >= ctx.phase_finish_ms.unwrap_or(0) {
                if let Some(sample) = ctx.calibration.pending_sample.as_mut() {
                    sample.stabilizing_finish_ms = Some(now_ms);
                    sample.post_mixing_ec = sensors.ec;
                    sample.post_mixing_ph = sensors.ph;
                }

                if let Some(before) = ctx.safety.last_ec_before_dose {
                    if sensors.ec < before + config.ec_ack_threshold {
                        ctx.dosing.retry_ec = ctx.dosing.retry_ec.saturating_add(1);
                        if ctx.dosing.retry_ec >= 3 {
                            ctx.phase = SystemPhase::Fault(FaultCode::EcDosingFailed);
                            let _ = mqtt_tx.send("[ORCH] ec_ack_failed_after_3_retries".to_string());
                            return;
                        }
                    } else {
                        ctx.dosing.retry_ec = 0;
                    }
                }

                if let (Some(before), Some(is_up)) = (ctx.safety.last_ph_before_dose, ctx.safety.last_ph_dose_up) {
                    let moved = if is_up { sensors.ph - before } else { before - sensors.ph };
                    if moved < config.ph_ack_threshold {
                        ctx.dosing.retry_ph = ctx.dosing.retry_ph.saturating_add(1);
                        if ctx.dosing.retry_ph >= 3 {
                            ctx.phase = SystemPhase::Fault(FaultCode::PhDosingFailed);
                            let _ = mqtt_tx.send("[ORCH] ph_ack_failed_after_3_retries".to_string());
                            return;
                        }
                    } else {
                        ctx.dosing.retry_ph = 0;
                    }
                }

                if let Some(sample) = ctx.calibration.pending_sample.take() {
                    let ec_response = sample.post_mixing_ec - sample.start_ec;
                    if sample.dose_a_ml > 0.0 || sample.dose_b_ml > 0.0 {
                        ctx.tuner.on_dosing_ack(ec_response, config.ec_ack_threshold, config, now_ms / 1000);
                    }
                    let _ = mqtt_tx.send(format!(
                        "[EMA SAMPLE] cycle={} ec:{:.3}->{:.3} ph:{:.3}->{:.3}",
                        sample.cycle_id, sample.start_ec, sample.post_mixing_ec, sample.start_ph, sample.post_mixing_ph
                    ));
                }

                ctx.phase = SystemPhase::Cooldown;
                ctx.phase_finish_ms = Some(now_ms + config.cooldown_sec as u64 * 1000);
            }
        }

        SystemPhase::Cooldown => {
            if now_ms >= ctx.phase_finish_ms.unwrap_or(0) {
                ctx.phase = SystemPhase::Monitoring;
                ctx.phase_finish_ms = None;
            }
        }

        SystemPhase::Fault(_) | SystemPhase::EmergencyStop(_) => {}
        _ => {}
    }

    let now_sec = now_ms / 1000;
    PeripheralController::tick_scheduled_mixing(&mut ctx.peripherals, now_sec, config);
    PeripheralController::tick_misting(&mut ctx.peripherals, pumps, sensors, now_ms, config);
    let is_dosing_active = matches!(
        ctx.phase,
        SystemPhase::DosingEC | SystemPhase::DosingPH | SystemPhase::ActiveMixing
    );
    PeripheralController::tick_osaka(&mut ctx.peripherals, pumps, is_dosing_active, config);
}
