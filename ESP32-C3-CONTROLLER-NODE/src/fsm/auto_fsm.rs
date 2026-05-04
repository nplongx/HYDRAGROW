use std::str::FromStr;
use std::sync::mpsc::Sender;

use chrono::{Local, TimeZone};
use cron::Schedule;
use hydragrow_shared::ControllerConfig;
use log::{debug, error, info, warn};

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
                fsm_mqtt_tx, // <--- TRUYỀN fsm_mqtt_tx XUỐNG DƯỚI
            );
        }

        SystemState::WaterRefilling { ref trigger, target_level, start_time, start_level, start_ec } => {
            let duration_sec = current_time_ms.saturating_sub(start_time) / 1000;
            let target_reached = sensors.water_level >= target_level;
            let timeout = duration_sec > config.max_refill_duration_sec as u64;

            if target_reached || timeout {
                let _ = pump_ctrl.set_water_pump(WaterDirection::Stop);
                ctx.pump_status.water_pump_in = false;
                ctx.pump_status.water_pump_out = false;
                ctx.fsm_osaka_active = true;

                let report_json = format!(
                    r#"[WATER EVENT] {{ "trigger": "{}", "level_before": {:.1}, "level_after": {:.1}, "duration_sec": {}, "ec_before": {:.2}, "ec_after": {:.2}, "success": {} }}"#,
                    trigger, start_level, sensors.water_level, duration_sec, start_ec, sensors.ec, target_reached
                );
                let _ = fsm_mqtt_tx.send(report_json);

                ctx.current_state = SystemState::ActiveMixing {
                    finish_time: current_time_ms + (config.active_mixing_sec as u64 * 1000),
                };
            }
        }

        SystemState::WaterDraining { ref trigger, target_level, start_time, start_level, start_ec } => {
            let duration_sec = current_time_ms.saturating_sub(start_time) / 1000;
            let target_reached = sensors.water_level <= target_level;
            let timeout = duration_sec > config.max_drain_duration_sec as u64;

            if target_reached || timeout {
                let _ = pump_ctrl.set_water_pump(WaterDirection::Stop);
                ctx.pump_status.water_pump_in = false;
                ctx.pump_status.water_pump_out = false;
                ctx.fsm_osaka_active = false;

                let report_json = format!(
                    r#"[WATER EVENT] {{ "trigger": "{}", "level_before": {:.1}, "level_after": {:.1}, "duration_sec": {}, "ec_before": {:.2}, "ec_after": {:.2}, "success": {} }}"#,
                    trigger, start_level, sensors.water_level, duration_sec, start_ec, sensors.ec, target_reached
                );
                let _ = fsm_mqtt_tx.send(report_json);

                ctx.current_state = SystemState::Stabilizing {
                    finish_time: current_time_ms + (config.sensor_stabilize_sec as u64 * 1000),
                };
            }
        }

        SystemState::StartingOsakaPump { finish_time, ref pending_action } => {
            if current_time_ms >= finish_time {
                let action = pending_action.clone();
                handle_osaka_ready(current_time_ms, config, sensors, ctx, pump_ctrl, action);
            }
        }

        SystemState::DosingPumpA { next_toggle_time, dose_target_ml, delivered_ml_est, dose_b_ml, pulse_on, pulse_count, max_pulse_count, pulse_on_ms, pulse_off_ms, pwm_percent, active_capacity_ml_per_sec, target_ec, start_ec, start_ph } => {
            if current_time_ms >= next_toggle_time {
                handle_dosing_pump_a_tick(
                    current_time_ms, config, ctx, pump_ctrl,
                    DosingPumpAState { dose_target_ml, delivered_ml_est, dose_b_ml, pulse_on, pulse_count, max_pulse_count, pulse_on_ms, pulse_off_ms, pwm_percent, active_capacity_ml_per_sec, target_ec, start_ec, start_ph },
                );
            }
        }

        SystemState::WaitingBetweenDose { finish_time, dose_b_ml, target_ec, start_ec, start_ph, dose_a_ml_reported } => {
            if current_time_ms >= finish_time {
                handle_waiting_between_dose(
                    current_time_ms, config, sensors, ctx, pump_ctrl, dose_b_ml, target_ec, start_ec, start_ph, dose_a_ml_reported,
                );
            }
        }

        SystemState::DosingPumpB { next_toggle_time, dose_target_ml, delivered_ml_est, pulse_on, pulse_count, max_pulse_count, pulse_on_ms, pulse_off_ms, pwm_percent, active_capacity_ml_per_sec, target_ec, start_ec, start_ph, dose_a_ml_reported } => {
            if current_time_ms >= next_toggle_time {
                handle_dosing_pump_b_tick(
                    current_time_ms, config, ctx, pump_ctrl,
                    DosingPumpBState { dose_target_ml, delivered_ml_est, pulse_on, pulse_count, max_pulse_count, pulse_on_ms, pulse_off_ms, pwm_percent, active_capacity_ml_per_sec, target_ec, start_ec, start_ph, dose_a_ml_reported },
                );
            }
        }

        SystemState::DosingPH { next_toggle_time, is_up, dose_target_ml, delivered_ml_est, pulse_on, pulse_count, max_pulse_count, pulse_on_ms, pulse_off_ms, pwm_percent, active_capacity_ml_per_sec, target_ph, start_ec, start_ph } => {
            if current_time_ms >= next_toggle_time {
                handle_dosing_ph_tick(
                    current_time_ms, config, ctx, pump_ctrl,
                    DosingPhState { is_up, dose_target_ml, delivered_ml_est, pulse_on, pulse_count, max_pulse_count, pulse_on_ms, pulse_off_ms, pwm_percent, active_capacity_ml_per_sec, target_ph, start_ec, start_ph },
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
                    sample.post_mixing_ec = sensors.ec;
                    sample.post_mixing_ph = sensors.ph;
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
                    
                    let duration_ms = current_time_ms.saturating_sub(sample.start_ms);
                    let delta_ec = sensors.ec - sample.start_ec;
                    let delta_ph = sensors.ph - sample.start_ph;
                    let error_ec = sample.target_ec - sensors.ec;
                    let error_ph = sample.target_ph - sensors.ph;

                    let ema_ph_shift_used = if sample.dose_ph_up_ml > 0.0 {
                        config.ph_shift_up_per_ml
                    } else {
                        config.ph_shift_down_per_ml
                    };

                    let report_json = format!(
                        r#"[DOSING CYCLE] {{ "cycle_id": "{}", "trigger": "{}", "pre": {{ "ec": {:.2}, "ph": {:.2}, "water_level": {:.1} }}, "dose": {{ "pump_a_ml": {:.2}, "pump_b_ml": {:.2}, "ph_up_ml": {:.2}, "ph_down_ml": {:.2} }}, "post_mixing": {{ "ec": {:.2}, "ph": {:.2} }}, "post_stable": {{ "ec": {:.2}, "ph": {:.2} }}, "delta_ec": {:.2}, "delta_ph": {:.2}, "target_ec": {:.2}, "target_ph": {:.2}, "error_ec": {:.2}, "error_ph": {:.2}, "duration_ms": {}, "ema_ec_gain_used": {:.4}, "ema_ph_shift_used": {:.4} }}"#,
                        sample.cycle_id, sample.trigger, sample.start_ec, sample.start_ph, sample.start_water_level,
                        sample.dose_a_ml, sample.dose_b_ml, sample.dose_ph_up_ml, sample.dose_ph_down_ml,
                        sample.post_mixing_ec, sample.post_mixing_ph, sensors.ec, sensors.ph,
                        delta_ec, delta_ph, sample.target_ec, sample.target_ph, error_ec, error_ph, duration_ms,
                        config.ec_gain_per_ml, ema_ph_shift_used
                    );
                    let _ = dosing_report_tx.send(report_json);
                }
                apply_runtime_calibration_ema(sensors, shared_config, ctx, fsm_mqtt_tx);
                ctx.current_state = SystemState::DosingCycleComplete;
            }
        }

        SystemState::EmergencyStop(_) => {}
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
    fsm_mqtt_tx: &Sender<String>,
) {
    ctx.verify_sensor_ack(sensors, config, current_time_sec);

    if try_scheduled_water_change(current_time_ms, current_time_sec, config, sensors, ctx, pump_ctrl, nvs, fsm_mqtt_tx) {
        return;
    }
    if try_auto_refill(current_time_ms, current_time_sec, config, sensors, ctx, pump_ctrl, fsm_mqtt_tx) {
        return;
    }
    if try_auto_drain_overflow(current_time_ms, current_time_sec, config, sensors, ctx, pump_ctrl, fsm_mqtt_tx) {
        return;
    }
    if try_auto_dilute(current_time_ms, current_time_sec, config, sensors, ctx, pump_ctrl, fsm_mqtt_tx) {
        return;
    }
    
    handle_dosing_decisions(
        current_time_ms,
        current_time_sec,
        max_hourly_ml,
        config,
        sensors,
        ctx,
        pump_ctrl,
        nvs,
        fsm_mqtt_tx,
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
    fsm_mqtt_tx: &Sender<String>,
) -> bool {
    if !(config.enable_water_level_sensor && config.scheduled_water_change_enabled && !config.water_change_cron.is_empty()) {
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

    let target = (sensors.water_level - config.scheduled_drain_amount_cm).max(config.water_level_min);
    ctx.last_water_change_time = current_time_sec;
    if let Some(flash) = nvs.as_mut() {
        let _ = flash.set_u64("last_w_change", current_time_sec);
    }

    if !ctx.check_and_record_drain_limit(current_time_sec, config.max_drain_cycles_per_hour as u32) {
        let msg = format!("Quá giới hạn mở van xả nước trong 1 giờ (max: {} lần).", config.max_drain_cycles_per_hour);
        let alert_json = format!(r#"[SYSTEM ALERT] {{ "type": "rate_limit", "source": "drain", "message": "{}" }}"#, msg);
        let _ = fsm_mqtt_tx.send(alert_json);
        
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("TOO_MANY_DRAINS".to_string());
        return true;
    }

    ctx.mark_pending_sample_water_change_violation();
    ctx.current_state = SystemState::WaterDraining {
        trigger: "scheduled_change".to_string(),
        target_level: target,
        start_time: current_time_ms,
        start_level: sensors.water_level,
        start_ec: sensors.ec,
    };
    let _ = pump_ctrl.set_water_pump(WaterDirection::Out);
    ctx.pump_status.water_pump_out = true;
    ctx.pump_status.water_pump_in = false;
    ctx.fsm_osaka_active = false;
    true
}

fn try_auto_refill(
    current_time_ms: u64,
    current_time_sec: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    fsm_mqtt_tx: &Sender<String>,
) -> bool {
    if !(config.enable_water_level_sensor && config.auto_refill_enabled && sensors.water_level < (config.water_level_target - config.water_level_tolerance)) {
        return false;
    }

    if ctx.water_refill_retry_count >= 3 {
        let msg = "Hủy cấp nước: Đã thử 3 lần nhưng mực nước không tăng (kẹt phao hoặc hết nước nguồn).";
        warn!("🚨 {}", msg);
        let alert_json = format!(r#"[SYSTEM ALERT] {{ "type": "fault", "source": "refill", "message": "{}" }}"#, msg);
        let _ = fsm_mqtt_tx.send(alert_json);
        
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("WATER_REFILL_FAILED".to_string());
        return true;
    }

    if !ctx.check_and_record_refill_limit(current_time_sec, config.max_refill_cycles_per_hour as u32) {
        let msg = format!("Quá giới hạn bơm nước vào bồn trong 1 giờ (max: {} lần).", config.max_refill_cycles_per_hour);
        let alert_json = format!(r#"[SYSTEM ALERT] {{ "type": "rate_limit", "source": "refill", "message": "{}" }}"#, msg);
        let _ = fsm_mqtt_tx.send(alert_json);
        
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("TOO_MANY_REFILLS".to_string());
        return true;
    }

    ctx.last_water_before_refill = Some(sensors.water_level);
    ctx.mark_pending_sample_water_change_violation();
    
    ctx.current_state = SystemState::WaterRefilling {
        trigger: "auto_refill".to_string(),
        target_level: config.water_level_target,
        start_time: current_time_ms,
        start_level: sensors.water_level,
        start_ec: sensors.ec,
    };

    let _ = pump_ctrl.set_water_pump(WaterDirection::In);
    ctx.pump_status.water_pump_in = true;
    ctx.pump_status.water_pump_out = false;
    ctx.fsm_osaka_active = false;
    true
}

fn try_auto_drain_overflow(
    current_time_ms: u64,
    current_time_sec: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    fsm_mqtt_tx: &Sender<String>,
) -> bool {
    if !(config.enable_water_level_sensor && config.auto_drain_overflow && sensors.water_level > config.water_level_max) {
        return false;
    }

    if !ctx.check_and_record_drain_limit(current_time_sec, config.max_drain_cycles_per_hour as u32) {
        let msg = format!("Quá giới hạn xả nước tràn trong 1 giờ (max: {} lần).", config.max_drain_cycles_per_hour);
        let alert_json = format!(r#"[SYSTEM ALERT] {{ "type": "rate_limit", "source": "drain_overflow", "message": "{}" }}"#, msg);
        let _ = fsm_mqtt_tx.send(alert_json);
        
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("TOO_MANY_DRAINS".to_string());
        return true;
    }

    ctx.mark_pending_sample_water_change_violation();
    ctx.current_state = SystemState::WaterDraining {
        trigger: "auto_drain".to_string(),
        target_level: config.water_level_target,
        start_time: current_time_ms,
        start_level: sensors.water_level,
        start_ec: sensors.ec,
    };
    let _ = pump_ctrl.set_water_pump(WaterDirection::Out);
    ctx.pump_status.water_pump_out = true;
    ctx.pump_status.water_pump_in = false;
    ctx.fsm_osaka_active = false;
    true
}

fn try_auto_dilute(
    current_time_ms: u64,
    current_time_sec: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    fsm_mqtt_tx: &Sender<String>,
) -> bool {
    if !(config.enable_ec_sensor && config.enable_water_level_sensor && config.auto_dilute_enabled && sensors.ec > (config.ec_target + config.ec_tolerance)) {
        return false;
    }

    if !ctx.check_and_record_drain_limit(current_time_sec, config.max_drain_cycles_per_hour as u32) {
        let msg = format!("Quá giới hạn xả nước pha loãng trong 1 giờ (max: {} lần).", config.max_drain_cycles_per_hour);
        let alert_json = format!(r#"[SYSTEM ALERT] {{ "type": "rate_limit", "source": "dilute", "message": "{}" }}"#, msg);
        let _ = fsm_mqtt_tx.send(alert_json);
        
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("TOO_MANY_DRAINS".to_string());
        return true;
    }

    let target = (sensors.water_level - config.dilute_drain_amount_cm).max(config.water_level_min);
    ctx.mark_pending_sample_water_change_violation();
    ctx.current_state = SystemState::WaterDraining {
        trigger: "dilute".to_string(),
        target_level: target,
        start_time: current_time_ms,
        start_level: sensors.water_level,
        start_ec: sensors.ec,
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
    fsm_mqtt_tx: &Sender<String>,
) {
    let mut is_dosing_active = false;

    if !is_dosing_active {
        is_dosing_active = try_scheduled_dosing(
            current_time_ms, current_time_sec, max_hourly_ml, config, sensors, ctx, pump_ctrl, nvs, fsm_mqtt_tx,
        );
    }

    if !is_dosing_active {
        is_dosing_active = try_ec_dosing(
            current_time_ms, current_time_sec, max_hourly_ml, config, sensors, ctx, pump_ctrl, fsm_mqtt_tx,
        );
    }

    if !is_dosing_active {
        is_dosing_active = try_ph_dosing(
            current_time_ms, current_time_sec, max_hourly_ml, config, sensors, ctx, pump_ctrl, fsm_mqtt_tx,
        );
    }

    if !is_dosing_active {
        ctx.fsm_osaka_active = false;
    }
}

#[allow(clippy::too_many_arguments)]
fn try_scheduled_dosing(
    current_time_ms: u64,
    current_time_sec: u64,
    max_hourly_ml: f32,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    nvs: &mut Option<EspDefaultNvs>,
    fsm_mqtt_tx: &Sender<String>,
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
            "NutrientA", current_time_sec, config.scheduled_dose_a_ml, max_hourly_ml,
        );
    let allow_b = config.scheduled_dose_b_ml <= 0.0
        || ctx.can_dose_within_hourly_limit(
            "NutrientB", current_time_sec, config.scheduled_dose_b_ml, max_hourly_ml,
        );

    if !(allow_a && allow_b) {
        let msg = format!("⚠️ [SCHEDULED] Yêu cầu lượng châm làm vượt quá giới hạn an toàn (Max: {}ml/h). Đã hủy lịch!", max_hourly_ml);
        warn!("{}", msg);
        let alert_json = format!(r#"[SYSTEM ALERT] {{ "type": "rate_limit", "source": "scheduled_dosing", "message": "{}" }}"#, msg);
        let _ = fsm_mqtt_tx.send(alert_json);
        
        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("MAX_HOURLY_DOSE_SCHED".to_string());
        return true;
    }

    if config.scheduled_dose_a_ml > 0.0 {
        let _ = ctx.reserve_dose_if_within_hourly_limit("NutrientA", current_time_sec, config.scheduled_dose_a_ml, max_hourly_ml);
    }
    if config.scheduled_dose_b_ml > 0.0 {
        let _ = ctx.reserve_dose_if_within_hourly_limit("NutrientB", current_time_sec, config.scheduled_dose_b_ml, max_hourly_ml);
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

#[allow(clippy::too_many_arguments)]
fn try_ec_dosing(
    current_time_ms: u64,
    current_time_sec: u64,
    max_hourly_ml: f32,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    fsm_mqtt_tx: &Sender<String>,
) -> bool {
    if !(config.enable_ec_sensor && sensors.ec < (config.ec_target - config.ec_tolerance)) {
        return false;
    }

    if ctx.ec_retry_count >= 3 {
        let msg = "🚨 Hủy bù EC: Đã bơm thử 3 lần nhưng cảm biến EC không tăng.";
        warn!("{}", msg);
        let alert_json = format!(r#"[SYSTEM ALERT] {{ "type": "fault", "source": "ec_dosing", "message": "{}" }}"#, msg);
        let _ = fsm_mqtt_tx.send(alert_json);
        
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

    let can_a = ctx.can_dose_within_hourly_limit("NutrientA", current_time_sec, dose_ml, max_hourly_ml);
    let can_b = ctx.can_dose_within_hourly_limit("NutrientB", current_time_sec, dose_ml, max_hourly_ml);
    
    if !(can_a && can_b) {
        let msg = format!("⚠️ [EC] Bơm bị khóa! Yêu cầu {:.2}ml làm vượt giới hạn giờ (Max: {}ml/h)", dose_ml, max_hourly_ml);
        warn!("{}", msg);
        
        let alert_json = format!(r#"[SYSTEM ALERT] {{ "type": "rate_limit", "source": "ec_dosing", "message": "{}" }}"#, msg);
        let _ = fsm_mqtt_tx.send(alert_json);

        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("MAX_HOURLY_DOSE_EC".to_string());
        return true;
    }

    info!(
        "🧪 [EC DOSING] Bắt đầu bù EC. Cần tăng: {:.2} (Hiện: {:.2}, Mục tiêu: {:.2}). Liều lượng tính toán: {:.2}ml (Deadband Scale: {:.2})", 
        ec_error, sensors.ec, config.ec_target, dose_ml, deadband_scale
    );

    let _ = ctx.reserve_dose_if_within_hourly_limit("NutrientA", current_time_sec, dose_ml, max_hourly_ml);
    let _ = ctx.reserve_dose_if_within_hourly_limit("NutrientB", current_time_sec, dose_ml, max_hourly_ml);
    
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

#[allow(clippy::too_many_arguments)]
fn try_ph_dosing(
    current_time_ms: u64,
    current_time_sec: u64,
    max_hourly_ml: f32,
    config: &ControllerConfig,
    sensors: &SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    fsm_mqtt_tx: &Sender<String>,
) -> bool {
    if !(config.enable_ph_sensor && (sensors.ph - config.ph_target).abs() > config.ph_tolerance) {
        return false;
    }

    if ctx.ph_retry_count >= 3 {
        let msg = "🚨 Hủy bù pH: Đã bơm thử 3 lần nhưng cảm biến pH không đổi hướng.";
        warn!("{}", msg);
        let alert_json = format!(r#"[SYSTEM ALERT] {{ "type": "fault", "source": "ph_dosing", "message": "{}" }}"#, msg);
        let _ = fsm_mqtt_tx.send(alert_json);
        
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
            error!("❌ [PH DOSING] Cấu hình bơm {} không hợp lệ hoặc PWM ({}%) quá thấp. Hủy tác vụ.", pump_name, safe_pwm);
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
    
    if !ctx.reserve_dose_if_within_hourly_limit(ph_pump_name, current_time_sec, dose_ml, max_hourly_ml) {
        let msg = format!("⚠️ [{}] Bơm bị khóa! Yêu cầu {:.2}ml làm vượt giới hạn giờ (Max: {}ml/h)", ph_pump_name, dose_ml, max_hourly_ml);
        warn!("{}", msg);
        
        let alert_json = format!(r#"[SYSTEM ALERT] {{ "type": "rate_limit", "source": "ph_dosing", "pump": "{}", "message": "{}" }}"#, ph_pump_name, msg);
        let _ = fsm_mqtt_tx.send(alert_json);

        ctx.stop_all_pumps(pump_ctrl);
        ctx.current_state = SystemState::SystemFault("MAX_HOURLY_DOSE_PH".to_string());
        return true;
    }

    let final_dose_ml = (diff / ratio * config.ph_step_ratio).clamp(0.0, config.max_dose_per_cycle);
    if final_dose_ml <= 0.0 {
        return false;
    }

    info!(
        "🧪 [PH DOSING] Bắt đầu bù pH ({}). Lệch: {:.2} (Hiện: {:.2}, Mục tiêu: {:.2}). Liều lượng: {:.2}ml", 
        if is_ph_up { "UP ⬆️" } else { "DOWN ⬇️" }, diff, sensors.ph, config.ph_target, final_dose_ml
    );

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
                    current_time_ms, config, sensors, ctx, pump_ctrl, dose_a_ml, dose_b_ml, pwm_percent, sensors.ec,
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
        PendingDose::EC { dose_ml, target_ec, pwm_percent } => {
            start_dosing_pump_a(
                current_time_ms, config, sensors, ctx, pump_ctrl, dose_ml, dose_ml, pwm_percent, target_ec,
            );
        }
        PendingDose::PH { is_up, dose_ml, target_ph, pwm_percent } => {
            start_dosing_ph(
                current_time_ms, config, sensors, ctx, pump_ctrl, is_up, dose_ml, target_ph, pwm_percent,
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
            error!("❌ [PUMP A SETUP] Bỏ qua bơm A: Cấu hình lưu lượng hoặc PWM ({}%) không hợp lệ.", dose_pwm);
            ctx.stop_all_pumps(pump_ctrl);
            ctx.current_state = SystemState::Monitoring;
            return;
        }
    };

    let (pulse_on_ms, pulse_off_ms, max_pulse_count) =
        pulse_params(dose_a_ml, active_capacity_a, config);

    debug!(
        "⚙️ [PUMP A SETUP] Target: {:.2}ml. Tốc độ: {:.2}ml/s (PWM: {}%). Chế độ: {}. Pulse [ON: {}ms, OFF: {}ms, Max: {}]",
        dose_a_ml, active_capacity_a, dose_pwm, 
        if dose_a_ml < config.dosing_min_dose_ml { "PULSE" } else { "LIÊN TỤC" },
        pulse_on_ms, pulse_off_ms, max_pulse_count
    );

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
            let pump_name = if is_up { "PhUp" } else { "PhDown" };
            error!("❌ [PUMP PH SETUP] Bỏ qua bơm {}: Cấu hình lưu lượng hoặc PWM ({}%) không hợp lệ.", pump_name, dose_pwm);
            ctx.stop_all_pumps(pump_ctrl);
            ctx.current_state = SystemState::Monitoring;
            return;
        }
    };

    let (pulse_on_ms, pulse_off_ms, max_pulse_count) =
        pulse_params(dose_ml, active_capacity, config);

    debug!(
        "⚙️ [PUMP PH SETUP] Target: {:.2}ml. Tốc độ: {:.2}ml/s (PWM: {}%). Chế độ: {}. Pulse [ON: {}ms, OFF: {}ms, Max: {}]",
        dose_ml, active_capacity, dose_pwm, 
        if dose_ml < config.dosing_min_dose_ml { "PULSE" } else { "LIÊN TỤC" },
        pulse_on_ms, pulse_off_ms, max_pulse_count
    );

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

        debug!(
            "💧 [PUMP A TICK] Ngắt bơm (Pulse: {}/{}). Đã bơm ước tính: {:.2}/{:.2} ml", 
            s.pulse_count, s.max_pulse_count, s.delivered_ml_est, s.dose_target_ml
        );

        if s.delivered_ml_est >= s.dose_target_ml || s.pulse_count >= s.max_pulse_count {
            info!("✅ [PUMP A DONE] Hoàn thành châm dung dịch A. Chuẩn bị chuyển pha.");
            ctx.set_pulse_status(false, s.pulse_count);
            ctx.current_state = SystemState::WaitingBetweenDose {
                finish_time: current_time_ms + (config.delay_between_a_and_b_sec as u64 * 1000),
                dose_b_ml: s.dose_b_ml,
                target_ec: s.target_ec,
                start_ec: s.start_ec,
                start_ph: s.start_ph,
                dose_a_ml_reported: s.delivered_ml_est,
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
            
        debug!("💧 [PUMP A TICK] Bật lại bơm (Bắt đầu Pulse {}).", next_count);

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
                    error!("❌ [PUMP B SETUP] Bỏ qua bơm B: Cấu hình lưu lượng hoặc PWM ({}%) không hợp lệ.", dose_pwm);
                    ctx.stop_all_pumps(pump_ctrl);
                    ctx.current_state = SystemState::Monitoring;
                    return;
                }
            };

        let (pulse_on_ms, pulse_off_ms, max_pulse_count) =
            pulse_params(dose_b_ml, active_capacity_b, config);

        debug!(
            "⚙️ [PUMP B SETUP] Target: {:.2}ml. Tốc độ: {:.2}ml/s (PWM: {}%). Chế độ: {}. Pulse [ON: {}ms, OFF: {}ms, Max: {}]",
            dose_b_ml, active_capacity_b, dose_pwm, 
            if dose_b_ml < config.dosing_min_dose_ml { "PULSE" } else { "LIÊN TỤC" },
            pulse_on_ms, pulse_off_ms, max_pulse_count
        );

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
    s: DosingPumpBState,
) {
    if s.pulse_on {
        let _ = pump_ctrl.set_dosing_pump_pulse(PumpType::NutrientB, false, 0);
        ctx.pump_status.pump_b = false;
        ctx.pump_status.pump_b_pwm = Some(0);

        debug!(
            "💧 [PUMP B TICK] Ngắt bơm (Pulse: {}/{}). Đã bơm ước tính: {:.2}/{:.2} ml", 
            s.pulse_count, s.max_pulse_count, s.delivered_ml_est, s.dose_target_ml
        );

        if s.delivered_ml_est >= s.dose_target_ml || s.pulse_count >= s.max_pulse_count {
            info!("✅ [PUMP B DONE] Hoàn thành châm dung dịch B. Chuẩn bị chuyển pha Active Mixing.");
            ctx.set_pulse_status(false, s.pulse_count);
            
            let pump_b_ml_reported = s.delivered_ml_est; // Lưu lại lượng thực tế

            start_pending_calibration_sample(
                ctx,
                s.start_ec,
                s.start_ph,
                s.dose_a_ml_reported,
                pump_b_ml_reported,
                0.0, // ph_up_ml
                0.0, // ph_down_ml
                current_time_ms,
                config,
            );

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
            
        debug!("💧 [PUMP B TICK] Bật lại bơm (Bắt đầu Pulse {}).", next_count);

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

        let pump_name = if s.is_up { "PH UP" } else { "PH DOWN" };
        debug!(
            "💧 [PUMP {} TICK] Ngắt bơm (Pulse: {}/{}). Đã bơm ước tính: {:.2}/{:.2} ml", 
            pump_name, s.pulse_count, s.max_pulse_count, s.delivered_ml_est, s.dose_target_ml
        );

        if s.delivered_ml_est >= s.dose_target_ml || s.pulse_count >= s.max_pulse_count {
            info!("✅ [PUMP {} DONE] Hoàn thành châm pH. Chuẩn bị chuyển pha Active Mixing.", pump_name);
            ctx.set_pulse_status(false, s.pulse_count);
            
            let ph_up_ml = if s.is_up {
                s.delivered_ml_est            } else {
                0.0
            };
            let ph_down_ml = if !s.is_up {
                s.delivered_ml_est
            } else {
                0.0
            };

            start_pending_calibration_sample(
                ctx,
                s.start_ec,
                s.start_ph,
                0.0, // pump_a_ml
                0.0, // pump_b_ml
                ph_up_ml,
                ph_down_ml,
                current_time_ms,
                config,
            );

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

        let pump_name = if s.is_up { "PH UP" } else { "PH DOWN" };
        debug!("💧 [PUMP {} TICK] Bật lại bơm (Bắt đầu Pulse {}).", pump_name, next_count);

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
