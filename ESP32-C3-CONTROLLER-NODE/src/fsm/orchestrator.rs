use std::str::FromStr;
use std::sync::mpsc::Sender;

use chrono::Local;
use cron::Schedule;

use esp_idf_svc::nvs::EspDefaultNvs;
use hydragrow_shared::{
    AlertMetadata, BasicSystemLogMetadata, ControllerConfig, LogCategory, LogLevel, SensorData,
    SystemLogEvent, WaterMetadata,
};
use log::warn;

use crate::{
    fsm::{optimizer::apply_safety_guardrails, peripheral::PeripheralController},
    pump::{PumpController, WaterDirection},
};

use super::{
    actors::dosing_actor::DosingEvent,
    matrix::{ControlVector, StateDeltaVector},
    phases::{FaultCode, SystemPhase},
    system_context::{AutoTuner, NvsSnapshot, SystemContext},
    types::PendingCalibrationSample,
    utils::send_system_log,
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
        let ec_val = sensors.ec as f32;
        let ph_val = sensors.ph as f32;
        let w_level = sensors.water_level as f32;
        let temp_val = sensors.temp as f32;

        // Tính toán độ lệch cơ bản
        let ec_delta = (config.ec_target - ec_val).max(0.0);
        let ph_delta = config.ph_target - ph_val;
        let water_delta = config.water_level_target - w_level;
        let temp_delta = config.misting_temp_threshold - temp_val;

        // 🛡️ CHỐT CHẶN BẢO VỆ MỚI: Chỉ tính sai số nếu cấu hình cho phép bật cảm biến (enable_sensor)
        // Nếu cảm biến bị tắt, hạ mức delta mục tiêu về 0.0 để cô lập trục tính toán Moore-Penrose hoàn toàn
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

        // Nếu tất cả các trục đều nằm trong deadband hoặc bị tắt cảm biến -> Đứng im trạng thái nghỉ (Idle)
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

                // 🛡️ LƯỚI BẢO VỆ GIAI ĐOẠN 5: Cắt tỉa và khóa chéo ranh giới vật lý cứng trước khi hạ lệnh xuống driver
                apply_safety_guardrails(&mut control, ec_val, ph_val, w_level, config);

                if control.nutrient_a_ml == 0.0
                    && control.nutrient_b_ml == 0.0
                    && control.ph_up_ml == 0.0
                    && control.ph_down_ml == 0.0
                    && control.water_in_sec == 0.0
                    && control.water_out_sec == 0.0
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
        if let Ok(schedule) = Schedule::from_str(&ctx.water_change_cron) {
            if let Some(next) = schedule.upcoming(Local).next() {
                ctx.next_water_change_trigger_sec = Some(next.timestamp() as u64);
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

    // 🛡️ CHỐT CHẶN AN TOÀN TOÁN HỌC: Đánh dấu hủy chu kỳ học ma trận hiện tại vì cấu trúc bồn nước sắp thay đổi hoàn toàn
    if let Some(sample) = ctx.calibration.pending_sample.as_mut() {
        sample.invalid_by_water_change = true;
    }

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
    ctx: &mut SystemContext,
    sensors: &SensorData,
    config: &ControllerConfig,
) -> bool {
    let mut is_noisy = false;
    let ec_val = sensors.ec as f32;
    let ph_val = sensors.ph as f32;

    if config.enable_ec_sensor && !sensors.err_ec.unwrap_or(false) {
        if let Some(prev_ec) = ctx.peripherals.previous_ec {
            if (ec_val - prev_ec).abs() > config.max_ec_delta {
                is_noisy = true;
            }
        }
        ctx.peripherals.previous_ec = Some(ec_val);
    }
    if config.enable_ph_sensor && !sensors.err_ph.unwrap_or(false) {
        if let Some(prev_ph) = ctx.peripherals.previous_ph {
            if (ph_val - prev_ph).abs() > config.max_ph_delta {
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
    post_water: f32,
    post_temp: f32,
) {
    tuner.kalman.predict();

    let delta_ec = post_ec - sample.start_ec;
    let delta_ph = post_ph - sample.start_ph;
    let delta_water = post_water - sample.start_water_level;
    let delta_temp = post_temp - sample.start_temp;

    // --- CỘT 0 & 1: Học đặc tính châm thuốc phân bón (Hàng 0: EC) ---
    if sample.dose_a_ml > 0.0 {
        let k0 = tuner.kalman.update_and_get_gain(0);
        tuner
            .interaction_matrix
            .update_column(0, sample.dose_a_ml, delta_ec, 0, k0);
    }
    if sample.dose_b_ml > 0.0 {
        let k1 = tuner.kalman.update_and_get_gain(1);
        tuner
            .interaction_matrix
            .update_column(1, sample.dose_b_ml, delta_ec, 0, k1);
    }

    // --- CỘT 2 & 3: Học đặc tính axit/kiềm (Hàng 1: pH) ---
    if sample.dose_ph_up_ml > 1e-3 {
        let k2 = tuner.kalman.update_and_get_gain(2);
        tuner
            .interaction_matrix
            .update_column(2, sample.dose_ph_up_ml, delta_ph, 1, k2);
    }
    if sample.dose_ph_down_ml > 1e-3 {
        let k3 = tuner.kalman.update_and_get_gain(3);
        tuner
            .interaction_matrix
            .update_column(3, sample.dose_ph_down_ml, delta_ph, 1, k3);
    }

    // --- CỘT 4: Học đặc tính Bơm cấp nước vào (Hàng 2: Mực nước & Hàng 0: Pha loãng EC) ---
    if sample.water_in_sec > 0.1 {
        let k4 = tuner.kalman.update_and_get_gain(4);
        tuner
            .interaction_matrix
            .update_column(4, sample.water_in_sec, delta_water, 2, k4);
        tuner
            .interaction_matrix
            .update_column(4, sample.water_in_sec, delta_ec, 0, k4);
    }

    // --- CỘT 5: Học đặc tính Bơm xả nước ra ngoài (Hàng 2: Mực nước) ---
    if sample.water_out_sec > 0.1 {
        let k5 = tuner.kalman.update_and_get_gain(5);
        tuner
            .interaction_matrix
            .update_column(5, sample.water_out_sec, delta_water, 2, k5);
    }

    // --- CỘT 7: Học đặc tính Van phun sương giải nhiệt (Hàng 2: Hao nước & Hàng 3: Tụt nhiệt độ) ---
    let actual_misting_sec = (sample
        .stabilizing_finish_ms
        .unwrap_or(sample.start_ms)
        .saturating_sub(sample.start_ms) as f32
        / 1000.0)
        .min(30.0);

    if actual_misting_sec > 0.1 {
        let k7 = tuner.kalman.update_and_get_gain(7);
        tuner
            .interaction_matrix
            .update_column(7, actual_misting_sec, delta_water, 2, k7);
        tuner
            .interaction_matrix
            .update_column(7, actual_misting_sec, delta_temp, 3, k7);
    }

    tuner.matrix_update_count = tuner.matrix_update_count.saturating_add(1);

    let c0 = tuner.kalman.confidence(0);
    let c1 = tuner.kalman.confidence(1);
    let c2 = tuner.kalman.confidence(2);
    let c3 = tuner.kalman.confidence(3);
    let c4 = tuner.kalman.confidence(4);
    let c5 = tuner.kalman.confidence(5);

    let is_now_warm = tuner.matrix_update_count >= 5
        && c0 > 0.75
        && c1 > 0.75
        && c2 > 0.75
        && c3 > 0.75
        && c4 > 0.75
        && c5 > 0.75;

    if is_now_warm && c0 > 0.90 {
        tuner.adaptive_ec_ratio = (tuner.adaptive_ec_ratio + 0.02).min(1.0);
    }

    tuner.matrix_is_warm = is_now_warm;
}

fn apply_decision(
    decision: OrchestratorDecision,
    ctx: &mut SystemContext,
    config: &ControllerConfig,
    sensors: &SensorData,
    now_ms: u64,
    mqtt_tx: &Sender<String>,
    pumps: &mut PumpController,
) {
    match decision {
        OrchestratorDecision::ExecuteMimoCycle {
            control,
            target_ec,
            target_ph,
            pwm,
        } => {
            let total_ml_this_cycle = control.nutrient_a_ml
                + control.nutrient_b_ml
                + control.ph_up_ml
                + control.ph_down_ml;
            if total_ml_this_cycle > 0.0
                && !ctx.safety.check_hourly_dose(
                    "dosing_total",
                    now_ms / 1000,
                    total_ml_this_cycle,
                    config.max_dose_per_hour,
                )
            {
                ctx.phase = SystemPhase::Fault(FaultCode::MaxHourlyDoseEc);
                return;
            }

            if control.water_in_sec > 0.0 {
                let _ = pumps.set_water_pump(WaterDirection::In);
                ctx.peripherals.pump_status.water_pump_in = true;
            }
            if control.water_out_sec > 0.0 {
                let _ = pumps.set_water_pump(WaterDirection::Out);
                ctx.peripherals.pump_status.water_pump_out = true;
            }
            if control.misting_sec > 0.0 {
                let _ = pumps.set_mist_valve(true);
                ctx.peripherals.pump_status.mist_valve = true;
                ctx.peripherals.is_misting_active = true;
            }

            ctx.dosing
                .start_matrix_cycle(now_ms, &control, target_ec, target_ph, pwm, config, sensors);

            let hardware_run_ms = (control
                .water_in_sec
                .max(control.water_out_sec)
                .max(control.misting_sec)
                * 1000.0) as u64;

            ctx.phase = SystemPhase::DosingEC;
            ctx.phase_start_ms = Some(now_ms);
            ctx.phase_finish_ms = Some(now_ms + hardware_run_ms + 5000);

            ctx.safety.last_ec_before_dose = Some(sensors.ec as f32);
            ctx.safety.last_ph_before_dose = Some(sensors.ph as f32);
            ctx.stabilizer_tracker.reset();

            send_system_log(
                mqtt_tx,
                &config.device_id,
                LogLevel::Info,
                LogCategory::System,
                "Kích hoạt chu kỳ đa biến MIMO",
                SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
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
                }),
            );
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
    sensor_last_update_ms: u64,
    ctx: &mut SystemContext,
    pumps: &mut PumpController,
    nvs: &mut Option<EspDefaultNvs>,
    dosing_report_tx: &Sender<String>,
    mqtt_tx: &Sender<String>,
) {
    let sensor_timeout_ms: u64 = 90_000;
    if now_ms.saturating_sub(sensor_last_update_ms) > sensor_timeout_ms {
        if !matches!(ctx.phase, SystemPhase::Monitoring | SystemPhase::Cooldown) {
            ctx.phase = SystemPhase::Fault(FaultCode::SensorTimeout);
        }
        return;
    }

    if check_sensor_noise(ctx, sensors, config) {
        return;
    }

    match &ctx.phase.clone() {
        SystemPhase::Monitoring => {
            let now_sec = now_ms / 1000;
            if let Some(decision) = check_scheduled_water_change(ctx, config, sensors, now_sec, nvs)
            {
                apply_decision(decision, ctx, config, sensors, now_ms, mqtt_tx, pumps);
            } else {
                let decision = MonitoringMatrixResult::solve_mimo(sensors, config, ctx);
                apply_decision(decision, ctx, config, sensors, now_ms, mqtt_tx, pumps);
            }
        }

        SystemPhase::DosingEC | SystemPhase::DosingPH => {
            let elapsed_ms = now_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(now_ms));

            if ctx.peripherals.pump_status.water_pump_in
                && elapsed_ms >= (config.max_refill_duration_sec as u64 * 1000)
            {
                let _ = pumps.set_water_pump(WaterDirection::Stop);
                ctx.peripherals.pump_status.water_pump_in = false;
            }
            if ctx.peripherals.pump_status.water_pump_out
                && elapsed_ms >= (config.max_drain_duration_sec as u64 * 1000)
            {
                let _ = pumps.set_water_pump(WaterDirection::Stop);
                ctx.peripherals.pump_status.water_pump_out = false;
            }

            match ctx.dosing.tick(now_ms, config, pumps) {
                DosingEvent::Pending => {}
                DosingEvent::SoftStartDone => {}
                DosingEvent::PulseToggle {
                    pump: _,
                    pulse_on: _,
                } => {}
                DosingEvent::PhaseTransition => {}
                DosingEvent::CycleComplete {
                    dose_a_ml,
                    dose_b_ml,
                    ph_up_ml,
                    ph_down_ml,
                } => {
                    let _ = pumps.set_water_pump(WaterDirection::Stop);
                    let _ = pumps.set_mist_valve(false);
                    ctx.peripherals.pump_status.water_pump_in = false;
                    ctx.peripherals.pump_status.water_pump_out = false;
                    ctx.peripherals.pump_status.mist_valve = false;
                    ctx.peripherals.is_misting_active = false;

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

                    ctx.calibration.start_sample(PendingCalibrationSample {
                        cycle_id: format!("mimo-{now_ms}"),
                        trigger: "mimo_matrix_control".to_string(),
                        start_ec: ctx.safety.last_ec_before_dose.unwrap_or(sensors.ec as f32),
                        start_ph: ctx.safety.last_ph_before_dose.unwrap_or(sensors.ph as f32),
                        start_water_level: sensors.water_level as f32,
                        start_temp: sensors.temp as f32,
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
                    });

                    ctx.phase = SystemPhase::ActiveMixing;
                    ctx.phase_start_ms = Some(now_ms);
                    ctx.phase_finish_ms =
                        Some(now_ms + ctx.diagnostic.adaptive_mixing_sec as u64 * 1000);
                    ctx.stabilizer_tracker.reset();
                }
                DosingEvent::Failed(code) => {
                    ctx.phase = SystemPhase::Fault(code);
                }
            }
        }

        SystemPhase::ActiveMixing => {
            ctx.stabilizer_tracker
                .push(sensors.ec as f32, sensors.ph as f32);

            let elapsed_ms = now_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(now_ms));
            let min_mixing_ms = 15_000;
            let max_mixing_timeout = now_ms >= ctx.phase_finish_ms.unwrap_or(0);

            if (elapsed_ms >= min_mixing_ms && ctx.stabilizer_tracker.is_stable(config))
                || max_mixing_timeout
            {
                if let Some(sample) = ctx.calibration.pending_sample.as_mut() {
                    sample.stabilizing_start_ms = Some(now_ms);
                }

                ctx.phase = SystemPhase::Stabilizing;
                ctx.phase_start_ms = Some(now_ms);
                ctx.phase_finish_ms =
                    Some(now_ms + ctx.diagnostic.adaptive_stabilize_sec as u64 * 1000);
                ctx.stabilizer_tracker.reset();
            }
        }

        SystemPhase::Stabilizing => {
            ctx.stabilizer_tracker
                .push(sensors.ec as f32, sensors.ph as f32);

            let elapsed_ms = now_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(now_ms));
            let min_stabilize_ms = 10_000;
            let max_stabilize_timeout = now_ms >= ctx.phase_finish_ms.unwrap_or(0);

            if (elapsed_ms >= min_stabilize_ms && ctx.stabilizer_tracker.is_stable(config))
                || max_stabilize_timeout
            {
                if let Some(mut sample) = ctx.calibration.finalize() {
                    ctx.dosing_cycle_count = ctx.dosing_cycle_count.saturating_add(1);

                    sample.stabilizing_finish_ms = Some(now_ms);
                    let final_ec = sensors.ec as f32;
                    let final_ph = sensors.ph as f32;
                    let final_water = sensors.water_level as f32;
                    let final_temp = sensors.temp as f32;

                    sample.post_mixing_ec = final_ec;
                    sample.post_mixing_ph = final_ph;

                    let actual_delta_ec = final_ec - sample.start_ec;
                    let actual_delta_ph = final_ph - sample.start_ph;
                    let actual_delta_water = final_water - sample.start_water_level;

                    if let Err(hardware_fault_code) = ctx.diagnostic.diagnose_hardware_fault(
                        &sample,
                        actual_delta_ec,
                        actual_delta_ph,
                        actual_delta_water,
                        config,
                    ) {
                        ctx.phase = SystemPhase::Fault(hardware_fault_code);
                        return;
                    }

                    let total_spent_mixing_ms = now_ms.saturating_sub(
                        sample.active_mixing_finish_ms
                            - (ctx.diagnostic.adaptive_mixing_sec as u64 * 1000),
                    );
                    let total_spent_stabilize_ms = elapsed_ms;
                    ctx.diagnostic
                        .learn_fluid_dynamics(total_spent_mixing_ms, total_spent_stabilize_ms);

                    if !sample.invalid_by_noise && !sample.invalid_by_water_change {
                        update_interaction_matrix(
                            &mut ctx.tuner,
                            &sample,
                            final_ec,
                            final_ph,
                            final_water,
                            final_temp,
                        );
                    } else {
                        warn!("⚠️ [GUARDRAIL] Bỏ qua bước cập nhật ma trận Kalman do dữ liệu mẫu nhiễm tạp chất thông tin.");
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
                }

                ctx.phase = SystemPhase::Cooldown;
                ctx.phase_finish_ms = Some(now_ms + config.cooldown_sec.max(0) as u64 * 1000);
            }
        }

        SystemPhase::Cooldown => {
            if now_ms >= ctx.phase_finish_ms.unwrap_or(0) {
                ctx.phase = SystemPhase::Monitoring;
                ctx.phase_start_ms = None;
                ctx.phase_finish_ms = None;
            }
        }
        _ => {}
    }

    let now_sec = now_ms / 1000;
    // Chỉ cho phép chạy sục trộn định kỳ hoặc các tác vụ ngoại vi khi FSM rảnh rỗi (Monitoring)
    // Tuyệt đối không cho phép chạy chèn dòng khi đang nằm trong pha khóa bảo vệ Cooldown hoặc đang châm hóa chất
    if matches!(ctx.phase, SystemPhase::Monitoring) {
        PeripheralController::tick_scheduled_mixing(&mut ctx.peripherals, now_sec, config);
        let is_dosing_active = false;
        PeripheralController::tick_osaka(&mut ctx.peripherals, pumps, is_dosing_active, config);
    } else {
        // Nếu đang trong Cooldown hoặc châm hóa chất, ép trạng thái sục tuần hoàn định kỳ về nghỉ an toàn
        ctx.peripherals.is_scheduled_mixing_active = false;
    }

    // Giữ mạch phun sương giải nhiệt độc lập để bảo vệ sự sống cho cây trồng nếu nhiệt độ phòng vượt ngưỡng
    PeripheralController::tick_misting(&mut ctx.peripherals, pumps, sensors, now_ms, config);
}

