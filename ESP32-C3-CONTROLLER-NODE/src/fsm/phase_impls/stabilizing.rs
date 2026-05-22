// src/fsm/phase_impls/stabilizing.rs
use hydragrow_shared::{
    BasicSystemLogMetadata, ControllerConfig, DoseData, DosingReportPayload, LogCategory, LogLevel,
    PhaseData, SensorData,
};
use log::warn;

use crate::fsm::events::OrchestratorEvent;
use crate::fsm::phase_impls::SystemPhase;
use crate::fsm::phase_tick::PhaseTick;
use crate::fsm::system_context::{NvsSnapshot, SystemContext};
use crate::fsm::tick_result::TickResult;

pub struct StabilizingPhase;

impl PhaseTick for StabilizingPhase {
    fn tick(
        &self,
        now_ms: u64,
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();
        let elapsed_ms = now_ms.saturating_sub(ctx.phase_start_ms.unwrap_or(now_ms));
        let min_stabilize_ms = 10_000;
        let max_stabilize_timeout = now_ms >= ctx.phase_finish_ms.unwrap_or(0);

        if !((elapsed_ms >= min_stabilize_ms && ctx.stabilizer_tracker.is_stable(config))
            || max_stabilize_timeout)
        {
            return result;
        }

        result.delta.dosing_cycle_count_increment = true;

        if let Some(s) = ctx.calibration.pending_sample.as_mut() {
            s.stabilizing_finish_ms = Some(now_ms);
        }

        if let Some(sample) = &ctx.calibration.pending_sample {
            let final_ec = sensors.ec;
            let final_ph = sensors.ph;
            let final_water = sensors.water_level;
            let actual_delta_ec = final_ec - sample.start_ec;
            let actual_delta_ph = final_ph - sample.start_ph;
            let actual_delta_water = final_water - sample.start_water_level;

            if let Err(fault_code) = ctx.diagnostic.diagnose_hardware_fault(
                sample,
                actual_delta_ec,
                actual_delta_ph,
                actual_delta_water,
                config,
            ) {
                result.delta.phase = Some(SystemPhase::Fault(fault_code));
                return result;
            }

            // ADAPTIVE LEARNING PIPELINE — chỉ học khi hardware diagnostic đã pass
            let did_learn = ctx.tuner.learn_from_cycle(
                sample,
                sensors.ec,
                sensors.ph,
                sensors.water_level,
                sensors.temp,
                config,
                now_ms / 1000,
            );

            // Học fluid dynamics (thời gian mixing/stabilizing tối ưu)
            if did_learn {
                if let (Some(mixing_finish), Some(stab_start)) = (
                    Some(sample.active_mixing_finish_ms),
                    sample.stabilizing_start_ms,
                ) {
                    let actual_mixing_ms = mixing_finish.saturating_sub(sample.start_ms);
                    let actual_stabilize_ms = now_ms.saturating_sub(stab_start);
                    if actual_mixing_ms > 1000 && actual_stabilize_ms > 1000 {
                        ctx.diagnostic
                            .learn_fluid_dynamics(actual_mixing_ms, actual_stabilize_ms);
                    }
                }
            }

            if did_learn {
                result
                    .events
                    .push(OrchestratorEvent::PublishCalibrationUpdate);
            } else {
                warn!("⚠️ [GUARDRAIL] Bỏ qua cập nhật ma trận Kalman do dữ liệu bất thường (invalid_by_noise={}, invalid_by_water_change={}).",
        sample.invalid_by_noise, sample.invalid_by_water_change);
            }

            // Build human-readable message
            let human_message =
                build_human_message(sample, final_ec, final_ph, final_water, config);

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
                source: "stabilizing_phase".to_string(),
                message: human_message.trim().to_string(),
                skip_reason: None,
                cycle_id: Some(sample.cycle_id.clone()),
            })
            .to_string();

            result.events.push(OrchestratorEvent::PublishSystemLog {
                payload_json: log_payload,
            });
        }

        let snapshot = NvsSnapshot::from_context(ctx, now_ms / 1000);
        if serde_json::to_string(&snapshot).is_ok() {
            result.events.push(OrchestratorEvent::SaveNvsSnapshot);
        }

        result.delta.phase = Some(SystemPhase::Cooldown);
        result.delta.phase_finish_ms =
            Some(Some(now_ms + config.cooldown_sec.max(0) as u64 * 1000));

        result
    }
}

fn build_human_message(
    sample: &crate::fsm::types::PendingCalibrationSample,
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
