// src/core/fsm/phases/monitoring.rs

use cron::Schedule;
use hydragrow_shared::fsm::{FaultCode, SystemPhase};
use hydragrow_shared::log::{LogCategory, LogLevel, UnifiedSystemLog};
use hydragrow_shared::{ControllerConfig, SensorData};
use log::warn;
use std::str::FromStr;

use crate::WaterDirection;
use crate::core::actors::dosing_actor::{DosingPlanResult, calculate_channel_dosing_duration_ms};
use crate::core::adaptive::matrix::ControlVector;
use crate::core::adaptive::solver::{SolveResult, select_solver};
use crate::core::fsm::context::SystemContext;
use crate::core::fsm::events::OrchestratorEvent;
use crate::core::fsm::phase_tick::PhaseTick;
use crate::core::fsm::tick_result::{CalibrationDelta, ContextDelta, TickResult};
use crate::core::optimizer::plan_water_operation;
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
            check_scheduled_water_change(ctx, config, now_sec, &mut result.delta, sensors)
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
// Thứ tự ưu tiên (Precedence): stage interval > static interval > cron
fn check_scheduled_water_change(
    ctx: &SystemContext,
    config: &ControllerConfig,
    now_sec: u64,
    delta: &mut ContextDelta,
    sensors: &SensorData,
) -> Option<SolveResult> {
    if !config.enable_water_level_sensor || !config.scheduled_water_change_enabled {
        return None;
    }

    // 1. Stage interval: Ưu tiên cao nhất nếu có active recipe và stage cấu hình interval
    let stage_interval = config
        .active_recipe
        .as_ref()
        .and_then(|r| ctx.current_stage_index.and_then(|idx| r.stages.get(idx)))
        .and_then(|s| s.water_change_interval_days);

    let (effective_days, is_interval) = if let Some(days) = stage_interval {
        (Some(days), true)
    } else if let Some(days) = config.water_change_interval_days {
        // 2. Static interval: Ưu tiên nhì nếu config có cấu hình ngày
        (Some(days), true)
    } else {
        // 3. Cron: Ưu tiên ba
        (None, false)
    };

    if is_interval {
        let days = effective_days.unwrap_or(0);
        if days == 0 {
            return None;
        }

        let interval_sec = (days as u64) * 86400;
        let base_sec = if ctx.last_water_change_sec != 0 {
            ctx.last_water_change_sec
        } else if let Some(recipe) = &config.active_recipe {
            recipe.start_time_sec
        } else {
            now_sec
        };

        let next_due = base_sec.saturating_add(interval_sec);
        if ctx.next_water_change_trigger_sec != Some(next_due) {
            delta.next_water_change_trigger_sec = Some(Some(next_due));
        }

        if now_sec < next_due {
            return None;
        }

        delta.last_water_change_sec = Some(now_sec);
        delta.next_water_change_trigger_sec = Some(Some(now_sec.saturating_add(interval_sec)));
    } else if !config.water_change_cron.trim().is_empty() {
        let schedule = match Schedule::from_str(&config.water_change_cron) {
            Ok(s) => s,
            Err(e) => {
                warn!("  [WATER_CHANGE] Parse cron biểu thức thất bại: {:?}", e);
                return None;
            }
        };

        let now_utc = chrono::DateTime::from_timestamp(now_sec as i64, 0).unwrap_or_default();
        let mut current_next_trigger = ctx.next_water_change_trigger_sec;

        if ctx.water_change_cron != config.water_change_cron || current_next_trigger.is_none() {
            delta.water_change_cron = Some(config.water_change_cron.clone());
            if let Some(next_dt) = schedule.after(&now_utc).next() {
                let ts = next_dt.timestamp() as u64;
                delta.next_water_change_trigger_sec = Some(Some(ts));
                current_next_trigger = Some(ts);
            }
        }

        let next_trigger = current_next_trigger?;
        if now_sec < next_trigger {
            return None;
        }

        let future_utc = now_utc + chrono::Duration::seconds(1);
        if let Some(next_dt) = schedule.after(&future_utc).next() {
            delta.next_water_change_trigger_sec = Some(Some(next_dt.timestamp() as u64));
        }
        delta.last_water_change_sec = Some(now_sec);
    } else {
        return None;
    }

    let drain_amount = if let Some(recipe) = &config.active_recipe {
        ctx.current_stage_index
            .and_then(|idx| recipe.stages.get(idx))
            .and_then(|s| s.water_change_drain_cm)
            .unwrap_or(config.scheduled_drain_amount_cm)
    } else {
        config.scheduled_drain_amount_cm
    };

    let plan = match plan_water_operation(
        WaterDirection::Out,
        drain_amount,
        sensors.water_level,
        None,
        config,
    ) {
        Some(p) => p,
        None => {
            warn!(
                "  [WATER_CHANGE] Kế hoạch xả nước định kỳ bị từ chối do vi phạm an toàn mực nước."
            );
            return None;
        }
    };

    let mut peri_delta = delta.peripherals.take().unwrap_or_default();
    peri_delta.last_mixing_start_sec = Some(now_sec);
    peri_delta.is_scheduled_mixing_active = Some(false);
    delta.peripherals = Some(peri_delta);
    delta.calibration = Some(CalibrationDelta::Invalidate);

    let control = ControlVector {
        water_out_sec: plan.duration_sec,
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
            let override_active = ctx.safety.is_override_active(uptime_ms);

            if !override_active {
                let requested_ec_ml = control.nutrient_a_ml + control.nutrient_b_ml;
                let requested_ph_ml = control.ph_up_ml + control.ph_down_ml;

                if requested_ec_ml > 0.0
                    && !ctx.safety.peek_total_hourly_dose(
                        uptime_sec,
                        requested_ec_ml,
                        config.max_dose_per_hour,
                    )
                {
                    result.delta.phase = Some(SystemPhase::Fault(FaultCode::MaxHourlyDoseEc));
                    return result;
                }

                if requested_ph_ml > 0.0
                    && !ctx.safety.peek_total_hourly_dose(
                        uptime_sec,
                        requested_ec_ml + requested_ph_ml,
                        config.max_dose_per_hour,
                    )
                {
                    result.delta.phase = Some(SystemPhase::Fault(FaultCode::MaxHourlyDosePh));
                    return result;
                }

                if control.water_in_sec > 0.0
                    && !ctx
                        .safety
                        .peek_refill(uptime_sec, config.max_refill_cycles_per_hour as u32)
                {
                    warn!("  [SAFETY] Vượt quá giới hạn chu kỳ cấp nước / giờ.");
                    result.delta.phase = Some(SystemPhase::Fault(FaultCode::TooManyRefills));
                    return result;
                }

                if control.water_out_sec > 0.0
                    && !ctx
                        .safety
                        .peek_drain(uptime_sec, config.max_drain_cycles_per_hour as u32)
                {
                    warn!("  [SAFETY] Vượt quá giới hạn chu kỳ xả nước / giờ.");
                    result.delta.phase = Some(SystemPhase::Fault(FaultCode::TooManyDrains));
                    return result;
                }
            }

            // Ghi nhận (commit) transactional khi tất cả các kiểm tra an toàn đều đã đạt
            if control.water_in_sec > 0.0 {
                ctx.safety
                    .record_refill(uptime_sec, config.max_refill_cycles_per_hour as u32);
            }
            if control.water_out_sec > 0.0 {
                ctx.safety
                    .record_drain(uptime_sec, config.max_drain_cycles_per_hour as u32);
            }

            // =========================================================================
            // LÊN LỊCH PHẦN CỨNG (NƯỚC, SƯƠNG, BƠM ĐỊNH LƯỢNG)
            // =========================================================================
            if control.water_in_sec > 0.0 {
                result.events.push(OrchestratorEvent::SetWaterPump {
                    direction: WaterDirection::In,
                });
                peri_delta.water_pump_in = Some(true);
                if !ctx.peripherals.pump_status.water_pump_in {
                    peri_delta.water_pump_started_uptime_ms = Some(Some(uptime_ms));
                }
                let trigger = if is_water_change {
                    "scheduled_water_change"
                } else {
                    "mimo_dosing"
                };
                ctx.water
                    .start_fill(uptime_ms, config.water_level_target, sensors, trigger);
            }
            if control.water_out_sec > 0.0 {
                result.events.push(OrchestratorEvent::SetWaterPump {
                    direction: WaterDirection::Out,
                });
                peri_delta.water_pump_out = Some(true);
                if !ctx.peripherals.pump_status.water_pump_out {
                    peri_delta.water_pump_started_uptime_ms = Some(Some(uptime_ms));
                }
                let (target_level, trigger) = if is_water_change {
                    let drain_amount = if let Some(recipe) = &config.active_recipe {
                        ctx.current_stage_index
                            .and_then(|idx| recipe.stages.get(idx))
                            .and_then(|s| s.water_change_drain_cm)
                            .unwrap_or(config.scheduled_drain_amount_cm)
                    } else {
                        config.scheduled_drain_amount_cm
                    };
                    let target = if drain_amount > 0.0 {
                        (sensors.water_level - drain_amount).max(config.water_level_critical_min)
                    } else {
                        config.water_level_critical_min
                    };
                    (target, "scheduled_water_change")
                } else {
                    (config.water_level_target, "mimo_dosing")
                };

                ctx.water
                    .start_drain(uptime_ms, target_level, sensors, trigger);
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

            // Truyền uptime_ms vào DosingActor để lập kế hoạch châm định lượng transactional
            let dosing_plan = ctx.dosing.start_matrix_cycle(
                uptime_ms, &control, target_ec, target_ph, pwm, config, sensors,
            );

            let mut active_dosing_jobs = 0;
            if let DosingPlanResult::Prepared(ref jobs) = dosing_plan {
                for job in jobs {
                    let pump_name = match job.pump {
                        DosePumpKind::PumpA => "NutrientA",
                        DosePumpKind::PumpB => "NutrientB",
                        DosePumpKind::PhUp => {
                            peri_delta.ph_up = Some(true);
                            "PhUp"
                        }
                        DosePumpKind::PhDown => {
                            peri_delta.ph_down = Some(true);
                            "PhDown"
                        }
                    };
                    ctx.safety
                        .commit_hourly_dose(pump_name, uptime_sec, job.target_ml);
                    active_dosing_jobs += 1;
                }
            }

            let water_active = control.water_in_sec > 0.0 || control.water_out_sec > 0.0;
            let misting_active = control.misting_sec > 0.0;

            if active_dosing_jobs == 0 && !water_active && !misting_active {
                warn!(
                    "  [MONITORING] Không có tác vụ khả thi nào được chuẩn bị. Giữ nguyên trạng thái Monitoring."
                );
                result.delta.peripherals = Some(peri_delta);
                return result;
            }

            let mut dosing_time_ms = 0u64;
            if active_dosing_jobs > 0 {
                dosing_time_ms += config.soft_start_duration as u64;
                if control.nutrient_a_ml > 0.0 {
                    let flow_a = effective_flow_ml_per_sec(DosePumpKind::PumpA, safe_pwm, config)
                        .unwrap_or(1.0);
                    dosing_time_ms +=
                        calculate_channel_dosing_duration_ms(control.nutrient_a_ml, flow_a, config);

                    // Trạm FSM bơm theo tuần tự: Bơm A xong sẽ trễ (delay) rồi mới bơm B
                    if control.nutrient_b_ml > 0.0 {
                        dosing_time_ms += (config.delay_between_a_and_b_sec as u64) * 1000;
                    }
                }
                if control.nutrient_b_ml > 0.0 {
                    let flow_b = effective_flow_ml_per_sec(DosePumpKind::PumpB, safe_pwm, config)
                        .unwrap_or(1.0);
                    dosing_time_ms +=
                        calculate_channel_dosing_duration_ms(control.nutrient_b_ml, flow_b, config);
                }
                if control.ph_up_ml > 0.0 {
                    let flow_up = effective_flow_ml_per_sec(DosePumpKind::PhUp, safe_pwm, config)
                        .unwrap_or(1.0);
                    dosing_time_ms +=
                        calculate_channel_dosing_duration_ms(control.ph_up_ml, flow_up, config);
                }
                if control.ph_down_ml > 0.0 {
                    let flow_down =
                        effective_flow_ml_per_sec(DosePumpKind::PhDown, safe_pwm, config)
                            .unwrap_or(1.0);
                    dosing_time_ms +=
                        calculate_channel_dosing_duration_ms(control.ph_down_ml, flow_down, config);
                }
            }

            let water_time_ms = (control.water_in_sec.max(control.water_out_sec) * 1000.0) as u64;
            let misting_time_ms = (control.misting_sec * 1000.0) as u64;

            // Chọn ra khoảng thời gian lớn nhất giữa tất cả các phần cứng đang chạy
            let hardware_run_ms = water_time_ms.max(misting_time_ms).max(dosing_time_ms);

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
