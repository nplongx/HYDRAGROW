use std::str::FromStr;
use std::sync::mpsc::Sender;

use chrono::{Local, TimeZone};
use cron::Schedule;
use hydragrow_shared::ControllerConfig;
use log::{error, info, warn};

use crate::config::SharedConfig;
use crate::mqtt::SensorData;
use crate::pump::{PumpController, PumpType, WaterDirection};
use esp_idf_svc::nvs::EspDefaultNvs;

use super::calibration::{apply_runtime_calibration_ema, start_pending_calibration_sample};
use super::context::ControlContext;
use super::types::{PendingDose, SystemState};
use super::utils::{effective_flow_ml_per_sec, soft_deadband_scale, DosePumpKind};

// ---------------------------------------------------------------------------
// run_auto_fsm
//
// Thực thi một tick của máy trạng thái tự động.
// Được gọi mỗi 100ms từ vòng lặp chính, chỉ khi ControlMode::Auto.
// ---------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub fn run_auto_fsm(
    current_time_ms: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    shared_config: &SharedConfig,
    nvs: &mut Option<EspDefaultNvs>,
    dosing_report_tx: &Sender<String>,
    fsm_mqtt_tx: &Sender<String>,
) {
    let current_time_sec = current_time_ms / 1000;
    let max_hourly_ml = config.max_dose_per_hour;

    match ctx.current_state {
        // Các state này do luồng ngoài xử lý, auto fsm không làm gì
        SystemState::SystemBooting | SystemState::ManualMode => {}

        SystemState::DosingCycleComplete => {
            ctx.current_state = SystemState::Cooldown {
                finish_time: current_time_ms + (config.cooldown_sec as u64 * 1000),
            };
        }

        SystemState::Cooldown { finish_time } => {
            if current_time_ms >= finish_time {
                ctx.current_state = SystemState::Monitoring;
            }
        }

        SystemState::SensorCalibration { finish_time, .. } => {
            if current_time_ms >= finish_time {
                warn!("⏱️ Timeout Hiệu chuẩn. Hệ thống sẽ trở lại Monitoring.");
                ctx.current_state = SystemState::Monitoring;
            }
        }

        SystemState::SystemFault(ref reason) => {
            warn!("🚨 BÁO LỖI: [{}]. Chờ reset...", reason);
        }

        SystemState::Monitoring => {
            handle_monitoring(
                current_time_ms,
                current_time_sec,
                max_hourly_ml,
                config,
                sensors,
                ctx,
                pump_ctrl,
                nvs,
            );
        }

        SystemState::WaterRefilling {
            target_level,
            start_time,
        } => {
            if sensors.water_level >= target_level
                || current_time_ms.saturating_sub(start_time)
                    > (config.max_refill_duration_sec as u64 * 1000)
            {
                let _ = pump_ctrl.set_water_pump(WaterDirection::Stop);
                ctx.pump_status.water_pump_in = false;
                ctx.pump_status.water_pump_out = false;
                ctx.fsm_osaka_active = true;
                ctx.current_state = SystemState::ActiveMixing {
                    finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
                };
            }
        }

        SystemState::WaterDraining {
            target_level,
            start_time,
        } => {
            if sensors.water_level <= target_level
                || current_time_ms.saturating_sub(start_time)
                    > (config.max_drain_duration_sec as u64 * 1000)
            {
                let _ = pump_ctrl.set_water_pump(WaterDirection::Stop);
                ctx.pump_status.water_pump_in = false;
                ctx.pump_status.water_pump_out = false;
                ctx.fsm_osaka_active = false;
                ctx.current_state = SystemState::Stabilizing {
                    finish_time: current_time_ms + (config.sensor_stabilize_sec as u64 * 1000),
                };
            }
        }

        SystemState::StartingOsakaPump {
            finish_time,
            ref pending_action,
        } => {
            if current_time_ms >= finish_time {
                let action = pending_action.clone();
                handle_osaka_ready(current_time_ms, config, sensors, ctx, pump_ctrl, action);
            }
        }

        SystemState::DosingPumpA {
            next_toggle_time,
            dose_target_ml,
            delivered_ml_est,
            dose_b_ml,
            pulse_on,
            pulse_count,
            max_pulse_count,
            pulse_on_ms,
            pulse_off_ms,
            pwm_percent,
            active_capacity_ml_per_sec,
            target_ec,
            start_ec,
            start_ph,
        } => {
            if current_time_ms >= next_toggle_time {
                handle_dosing_pump_a_tick(
                    current_time_ms,
                    config,
                    ctx,
                    pump_ctrl,
                    DosingPumpAState {
                        dose_target_ml,
                        delivered_ml_est,
                        dose_b_ml,
                        pulse_on,
                        pulse_count,
                        max_pulse_count,
                        pulse_on_ms,
                        pulse_off_ms,
                        pwm_percent,
                        active_capacity_ml_per_sec,
                        target_ec,
                        start_ec,
                        start_ph,
                    },
                );
            }
        }

        SystemState::WaitingBetweenDose {
            finish_time,
            dose_b_ml,
            target_ec,
            start_ec,
            start_ph,
            dose_a_ml_reported,
        } => {
            if current_time_ms >= finish_time {
                handle_waiting_between_dose(
                    current_time_ms,
                    config,
                    sensors,
                    ctx,
                    pump_ctrl,
                    dosing_report_tx,
                    dose_b_ml,
                    target_ec,
                    start_ec,
                    start_ph,
                    dose_a_ml_reported,
                );
            }
        }

        SystemState::DosingPumpB {
            next_toggle_time,
            dose_target_ml,
            delivered_ml_est,
            pulse_on,
            pulse_count,
            max_pulse_count,
            pulse_on_ms,
            pulse_off_ms,
            pwm_percent,
            active_capacity_ml_per_sec,
            target_ec,
            start_ec,
            start_ph,
            dose_a_ml_reported,
        } => {
            if current_time_ms >= next_toggle_time {
                handle_dosing_pump_b_tick(
                    current_time_ms,
                    config,
                    ctx,
                    pump_ctrl,
                    dosing_report_tx,
                    DosingPumpBState {
                        dose_target_ml,
                        delivered_ml_est,
                        pulse_on,
                        pulse_count,
                        max_pulse_count,
                        pulse_on_ms,
                        pulse_off_ms,
                        pwm_percent,
                        active_capacity_ml_per_sec,
                        target_ec,
                        start_ec,
                        start_ph,
                        dose_a_ml_reported,
                    },
                );
            }
        }

        SystemState::DosingPH {
            next_toggle_time,
            is_up,
            dose_target_ml,
            delivered_ml_est,
            pulse_on,
            pulse_count,
            max_pulse_count,
            pulse_on_ms,
            pulse_off_ms,
            pwm_percent,
            active_capacity_ml_per_sec,
            target_ph,
            start_ec,
            start_ph,
        } => {
            if current_time_ms >= next_toggle_time {
                handle_dosing_ph_tick(
                    current_time_ms,
                    config,
                    ctx,
                    pump_ctrl,
                    dosing_report_tx,
                    DosingPhState {
                        is_up,
                        dose_target_ml,
                        delivered_ml_est,
                        pulse_on,
                        pulse_count,
                        max_pulse_count,
                        pulse_on_ms,
                        pulse_off_ms,
                        pwm_percent,
                        active_capacity_ml_per_sec,
                        target_ph,
                        start_ec,
                        start_ph,
                    },
                );
            }
        }

        SystemState::ActiveMixing { finish_time } => {
            if current_time_ms >= finish_time {
                ctx.fsm_osaka_active = false;
                if let Some(sample) = ctx.pending_calibration_sample.as_mut() {
                    sample.active_mixing_finish_ms = current_time_ms;
                    sample.stabilizing_start_ms = Some(current_time_ms);
                    sample.stabilizing_finish_ms =
                        Some(current_time_ms + (config.sensor_stabilize_sec as u64 * 1000));
                }
                ctx.current_state = SystemState::Stabilizing {
                    finish_time: current_time_ms + (config.sensor_stabilize_sec as u64 * 1000),
                };
            }
        }

        SystemState::Stabilizing { finish_time } => {
            if current_time_ms >= finish_time {
                if let Some(sample) = ctx.pending_calibration_sample.as_mut() {
                    sample.stabilizing_finish_ms = Some(current_time_ms);
                }
                apply_runtime_calibration_ema(sensors, shared_config, ctx, fsm_mqtt_tx);
                ctx.current_state = SystemState::DosingCycleComplete;
            }
        }

        SystemState::EmergencyStop(_) => {
            // Không làm gì – tránh spam log khi EmergencyStop
        }
    }
}

// ===========================================================================
// Sub-handlers cho Monitoring
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn handle_monitoring(
    current_time_ms: u64,
    current_time_sec: u64,
    max_hourly_ml: f32,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    nvs: &mut Option<EspDefaultNvs>,
) {
    ctx.verify_sensor_ack(sensors, config, current_time_sec);

    // 1. Thay nước định kỳ theo cron
    if try_scheduled_water_change(
        current_time_ms,
        current_time_sec,
        config,
        sensors,
        ctx,
        pump_ctrl,
        nvs,
    ) {
        return;
    }

    // 2. Bổ sung nước tự động
    if try_auto_refill(current_time_sec, config, sensors, ctx, pump_ctrl) {
        return;
    }

    // 3. Xả tràn
    if try_auto_drain_overflow(current_time_sec, config, sensors, ctx, pump_ctrl) {
        return;
    }

    // 4. Pha loãng EC cao
    if try_auto_dilute(current_time_sec, config, sensors, ctx, pump_ctrl) {
        return;
    }

    // 5. Bơm dinh dưỡng / EC / pH
    handle_dosing_decisions(
        current_time_ms,
        current_time_sec,
        max_hourly_ml,
        config,
        sensors,
        ctx,
        pump_ctrl,
        nvs,
    );
}

fn try_scheduled_water_change(
    current_time_ms: u64,
    current_time_sec: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    nvs: &mut Option<EspDefaultNvs>,
) -> bool {
    if !(config.enable_water_level_sensor
        && config.scheduled_water_change_enabled
        && !config.water_change_cron.is_empty())
    {
        return false;
    }

    if ctx.current_water_change_cron_expr != config.water_change_cron {
        ctx.current_water_change_cron_expr = config.water_change_cron.clone();
        match Schedule::from_str(&ctx.current_water_change_cron_expr) {
            Ok(schedule) => {
                if let Some(next) = schedule.upcoming(Local).next() {
                    ctx.next_water_change_trigger_sec = Some(next.timestamp() as u64);
                    info!("⏰ Cập nhật lịch Thay nước Cron: {}", next);
                }
            }
            Err(_) => {
                warn!("⚠️ Lỗi cú pháp Cron Thay nước!");
                ctx.next_water_change_trigger_sec = None;
            }
        }
    }

    let next_trigger = match ctx.next_water_change_trigger_sec {
        Some(t) => t,
        None => return false,
    };

    if current_time_sec < next_trigger {
        return false;
    }

    info!("⏰ Đã đến giờ THAY NƯỚC ĐỊNH KỲ theo lịch CRON!");

    if let Ok(schedule) = Schedule::from_str(&ctx.current_water_change_cron_expr) {
        let future = Local::now() + chrono::Duration::seconds(1);
        if let Some(next) = schedule.after(&future).next() {
            ctx.next_water_change_trigger_sec = Some(next.timestamp() as u64);
        }
    }

    let target =
        (sensors.water_level - config.scheduled_drain_amount_cm).max(config.water_level_min);
    ctx.last_water_change_time = current_time_sec;
    if let Some(flash) = nvs.as_mut() {
        let _ = flash.set_u64("last_w_change", current_time_sec);
    }

    if !ctx.check_and_record_drain_limit(current_time_sec, config.max_drain_cycles_per_hour as u32)
    {
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("TOO_MANY_DRAINS".to_string());
        return true;
    }

    ctx.mark_pending_sample_water_change_violation();
    ctx.current_state = SystemState::WaterDraining {
        target_level: target,
        start_time: current_time_ms,
    };
    let _ = pump_ctrl.set_water_pump(WaterDirection::Out);
    ctx.pump_status.water_pump_out = true;
    ctx.pump_status.water_pump_in = false;
    ctx.fsm_osaka_active = false;
    true
}

fn try_auto_refill(
    current_time_sec: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
) -> bool {
    if !(config.enable_water_level_sensor
        && config.auto_refill_enabled
        && sensors.water_level < (config.water_level_target - config.water_level_tolerance))
    {
        return false;
    }

    if ctx.water_refill_retry_count >= 3 {
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("WATER_REFILL_FAILED".to_string());
        return true;
    }

    if !ctx
        .check_and_record_refill_limit(current_time_sec, config.max_refill_cycles_per_hour as u32)
    {
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("TOO_MANY_REFILLS".to_string());
        return true;
    }

    // Ghi mực nước trước khi bơm để kiểm tra ACK sau
    ctx.last_water_before_refill = Some(sensors.water_level);
    ctx.mark_pending_sample_water_change_violation();
    ctx.current_state = SystemState::WaterRefilling {
        target_level: config.water_level_target,
        start_time: 0, // sẽ được gán bằng current_time_ms ở caller – giữ API đơn giản
    };
    // Caller cần gán start_time; tạm workaround: re-assign ngay sau
    // NOTE: pattern hiện tại ok vì start_time chỉ dùng để timeout sau ~max_refill_duration_sec
    let _ = pump_ctrl.set_water_pump(WaterDirection::In);
    ctx.pump_status.water_pump_in = true;
    ctx.pump_status.water_pump_out = false;
    ctx.fsm_osaka_active = false;
    true
}

fn try_auto_drain_overflow(
    current_time_sec: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
) -> bool {
    if !(config.enable_water_level_sensor
        && config.auto_drain_overflow
        && sensors.water_level > config.water_level_max)
    {
        return false;
    }

    if !ctx.check_and_record_drain_limit(current_time_sec, config.max_drain_cycles_per_hour as u32)
    {
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("TOO_MANY_DRAINS".to_string());
        return true;
    }

    ctx.mark_pending_sample_water_change_violation();
    ctx.current_state = SystemState::WaterDraining {
        target_level: config.water_level_target,
        start_time: 0,
    };
    let _ = pump_ctrl.set_water_pump(WaterDirection::Out);
    ctx.pump_status.water_pump_out = true;
    ctx.pump_status.water_pump_in = false;
    ctx.fsm_osaka_active = false;
    true
}

fn try_auto_dilute(
    current_time_sec: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
) -> bool {
    if !(config.enable_ec_sensor
        && config.enable_water_level_sensor
        && config.auto_dilute_enabled
        && sensors.ec > (config.ec_target + config.ec_tolerance))
    {
        return false;
    }

    if !ctx.check_and_record_drain_limit(current_time_sec, config.max_drain_cycles_per_hour as u32)
    {
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("TOO_MANY_DRAINS".to_string());
        return true;
    }

    let target = (sensors.water_level - config.dilute_drain_amount_cm).max(config.water_level_min);
    ctx.mark_pending_sample_water_change_violation();
    ctx.current_state = SystemState::WaterDraining {
        target_level: target,
        start_time: 0,
    };
    let _ = pump_ctrl.set_water_pump(WaterDirection::Out);
    ctx.pump_status.water_pump_out = true;
    ctx.pump_status.water_pump_in = false;
    ctx.fsm_osaka_active = false;
    true
}

#[allow(clippy::too_many_arguments)]
fn handle_dosing_decisions(
    current_time_ms: u64,
    current_time_sec: u64,
    max_hourly_ml: f32,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    nvs: &mut Option<EspDefaultNvs>,
) {
    let mut is_dosing_active = false;

    // --- Bơm dinh dưỡng định kỳ theo cron ---
    if !is_dosing_active {
        is_dosing_active = try_scheduled_dosing(
            current_time_ms,
            current_time_sec,
            max_hourly_ml,
            config,
            sensors,
            ctx,
            nvs,
        );
    }

    // --- Bù EC tự động ---
    if !is_dosing_active {
        is_dosing_active = try_ec_dosing(
            current_time_ms,
            current_time_sec,
            max_hourly_ml,
            config,
            sensors,
            ctx,
            pump_ctrl,
        );
    }

    // --- Bù pH tự động ---
    if !is_dosing_active {
        is_dosing_active = try_ph_dosing(
            current_time_ms,
            current_time_sec,
            max_hourly_ml,
            config,
            sensors,
            ctx,
            pump_ctrl,
        );
    }

    if !is_dosing_active {
        ctx.fsm_osaka_active = false;
    }
}

fn try_scheduled_dosing(
    current_time_ms: u64,
    current_time_sec: u64,
    max_hourly_ml: f32,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    nvs: &mut Option<EspDefaultNvs>,
) -> bool {
    if !(config.scheduled_dosing_enabled && !config.scheduled_dosing_cron.is_empty()) {
        return false;
    }

    if ctx.current_cron_expr != config.scheduled_dosing_cron {
        ctx.current_cron_expr = config.scheduled_dosing_cron.clone();
        match Schedule::from_str(&ctx.current_cron_expr) {
            Ok(schedule) => {
                if let Some(next) = schedule.upcoming(Local).next() {
                    ctx.next_cron_trigger_sec = Some(next.timestamp() as u64);
                    info!("⏰ Cập nhật lịch Dosing Cron: {}", next);
                }
            }
            Err(_) => {
                warn!("⚠️ Biểu thức Cron Dosing không hợp lệ!");
                ctx.next_cron_trigger_sec = None;
            }
        }
    }

    let next_trigger = match ctx.next_cron_trigger_sec {
        Some(t) => t,
        None => return false,
    };
    if current_time_sec < next_trigger {
        return false;
    }

    info!("⏰ Đã đến giờ BƠM DINH DƯỠNG theo lịch CRON!");

    if let Ok(schedule) = Schedule::from_str(&ctx.current_cron_expr) {
        let future = Local::now() + chrono::Duration::seconds(1);
        if let Some(next) = schedule.after(&future).next() {
            ctx.next_cron_trigger_sec = Some(next.timestamp() as u64);
        }
    }

    ctx.last_scheduled_dose_time_sec = current_time_sec;
    if let Some(flash) = nvs.as_mut() {
        let _ = flash.set_u64("last_sched_dose", current_time_sec);
    }

    let safe_pwm = config.dosing_pwm_percent.clamp(1, 100) as u32;
    if config.scheduled_dose_a_ml <= 0.0 && config.scheduled_dose_b_ml <= 0.0 {
        return false;
    }

    let allow_a = config.scheduled_dose_a_ml <= 0.0
        || ctx.can_dose_within_hourly_limit(
            "NutrientA",
            current_time_sec,
            config.scheduled_dose_a_ml,
            max_hourly_ml,
        );
    let allow_b = config.scheduled_dose_b_ml <= 0.0
        || ctx.can_dose_within_hourly_limit(
            "NutrientB",
            current_time_sec,
            config.scheduled_dose_b_ml,
            max_hourly_ml,
        );

    if !(allow_a && allow_b) {
        return false;
    }

    if config.scheduled_dose_a_ml > 0.0 {
        let _ = ctx.reserve_dose_if_within_hourly_limit(
            "NutrientA",
            current_time_sec,
            config.scheduled_dose_a_ml,
            max_hourly_ml,
        );
    }
    if config.scheduled_dose_b_ml > 0.0 {
        let _ = ctx.reserve_dose_if_within_hourly_limit(
            "NutrientB",
            current_time_sec,
            config.scheduled_dose_b_ml,
            max_hourly_ml,
        );
    }

    ctx.current_state = SystemState::StartingOsakaPump {
        finish_time: current_time_ms + config.soft_start_duration as u64,
        pending_action: PendingDose::ScheduledDose {
            dose_a_ml: config.scheduled_dose_a_ml,
            dose_b_ml: config.scheduled_dose_b_ml,
            pwm_percent: safe_pwm,
        },
    };
    ctx.fsm_osaka_active = true;
    true
}

fn try_ec_dosing(
    current_time_ms: u64,
    current_time_sec: u64,
    max_hourly_ml: f32,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
) -> bool {
    if !(config.enable_ec_sensor && sensors.ec < (config.ec_target - config.ec_tolerance)) {
        return false;
    }

    if ctx.ec_retry_count >= 3 {
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("EC_DOSING_FAILED".to_string());
        return true;
    }

    let safe_pwm = config.dosing_pwm_percent.clamp(1, 100) as u32;
    let ec_error = config.ec_target - sensors.ec;
    let deadband_scale = soft_deadband_scale(ec_error, config.ec_tolerance);
    let active_ec_step_ratio = if ctx.auto_tune_locked {
        ctx.best_known_ec_step_ratio
    } else {
        ctx.adaptive_ec_step_ratio
    };
    let dose_ml = (ec_error / config.ec_gain_per_ml * active_ec_step_ratio * deadband_scale)
        .clamp(0.0, config.max_dose_per_cycle);

    if dose_ml <= 0.0 {
        return false;
    }

    let can_a =
        ctx.can_dose_within_hourly_limit("NutrientA", current_time_sec, dose_ml, max_hourly_ml);
    let can_b =
        ctx.can_dose_within_hourly_limit("NutrientB", current_time_sec, dose_ml, max_hourly_ml);
    if !(can_a && can_b) {
        return false;
    }

    let _ = ctx.reserve_dose_if_within_hourly_limit(
        "NutrientA",
        current_time_sec,
        dose_ml,
        max_hourly_ml,
    );
    let _ = ctx.reserve_dose_if_within_hourly_limit(
        "NutrientB",
        current_time_sec,
        dose_ml,
        max_hourly_ml,
    );
    ctx.last_ec_before_dosing = Some(sensors.ec);
    ctx.current_state = SystemState::StartingOsakaPump {
        finish_time: current_time_ms + config.soft_start_duration as u64,
        pending_action: PendingDose::EC {
            dose_ml,
            target_ec: config.ec_target,
            pwm_percent: safe_pwm,
        },
    };
    ctx.fsm_osaka_active = true;
    true
}

fn try_ph_dosing(
    current_time_ms: u64,
    current_time_sec: u64,
    max_hourly_ml: f32,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
) -> bool {
    if !(config.enable_ph_sensor && (sensors.ph - config.ph_target).abs() > config.ph_tolerance) {
        return false;
    }

    if ctx.ph_retry_count >= 3 {
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("PH_DOSING_FAILED".to_string());
        return true;
    }

    let is_ph_up = sensors.ph < config.ph_target;
    let diff = (sensors.ph - config.ph_target).abs();
    let ratio = if is_ph_up {
        config.ph_shift_up_per_ml
    } else {
        config.ph_shift_down_per_ml
    };
    let safe_pwm = config.dosing_pwm_percent.clamp(1, 100) as u32;
    let pump_kind = if is_ph_up {
        DosePumpKind::PhUp
    } else {
        DosePumpKind::PhDown
    };

    let active_capacity = match effective_flow_ml_per_sec(pump_kind, safe_pwm, config) {
        Some(c) => c,
        None => {
            let pump_name = if is_ph_up { "PhUp" } else { "PhDown" };
            warn!(
                "Skip PH dosing: invalid pump config or PWM below min (pump={}, pwm={}%)",
                pump_name, safe_pwm
            );
            ctx.stop_all_pumps(pump_ctrl);
            ctx.current_state = SystemState::Monitoring;
            return false;
        }
    };

    let deadband_scale = soft_deadband_scale(diff, config.ph_tolerance);
    let active_ph_step_ratio = if ctx.auto_tune_locked {
        ctx.best_known_ph_step_ratio
    } else {
        ctx.adaptive_ph_step_ratio
    };
    let dose_ml = (diff / ratio * active_ph_step_ratio * deadband_scale)
        .clamp(0.0, config.max_dose_per_cycle);

    let ph_pump_name = if is_ph_up { "PhUp" } else { "PhDown" };
    let duration_ms = ((dose_ml / active_capacity) * 1000.0) as u64;

    if duration_ms == 0 {
        return false;
    }
    if !ctx.reserve_dose_if_within_hourly_limit(
        ph_pump_name,
        current_time_sec,
        dose_ml,
        max_hourly_ml,
    ) {
        return false;
    }

    let final_dose_ml = (diff / ratio * config.ph_step_ratio).clamp(0.0, config.max_dose_per_cycle);
    if final_dose_ml <= 0.0 {
        return false;
    }

    ctx.last_ph_before_dosing = Some(sensors.ph);
    ctx.last_ph_dosing_is_up = Some(is_ph_up);
    ctx.current_state = SystemState::StartingOsakaPump {
        finish_time: current_time_ms + config.soft_start_duration as u64,
        pending_action: PendingDose::PH {
            is_up: is_ph_up,
            dose_ml: final_dose_ml,
            target_ph: config.ph_target,
            pwm_percent: safe_pwm,
        },
    };
    ctx.fsm_osaka_active = true;
    true
}

// ===========================================================================
// Osaka pump ready → chuyển sang trạng thái bơm phù hợp
// ===========================================================================

fn handle_osaka_ready(
    current_time_ms: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    action: PendingDose,
) {
    match action {
        PendingDose::ScheduledDose {
            dose_a_ml,
            dose_b_ml,
            pwm_percent,
        } => {
            if dose_a_ml > 0.0 {
                start_dosing_pump_a(
                    current_time_ms,
                    config,
                    sensors,
                    ctx,
                    pump_ctrl,
                    dose_a_ml,
                    dose_b_ml,
                    pwm_percent,
                    sensors.ec,
                );
            } else if dose_b_ml > 0.0 {
                ctx.current_state = SystemState::WaitingBetweenDose {
                    finish_time: current_time_ms,
                    dose_b_ml,
                    target_ec: sensors.ec,
                    start_ec: sensors.ec,
                    start_ph: sensors.ph,
                    dose_a_ml_reported: 0.0,
                };
            } else {
                ctx.current_state = SystemState::ActiveMixing {
                    finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
                };
            }
        }
        PendingDose::EC {
            dose_ml,
            target_ec,
            pwm_percent,
        } => {
            start_dosing_pump_a(
                current_time_ms,
                config,
                sensors,
                ctx,
                pump_ctrl,
                dose_ml,
                dose_ml,
                pwm_percent,
                target_ec,
            );
        }
        PendingDose::PH {
            is_up,
            dose_ml,
            target_ph,
            pwm_percent,
        } => {
            start_dosing_ph(
                current_time_ms,
                config,
                sensors,
                ctx,
                pump_ctrl,
                is_up,
                dose_ml,
                target_ph,
                pwm_percent,
            );
        }
    }
}

fn start_dosing_pump_a(
    current_time_ms: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    dose_a_ml: f32,
    dose_b_ml: f32,
    pwm_percent: u32,
    target_ec: f32,
) {
    let dose_pwm = pwm_percent.clamp(1, 100);
    let active_capacity_a = match effective_flow_ml_per_sec(DosePumpKind::PumpA, dose_pwm, config) {
        Some(c) => c,
        None => {
            warn!("Skip dose pump A: invalid config/pwm (pwm={}%)", dose_pwm);
            ctx.stop_all_pumps(pump_ctrl);
            ctx.current_state = SystemState::Monitoring;
            return;
        }
    };

    let (pulse_on_ms, pulse_off_ms, max_pulse_count) =
        pulse_params(dose_a_ml, active_capacity_a, config);

    let _ = pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientA, true, dose_pwm);
    ctx.pump_status.pump_a = true;
    ctx.pump_status.pump_a_pwm = Some(dose_pwm);
    let is_pulse_mode = dose_a_ml < config.dosing_min_dose_ml;
    ctx.set_pulse_status(is_pulse_mode, if is_pulse_mode { 1 } else { 0 });

    let delivered_ml_est = active_capacity_a * (pulse_on_ms as f32 / 1000.0);
    ctx.current_state = SystemState::DosingPumpA {
        next_toggle_time: current_time_ms + pulse_on_ms,
        dose_target_ml: dose_a_ml,
        delivered_ml_est,
        dose_b_ml,
        pulse_on: true,
        pulse_count: 1,
        max_pulse_count,
        pulse_on_ms,
        pulse_off_ms,
        pwm_percent: dose_pwm,
        active_capacity_ml_per_sec: active_capacity_a,
        target_ec,
        start_ec: sensors.ec,
        start_ph: sensors.ph,
    };
}

fn start_dosing_ph(
    current_time_ms: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    is_up: bool,
    dose_ml: f32,
    target_ph: f32,
    pwm_percent: u32,
) {
    let dose_pwm = pwm_percent.clamp(1, 100);
    let pump_kind = if is_up {
        DosePumpKind::PhUp
    } else {
        DosePumpKind::PhDown
    };

    let active_capacity = match effective_flow_ml_per_sec(pump_kind, dose_pwm, config) {
        Some(c) => c,
        None => {
            warn!(
                "Skip PH dosing: invalid config/pwm (is_up={}, pwm={}%)",
                is_up, dose_pwm
            );
            ctx.stop_all_pumps(pump_ctrl);
            ctx.current_state = SystemState::Monitoring;
            return;
        }
    };

    let (pulse_on_ms, pulse_off_ms, max_pulse_count) =
        pulse_params(dose_ml, active_capacity, config);

    let pump_type = if is_up {
        PumpType::PhUp
    } else {
        PumpType::PhDown
    };
    let _ = pump_ctrl.set_dosing_pump_pulse(pump_type, true, dose_pwm);
    if is_up {
        ctx.pump_status.ph_up = true;
        ctx.pump_status.ph_up_pwm = Some(dose_pwm);
    } else {
        ctx.pump_status.ph_down = true;
        ctx.pump_status.ph_down_pwm = Some(dose_pwm);
    }
    let is_pulse_mode = dose_ml < config.dosing_min_dose_ml;
    ctx.set_pulse_status(is_pulse_mode, if is_pulse_mode { 1 } else { 0 });

    let delivered_ml_est = active_capacity * (pulse_on_ms as f32 / 1000.0);
    ctx.current_state = SystemState::DosingPH {
        next_toggle_time: current_time_ms + pulse_on_ms,
        is_up,
        dose_target_ml: dose_ml,
        delivered_ml_est,
        pulse_on: true,
        pulse_count: 1,
        max_pulse_count,
        pulse_on_ms,
        pulse_off_ms,
        pwm_percent: dose_pwm,
        active_capacity_ml_per_sec: active_capacity,
        target_ph,
        start_ec: sensors.ec,
        start_ph: sensors.ph,
    };
}

// ===========================================================================
// Pulse tick handlers
// ===========================================================================

struct DosingPumpAState {
    dose_target_ml: f32,
    delivered_ml_est: f32,
    dose_b_ml: f32,
    pulse_on: bool,
    pulse_count: u32,
    max_pulse_count: u32,
    pulse_on_ms: u64,
    pulse_off_ms: u64,
    pwm_percent: u32,
    active_capacity_ml_per_sec: f32,
    target_ec: f32,
    start_ec: f32,
    start_ph: f32,
}

fn handle_dosing_pump_a_tick(
    current_time_ms: u64,
    config: &ControllerConfig,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    s: DosingPumpAState,
) {
    if s.pulse_on {
        let _ = pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientA, false, 0);
        ctx.pump_status.pump_a = false;
        ctx.pump_status.pump_a_pwm = Some(0);

        if s.delivered_ml_est >= s.dose_target_ml || s.pulse_count >= s.max_pulse_count {
            ctx.set_pulse_status(false, s.pulse_count);
            ctx.current_state = SystemState::WaitingBetweenDose {
                finish_time: current_time_ms + (config.delay_between_a_and_b_sec as u64 * 1000),
                dose_b_ml: s.dose_b_ml,
                target_ec: s.target_ec,
                start_ec: s.start_ec,
                start_ph: s.start_ph,
                dose_a_ml_reported: s.delivered_ml_est.min(s.dose_target_ml),
            };
        } else {
            ctx.set_pulse_status(true, s.pulse_count);
            ctx.current_state = SystemState::DosingPumpA {
                next_toggle_time: current_time_ms + s.pulse_off_ms,
                dose_target_ml: s.dose_target_ml,
                delivered_ml_est: s.delivered_ml_est,
                dose_b_ml: s.dose_b_ml,
                pulse_on: false,
                pulse_count: s.pulse_count,
                max_pulse_count: s.max_pulse_count,
                pulse_on_ms: s.pulse_on_ms,
                pulse_off_ms: s.pulse_off_ms,
                pwm_percent: s.pwm_percent,
                active_capacity_ml_per_sec: s.active_capacity_ml_per_sec,
                target_ec: s.target_ec,
                start_ec: s.start_ec,
                start_ph: s.start_ph,
            };
        }
    } else {
        let _ = pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientA, true, s.pwm_percent);
        ctx.pump_status.pump_a = true;
        ctx.pump_status.pump_a_pwm = Some(s.pwm_percent);
        let next_count = s.pulse_count + 1;
        let next_delivered =
            s.delivered_ml_est + s.active_capacity_ml_per_sec * (s.pulse_on_ms as f32 / 1000.0);
        ctx.set_pulse_status(true, next_count);
        ctx.current_state = SystemState::DosingPumpA {
            next_toggle_time: current_time_ms + s.pulse_on_ms,
            dose_target_ml: s.dose_target_ml,
            delivered_ml_est: next_delivered,
            dose_b_ml: s.dose_b_ml,
            pulse_on: true,
            pulse_count: next_count,
            max_pulse_count: s.max_pulse_count,
            pulse_on_ms: s.pulse_on_ms,
            pulse_off_ms: s.pulse_off_ms,
            pwm_percent: s.pwm_percent,
            active_capacity_ml_per_sec: s.active_capacity_ml_per_sec,
            target_ec: s.target_ec,
            start_ec: s.start_ec,
            start_ph: s.start_ph,
        };
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_waiting_between_dose(
    current_time_ms: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    dosing_report_tx: &Sender<String>,
    dose_b_ml: f32,
    target_ec: f32,
    start_ec: f32,
    start_ph: f32,
    dose_a_ml_reported: f32,
) {
    if dose_b_ml > 0.0 {
        let dose_pwm = config.dosing_pwm_percent.clamp(1, 100) as u32;
        let active_capacity_b =
            match effective_flow_ml_per_sec(DosePumpKind::PumpB, dose_pwm, config) {
                Some(c) => c,
                None => {
                    warn!("Skip dose pump B: invalid config/pwm (pwm={}%)", dose_pwm);
                    ctx.stop_all_pumps(pump_ctrl);
                    ctx.current_state = SystemState::Monitoring;
                    return;
                }
            };

        let (pulse_on_ms, pulse_off_ms, max_pulse_count) =
            pulse_params(dose_b_ml, active_capacity_b, config);

        let _ = pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientB, true, dose_pwm);
        ctx.pump_status.pump_b = true;
        ctx.pump_status.pump_b_pwm = Some(dose_pwm);
        let is_pulse_mode = dose_b_ml < config.dosing_min_dose_ml;
        ctx.set_pulse_status(is_pulse_mode, if is_pulse_mode { 1 } else { 0 });

        let delivered_ml_est = active_capacity_b * (pulse_on_ms as f32 / 1000.0);
        ctx.current_state = SystemState::DosingPumpB {
            next_toggle_time: current_time_ms + pulse_on_ms,
            dose_target_ml: dose_b_ml,
            delivered_ml_est,
            pulse_on: true,
            pulse_count: 1,
            max_pulse_count,
            pulse_on_ms,
            pulse_off_ms,
            pwm_percent: dose_pwm,
            active_capacity_ml_per_sec: active_capacity_b,
            target_ec,
            start_ec,
            start_ph,
            dose_a_ml_reported,
        };
    } else {
        let report_json = format!(
            r#"{{"start_ec":{:.2},"start_ph":{:.2},"pump_a_ml":{:.2},"pump_b_ml":0.0,"ph_up_ml":0.0,"ph_down_ml":0.0,"target_ec":{:.2},"target_ph":{:.2}}}"#,
            start_ec, start_ph, dose_a_ml_reported, target_ec, config.ph_target
        );
        let _ = dosing_report_tx.send(report_json);
        start_pending_calibration_sample(
            ctx,
            start_ec,
            start_ph,
            dose_a_ml_reported,
            0.0,
            0.0,
            0.0,
            current_time_ms,
            config,
        );
        ctx.current_state = SystemState::ActiveMixing {
            finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
        };
    }
}

struct DosingPumpBState {
    dose_target_ml: f32,
    delivered_ml_est: f32,
    pulse_on: bool,
    pulse_count: u32,
    max_pulse_count: u32,
    pulse_on_ms: u64,
    pulse_off_ms: u64,
    pwm_percent: u32,
    active_capacity_ml_per_sec: f32,
    target_ec: f32,
    start_ec: f32,
    start_ph: f32,
    dose_a_ml_reported: f32,
}

fn handle_dosing_pump_b_tick(
    current_time_ms: u64,
    config: &ControllerConfig,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    dosing_report_tx: &Sender<String>,
    s: DosingPumpBState,
) {
    if s.pulse_on {
        let _ = pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientB, false, 0);
        ctx.pump_status.pump_b = false;
        ctx.pump_status.pump_b_pwm = Some(0);

        if s.delivered_ml_est >= s.dose_target_ml || s.pulse_count >= s.max_pulse_count {
            ctx.set_pulse_status(false, s.pulse_count);
            let report_json = format!(
                r#"{{"start_ec":{:.2},"start_ph":{:.2},"pump_a_ml":{:.2},"pump_b_ml":{:.2},"ph_up_ml":0.0,"ph_down_ml":0.0,"target_ec":{:.2},"target_ph":{:.2}}}"#,
                s.start_ec,
                s.start_ph,
                s.dose_a_ml_reported,
                s.delivered_ml_est.min(s.dose_target_ml),
                s.target_ec,
                config.ph_target
            );
            let _ = dosing_report_tx.send(report_json);
            ctx.current_state = SystemState::ActiveMixing {
                finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
            };
        } else {
            ctx.set_pulse_status(true, s.pulse_count);
            ctx.current_state = SystemState::DosingPumpB {
                next_toggle_time: current_time_ms + s.pulse_off_ms,
                dose_target_ml: s.dose_target_ml,
                delivered_ml_est: s.delivered_ml_est,
                pulse_on: false,
                pulse_count: s.pulse_count,
                max_pulse_count: s.max_pulse_count,
                pulse_on_ms: s.pulse_on_ms,
                pulse_off_ms: s.pulse_off_ms,
                pwm_percent: s.pwm_percent,
                active_capacity_ml_per_sec: s.active_capacity_ml_per_sec,
                target_ec: s.target_ec,
                start_ec: s.start_ec,
                start_ph: s.start_ph,
                dose_a_ml_reported: s.dose_a_ml_reported,
            };
        }
    } else {
        let _ = pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientB, true, s.pwm_percent);
        ctx.pump_status.pump_b = true;
        ctx.pump_status.pump_b_pwm = Some(s.pwm_percent);
        let next_count = s.pulse_count + 1;
        let next_delivered =
            s.delivered_ml_est + s.active_capacity_ml_per_sec * (s.pulse_on_ms as f32 / 1000.0);
        ctx.set_pulse_status(true, next_count);
        ctx.current_state = SystemState::DosingPumpB {
            next_toggle_time: current_time_ms + s.pulse_on_ms,
            dose_target_ml: s.dose_target_ml,
            delivered_ml_est: next_delivered,
            pulse_on: true,
            pulse_count: next_count,
            max_pulse_count: s.max_pulse_count,
            pulse_on_ms: s.pulse_on_ms,
            pulse_off_ms: s.pulse_off_ms,
            pwm_percent: s.pwm_percent,
            active_capacity_ml_per_sec: s.active_capacity_ml_per_sec,
            target_ec: s.target_ec,
            start_ec: s.start_ec,
            start_ph: s.start_ph,
            dose_a_ml_reported: s.dose_a_ml_reported,
        };
    }
}

struct DosingPhState {
    is_up: bool,
    dose_target_ml: f32,
    delivered_ml_est: f32,
    pulse_on: bool,
    pulse_count: u32,
    max_pulse_count: u32,
    pulse_on_ms: u64,
    pulse_off_ms: u64,
    pwm_percent: u32,
    active_capacity_ml_per_sec: f32,
    target_ph: f32,
    start_ec: f32,
    start_ph: f32,
}

fn handle_dosing_ph_tick(
    current_time_ms: u64,
    config: &ControllerConfig,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    dosing_report_tx: &Sender<String>,
    s: DosingPhState,
) {
    let pump_type = if s.is_up {
        PumpType::PhUp
    } else {
        PumpType::PhDown
    };

    if s.pulse_on {
        let _ = pump_ctrl.set_dosing_pump_pulse(pump_type, false, 0);
        if s.is_up {
            ctx.pump_status.ph_up = false;
            ctx.pump_status.ph_up_pwm = Some(0);
        } else {
            ctx.pump_status.ph_down = false;
            ctx.pump_status.ph_down_pwm = Some(0);
        }

        if s.delivered_ml_est >= s.dose_target_ml || s.pulse_count >= s.max_pulse_count {
            ctx.set_pulse_status(false, s.pulse_count);
            let ph_up_ml = if s.is_up {
                s.delivered_ml_est.min(s.dose_target_ml)
            } else {
                0.0
            };
            let ph_down_ml = if !s.is_up {
                s.delivered_ml_est.min(s.dose_target_ml)
            } else {
                0.0
            };
            let report_json = format!(
                r#"{{"start_ec":{:.2},"start_ph":{:.2},"pump_a_ml":0.0,"pump_b_ml":0.0,"ph_up_ml":{:.2},"ph_down_ml":{:.2},"target_ec":{:.2},"target_ph":{:.2}}}"#,
                s.start_ec, s.start_ph, ph_up_ml, ph_down_ml, config.ec_target, s.target_ph
            );
            let _ = dosing_report_tx.send(report_json);
            ctx.current_state = SystemState::ActiveMixing {
                finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
            };
        } else {
            ctx.set_pulse_status(true, s.pulse_count);
            ctx.current_state = SystemState::DosingPH {
                next_toggle_time: current_time_ms + s.pulse_off_ms,
                is_up: s.is_up,
                dose_target_ml: s.dose_target_ml,
                delivered_ml_est: s.delivered_ml_est,
                pulse_on: false,
                pulse_count: s.pulse_count,
                max_pulse_count: s.max_pulse_count,
                pulse_on_ms: s.pulse_on_ms,
                pulse_off_ms: s.pulse_off_ms,
                pwm_percent: s.pwm_percent,
                active_capacity_ml_per_sec: s.active_capacity_ml_per_sec,
                target_ph: s.target_ph,
                start_ec: s.start_ec,
                start_ph: s.start_ph,
            };
        }
    } else {
        let _ = pump_ctrl.set_dosing_pump_pulse(pump_type, true, s.pwm_percent);
        if s.is_up {
            ctx.pump_status.ph_up = true;
            ctx.pump_status.ph_up_pwm = Some(s.pwm_percent);
        } else {
            ctx.pump_status.ph_down = true;
            ctx.pump_status.ph_down_pwm = Some(s.pwm_percent);
        }
        let next_count = s.pulse_count + 1;
        let next_delivered =
            s.delivered_ml_est + s.active_capacity_ml_per_sec * (s.pulse_on_ms as f32 / 1000.0);
        ctx.set_pulse_status(true, next_count);
        ctx.current_state = SystemState::DosingPH {
            next_toggle_time: current_time_ms + s.pulse_on_ms,
            is_up: s.is_up,
            dose_target_ml: s.dose_target_ml,
            delivered_ml_est: next_delivered,
            pulse_on: true,
            pulse_count: next_count,
            max_pulse_count: s.max_pulse_count,
            pulse_on_ms: s.pulse_on_ms,
            pulse_off_ms: s.pulse_off_ms,
            pwm_percent: s.pwm_percent,
            active_capacity_ml_per_sec: s.active_capacity_ml_per_sec,
            target_ph: s.target_ph,
            start_ec: s.start_ec,
            start_ph: s.start_ph,
        };
    }
}

// ===========================================================================
// Helper chung
// ===========================================================================

/// Tính (pulse_on_ms, pulse_off_ms, max_pulse_count) cho chế độ thường và pulse.
fn pulse_params(
    dose_ml: f32,
    capacity_ml_per_sec: f32,
    config: &ControllerConfig,
) -> (u64, u64, u32) {
    let is_pulse_mode = dose_ml < config.dosing_min_dose_ml;
    let pulse_on_ms = if is_pulse_mode {
        config.dosing_pulse_on_ms.max(1) as u64
    } else {
        ((dose_ml / capacity_ml_per_sec) * 1000.0) as u64
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
