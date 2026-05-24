use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::log::{LogCategory, LogLevel, UnifiedSystemLog};
// src/fsm/phases/monitoring.rs
use hydragrow_shared::{ControllerConfig, SensorData};
use log::warn;

use crate::fsm::events::OrchestratorEvent;
use crate::fsm::phase_impls::FaultCode;
use crate::fsm::phase_tick::PhaseTick;
use crate::fsm::solver::select_solver;
use crate::fsm::solver::SolveResult;
use crate::fsm::system_context::SystemContext;
use crate::fsm::tick_result::{CalibrationDelta, ContextDelta, PeripheralDelta, TickResult};
use crate::fsm::types::PendingCalibrationSample;
use crate::pump::WaterDirection;

use chrono::Local;
use cron::Schedule;
use std::str::FromStr;

pub struct MonitoringPhase;

impl PhaseTick for MonitoringPhase {
    fn tick(
        &self,
        now_ms: u64,
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();
        let now_sec = now_ms / 1000;

        // Check scheduled water change
        if let Some(water_change_result) =
            check_scheduled_water_change(ctx, config, now_sec, &mut result.delta)
        {
            return apply_decision(
                water_change_result,
                ctx,
                config,
                sensors,
                now_ms,
                result,
                true,
            );
        }

        // Solve MIMO
        let solver = select_solver(ctx);
        let decision = solver.solve(sensors, config, ctx);
        apply_decision(decision, ctx, config, sensors, now_ms, result, false)
    }
}

fn check_scheduled_water_change(
    ctx: &SystemContext,
    config: &ControllerConfig,
    now_sec: u64,
    delta: &mut ContextDelta,
) -> Option<crate::fsm::solver::SolveResult> {
    if !(config.enable_water_level_sensor
        && config.scheduled_water_change_enabled
        && !config.water_change_cron.is_empty())
    {
        return None;
    }

    let mut current_next_trigger = ctx.next_water_change_trigger_sec;

    if ctx.water_change_cron != config.water_change_cron {
        delta.water_change_cron = Some(config.water_change_cron.clone());
        if let Ok(schedule) = Schedule::from_str(&config.water_change_cron) {
            if let Some(next) = schedule.upcoming(Local).next() {
                let ts = next.timestamp() as u64;
                delta.next_water_change_trigger_sec = Some(Some(ts));
                current_next_trigger = Some(ts);
            }
        }
    }

    let next_trigger = current_next_trigger?;
    if now_sec < next_trigger {
        return None;
    }

    if let Ok(schedule) = Schedule::from_str(&config.water_change_cron) {
        let future = Local::now() + chrono::Duration::seconds(1);
        if let Some(next) = schedule.after(&future).next() {
            delta.next_water_change_trigger_sec = Some(Some(next.timestamp() as u64));
        }
    }

    delta.last_water_change_sec = Some(now_sec);

    let mut peri_delta = delta.peripherals.take().unwrap_or_default();
    peri_delta.last_mixing_start_sec = Some(now_sec);
    peri_delta.is_scheduled_mixing_active = Some(false);
    delta.peripherals = Some(peri_delta);

    delta.calibration = Some(CalibrationDelta::Invalidate);

    let mut control = crate::fsm::matrix::ControlVector::default();
    control.water_out_sec = config.max_drain_duration_sec as f32;

    Some(SolveResult::Execute {
        control,
        target_ec: config.ec_target,
        target_ph: config.ph_target,
        pwm: config.dosing_pwm_percent as u32,
    })
}

fn apply_decision(
    decision: SolveResult,
    ctx: &mut SystemContext,
    config: &ControllerConfig,
    sensors: &SensorData,
    now_ms: u64,
    mut result: TickResult,
    is_water_change: bool,
) -> TickResult {
    match decision {
        SolveResult::Execute {
            control,
            target_ec,
            target_ph,
            pwm,
        } => {
            let mut peri_delta = result.delta.peripherals.take().unwrap_or_default();
            if is_water_change {
                ctx.tuner.on_water_change();
                log::info!("🔄 [MONITORING] Scheduled water change: AutoTuner trackers reset.");
            }
            // Safety budget checks
            if control.nutrient_a_ml > 0.0
                && !ctx.safety.check_hourly_dose(
                    "NutrientA",
                    now_ms / 1000,
                    control.nutrient_a_ml,
                    config.max_dose_per_hour / 2.0,
                )
            {
                result.delta.phase = Some(SystemPhase::Fault(FaultCode::MaxHourlyDoseEc));
                return result;
            }
            if control.nutrient_b_ml > 0.0
                && !ctx.safety.check_hourly_dose(
                    "NutrientB",
                    now_ms / 1000,
                    control.nutrient_b_ml,
                    config.max_dose_per_hour / 2.0,
                )
            {
                result.delta.phase = Some(SystemPhase::Fault(FaultCode::MaxHourlyDoseEc));
                return result;
            }
            if control.ph_up_ml > 0.0 {
                let _ = ctx.safety.check_hourly_dose(
                    "PhUp",
                    now_ms / 1000,
                    control.ph_up_ml,
                    config.max_dose_per_hour / 4.0,
                );
            }
            if control.ph_down_ml > 0.0 {
                let _ = ctx.safety.check_hourly_dose(
                    "PhDown",
                    now_ms / 1000,
                    control.ph_down_ml,
                    config.max_dose_per_hour / 4.0,
                );
            }

            if control.water_in_sec > 0.0
                && !ctx
                    .safety
                    .record_refill(now_ms / 1000, config.max_refill_cycles_per_hour as u32)
            {
                warn!("⚠️ [SAFETY] Vượt giới hạn cấp nước/giờ.");
                result.delta.phase = Some(SystemPhase::Fault(FaultCode::TooManyRefills));
                return result;
            }
            if control.water_out_sec > 0.0
                && !ctx
                    .safety
                    .record_drain(now_ms / 1000, config.max_drain_cycles_per_hour as u32)
            {
                warn!("⚠️ [SAFETY] Vượt giới hạn xả nước/giờ.");
                result.delta.phase = Some(SystemPhase::Fault(FaultCode::TooManyDrains));
                return result;
            }

            if control.water_in_sec > 0.0 {
                result.events.push(OrchestratorEvent::SetWaterPump {
                    direction: WaterDirection::In,
                });
                peri_delta.water_pump_in = Some(true);
            }
            if control.water_out_sec > 0.0 {
                result.events.push(OrchestratorEvent::SetWaterPump {
                    direction: WaterDirection::Out,
                });
                peri_delta.water_pump_out = Some(true);
            }
            if control.misting_sec > 0.0 {
                result
                    .events
                    .push(OrchestratorEvent::SetMistValve { on: true });
                peri_delta.mist_valve = Some(true);
                peri_delta.is_misting_active = Some(true);
                peri_delta.misting_started_by_dosing = Some(true);
            }

            ctx.dosing
                .start_matrix_cycle(now_ms, &control, target_ec, target_ph, pwm, config, sensors);

            let hardware_run_ms = (control
                .water_in_sec
                .max(control.water_out_sec)
                .max(control.misting_sec)
                * 1000.0) as u64;

            result.delta.phase = Some(SystemPhase::MimoDosing);
            result.delta.phase_start_ms = Some(Some(now_ms));
            result.delta.phase_finish_ms = Some(Some(now_ms + hardware_run_ms + 5000));

            peri_delta.last_ec_before_dose = Some(Some(sensors.ec));
            peri_delta.last_ph_before_dose = Some(Some(sensors.ph));
            result.delta.reset_stabilizer = true;

            let log_payload = UnifiedSystemLog::build_basic_log_json_with_ts(
                &config.device_id,
                LogLevel::Info,
                LogCategory::Dosing,
                "Bắt đầu chu kỳ MIMO",
                format!(
                    "A/B: {:.1}ml | pH_Up/Down: {:.1}/{:.1}ml | Water_In: {:.1}s",
                    control.nutrient_a_ml,
                    control.ph_up_ml,
                    control.ph_down_ml,
                    control.water_in_sec
                ),
                Some(&format!("mimo-{now_ms}")),
                "monitoring_phase",
                now_ms,
            );

            result.events.push(OrchestratorEvent::PublishSystemLog {
                payload_json: log_payload,
            });
            result.delta.peripherals = Some(peri_delta);
        }
        SolveResult::Idle => {}
    }
    result
}
