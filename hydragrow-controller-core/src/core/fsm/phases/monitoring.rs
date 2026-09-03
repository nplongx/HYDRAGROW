// src/core/fsm/phases/monitoring.rs

use chrono::Local;
use cron::Schedule;
use hydragrow_shared::fsm::{FaultCode, SystemPhase};
use hydragrow_shared::log::{LogCategory, LogLevel, UnifiedSystemLog};
use hydragrow_shared::{ControllerConfig, SensorData};
use log::warn;
use std::str::FromStr;

use crate::WaterDirection;
use crate::core::adaptive::matrix::ControlVector;
use crate::core::adaptive::solver::{SolveResult, select_solver};
use crate::core::fsm::context::SystemContext;
use crate::core::fsm::events::OrchestratorEvent;
use crate::core::fsm::phase_tick::PhaseTick;
use crate::core::fsm::tick_result::{CalibrationDelta, ContextDelta, TickResult};
use crate::utils::{DosePumpKind, effective_flow_ml_per_sec};

pub struct MonitoringPhase;

impl PhaseTick for MonitoringPhase {
    fn tick(
        &self,
        now_ms: u64,
        uptime_ms: u64, // Dùng uptime_ms từ interface để chống nhiễu NTP
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();

        // Cronjob dùng giờ Wall Time
        let now_sec = now_ms / 1000;

        // 1. Kiểm tra lịch xả/thay nước định kỳ (Cronjob)
        if let Some(water_change_result) =
            check_scheduled_water_change(ctx, config, now_sec, &mut result.delta)
        {
            // Truyền thêm uptime_ms vào apply_decision
            return apply_decision(
                water_change_result,
                ctx,
                config,
                sensors,
                now_ms,
                uptime_ms,
                result,
                true,
            );
        }

        // 2. Chạy Solver (ColdPath hoặc WarmPath tùy trạng thái ma trận)
        let solver = select_solver(ctx);
        let decision = solver.solve(sensors, config, ctx);

        apply_decision(
            decision, ctx, config, sensors, now_ms, uptime_ms, result, false,
        )
    }
}

// Hàm kiểm tra lịch trình xả nước định kỳ (chạy theo Wall Time `now_sec`)
fn check_scheduled_water_change(
    ctx: &SystemContext,
    config: &ControllerConfig,
    now_sec: u64,
    delta: &mut ContextDelta,
) -> Option<SolveResult> {
    if !(config.enable_water_level_sensor
        && config.scheduled_water_change_enabled
        && !config.water_change_cron.is_empty())
    {
        return None;
    }

    let mut current_next_trigger = ctx.next_water_change_trigger_sec;
    if ctx.water_change_cron != config.water_change_cron {
        delta.water_change_cron = Some(config.water_change_cron.clone());
        if let Ok(schedule) = Schedule::from_str(&config.water_change_cron)
            && let Some(next) = schedule.upcoming(Local).next()
        {
            let ts = next.timestamp() as u64;
            delta.next_water_change_trigger_sec = Some(Some(ts));
            current_next_trigger = Some(ts);
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

    let control = ControlVector {
        water_out_sec: config.max_drain_duration_sec as f32,
        ..Default::default()
    };

    Some(SolveResult::Execute {
        control,
        target_ec: config.ec_target,
        target_ph: config.ph_target,
        pwm: config.dosing_pwm_percent as u32,
    })
}

// Xử lý và áp dụng quyết định từ AI/Solver xuống Hardware
#[allow(clippy::too_many_arguments)]
fn apply_decision(
    decision: SolveResult,
    ctx: &mut SystemContext,
    config: &ControllerConfig,
    sensors: &SensorData,
    now_ms: u64,
    uptime_ms: u64, // Nhận thêm tham số uptime_ms
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

            // Tính số giây uptime cho Safety Budget (Miễn nhiễm rủi ro do NTP Jump)
            let uptime_sec = uptime_ms / 1000;

            if is_water_change {
                ctx.tuner.on_water_change();
                log::info!("  [MONITORING] Thay nước theo lịch: Reset AutoTuner trackers.");
                result.events.push(OrchestratorEvent::SaveLastWaterChange {
                    timestamp_sec: now_ms / 1000, // Ghi NVS theo giờ Wall Time
                });
            }

            // =========================================================================
            // KIỂM TRA LƯỚI AN TOÀN (SAFETY BUDGETS) DỰA TRÊN UPTIME_SEC
            // =========================================================================
            let nutrient_a_ok = control.nutrient_a_ml <= 0.0
                || ctx.safety.peek_hourly_dose(
                    "NutrientA",
                    uptime_sec,
                    control.nutrient_a_ml,
                    config.max_dose_per_hour / 2.0,
                );
            let nutrient_b_ok = control.nutrient_b_ml <= 0.0
                || ctx.safety.peek_hourly_dose(
                    "NutrientB",
                    uptime_sec,
                    control.nutrient_b_ml,
                    config.max_dose_per_hour / 2.0,
                );

            if !nutrient_a_ok || !nutrient_b_ok {
                peri_delta.pump_a = Some(control.nutrient_a_ml > 0.0);
                peri_delta.pump_b = Some(control.nutrient_b_ml > 0.0);
                result.delta.phase = Some(SystemPhase::Fault(FaultCode::MaxHourlyDoseEc));
                result.delta.peripherals = Some(peri_delta);
                return result;
            }

            if control.nutrient_a_ml > 0.0 {
                ctx.safety
                    .commit_hourly_dose("NutrientA", uptime_sec, control.nutrient_a_ml);
            }
            if control.nutrient_b_ml > 0.0 {
                ctx.safety
                    .commit_hourly_dose("NutrientB", uptime_sec, control.nutrient_b_ml);
            }
            if control.ph_up_ml > 0.0 {
                let _ = ctx.safety.check_hourly_dose(
                    "PhUp",
                    uptime_sec,
                    control.ph_up_ml,
                    config.max_dose_per_hour / 4.0,
                );
                peri_delta.ph_up = Some(true);
            }
            if control.ph_down_ml > 0.0 {
                let _ = ctx.safety.check_hourly_dose(
                    "PhDown",
                    uptime_sec,
                    control.ph_down_ml,
                    config.max_dose_per_hour / 4.0,
                );
                peri_delta.ph_down = Some(true);
            }
            if control.water_in_sec > 0.0
                && !ctx
                    .safety
                    .record_refill(uptime_sec, config.max_refill_cycles_per_hour as u32)
            {
                warn!("  [SAFETY] Vượt quá giới hạn chu kỳ cấp nước / giờ.");
                result.delta.phase = Some(SystemPhase::Fault(FaultCode::TooManyRefills));
                return result;
            }
            if control.water_out_sec > 0.0
                && !ctx
                    .safety
                    .record_drain(uptime_sec, config.max_drain_cycles_per_hour as u32)
            {
                warn!("  [SAFETY] Vượt quá giới hạn chu kỳ xả nước / giờ.");
                result.delta.phase = Some(SystemPhase::Fault(FaultCode::TooManyDrains));
                return result;
            }

            // =========================================================================
            // LÊN LỊCH PHẦN CỨNG (NƯỚC, SƯƠNG, BƠM ĐỊNH LƯỢNG)
            // =========================================================================
            if control.water_in_sec > 0.0 {
                result.events.push(OrchestratorEvent::SetWaterPump {
                    direction: WaterDirection::In,
                });
                peri_delta.water_pump_in = Some(true);
                peri_delta.water_pump_started_uptime_ms = Some(Some(uptime_ms));
            }
            if control.water_out_sec > 0.0 {
                result.events.push(OrchestratorEvent::SetWaterPump {
                    direction: WaterDirection::Out,
                });
                peri_delta.water_pump_out = Some(true);
                peri_delta.water_pump_started_uptime_ms = Some(Some(uptime_ms));
            }
            if control.misting_sec > 0.0 {
                result
                    .events
                    .push(OrchestratorEvent::SetMistValve { on: true });
                peri_delta.mist_valve = Some(true);
                peri_delta.is_misting_active = Some(true);
                // Đánh dấu để Phun sương do MIMO yêu cầu không bị ghi đè bởi Phun sương định kỳ
                peri_delta.misting_started_by_dosing = Some(true);
            }

            // =================================================================
            // [VÁ BUG]: TÍNH TOÁN CHÍNH XÁC THỜI GIAN CẦN THIẾT CHO DOSING ACTOR
            // =================================================================
            let safe_pwm = pwm.clamp(1, 100);
            let mut dosing_time_sec = (config.soft_start_duration as f32) / 1000.0;

            if control.nutrient_a_ml > 0.0 {
                let flow_a =
                    effective_flow_ml_per_sec(DosePumpKind::PumpA, safe_pwm, config).unwrap_or(1.0);
                dosing_time_sec += control.nutrient_a_ml / flow_a;

                // Trạm FSM bơm theo tuần tự: Bơm A xong sẽ trễ (delay) rồi mới bơm B
                if control.nutrient_b_ml > 0.0 {
                    dosing_time_sec += config.delay_between_a_and_b_sec as f32;
                }
            }
            if control.nutrient_b_ml > 0.0 {
                let flow_b =
                    effective_flow_ml_per_sec(DosePumpKind::PumpB, safe_pwm, config).unwrap_or(1.0);
                dosing_time_sec += control.nutrient_b_ml / flow_b;
            }
            if control.ph_up_ml > 0.0 {
                let flow_up =
                    effective_flow_ml_per_sec(DosePumpKind::PhUp, safe_pwm, config).unwrap_or(1.0);
                dosing_time_sec += control.ph_up_ml / flow_up;
            }
            if control.ph_down_ml > 0.0 {
                let flow_down = effective_flow_ml_per_sec(DosePumpKind::PhDown, safe_pwm, config)
                    .unwrap_or(1.0);
                dosing_time_sec += control.ph_down_ml / flow_down;
            }

            // Chọn ra khoảng thời gian lớn nhất giữa tất cả các phần cứng đang chạy
            let hardware_run_ms = (control
                .water_in_sec
                .max(control.water_out_sec)
                .max(control.misting_sec)
                .max(dosing_time_sec)
                * 1000.0) as u64;

            // Truyền uptime_ms vào DosingActor để bơm xung PWM chính xác trên Monotonic Time
            ctx.dosing.start_matrix_cycle(
                uptime_ms, &control, target_ec, target_ph, pwm, config, sensors,
            );

            // =========================================================================
            // CẬP NHẬT TRẠNG THÁI VÀ TIMEOUT CHO PHA MIMO DOSING BẰNG UPTIME_MS
            // =========================================================================
            result.delta.phase = Some(SystemPhase::MimoDosing);
            result.delta.phase_start_ms = Some(Some(uptime_ms));

            // Timeout cứng của toàn bộ Phase (Cộng dư thêm 5 giây an toàn)
            result.delta.phase_finish_ms = Some(Some(uptime_ms + hardware_run_ms + 5000));

            peri_delta.last_ec_before_dose = Some(Some(sensors.ec));
            peri_delta.last_ph_before_dose = Some(Some(sensors.ph));
            result.delta.reset_stabilizer = true;

            // Log cho con người đọc vẫn giữ nguyên Wall Time (`now_ms`)
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
