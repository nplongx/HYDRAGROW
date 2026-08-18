// src/core/fsm/phases/stabilizing.rs
//! Phase Stabilizing — Chờ chỉ số EC/pH ổn định sau sục trộn và kích hoạt Pipeline Thích ứng (Adaptive Learning).
//! Thuộc tầng Pure Core: Không phụ thuộc phần cứng ESP-IDF hay I/O trực tiếp.

use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::log::{LogCategory, LogLevel, UnifiedSystemLog};
use hydragrow_shared::telemetry::cycle::{
    CycleOutcome, DosingDoseRecord, DosingPhaseSnapshot, KalmanLearningData,
};
use hydragrow_shared::telemetry::DosingCycleEvent;
use hydragrow_shared::{ControllerConfig, DoseData, DosingReportPayload, PhaseData, SensorData};
use log::warn;

use crate::core::fsm::context::{NvsSnapshot, SystemContext};
use crate::core::fsm::events::OrchestratorEvent;
use crate::core::fsm::phase_tick::PhaseTick;
use crate::core::fsm::tick_result::TickResult;
use crate::core::fsm::types::PendingCalibrationSample;

pub struct StabilizingPhase;

impl PhaseTick for StabilizingPhase {
    fn tick(
        &self,
        now_ms: u64,
        uptime_ms: u64, // SỬA: Thêm tham số uptime_ms
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();

        // SỬA: Dùng uptime_ms để tính thời gian trôi qua cực kỳ an toàn
        let elapsed_ms = uptime_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(uptime_ms));
        let min_stabilize_ms = 10_000;

        // SỬA: Dùng uptime_ms để check timeout
        let max_stabilize_timeout = uptime_ms >= ctx.phase_finish_ms.unwrap_or(0);

        // 1. Kiểm tra điều kiện hoàn thành Phase (Chỉ số đã ổn định hoặc Timeout)
        let is_ready = (elapsed_ms >= min_stabilize_ms && ctx.stabilizer_tracker.is_stable(config))
            || max_stabilize_timeout;

        if !is_ready {
            return result;
        }

        result.delta.dosing_cycle_count_increment = true;

        // 2. Chốt mẫu Calibration đang chờ
        if let Some(s) = ctx.calibration.pending_sample.as_mut() {
            s.stabilizing_finish_ms = Some(uptime_ms); // SỬA: Dùng uptime_ms vì start_ms cũng đang dùng uptime
        }

        if let Some(sample) = ctx.calibration.finalize() {
            let total_nutrient = sample.dose_a_ml + sample.dose_b_ml;
            let total_ph_agent = sample.dose_ph_up_ml + sample.dose_ph_down_ml;
            let actual_delta_ec = sensors.ec - sample.start_ec;
            let actual_delta_ph = sensors.ph - sample.start_ph;
            let actual_delta_water = sensors.water_level - sample.start_water_level;

            // A. Kiểm tra chẩn đoán lỗi phần cứng (Hardware Fault Diagnostics)
            if let Err(fault_code) = ctx.diagnostic.diagnose_hardware_fault(
                total_nutrient,
                total_ph_agent,
                sample.water_in_sec,
                sample.water_out_sec,
                actual_delta_ec,
                actual_delta_ph,
                actual_delta_water,
                config,
            ) {
                result.delta.phase = Some(SystemPhase::Fault(fault_code));
                return result;
            }

            // B. Chạy Pipeline Học máy Thích ứng (Adaptive Learning)
            let did_learn = ctx.tuner.learn_from_cycle(
                &sample,
                sensors.ec,
                sensors.ph,
                sensors.water_level,
                sensors.temp,
                config,
                uptime_ms / 1000, // SỬA: Dùng uptime để tránh rối loạn thời gian học trong nội bộ tuner
            );

            // C. Học đặc tính thủy động học (Fluid Dynamics - Thời gian sục/ổn định tối ưu)
            if did_learn {
                if let (Some(mixing_finish), Some(stab_start)) = (
                    Some(sample.active_mixing_finish_ms),
                    sample.stabilizing_start_ms,
                ) {
                    // Cả 3 biến này đều đang lưu bằng uptime_ms nên phép trừ này là chính xác tuyệt đối
                    let actual_mixing_ms = mixing_finish.saturating_sub(sample.start_ms);
                    let actual_stabilize_ms = uptime_ms.saturating_sub(stab_start);

                    if actual_mixing_ms > 1000 && actual_stabilize_ms > 1000 {
                        ctx.diagnostic
                            .learn_fluid_dynamics(actual_mixing_ms, actual_stabilize_ms);
                    }
                }
                result
                    .events
                    .push(OrchestratorEvent::PublishCalibrationUpdate);
            } else {
                warn!(
                    "⚠️ [GUARDRAIL] Bỏ qua cập nhật ma trận Kalman do dữ liệu bất thường (noise={}, water_change={}).",
                    sample.invalid_by_noise, sample.invalid_by_water_change
                );
            }

            // D. Tạo và phát các bản tin Telemetry, Report & System Logs
            // SỬA: Truyền cả now_ms (để ghi log) và uptime_ms (để tính duration)
            push_telemetry_events(
                &sample,
                sensors,
                config,
                ctx,
                now_ms,
                uptime_ms,
                did_learn,
                &mut result,
            );
        }

        // 3. Lưu NVS Snapshot xuống Flash
        let snapshot = NvsSnapshot::from_context(ctx, uptime_ms / 1000); // SỬA: Khớp với logic budget trong monitoring
        if serde_json::to_string(&snapshot).is_ok() {
            result.events.push(OrchestratorEvent::SaveNvsSnapshot);
        }

        // 4. Chuyển Phase sang Cooldown
        result.delta.phase = Some(SystemPhase::Cooldown);

        // SỬA: Cắm cờ phase_finish_ms bằng uptime tương lai
        result.delta.phase_finish_ms =
            Some(Some(uptime_ms + config.cooldown_sec.max(0) as u64 * 1000));

        result
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn push_telemetry_events(
    sample: &PendingCalibrationSample,
    sensors: &SensorData,
    config: &ControllerConfig,
    ctx: &SystemContext,
    now_ms: u64,
    uptime_ms: u64, // SỬA: Nhận thêm uptime_ms
    did_learn: bool,
    result: &mut TickResult,
) {
    let final_ec = sensors.ec;
    let final_ph = sensors.ph;
    let final_water = sensors.water_level;

    // 1. DosingCycleEvent (Backend Cloud Telemetry)
    let cycle_event = build_dosing_cycle_event(
        sample,
        final_ec,
        final_ph,
        final_water,
        config,
        ctx,
        now_ms,
        uptime_ms,
        did_learn,
    );
    if let Ok(json) = serde_json::to_string(&cycle_event) {
        result
            .events
            .push(OrchestratorEvent::PublishDosingCycle { cycle_json: json });
    }

    // 2. DosingReportPayload (Báo cáo Analytics chi tiết)
    let report = build_dosing_report_payload(sample, final_ec, final_ph, config, ctx, uptime_ms);
    if let Ok(json) = serde_json::to_string(&report) {
        result
            .events
            .push(OrchestratorEvent::PublishDosingReport { report_json: json });
    }

    // 3. System Log đọc được cho người dùng (Chỉ cần now_ms cho timestamp hiển thị)
    let human_message = build_human_message(sample, final_ec, final_ph, final_water, config);
    let log_payload = UnifiedSystemLog::build_basic_log_json_with_ts(
        &config.device_id,
        LogLevel::Success,
        LogCategory::Dosing,
        "Chu kỳ MIMO hoàn tất",
        human_message.trim().to_string(),
        Some(&sample.cycle_id),
        "stabilizing_phase",
        now_ms,
    );
    result.events.push(OrchestratorEvent::PublishSystemLog {
        payload_json: log_payload,
    });
}

fn build_dosing_cycle_event(
    sample: &PendingCalibrationSample,
    final_ec: f32,
    final_ph: f32,
    final_water: f32,
    config: &ControllerConfig,
    ctx: &SystemContext,
    now_ms: u64,
    uptime_ms: u64, // SỬA: Nhận thêm uptime_ms
    did_learn: bool,
) -> DosingCycleEvent {
    let ec_ok = (sample.target_ec - final_ec).abs() <= config.ec_tolerance;
    let ph_ok = (sample.target_ph - final_ph).abs() <= config.ph_tolerance;

    let outcome = if ec_ok && ph_ok {
        CycleOutcome::Success
    } else {
        CycleOutcome::PartialSuccess {
            ec_reached: ec_ok,
            ph_reached: ph_ok,
        }
    };

    let kalman = if did_learn {
        Some(KalmanLearningData {
            ec_gain_before: config.ec_gain_per_ml,
            ec_gain_after: ctx
                .tuner
                .gain_learner
                .effective_ec_gain(config.ec_gain_per_ml),
            ph_up_gain_before: config.ph_shift_up_per_ml,
            ph_up_gain_after: ctx
                .tuner
                .gain_learner
                .effective_ph_up_gain(config.ph_shift_up_per_ml),
            ph_down_gain_before: config.ph_shift_down_per_ml,
            ph_down_gain_after: ctx
                .tuner
                .gain_learner
                .effective_ph_down_gain(config.ph_shift_down_per_ml),
            matrix_update_count: ctx.tuner.matrix_update_count,
            matrix_is_warm: ctx.tuner.matrix_is_warm,
            adaptive_mixing_sec: ctx.diagnostic.adaptive_mixing_sec,
            adaptive_stabilize_sec: ctx.diagnostic.adaptive_stabilize_sec,
        })
    } else {
        None
    };

    DosingCycleEvent {
        cycle_id: sample.cycle_id.clone(),
        device_id: config.device_id.clone(),
        trigger: sample.trigger.clone(),
        pre: DosingPhaseSnapshot {
            ec: sample.start_ec,
            ph: sample.start_ph,
            water_level: sample.start_water_level,
            temp: Some(sample.start_temp),
        },
        post_mixing: DosingPhaseSnapshot {
            ec: sample.post_mixing_ec,
            ph: sample.post_mixing_ph,
            water_level: final_water,
            temp: None,
        },
        post_stable: DosingPhaseSnapshot {
            ec: final_ec,
            ph: final_ph,
            water_level: final_water,
            temp: None,
        },
        target_ec: sample.target_ec,
        target_ph: sample.target_ph,
        dose: DosingDoseRecord {
            pump_a_ml: sample.dose_a_ml,
            pump_b_ml: sample.dose_b_ml,
            ph_up_ml: sample.dose_ph_up_ml,
            ph_down_ml: sample.dose_ph_down_ml,
            water_in_sec: sample.water_in_sec,
            water_out_sec: sample.water_out_sec,
        },
        outcome,
        // SỬA: Mọi phép tính duration phải dùng uptime_ms vì các mốc start_ms được lưu bằng uptime
        duration_ms: uptime_ms.saturating_sub(sample.start_ms),
        mixing_duration_ms: sample
            .active_mixing_finish_ms
            .saturating_sub(sample.start_ms),
        stabilize_duration_ms: uptime_ms
            .saturating_sub(sample.stabilizing_start_ms.unwrap_or(uptime_ms)),
        timestamp_ms: now_ms, // SỬA: Giữ nguyên now_ms để hiển thị thời gian gửi log chuẩn xác lên App
        kalman,
        season_id: None,
    }
}

fn build_dosing_report_payload(
    sample: &PendingCalibrationSample,
    final_ec: f32,
    final_ph: f32,
    config: &ControllerConfig,
    ctx: &SystemContext,
    uptime_ms: u64,
) -> DosingReportPayload {
    DosingReportPayload {
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
        delta_ec: final_ec - sample.start_ec,
        delta_ph: final_ph - sample.start_ph,
        target_ec: sample.target_ec,
        target_ph: sample.target_ph,
        error_ec: sample.target_ec - final_ec,
        error_ph: sample.target_ph - final_ph,
        duration_ms: uptime_ms.saturating_sub(sample.start_ms),
        ema_ec_gain_used: config.ec_gain_per_ml,
        ema_ph_shift_used: config.ph_shift_up_per_ml,

        // 🟢 Cập nhật gọi hàm trung bình ()
        step_ratio_ec: Some(ctx.tuner.adaptive_ec_ratio()),
        step_ratio_ph: Some(ctx.tuner.adaptive_ph_ratio()),

        stabilized_window_sec: Some(ctx.diagnostic.adaptive_stabilize_sec),
    }
}

fn build_human_message(
    sample: &PendingCalibrationSample,
    final_ec: f32,
    final_ph: f32,
    final_water: f32,
    config: &ControllerConfig,
) -> String {
    let mut msg = String::new();
    let actual_delta_ec = final_ec - sample.start_ec;

    if sample.dose_a_ml > 0.0 || sample.dose_b_ml > 0.0 {
        let total = sample.dose_a_ml + sample.dose_b_ml;
        if config.enable_ec_sensor && actual_delta_ec > 0.02 {
            msg.push_str(&format!(
                "Hệ thống đã bổ sung {:.1}ml dinh dưỡng (EC dâng từ {:.2} lên {:.2} mS/cm). ",
                total, sample.start_ec, final_ec
            ));
        } else {
            msg.push_str(&format!(
                "Hệ thống đã phân phối {:.1}ml dinh dưỡng vào bể chứa. ",
                total
            ));
        }
    }

    if sample.dose_ph_up_ml > 0.01 {
        msg.push_str(&format!(
            "Đã châm {:.1}ml kiềm, pH từ {:.2} về {:.2}. ",
            sample.dose_ph_up_ml, sample.start_ph, final_ph
        ));
    } else if sample.dose_ph_down_ml > 0.01 {
        msg.push_str(&format!(
            "Đã hạ pH từ {:.2} về {:.2} ({:.1}ml). ",
            sample.start_ph, final_ph, sample.dose_ph_down_ml
        ));
    }

    if sample.water_in_sec > 0.1 {
        msg.push_str(&format!(
            "Đã cấp nước {:.1}s, mực nước {:.1}%. ",
            sample.water_in_sec, final_water
        ));
    } else if sample.water_out_sec > 0.1 {
        msg.push_str(&format!(
            "Đã xả nước {:.1}s, mực nước {:.1}%. ",
            sample.water_out_sec, final_water
        ));
    }

    if msg.is_empty() {
        msg = format!(
            "MIMO: cân bằng sinh học hoàn hảo (pH: {:.2}, Mực nước: {:.1}%).",
            final_ph, final_water
        );
    }

    msg
}
