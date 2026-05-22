use std::str::FromStr;

use chrono::Local;
use cron::Schedule;

use hydragrow_shared::{
    BasicSystemLogMetadata, ControllerConfig, LogCategory, LogLevel, SensorData, SystemLogEvent,
};
use log::warn;

use crate::{
    fsm::{optimizer::apply_safety_guardrails, tick_result::TickResult},
    pump::WaterDirection,
};

use super::{
    actors::dosing_actor::DosingEvent,
    events::{DosingPumpTarget, OrchestratorEvent},
    matrix::{ControlVector, StateDeltaVector},
    phases::{FaultCode, SystemPhase},
    system_context::{NvsSnapshot, SystemContext},
    tick_result::{CalibrationDelta, ContextDelta, PeripheralDelta},
    types::PendingCalibrationSample,
};

enum OrchestratorDecision {
    ExecuteMimoCycle {
        control: ControlVector,
        target_ec: f32,
        target_ph: f32,
        pwm: u32,
    },
    Idle,
    Fault(FaultCode),
}

struct MonitoringMatrixResult;

impl MonitoringMatrixResult {
    /// GIẢI TOÁN TOÀN DIỆN MIMO: Ứng dụng toán học giả nghịch đảo Moore-Penrose điều khiển phối hợp đa biến
    fn solve_mimo(
        sensors: &SensorData,
        config: &ControllerConfig,
        ctx: &SystemContext,
    ) -> OrchestratorDecision {
        let ec_val = sensors.ec;
        let ph_val = sensors.ph;
        let w_level = sensors.water_level;
        let temp_val = sensors.temp;

        // Tính toán độ lệch mục tiêu cơ bản
        let ec_delta = (config.ec_target - ec_val).max(0.0);
        let ph_delta = config.ph_target - ph_val;
        let water_delta = config.water_level_target - w_level;
        let temp_delta = config.misting_temp_threshold - temp_val;

        // 🛡️ CHỐT CHẶN BẢO VỆ CẢM BIẾN: Chỉ tính sai số nếu cấu hình cho phép bật cảm biến tương ứng
        let safe_ec_delta = if config.enable_ec_sensor && ec_delta.abs() > config.ec_tolerance {
            ec_delta
        } else {
            0.0
        };

        let safe_ph_delta = if config.enable_ph_sensor && ph_delta.abs() > config.ph_tolerance {
            ph_delta
        } else {
            0.0
        };

        let safe_water_delta = if config.enable_water_level_sensor
            && water_delta.abs() > config.water_level_tolerance
        {
            water_delta
        } else {
            0.0
        };

        let safe_temp_delta = if config.enable_temp_sensor && temp_delta < 0.0 {
            temp_delta
        } else {
            0.0
        };

        // Nếu tất cả các trục đều nằm trong vùng deadband hoặc bị tắt cảm biến -> Nghỉ (Idle)
        if safe_ec_delta == 0.0
            && safe_ph_delta == 0.0
            && safe_water_delta == 0.0
            && safe_temp_delta == 0.0
        {
            return OrchestratorDecision::Idle;
        }

        // --- CHẾ ĐỘ COLD PATH: Học máy chưa ấm, nạp hằng số cấu hình tĩnh cũ làm dự phòng ---
        if !ctx.tuner.matrix_is_warm {
            let mut control = ControlVector::default();

            if config.enable_ec_sensor && safe_ec_delta > 0.0 {
                let gain = ctx
                    .tuner
                    .gain_learner
                    .effective_ec_gain(config.ec_gain_per_ml)
                    .max(0.0001);
                let step_ratio = if ctx.tuner.is_locked() {
                    ctx.tuner.best_ec_ratio
                } else {
                    ctx.tuner.active_ec_ratio()
                };
                let ml = (safe_ec_delta / gain * step_ratio).clamp(0.0, config.max_dose_per_cycle);
                control.nutrient_a_ml = ml;
                control.nutrient_b_ml = ml;
            }
            if config.enable_ph_sensor && safe_ph_delta.abs() > 0.0 {
                let is_up = safe_ph_delta > 0.0;
                let gain = if is_up {
                    ctx.tuner
                        .gain_learner
                        .effective_ph_up_gain(config.ph_shift_up_per_ml)
                } else {
                    ctx.tuner
                        .gain_learner
                        .effective_ph_down_gain(config.ph_shift_down_per_ml)
                }
                .max(0.0001);
                let step_ratio = if ctx.tuner.is_locked() {
                    ctx.tuner.best_ph_ratio
                } else {
                    ctx.tuner.adaptive_ph_ratio
                };
                let ml =
                    (safe_ph_delta.abs() / gain * step_ratio).clamp(0.0, config.max_dose_per_cycle);
                if is_up {
                    control.ph_up_ml = ml;
                } else {
                    control.ph_down_ml = ml;
                }
            }
            if config.enable_water_level_sensor && safe_water_delta > 0.0 {
                control.water_in_sec =
                    (safe_water_delta / 0.1).clamp(0.0, config.max_refill_duration_sec as f32);
            } else if config.enable_water_level_sensor
                && safe_water_delta < 0.0
                && config.auto_drain_overflow
            {
                control.water_out_sec =
                    (safe_water_delta.abs() / 0.1).clamp(0.0, config.max_drain_duration_sec as f32);
            }

            return OrchestratorDecision::ExecuteMimoCycle {
                control,
                target_ec: config.ec_target,
                target_ph: config.ph_target,
                pwm: config.dosing_pwm_percent as u32,
            };
        }

        // --- CHẾ ĐỘ WARM PATH: Giải toán phối hợp đa biến bằng lõi Moore-Penrose ---
        let target_error = StateDeltaVector {
            ec_delta: safe_ec_delta,
            ph_delta: safe_ph_delta,
            water_level_delta: safe_water_delta,
            temp_delta: safe_temp_delta,
        };

        match ctx.tuner.interaction_matrix.solve(&target_error) {
            Some(mut control) => {
                let ec_step = if ctx.tuner.is_locked() {
                    ctx.tuner.best_ec_ratio
                } else {
                    ctx.tuner.active_ec_ratio()
                };
                let ph_step = if ctx.tuner.is_locked() {
                    ctx.tuner.best_ph_ratio
                } else {
                    ctx.tuner.adaptive_ph_ratio
                };

                control.nutrient_a_ml *= ec_step;
                control.nutrient_b_ml *= ec_step;
                control.ph_up_ml *= ph_step;
                control.ph_down_ml *= ph_step;

                control.nutrient_a_ml = control.nutrient_a_ml.min(config.max_dose_per_cycle);
                control.nutrient_b_ml = control.nutrient_b_ml.min(config.max_dose_per_cycle);
                control.ph_up_ml = control.ph_up_ml.min(config.max_dose_per_cycle);
                control.ph_down_ml = control.ph_down_ml.min(config.max_dose_per_cycle);
                control.water_in_sec = control
                    .water_in_sec
                    .min(config.max_refill_duration_sec as f32);
                control.water_out_sec = control
                    .water_out_sec
                    .min(config.max_drain_duration_sec as f32);

                // Triệt tiêu lệnh châm chéo nếu cấu hình đã tắt cảm biến tương ứng
                if !config.enable_ec_sensor {
                    control.nutrient_a_ml = 0.0;
                    control.nutrient_b_ml = 0.0;
                }
                if !config.enable_ph_sensor {
                    control.ph_up_ml = 0.0;
                    control.ph_down_ml = 0.0;
                }
                if !config.enable_water_level_sensor {
                    control.water_in_sec = 0.0;
                    control.water_out_sec = 0.0;
                }

                // LƯỚI BẢO VỆ: Khóa chéo bảo vệ cứng vật lý
                apply_safety_guardrails(&mut control, ec_val, ph_val, w_level, config);

                if control.nutrient_a_ml == 0.0
                    && control.nutrient_b_ml == 0.0
                    && control.ph_up_ml == 0.0
                    && control.ph_down_ml == 0.0
                    && control.water_in_sec == 0.0
                    && control.water_out_sec == 0.0
                    && control.misting_sec == 0.0
                    && control.misting_sec == 0.0
                {
                    return OrchestratorDecision::Idle;
                }

                OrchestratorDecision::ExecuteMimoCycle {
                    control,
                    target_ec: config.ec_target,
                    target_ph: config.ph_target,
                    pwm: config.dosing_pwm_percent as u32,
                }
            }
            None => OrchestratorDecision::Idle,
        }
    }
}

fn check_scheduled_water_change(
    ctx: &SystemContext,
    config: &ControllerConfig,
    now_sec: u64,
    delta: &mut ContextDelta,
) -> Option<OrchestratorDecision> {
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

    let mut control = ControlVector::default();
    control.water_out_sec = config.max_drain_duration_sec as f32;

    Some(OrchestratorDecision::ExecuteMimoCycle {
        control,
        target_ec: config.ec_target,
        target_ph: config.ph_target,
        pwm: config.dosing_pwm_percent as u32,
    })
}

fn check_sensor_noise(
    ctx: &SystemContext,
    sensors: &SensorData,
    config: &ControllerConfig,
    delta: &mut ContextDelta,
) -> bool {
    let mut is_noisy = false;
    let ec_val = sensors.ec;
    let ph_val = sensors.ph;
    let mut peri_delta = PeripheralDelta::default();

    if config.enable_ec_sensor && !sensors.err_ec.unwrap_or(false) {
        if let Some(prev_ec) = ctx.peripherals.previous_ec {
            if (ec_val - prev_ec).abs() > config.max_ec_delta {
                is_noisy = true;
            }
        }
        peri_delta.previous_ec = Some(Some(ec_val));
    }
    if config.enable_ph_sensor && !sensors.err_ph.unwrap_or(false) {
        if let Some(prev_ph) = ctx.peripherals.previous_ph {
            if (ph_val - prev_ph).abs() > config.max_ph_delta {
                is_noisy = true;
            }
        }
        peri_delta.previous_ph = Some(Some(ph_val));
    }

    delta.peripherals = Some(peri_delta);
    is_noisy
}

fn build_result_for_decision(
    decision: OrchestratorDecision,
    ctx: &mut SystemContext,
    config: &ControllerConfig,
    sensors: &SensorData,
    now_ms: u64,
    mut result: TickResult,
) -> TickResult {
    match decision {
        OrchestratorDecision::ExecuteMimoCycle {
            control,
            target_ec,
            target_ph,
            pwm,
        } => {
            let mut peri_delta = result.delta.peripherals.take().unwrap_or_default();

            // Kiểm tra budget chạy an toàn theo giờ từng bơm riêng biệt
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

            // Thực thi giới hạn số chu kỳ xả/refill bồn nước
            if control.water_in_sec > 0.0
                && !ctx
                    .safety
                    .record_refill(now_ms / 1000, config.max_refill_cycles_per_hour as u32)
            {
                warn!("⚠️ [SAFETY] Vượt giới hạn cấp nước/giờ. Bỏ qua chu kỳ.");
                result.delta.phase = Some(SystemPhase::Fault(FaultCode::TooManyRefills));
                return result;
            }
            if control.water_out_sec > 0.0
                && !ctx
                    .safety
                    .record_drain(now_ms / 1000, config.max_drain_cycles_per_hour as u32)
            {
                warn!("⚠️ [SAFETY] Vượt giới hạn xả nước/giờ. Bỏ qua chu kỳ.");
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

            // Bridge logic cho Task 8: Trực tiếp trigger thông qua DosingActor bằng cách sao chép cục bộ
            ctx.dosing
                .start_matrix_cycle(now_ms, &control, target_ec, target_ph, pwm, config, sensors);
            // Ghi chú: Đồng bộ Actor State sẽ xử lý đầy đủ ở Task 8.

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

            let log_payload = serde_json::json!(BasicSystemLogMetadata {
                source: "orchestrator".to_string(),
                message: format!(
                    "A/B: {:.1}ml | pH_Up/Down: {:.1}/{:.1}ml | Water_In: {:.1}s",
                    control.nutrient_a_ml,
                    control.ph_up_ml,
                    control.ph_down_ml,
                    control.water_in_sec
                ),
                skip_reason: None,
                cycle_id: Some(format!("mimo-{now_ms}")),
            })
            .to_string();

            result.events.push(OrchestratorEvent::PublishSystemLog {
                payload_json: log_payload,
            });

            result.delta.peripherals = Some(peri_delta);
        }
        OrchestratorDecision::Fault(code) => {
            result.delta.phase = Some(SystemPhase::Fault(code));
        }
        OrchestratorDecision::Idle => {}
    }
    result
}

pub fn tick(
    now_ms: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    sensor_last_update_ms: u64,
    ctx: &mut SystemContext,
) -> TickResult {
    let mut result = TickResult::default();
    let sensor_timeout_ms: u64 = 90_000;

    if now_ms.saturating_sub(sensor_last_update_ms) > sensor_timeout_ms {
        if !matches!(ctx.phase, SystemPhase::Fault(_)) {
            log::error!(
                "🚨 [SENSOR TIMEOUT] Quá 90s không nhận được gói tin cảm biến mới. Khóa cứng FSM."
            );
            result.delta.phase = Some(SystemPhase::Fault(FaultCode::SensorTimeout));
            result.events.push(OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop,
            });
            result
                .events
                .push(OrchestratorEvent::SetMistValve { on: false });
            result
                .events
                .push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });

            let mut peri_delta = PeripheralDelta::default();
            peri_delta.osaka_pump = Some(false);
            peri_delta.osaka_pwm = Some(0);
            peri_delta.is_misting_active = Some(false);
            peri_delta.is_scheduled_mixing_active = Some(false);
            peri_delta.last_mist_toggle_time = Some(0);
            peri_delta.misting_started_by_dosing = Some(false);
            peri_delta.last_mixing_start_sec = Some(now_ms / 1000);
            result.delta.peripherals = Some(peri_delta);
        }
        return result;
    }

    if check_sensor_noise(ctx, sensors, config, &mut result.delta) {
        return result;
    }

    match &ctx.phase {
        SystemPhase::Monitoring => {
            let now_sec = now_ms / 1000;
            if let Some(decision) =
                check_scheduled_water_change(ctx, config, now_sec, &mut result.delta)
            {
                result = build_result_for_decision(decision, ctx, config, sensors, now_ms, result);
            } else {
                let decision = MonitoringMatrixResult::solve_mimo(sensors, config, ctx);
                result = build_result_for_decision(decision, ctx, config, sensors, now_ms, result);
            }
        }

        SystemPhase::MimoDosing => {
            let elapsed_ms = now_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(now_ms));
            let mut peri_delta = result.delta.peripherals.take().unwrap_or_default();

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

            if now_ms >= ctx.phase_finish_ms.unwrap_or(u64::MAX) + 5_000 {
                warn!("⚠️ [FSM] Dosing phase timeout cứng! Chuyển về Cooldown để tránh kẹt.");
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

            // Stateful Temporary Clone Pattern theo hướng dẫn Task 7/8
            let (dosing_event, hardware_events) = ctx.dosing.tick(now_ms, config);
            result.events.extend(hardware_events);

            match dosing_event {
                DosingEvent::Pending => {
                    if ctx.dosing.is_idle() {
                        let elapsed_ms =
                            now_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(now_ms));
                        let min_water_run_ms = 500_u64;
                        if elapsed_ms >= min_water_run_ms {
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

                            let water_in_spent = if ctx.peripherals.pump_status.water_pump_in {
                                let ms =
                                    elapsed_ms.min(config.max_refill_duration_sec as u64 * 1000);
                                ms as f32 / 1000.0
                            } else {
                                0.0
                            };
                            let water_out_spent = if ctx.peripherals.pump_status.water_pump_out {
                                let ms =
                                    elapsed_ms.min(config.max_drain_duration_sec as u64 * 1000);
                                ms as f32 / 1000.0
                            } else {
                                0.0
                            };

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
                }
                DosingEvent::SoftStartDone => {}
                DosingEvent::PulseToggle { pump, pulse_on } => {
                    let target_pump = match pump {
                        crate::fsm::actors::dosing_actor::PumpTarget::NutrientA { .. } => {
                            DosingPumpTarget::NutrientA
                        }
                        crate::fsm::actors::dosing_actor::PumpTarget::NutrientB => {
                            DosingPumpTarget::NutrientB
                        }
                        crate::fsm::actors::dosing_actor::PumpTarget::PhUp => {
                            DosingPumpTarget::PhUp
                        }
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
                DosingEvent::PhaseTransition => {}
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
        }

        SystemPhase::ActiveMixing => {
            let elapsed_ms = now_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(now_ms));
            let max_mixing_timeout = now_ms >= ctx.phase_finish_ms.unwrap_or(0);

            if (elapsed_ms >= 15_000 && ctx.stabilizer_tracker.is_stable(config))
                || max_mixing_timeout
            {
                result.delta.phase = Some(SystemPhase::Stabilizing);
                result.delta.phase_start_ms = Some(Some(now_ms));
                result.delta.phase_finish_ms = Some(Some(
                    now_ms + ctx.diagnostic.adaptive_stabilize_sec as u64 * 1000,
                ));
                result.delta.reset_stabilizer = true;
            }
        }

        SystemPhase::Stabilizing => {
            let elapsed_ms = now_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(now_ms));
            let min_stabilize_ms = 10_000;
            let max_stabilize_timeout = now_ms >= ctx.phase_finish_ms.unwrap_or(0);

            if (elapsed_ms >= min_stabilize_ms && ctx.stabilizer_tracker.is_stable(config))
                || max_stabilize_timeout
            {
                if let Some(sample) = &ctx.calibration.pending_sample {
                    result.delta.dosing_cycle_count_increment = true;

                    let final_ec = sensors.ec;
                    let final_ph = sensors.ph;
                    let final_water = sensors.water_level;
                    let final_temp = sensors.temp;

                    let actual_delta_ec = final_ec - sample.start_ec;
                    let actual_delta_ph = final_ph - sample.start_ph;
                    let actual_delta_water = final_water - sample.start_water_level;

                    if let Err(hardware_fault_code) = ctx.diagnostic.diagnose_hardware_fault(
                        sample,
                        actual_delta_ec,
                        actual_delta_ph,
                        actual_delta_water,
                        config,
                    ) {
                        result.delta.phase = Some(SystemPhase::Fault(hardware_fault_code));
                        return result;
                    }

                    if !sample.invalid_by_noise && !sample.invalid_by_water_change {
                        result
                            .events
                            .push(OrchestratorEvent::PublishCalibrationUpdate);
                    } else {
                        warn!("⚠️ [GUARDRAIL] Bỏ qua bước cập nhật ma trận Kalman do dữ liệu mẫu bất thường.");
                    }

                    let mut human_message = String::new();
                    if sample.dose_a_ml > 0.0 || sample.dose_b_ml > 0.0 {
                        let total_nutrient = sample.dose_a_ml + sample.dose_b_ml;
                        if config.enable_ec_sensor {
                            if actual_delta_ec > 0.02 {
                                human_message.push_str(&format!(
                                    "Hệ thống đã bổ sung {:.1}ml dinh dưỡng nuôi cây (EC dâng từ {:.2} lên {:.2} mS/cm). ",
                                    total_nutrient, sample.start_ec, final_ec
                                ));
                            } else {
                                human_message.push_str(&format!(
                                    "Hệ thống đã phân phối {:.1}ml dinh dưỡng hòa quyện đồng đều vào bể chứa. ",
                                    total_nutrient
                                ));
                            }
                        } else {
                            human_message.push_str(&format!(
                                "Hệ thống đã bổ sung {:.1}ml dinh dưỡng nuôi cây định lượng tự động theo chu kỳ. ",
                                total_nutrient
                            ));
                        }
                    }

                    if sample.dose_ph_up_ml > 0.01 {
                        human_message.push_str(&format!(
                            "Đã châm {:.1}ml dung dịch kiềm, kéo độ pH bồn phục hồi từ {:.2} về mức an toàn {:.2}. ",
                            sample.dose_ph_up_ml, sample.start_ph, final_ph
                        ));
                    } else if sample.dose_ph_down_ml > 0.01 {
                        human_message.push_str(&format!(
                            "Đã cân bằng axit trung hòa dung dịch bồn, hạ pH từ {:.2} về mức lý tưởng {:.2} (đã dùng {:.1}ml). ",
                            sample.start_ph, final_ph, sample.dose_ph_down_ml
                        ));
                    }

                    if sample.water_in_sec > 0.1 {
                        human_message.push_str(&format!(
                            "Đã mở van bổ sung thêm nước sạch nguồn trong {:.1}s để bù dung tích mực nước về mức {:.1}%. ",
                            sample.water_in_sec, final_water
                        ));
                    } else if sample.water_out_sec > 0.1 {
                        human_message.push_str(&format!(
                            "Đã kích hoạt bơm xả tràn thoát nước bớt trong {:.1}s, đưa hành trình mực nước bồn đạt {:.1}%. ",
                            sample.water_out_sec, final_water
                        ));
                    }

                    if human_message.is_empty() {
                        human_message = format!(
                            "Bộ giải toán MIMO rà soát: Cây đang phát triển lý tưởng, các trục chỉ số đạt trạng thái cân bằng sinh học hoàn hảo (pH: {:.2}, Mực nước: {:.1}%).",
                            final_ph, final_water
                        );
                    }

                    use hydragrow_shared::{DoseData, DosingReportPayload, PhaseData};
                    let report = DosingReportPayload {
                        cycle_id: sample.cycle_id.clone(),
                        trigger: sample.trigger.clone(),
                        pre: PhaseData {
                            ec: sample.start_ec,
                            ph: sample.start_ph,
                            water_level: Some(sample.start_water_level),
                        },
                        dose: DoseData {
                            pump_a_ml: sample.dose_a_ml,
                            pump_b_ml: sample.dose_b_ml,
                            ph_up_ml: sample.dose_ph_up_ml,
                            ph_down_ml: sample.dose_ph_down_ml,
                        },
                        post_mixing: PhaseData {
                            ec: sample.post_mixing_ec,
                            ph: sample.post_mixing_ph,
                            water_level: None,
                        },
                        post_stable: PhaseData {
                            ec: final_ec,
                            ph: final_ph,
                            water_level: None,
                        },
                        delta_ec: actual_delta_ec,
                        delta_ph: actual_delta_ph,
                        target_ec: sample.target_ec,
                        target_ph: sample.target_ph,
                        error_ec: sample.target_ec - final_ec,
                        error_ph: sample.target_ph - final_ph,
                        duration_ms: now_ms.saturating_sub(sample.start_ms),
                        ema_ec_gain_used: config.ec_gain_per_ml,
                        ema_ph_shift_used: config.ph_shift_up_per_ml,
                        step_ratio_ec: Some(ctx.tuner.active_ec_ratio()),
                        step_ratio_ph: Some(ctx.tuner.adaptive_ph_ratio),
                        stabilized_window_sec: Some(ctx.diagnostic.adaptive_stabilize_sec),
                    };

                    if let Ok(json) = serde_json::to_string(&report) {
                        result
                            .events
                            .push(OrchestratorEvent::PublishDosingReport { report_json: json });
                    }

                    let log_payload = serde_json::json!(BasicSystemLogMetadata {
                        source: "mimo_orchestrator".to_string(),
                        message: human_message.trim().to_string(),
                        skip_reason: None,
                        cycle_id: Some(sample.cycle_id.clone()),
                    })
                    .to_string();

                    result.events.push(OrchestratorEvent::PublishSystemLog {
                        payload_json: log_payload,
                    });
                }

                // Chụp runtime snapshot và chuẩn bị lưu xuống Flash qua tầng mod.rs
                let snapshot = NvsSnapshot::from_context(ctx, now_ms / 1000);
                if serde_json::to_string(&snapshot).is_ok() {
                    result.events.push(OrchestratorEvent::SaveNvsSnapshot);
                }

                result.delta.phase = Some(SystemPhase::Cooldown);
                result.delta.phase_finish_ms =
                    Some(Some(now_ms + config.cooldown_sec.max(0) as u64 * 1000));
            }
        }

        SystemPhase::Cooldown => {
            if now_ms >= ctx.phase_finish_ms.unwrap_or(0) {
                result.delta.phase = Some(SystemPhase::Monitoring);
                result.delta.phase_start_ms = Some(None);
                result.delta.phase_finish_ms = Some(None);
            }
        }
        _ => {}
    }

    let is_dosing_active = matches!(
        ctx.phase,
        SystemPhase::MimoDosing | SystemPhase::ActiveMixing | SystemPhase::Stabilizing
    );

    let mut peri_delta = result.delta.peripherals.take().unwrap_or_default();
    if is_dosing_active {
        peri_delta.is_scheduled_mixing_active = Some(false);
    }
    result.delta.peripherals = Some(peri_delta);

    result
}

