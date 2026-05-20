use std::str::FromStr;
use std::sync::mpsc::Sender;

use chrono::Local;
use cron::Schedule;

use esp_idf_svc::nvs::EspDefaultNvs;
use hydragrow_shared::{
    AlertMetadata, BasicSystemLogMetadata, ControllerConfig, LogCategory, LogLevel, SensorData,
    SystemLogEvent, WaterMetadata,
};

use crate::pump::PumpController;

use super::{
    actors::{dosing_actor::DosingEvent, water_actor::WaterEvent},
    optimizer::apply_deadband,
    peripheral::PeripheralController,
    phases::{FaultCode, SystemPhase},
    system_context::{AutoTuner, NvsSnapshot, SystemContext},
    types::PendingCalibrationSample,
    utils::send_system_log,
};

enum OrchestratorDecision {
    StartEcDosing {
        dose_ml: f32,
        target_ec: f32,
        pwm: u32,
    },
    StartPhDosing {
        is_up: bool,
        dose_ml: f32,
        target_ph: f32,
        pwm: u32,
    },
    StartWaterFill {
        target: f32,
    },
    StartWaterDrain {
        target: f32,
        trigger: String,
    },
    Idle,
    Fault(FaultCode),
}

struct MonitoringMatrixResult {
    pump_a_ml: f32,
    ph_agent_ml: f32,
}

impl MonitoringMatrixResult {
    fn solve(ec_delta: f32, ph_delta: f32, config: &ControllerConfig, ctx: &SystemContext) -> Self {
        let mut pump_a_ml = 0.0;
        let mut ph_agent_ml = 0.0;

        if ctx.tuner.matrix_is_warm {
            // Dùng matrix prediction để back-calculate dose
            // EC: dose_a = ec_delta / matrix[0][0] (col A → row EC)
            let ec_gain_from_matrix = ctx.tuner.interaction_matrix.get(0, 0).max(0.0001);
            if config.enable_ec_sensor && ec_delta > config.ec_tolerance {
                let deadband_scale = apply_deadband(ec_delta, config.ec_tolerance);
                let step_ratio = if ctx.tuner.is_locked() {
                    ctx.tuner.best_ec_ratio
                } else {
                    ctx.tuner.active_ec_ratio()
                };
                pump_a_ml = (ec_delta / ec_gain_from_matrix * step_ratio * deadband_scale)
                    .clamp(0.0, config.max_dose_per_cycle);
            }

            // pH: dùng matrix[1][2] (ph_up agent → row pH)
            if config.enable_ph_sensor && ph_delta.abs() > config.ph_tolerance {
                let is_up = ph_delta > 0.0;
                let delta = ph_delta.abs();
                let deadband_scale = apply_deadband(delta, config.ph_tolerance);
                let step_ratio = if ctx.tuner.is_locked() {
                    ctx.tuner.best_ph_ratio
                } else {
                    ctx.tuner.adaptive_ph_ratio
                };
                // col 2 = pH agent; sign: ph_up làm tăng pH (positive row 1)
                let ph_gain_from_matrix = ctx.tuner.interaction_matrix.get(1, 2).abs().max(0.0001);
                let dose_ml = (delta / ph_gain_from_matrix * step_ratio * deadband_scale)
                    .clamp(0.0, config.max_dose_per_cycle);
                ph_agent_ml = if is_up { dose_ml } else { -dose_ml };
            }
        } else {
            // Cold path: scalar gain (giữ nguyên logic cũ)
            if config.enable_ec_sensor && ec_delta > config.ec_tolerance {
                let deadband_scale = apply_deadband(ec_delta, config.ec_tolerance);
                let step_ratio = if ctx.tuner.is_locked() {
                    ctx.tuner.best_ec_ratio
                } else {
                    ctx.tuner.active_ec_ratio()
                };
                let effective_gain = ctx
                    .tuner
                    .gain_learner
                    .effective_ec_gain(config.ec_gain_per_ml)
                    .max(0.0001);
                pump_a_ml = (ec_delta / effective_gain * step_ratio * deadband_scale)
                    .clamp(0.0, config.max_dose_per_cycle);
            }

            if config.enable_ph_sensor && ph_delta.abs() > config.ph_tolerance {
                let is_up = ph_delta > 0.0;
                let delta = ph_delta.abs();
                let deadband_scale = apply_deadband(delta, config.ph_tolerance);
                let step_ratio = if ctx.tuner.is_locked() {
                    ctx.tuner.best_ph_ratio
                } else {
                    ctx.tuner.adaptive_ph_ratio
                };
                let effective_gain = if is_up {
                    ctx.tuner
                        .gain_learner
                        .effective_ph_up_gain(config.ph_shift_up_per_ml)
                } else {
                    ctx.tuner
                        .gain_learner
                        .effective_ph_down_gain(config.ph_shift_down_per_ml)
                }
                .max(0.0001);
                let dose_ml = (delta / effective_gain * step_ratio * deadband_scale)
                    .clamp(0.0, config.max_dose_per_cycle);
                ph_agent_ml = if is_up { dose_ml } else { -dose_ml };
            }
        }

        Self {
            pump_a_ml,
            ph_agent_ml,
        }
    }
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

    let w_level = sensors.water_level as f32; // f64 -> f32
    let target = (w_level - config.scheduled_drain_amount_cm).max(config.water_level_min);

    // THÊM: log trigger (không thể dùng mqtt_tx vì hàm không nhận tx)
    // Thay vào đó, trả về một wrapper để caller log.
    // → Giải pháp đơn giản: thêm log khi apply_decision nhận StartWaterDrain với trigger="scheduled_change"
    // → Đã cover bởi Step 2 ở trên với trigger field trong message.
    // → Không cần thêm gì ở đây.

    Some(OrchestratorDecision::StartWaterDrain {
        target,
        trigger: "scheduled_change".to_string(),
    })
}

fn check_scheduled_dosing(
    ctx: &mut SystemContext,
    config: &ControllerConfig,
    sensors: &SensorData,
    now_sec: u64,
) -> Option<OrchestratorDecision> {
    if !(config.enable_ec_sensor && config.is_enabled) {
        return None;
    }
    // Reuse cron field to support preventive dosing without backend schema change.
    if ctx.scheduled_dosing_cron != config.water_change_cron {
        ctx.scheduled_dosing_cron = config.water_change_cron.clone();
        match Schedule::from_str(&ctx.scheduled_dosing_cron) {
            Ok(schedule) => {
                if let Some(next) = schedule.upcoming(Local).next() {
                    ctx.next_scheduled_dosing_trigger_sec = Some(next.timestamp() as u64);
                }
            }
            Err(_) => {
                ctx.next_scheduled_dosing_trigger_sec = None;
                return None;
            }
        }
    }

    let next_trigger = ctx.next_scheduled_dosing_trigger_sec?;
    if now_sec < next_trigger {
        return None;
    }

    if let Ok(schedule) = Schedule::from_str(&ctx.scheduled_dosing_cron) {
        let future = Local::now() + chrono::Duration::seconds(1);
        if let Some(next) = schedule.after(&future).next() {
            ctx.next_scheduled_dosing_trigger_sec = Some(next.timestamp() as u64);
        }
    }

    let ec_val = sensors.ec as f32; // f64 -> f32
    let delta = (config.ec_target - ec_val).max(config.ec_tolerance * 0.5);
    let step_ratio = if ctx.tuner.is_locked() {
        ctx.tuner.best_ec_ratio
    } else {
        ctx.tuner.active_ec_ratio()
    };

    let effective_gain = ctx
        .tuner
        .gain_learner
        .effective_ec_gain(config.ec_gain_per_ml)
        .max(0.0001);

    let dose_ml =
        (delta / effective_gain * step_ratio * 0.35).clamp(0.0, config.max_dose_per_cycle);

    if dose_ml <= 0.0 {
        return None;
    }

    Some(OrchestratorDecision::StartEcDosing {
        dose_ml,
        target_ec: config.ec_target,
        pwm: config.dosing_pwm_percent.clamp(1, 100) as u32,
    })
}

fn decide_monitoring_matrix(
    sensors: &SensorData,
    config: &ControllerConfig,
    ctx: &SystemContext,
) -> OrchestratorDecision {
    let ec_val = sensors.ec as f32;
    let ph_val = sensors.ph as f32;
    let ec_delta = (config.ec_target - ec_val).max(0.0);
    let ph_delta = config.ph_target - ph_val;

    if !ctx.tuner.matrix_is_warm {
        if config.enable_ec_sensor && ec_val < (config.ec_target - config.ec_tolerance) {
            let deadband_scale = apply_deadband(ec_delta, config.ec_tolerance);
            let step_ratio = if ctx.tuner.is_locked() {
                ctx.tuner.best_ec_ratio
            } else {
                ctx.tuner.active_ec_ratio()
            };
            let effective_gain = ctx
                .tuner
                .gain_learner
                .effective_ec_gain(config.ec_gain_per_ml)
                .max(0.0001);
            let dose_ml = (ec_delta / effective_gain * step_ratio * deadband_scale)
                .clamp(0.0, config.max_dose_per_cycle);
            if dose_ml > 0.0 {
                return OrchestratorDecision::StartEcDosing {
                    dose_ml,
                    target_ec: config.ec_target,
                    pwm: config.dosing_pwm_percent.clamp(1, 100) as u32,
                };
            }
        }

        if config.enable_ph_sensor {
            if ph_val > (config.ph_target + config.ph_tolerance) {
                let delta = (ph_val - config.ph_target).max(0.0);
                let deadband_scale = apply_deadband(delta, config.ph_tolerance);
                let step_ratio = if ctx.tuner.is_locked() {
                    ctx.tuner.best_ph_ratio
                } else {
                    ctx.tuner.adaptive_ph_ratio
                };
                let effective_gain = ctx
                    .tuner
                    .gain_learner
                    .effective_ph_down_gain(config.ph_shift_down_per_ml)
                    .max(0.0001);
                let dose_ml = (delta / effective_gain * step_ratio * deadband_scale)
                    .clamp(0.0, config.max_dose_per_cycle);
                if dose_ml > 0.0 {
                    return OrchestratorDecision::StartPhDosing {
                        is_up: false,
                        dose_ml,
                        target_ph: config.ph_target,
                        pwm: config.dosing_pwm_percent.clamp(1, 100) as u32,
                    };
                }
            } else if ph_val < (config.ph_target - config.ph_tolerance) {
                let delta = (config.ph_target - ph_val).max(0.0);
                let deadband_scale = apply_deadband(delta, config.ph_tolerance);
                let step_ratio = if ctx.tuner.is_locked() {
                    ctx.tuner.best_ph_ratio
                } else {
                    ctx.tuner.adaptive_ph_ratio
                };
                let effective_gain = ctx
                    .tuner
                    .gain_learner
                    .effective_ph_up_gain(config.ph_shift_up_per_ml)
                    .max(0.0001);
                let dose_ml = (delta / effective_gain * step_ratio * deadband_scale)
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

        return OrchestratorDecision::Idle;
    }

    let optimized = MonitoringMatrixResult::solve(ec_delta, ph_delta, config, ctx);

    if config.enable_ec_sensor && optimized.pump_a_ml > 0.0 {
        return OrchestratorDecision::StartEcDosing {
            dose_ml: optimized.pump_a_ml,
            target_ec: config.ec_target,
            pwm: config.dosing_pwm_percent.clamp(1, 100) as u32,
        };
    }

    if config.enable_ph_sensor && optimized.ph_agent_ml.abs() >= config.dosing_min_dose_ml {
        return OrchestratorDecision::StartPhDosing {
            is_up: optimized.ph_agent_ml.is_sign_positive(),
            dose_ml: optimized.ph_agent_ml.abs(),
            target_ph: config.ph_target,
            pwm: config.dosing_pwm_percent.clamp(1, 100) as u32,
        };
    }

    OrchestratorDecision::Idle
}

fn decide_monitoring(
    sensors: &SensorData,
    config: &ControllerConfig,
    ctx: &SystemContext,
    _now_ms: u64,
) -> OrchestratorDecision {
    let w_level = sensors.water_level as f32; // f64 -> f32
    let ec_val = sensors.ec as f32; // f64 -> f32

    if config.enable_water_level_sensor && w_level < 0.0 {
        return OrchestratorDecision::Fault(FaultCode::SensorTimeout);
    }

    if config.enable_water_level_sensor
        && config.auto_drain_overflow
        && w_level > config.water_level_max
    {
        return OrchestratorDecision::StartWaterDrain {
            target: config.water_level_target,
            trigger: "overflow".to_string(),
        };
    }

    if config.enable_water_level_sensor
        && config.auto_refill_enabled
        && w_level < (config.water_level_target - config.water_level_tolerance)
    {
        return OrchestratorDecision::StartWaterFill {
            target: config.water_level_target,
        };
    }

    if config.enable_ec_sensor
        && config.enable_water_level_sensor
        && config.auto_dilute_enabled
        && ec_val > (config.ec_target + config.ec_tolerance)
    {
        return OrchestratorDecision::StartWaterDrain {
            target: config.water_level_target,
            trigger: "auto_dilute".to_string(),
        };
    }

    decide_monitoring_matrix(sensors, config, ctx)
}

fn check_sensor_noise(
    ctx: &mut SystemContext,
    sensors: &SensorData,
    config: &ControllerConfig,
) -> bool {
    let mut is_noisy = false;
    let ec_val = sensors.ec as f32;
    let ph_val = sensors.ph as f32;

    if config.enable_ec_sensor && !sensors.err_ec.unwrap_or(false) {
        if let Some(prev_ec) = ctx.peripherals.previous_ec {
            let delta = (ec_val - prev_ec).abs();
            if delta > config.max_ec_delta {
                is_noisy = true;
            }
        }
        ctx.peripherals.previous_ec = Some(ec_val);
    }

    if config.enable_ph_sensor && !sensors.err_ph.unwrap_or(false) {
        if let Some(prev_ph) = ctx.peripherals.previous_ph {
            let delta = (ph_val - prev_ph).abs();
            if delta > config.max_ph_delta {
                is_noisy = true;
            }
        }
        ctx.peripherals.previous_ph = Some(ph_val);
    }

    is_noisy
}

fn update_interaction_matrix(
    tuner: &mut AutoTuner,
    sample: &PendingCalibrationSample,
    post_ec: f32,
    post_ph: f32,
) {
    let ec_dose_ml = sample.dose_a_ml + sample.dose_b_ml;
    let ph_dose_ml = sample.dose_ph_up_ml + sample.dose_ph_down_ml;
    if ec_dose_ml <= 0.0 && ph_dose_ml <= 0.0 {
        return;
    }

    let observed_delta_ec = post_ec - sample.start_ec;
    let observed_delta_ph = post_ph - sample.start_ph;

    // Predict step: tăng uncertainty trước khi cập nhật
    tuner.kalman.predict();

    // Cập nhật từng kênh: gọi update_and_get_gain ngay trước khi dùng gain
    if sample.dose_a_ml > 0.0 {
        let k_a = tuner.kalman.update_and_get_gain(0);
        tuner.interaction_matrix.update_column(
            0,
            sample.dose_a_ml,
            observed_delta_ec,
            0, // row EC
            k_a,
        );
    }

    if sample.dose_b_ml > 0.0 {
        let k_b = tuner.kalman.update_and_get_gain(1);
        tuner.interaction_matrix.update_column(
            1,
            sample.dose_b_ml,
            observed_delta_ec,
            0, // row EC
            k_b,
        );
    }

    let net_ph_dose_ml = sample.dose_ph_up_ml - sample.dose_ph_down_ml;
    if net_ph_dose_ml.abs() > 1e-6 {
        let k_ph = tuner.kalman.update_and_get_gain(2);
        tuner.interaction_matrix.update_column(
            2,
            net_ph_dose_ml,
            observed_delta_ph,
            1, // row pH
            k_ph,
        );
    }

    tuner.matrix_update_count = tuner.matrix_update_count.saturating_add(1);
    tuner.matrix_is_warm = tuner.matrix_update_count >= 10;

    let just_became_warm = !tuner.matrix_is_warm && tuner.matrix_update_count >= 10;
    tuner.matrix_is_warm = tuner.matrix_update_count >= 10;

    if just_became_warm {
        log::info!(
            "🧠 [MATRIX] Ma trận tương tác đã đủ ấm sau {} chu kỳ! \
         Chuyển sang chế độ inference phối hợp EC/pH.",
            tuner.matrix_update_count
        );
    }

    let data = tuner.interaction_matrix.data;
    log::debug!(
        "[ORCH] Matrix updated: ec_a={:.6}, ec_b={:.6}, ph_ph={:.6}, updates={}, warm={}",
        data[0][0],
        data[0][1],
        data[1][2],
        tuner.matrix_update_count,
        tuner.matrix_is_warm,
    );
}

fn apply_decision(
    decision: OrchestratorDecision,
    ctx: &mut SystemContext,
    config: &ControllerConfig,
    sensors: &SensorData,
    now_ms: u64,
    mqtt_tx: &Sender<String>,
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
                .start_ec_cycle(now_ms, dose_ml, target_ec, pwm, config, sensors);
            ctx.phase = SystemPhase::DosingEC;
            ctx.safety.last_ec_before_dose = Some(sensors.ec as f32);

            let cycle_id = ctx
                .dosing
                .cycle_ctx
                .as_ref()
                .map(|c| c.cycle_id.clone())
                .unwrap_or_else(|| format!("ec-{now_ms}"));
            send_system_log(
                mqtt_tx,
                &config.device_id,
                LogLevel::Info,
                LogCategory::Dosing,
                "Bắt đầu châm EC",
                SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
                    source: "orchestrator".to_string(),
                    message: format!(
                        "Bơm A+B: {:.2}ml | EC hiện tại: {:.2} | Mục tiêu: {:.2} | PWM: {}%",
                        dose_ml, sensors.ec, target_ec, pwm
                    ),
                    skip_reason: None,
                    cycle_id: Some(cycle_id),
                }),
            );
        }
        OrchestratorDecision::StartPhDosing {
            is_up,
            dose_ml,
            target_ph,
            pwm,
        } => {
            if !ctx
                .safety
                .check_hourly_dose("ph", now_ms / 1000, dose_ml, config.max_dose_per_hour)
            {
                ctx.phase = SystemPhase::Fault(FaultCode::MaxHourlyDosePh);
                return;
            }
            ctx.dosing
                .start_ph_cycle(now_ms, is_up, dose_ml, target_ph, pwm, config, sensors);

            ctx.phase = SystemPhase::DosingPH;
            ctx.safety.last_ph_before_dose = Some(sensors.ph as f32);
            ctx.safety.last_ph_dose_up = Some(is_up);

            let cycle_id = ctx
                .dosing
                .cycle_ctx
                .as_ref()
                .map(|c| c.cycle_id.clone())
                .unwrap_or_else(|| format!("ph-{now_ms}"));
            let direction = if is_up { "Up" } else { "Down" };
            send_system_log(
                mqtt_tx,
                &config.device_id,
                LogLevel::Info,
                LogCategory::Dosing,
                &format!("Bắt đầu châm pH {direction}"),
                SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
                    source: "orchestrator".to_string(),
                    message: format!(
                        "pH {direction}: {:.2}ml | pH hiện tại: {:.2} | Mục tiêu: {:.2} | PWM: {}%",
                        dose_ml, sensors.ph, target_ph, pwm
                    ),
                    skip_reason: None,
                    cycle_id: Some(cycle_id),
                }),
            );
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
            ctx.safety.last_water_before_refill = Some(sensors.water_level as f32);

            send_system_log(
                mqtt_tx,
                &config.device_id,
                LogLevel::Info,
                LogCategory::Water,
                "Bắt đầu bơm nước vào",
                SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
                    source: "orchestrator".to_string(),
                    message: format!(
                        "Mức nước hiện tại: {:.1}cm | Mục tiêu: {:.1}cm | Timeout: {}s",
                        sensors.water_level, target, config.max_refill_duration_sec
                    ),
                    skip_reason: None,
                    cycle_id: None,
                }),
            );
        }
        OrchestratorDecision::StartWaterDrain { target, trigger } => {
            if !ctx
                .safety
                .record_drain(now_ms / 1000, config.max_drain_cycles_per_hour as u32)
            {
                ctx.phase = SystemPhase::Fault(FaultCode::TooManyDrains);
                return;
            }
            if trigger == "scheduled_change" || trigger == "manual_drain" {
                ctx.tuner.on_water_change();
            }
            ctx.water.start_drain(now_ms, target, sensors, &trigger);
            ctx.phase = SystemPhase::WaterDraining;

            send_system_log(
                mqtt_tx,
                &config.device_id,
                LogLevel::Info,
                LogCategory::Water,
                "Bắt đầu xả nước",
                SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
                    source: "orchestrator".to_string(),
                    message: format!(
                        "Trigger: {} | Mức nước hiện tại: {:.1}cm | Mục tiêu xả về: {:.1}cm",
                        trigger, sensors.water_level, target
                    ),
                    skip_reason: None,
                    cycle_id: None,
                }),
            );
        }
        OrchestratorDecision::Fault(code) => {
            ctx.phase = SystemPhase::Fault(code);
        }
        OrchestratorDecision::Idle => {}
    }
}

fn log_fault_transition(code: &FaultCode, mqtt_tx: &Sender<String>, device_id: &str) {
    send_system_log(
        mqtt_tx,
        device_id,
        LogLevel::Critical,
        LogCategory::Alert,
        &format!("Lỗi hệ thống: {}", code.as_str()),
        SystemLogEvent::SystemAlert(AlertMetadata {
            alert_type: code.as_str().to_string(),
            source: "fsm_orchestrator".to_string(),
            retry_count: 0,
            limit_value: None,
            threshold_before: None,
            threshold_after: None,
        }),
    );
}

fn set_fault_with_log(
    ctx: &mut SystemContext,
    code: FaultCode,
    mqtt_tx: &Sender<String>,
    device_id: &str,
) {
    log_fault_transition(&code, mqtt_tx, device_id);
    ctx.phase = SystemPhase::Fault(code);
}

#[allow(clippy::too_many_arguments)]
pub fn tick(
    now_ms: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    sensor_last_update_ms: u64, // THÊM: Biến phụ để biết lần cuối nhận dữ liệu cảm biến
    ctx: &mut SystemContext,
    pumps: &mut PumpController,
    nvs: &mut Option<EspDefaultNvs>,
    dosing_report_tx: &Sender<String>,
    mqtt_tx: &Sender<String>,
) {
    let sensor_timeout_ms: u64 = 90_000;

    let sensor_age_ms = now_ms.saturating_sub(sensor_last_update_ms);

    if sensor_age_ms > sensor_timeout_ms {
        if !matches!(ctx.phase, SystemPhase::Monitoring | SystemPhase::Cooldown) {
            set_fault_with_log(ctx, FaultCode::SensorTimeout, mqtt_tx, &config.device_id);
            send_system_log(
                mqtt_tx,
                &config.device_id,
                LogLevel::Critical,
                LogCategory::Sensor,
                "Sensor timeout — FSM tạm dừng",
                SystemLogEvent::SystemAlert(AlertMetadata {
                    alert_type: "SENSOR_TIMEOUT".to_string(),
                    source: "orchestrator".to_string(),
                    retry_count: 0,
                    limit_value: Some(sensor_timeout_ms as f32 / 1000.0),
                    threshold_before: Some(sensor_age_ms as f32 / 1000.0),
                    threshold_after: None,
                }),
            );
        }
        return;
    }

    let is_noisy = check_sensor_noise(ctx, sensors, config);
    if is_noisy {
        if let Some(sample) = ctx.calibration.pending_sample.as_mut() {
            sample.invalid_by_noise = true;
        }
        return;
    }

    match &ctx.phase.clone() {
        SystemPhase::Monitoring => {
            let now_sec = now_ms / 1000;
            if let Some(decision) = check_scheduled_water_change(ctx, config, sensors, now_sec, nvs)
            {
                apply_decision(decision, ctx, config, sensors, now_ms, mqtt_tx);

                // Log trạng thái matrix định kỳ (không spam)
                if ctx.dosing_cycle_count % 10 == 0 {
                    let flat = ctx.tuner.interaction_matrix.as_flat();
                    log::debug!(
                        "[MATRIX] warm={} updates={} ec_a={:.5} ec_b={:.5} ph_ph={:.5}",
                        ctx.tuner.matrix_is_warm,
                        ctx.tuner.matrix_update_count,
                        flat[0],
                        flat[1],
                        flat[5]
                    );
                }
            } else if let Some(decision) = check_scheduled_dosing(ctx, config, sensors, now_sec) {
                apply_decision(decision, ctx, config, sensors, now_ms, mqtt_tx);

                // Log trạng thái matrix định kỳ (không spam)
                if ctx.dosing_cycle_count % 10 == 0 {
                    let flat = ctx.tuner.interaction_matrix.as_flat();
                    log::debug!(
                        "[MATRIX] warm={} updates={} ec_a={:.5} ec_b={:.5} ph_ph={:.5}",
                        ctx.tuner.matrix_is_warm,
                        ctx.tuner.matrix_update_count,
                        flat[0],
                        flat[1],
                        flat[5]
                    );
                }
            } else {
                let decision = decide_monitoring(sensors, config, ctx, now_ms);
                apply_decision(decision, ctx, config, sensors, now_ms, mqtt_tx);

                // Log trạng thái matrix định kỳ (không spam)
                if ctx.dosing_cycle_count % 10 == 0 {
                    let flat = ctx.tuner.interaction_matrix.as_flat();
                    log::debug!(
                        "[MATRIX] warm={} updates={} ec_a={:.5} ec_b={:.5} ph_ph={:.5}",
                        ctx.tuner.matrix_is_warm,
                        ctx.tuner.matrix_update_count,
                        flat[0],
                        flat[1],
                        flat[5]
                    );
                }

                if matches!(ctx.phase, SystemPhase::Fault(_)) {
                    if let SystemPhase::Fault(code) = &ctx.phase {
                        log_fault_transition(code, mqtt_tx, &config.device_id);
                    }
                }
            }
        }

        SystemPhase::DosingEC | SystemPhase::DosingPH => {
            match ctx.dosing.tick(now_ms, config, pumps) {
                DosingEvent::Pending => {}
                DosingEvent::SoftStartDone => {
                    // let _ = mqtt_tx.send("[ORCH] Dosing soft-start completed".to_string());
                    log::debug!("[ORCH] Dosing soft-start completed");
                }
                DosingEvent::PulseToggle { pump, pulse_on } => {
                    // Cập nhật pump_status để frontend/backend biết pump nào đang pulse
                    match &pump {
                        crate::fsm::actors::dosing_actor::PumpTarget::NutrientA { .. } => {
                            ctx.peripherals.pump_status.pump_a = pulse_on;
                            ctx.peripherals.pump_status.dosing_pulse_active = Some(pulse_on);
                        }
                        crate::fsm::actors::dosing_actor::PumpTarget::NutrientB => {
                            ctx.peripherals.pump_status.pump_b = pulse_on;
                            ctx.peripherals.pump_status.dosing_pulse_active = Some(pulse_on);
                        }
                        crate::fsm::actors::dosing_actor::PumpTarget::PhUp => {
                            ctx.peripherals.pump_status.ph_up = pulse_on;
                        }
                        crate::fsm::actors::dosing_actor::PumpTarget::PhDown => {
                            ctx.peripherals.pump_status.ph_down = pulse_on;
                        }
                    }
                    if !pulse_on {
                        // Khi tắt pulse, increment counter
                        let prev = ctx.peripherals.pump_status.dosing_pulse_count.unwrap_or(0);
                        ctx.peripherals.pump_status.dosing_pulse_count =
                            Some(prev.saturating_add(1));
                    }
                    log::debug!("[ORCH] Dosing Pulse Toggle: {:?} -> ON: {}", pump, pulse_on);
                }
                DosingEvent::PhaseTransition => {
                    // let _ = mqtt_tx.send("[ORCH] Dosing Phase Transition".to_string());
                    log::debug!("[ORCH] A→B phase transition");
                }
                DosingEvent::CycleComplete {
                    dose_a_ml,
                    dose_b_ml,
                    ph_up_ml,
                    ph_down_ml,
                } => {
                    ctx.peripherals.pump_status.pump_a = false;
                    ctx.peripherals.pump_status.pump_b = false;
                    ctx.peripherals.pump_status.ph_up = false;
                    ctx.peripherals.pump_status.ph_down = false;
                    ctx.peripherals.pump_status.dosing_pulse_active = Some(false);

                    if let Some(c) = ctx.dosing.cycle_ctx.clone() {
                        ctx.calibration.start_sample(PendingCalibrationSample {
                            cycle_id: c.cycle_id,
                            trigger: c.trigger,
                            start_ec: sensors.ec as f32,
                            start_ph: sensors.ph as f32,
                            start_water_level: sensors.water_level as f32,
                            target_ec: c.target_ec,
                            target_ph: c.target_ph,
                            dose_a_ml,
                            dose_b_ml,
                            dose_ph_up_ml: ph_up_ml,
                            dose_ph_down_ml: ph_down_ml,
                            post_mixing_ec: c.post_mixing_ec,
                            post_mixing_ph: c.post_mixing_ph,
                            start_ms: c.start_ms,
                            active_mixing_finish_ms: now_ms
                                + config.active_mixing_sec as u64 * 1000,
                            stabilizing_start_ms: None,
                            stabilizing_finish_ms: None,
                            invalid_by_noise: false,
                            invalid_by_water_change: false,
                        });
                    }

                    send_system_log(
                        mqtt_tx,
                        &config.device_id,
                        LogLevel::Info,
                        LogCategory::Dosing,
                        "Hoàn tất bơm — bắt đầu hòa trộn",
                        SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
                            source: "dosing_actor".to_string(),
                            message: format!(
                    "A: {:.2}ml | B: {:.2}ml | pH Up: {:.2}ml | pH Down: {:.2}ml | Mixing: {}s",
                    dose_a_ml, dose_b_ml, ph_up_ml, ph_down_ml,
                    config.active_mixing_sec
                ),
                            skip_reason: None,
                            cycle_id: Some(c.cycle_id.clone()),
                        }),
                    );

                    ctx.phase = SystemPhase::ActiveMixing;
                    ctx.phase_finish_ms = Some(now_ms + config.active_mixing_sec as u64 * 1000);
                }
                DosingEvent::Failed(code) => {
                    log_fault_transition(&code, mqtt_tx, &config.device_id);
                    // let _ = mqtt_tx.send(format!("[ORCH] dosing_failed:{}", code.as_str()));
                    ctx.phase = SystemPhase::Fault(code);
                }
            }
        }

        SystemPhase::WaterRefilling | SystemPhase::WaterDraining => {
            let water_log_ctx = match &ctx.water.sub_state {
                super::actors::water_actor::WaterSubState::Filling { job }
                | super::actors::water_actor::WaterSubState::Draining { job } => {
                    Some((job.trigger.clone(), job.start_level, job.target_level))
                }
                super::actors::water_actor::WaterSubState::Idle => None,
            };

            let is_filling = matches!(ctx.phase, SystemPhase::WaterRefilling);

            match ctx.water.tick(now_ms, sensors, config, pumps) {
                WaterEvent::Done {
                    success,
                    duration_sec,
                } => {
                    send_system_log(
                        mqtt_tx,
                        &config.device_id,
                        if success {
                            LogLevel::Info
                        } else {
                            LogLevel::Warning
                        },
                        LogCategory::Water,
                        if success {
                            "Hoàn tất cấp/xả nước"
                        } else {
                            "Timeout cấp/xả nước"
                        },
                        SystemLogEvent::WaterEvent(WaterMetadata {
                            source: "water_actor".to_string(),
                            trigger: water_log_ctx
                                .as_ref()
                                .map(|x| x.0.clone())
                                .unwrap_or_else(|| "unknown".to_string()),
                            level_before: water_log_ctx.as_ref().map(|x| x.1).unwrap_or(0.0),
                            level_after: sensors.water_level as f32, // f64 -> f32
                            target_level: water_log_ctx.as_ref().map(|x| x.2).unwrap_or(0.0),
                            duration_sec,
                            success,
                            cycle_id: None,
                            retry_count: Some(ctx.water.retry_refill),
                        }),
                    );

                    if success {
                        ctx.phase = SystemPhase::ActiveMixing;
                        ctx.phase_finish_ms = Some(now_ms + config.active_mixing_sec as u64 * 1000);
                    } else {
                        if is_filling {
                            ctx.water.retry_refill = ctx.water.retry_refill.saturating_add(1);

                            if ctx.water.retry_refill >= 3 {
                                set_fault_with_log(
                                    ctx,
                                    FaultCode::WaterRefillFailed,
                                    mqtt_tx,
                                    &config.device_id,
                                );
                            } else {
                                let target = config.water_level_target;
                                ctx.water
                                    .start_fill(now_ms, target, sensors, "retry_auto_refill");
                                send_system_log(
                                    mqtt_tx,
                                    &config.device_id,
                                    LogLevel::Warning,
                                    LogCategory::Water,
                                    "Đang thử lại bơm nước",
                                    SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
                                        source: "orchestrator".to_string(),
                                        message: format!(
                                            "Lần thử {}/3 — mức nước hiện tại: {:.1}cm, mục tiêu: {:.1}cm",
                                            ctx.water.retry_refill, sensors.water_level, target
                                        ),
                                        skip_reason: None,
                                    }),
                                );
                            }
                        } else {
                            set_fault_with_log(
                                ctx,
                                FaultCode::WaterLevelCritical,
                                mqtt_tx,
                                &config.device_id,
                            );
                        }
                    }
                }
                WaterEvent::Pending => {}
            }
        }

        SystemPhase::ActiveMixing => {
            if now_ms >= ctx.phase_finish_ms.unwrap_or(0) {
                // LOG: vào Stabilizing
                let cycle_id = ctx
                    .calibration
                    .pending_sample
                    .as_ref()
                    .map(|s| s.cycle_id.clone());
                send_system_log(
                    mqtt_tx,
                    &config.device_id,
                    LogLevel::Info,
                    LogCategory::Dosing,
                    "Hòa trộn xong — chờ ổn định cảm biến",
                    SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
                        source: "orchestrator".to_string(),
                        message: format!(
                            "Đang chờ {}s để cảm biến ổn định | EC hiện tại: {:.2} | pH: {:.2}",
                            config.sensor_stabilize_sec, sensors.ec, sensors.ph
                        ),
                        skip_reason: None,
                        cycle_id,
                    }),
                );

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
                    sample.post_mixing_ec = sensors.ec as f32; // f64 -> f32
                    sample.post_mixing_ph = sensors.ph as f32; // f64 -> f32
                }

                if let Some(before) = ctx.safety.last_ec_before_dose {
                    if (sensors.ec as f32) < before + config.ec_ack_threshold {
                        ctx.dosing.retry_ec = ctx.dosing.retry_ec.saturating_add(1);
                        if ctx.dosing.retry_ec >= 3 {
                            set_fault_with_log(
                                ctx,
                                FaultCode::EcDosingFailed,
                                mqtt_tx,
                                &config.device_id,
                            );
                            // let _ =
                            //     mqtt_tx.send("[ORCH] ec_ack_failed_after_3_retries".to_string());
                            return;
                        }
                    } else {
                        ctx.dosing.retry_ec = 0;
                    }
                }

                if let (Some(before), Some(is_up)) =
                    (ctx.safety.last_ph_before_dose, ctx.safety.last_ph_dose_up)
                {
                    let ph_val = sensors.ph as f32;
                    let moved = if is_up {
                        ph_val - before
                    } else {
                        before - ph_val
                    };
                    if moved < config.ph_ack_threshold {
                        ctx.dosing.retry_ph = ctx.dosing.retry_ph.saturating_add(1);
                        if ctx.dosing.retry_ph >= 3 {
                            set_fault_with_log(
                                ctx,
                                FaultCode::PhDosingFailed,
                                mqtt_tx,
                                &config.device_id,
                            );
                            // let _ =
                            //     mqtt_tx.send("[ORCH] ph_ack_failed_after_3_retries".to_string());
                            if ctx.dosing.retry_ec >= 3 {
                                set_fault_with_log(
                                    ctx,
                                    FaultCode::PhDosingFailed,
                                    mqtt_tx,
                                    &config.device_id,
                                );
                                return;
                            }
                            return;
                        }
                    } else {
                        ctx.dosing.retry_ph = 0;
                    }
                }

                if let (Some(before), Some(is_up)) =
                    (ctx.safety.last_ph_before_dose, ctx.safety.last_ph_dose_up)
                {
                    let ph_val = sensors.ph as f32;
                    let ph_response = if is_up {
                        ph_val - before
                    } else {
                        before - ph_val
                    };
                    ctx.tuner.on_ph_dosing_ack(
                        ph_response,
                        config.ph_ack_threshold,
                        config,
                        is_up,
                        now_ms / 1000,
                    );
                }

                if let Some(sample) = ctx.calibration.finalize() {
                    ctx.dosing_cycle_count = ctx.dosing_cycle_count.saturating_add(1);
                    let ec_response = sample.post_mixing_ec - sample.start_ec;
                    let ec_dose_ml = sample.dose_a_ml + sample.dose_b_ml;
                    if ec_dose_ml > 0.0 {
                        ctx.tuner.gain_learner.update_ec_gain(
                            ec_dose_ml,
                            ec_response.max(0.0),
                            config,
                        );
                    }

                    let ph_val = sensors.ph as f32;
                    let ph_response_signed = ph_val - sample.start_ph;
                    if sample.dose_ph_up_ml > 0.0 {
                        ctx.tuner.gain_learner.update_ph_gain(
                            sample.dose_ph_up_ml,
                            ph_response_signed.max(0.0),
                            true,
                            config,
                        );
                    }
                    if sample.dose_ph_down_ml > 0.0 {
                        ctx.tuner.gain_learner.update_ph_gain(
                            sample.dose_ph_down_ml,
                            (-ph_response_signed).max(0.0),
                            false,
                            config,
                        );
                    }
                    if sample.dose_a_ml > 0.0 || sample.dose_b_ml > 0.0 {
                        ctx.tuner.on_ec_dosing_ack(
                            ec_response,
                            config.ec_ack_threshold,
                            config,
                            now_ms / 1000,
                        );
                    }
                    update_interaction_matrix(
                        &mut ctx.tuner,
                        &sample,
                        sensors.ec as f32,
                        sensors.ph as f32,
                    );

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
                            ec: sensors.ec as f32,
                            ph: sensors.ph as f32,
                            water_level: None,
                        },
                        delta_ec: (sensors.ec as f32) - sample.start_ec,
                        delta_ph: (sensors.ph as f32) - sample.start_ph,
                        target_ec: sample.target_ec,
                        target_ph: sample.target_ph,
                        error_ec: sample.target_ec - (sensors.ec as f32),
                        error_ph: sample.target_ph - (sensors.ph as f32),
                        duration_ms: now_ms.saturating_sub(sample.start_ms),
                        ema_ec_gain_used: config.ec_gain_per_ml,
                        ema_ph_shift_used: if sample.dose_ph_up_ml > 0.0 {
                            config.ph_shift_up_per_ml
                        } else {
                            config.ph_shift_down_per_ml
                        },
                        step_ratio_ec: Some(ctx.tuner.active_ec_ratio()),
                        step_ratio_ph: Some(ctx.tuner.adaptive_ph_ratio),
                        stabilized_window_sec: Some(config.sensor_stabilize_sec as u32),
                    };

                    if let Ok(json) = serde_json::to_string(&report) {
                        let _ = dosing_report_tx.send(json);
                    }
                    let calibration_payload =
                        ctx.tuner.to_mqtt_payload(&config.device_id, config, now_ms);
                    let _ = mqtt_tx.send(calibration_payload);
                }
                if let Some(flash) = nvs.as_mut() {
                    let snapshot = NvsSnapshot::from_context(ctx, now_ms / 1000);
                    if let Ok(serialized) = serde_json::to_string(&snapshot) {
                        let _ = flash.set_str("runtime_snap", &serialized);
                    }
                    let _ = flash.set_u64("last_w_change", ctx.last_water_change_sec);
                }

                ctx.phase = SystemPhase::Cooldown;
                let effective_cooldown = config.cooldown_sec.max(0) as u64;
                ctx.phase_finish_ms = Some(now_ms + effective_cooldown * 1000);
            }
        }

        SystemPhase::Cooldown => {
            if now_ms >= ctx.phase_finish_ms.unwrap_or(0) {
                ctx.phase = SystemPhase::Monitoring;
                ctx.phase_finish_ms = None;
            }
        }

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

#[cfg(test)]
pub fn solve_for_test(
    ec_delta: f32,
    ph_delta: f32,
    config: &hydragrow_shared::ControllerConfig,
    ctx: &crate::fsm::system_context::SystemContext,
) -> (f32, f32) {
    let r = MonitoringMatrixResult::solve(ec_delta, ph_delta, config, ctx);
    (r.pump_a_ml, r.ph_agent_ml)
}
