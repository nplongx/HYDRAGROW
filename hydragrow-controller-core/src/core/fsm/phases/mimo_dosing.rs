// src/fsm/phase_impls/mimo_dosing.rs
use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::{ControllerConfig, SensorData};
use log::warn;

use crate::WaterDirection;
use crate::core::actors::dosing_actor::{DosingEvent, PumpTarget};
use crate::core::actors::water_actor::{WaterEvent, WaterSubState};
use crate::core::fsm::tick_result::CalibrationDelta;
use crate::core::fsm::types::PendingCalibrationSample;
use crate::core::fsm::{
    DosingPumpTarget, OrchestratorEvent, PeripheralDelta, PhaseTick, SystemContext, TickResult,
};

pub struct MimoDosingPhase;

impl PhaseTick for MimoDosingPhase {
    fn tick(
        &self,
        now_ms: u64,
        uptime: u64, // Đã thêm tham số uptime
        config: &ControllerConfig,
        sensors: &SensorData,
        ctx: &mut SystemContext,
    ) -> TickResult {
        let mut result = TickResult::default();
        let mut peri_delta = PeripheralDelta::default();

        // SỬA: Dùng `uptime` để tính thời gian trôi qua, chống lỗi nhảy cóc thời gian
        let elapsed_ms = uptime.saturating_sub(ctx.phase_start_ms.unwrap_or(uptime));

        // 1. Kiểm tra Safety Timeout của bơm nước dựa trên water_pump_started_uptime_ms
        self.check_water_pump_timeouts(uptime, config, ctx, &mut result, &mut peri_delta);

        // 2. Hard Timeout toàn Phase -> Chuyển Cooldown
        // SỬA: Dùng `uptime` để so sánh và thiết lập mốc thời gian tương lai
        if uptime
            >= ctx
                .phase_finish_ms
                .unwrap_or(u64::MAX)
                .saturating_add(5_000)
        {
            warn!("⚠️ [FSM] Dosing phase timeout cứng! Chuyển về Cooldown.");
            stop_water_and_misting(ctx, &mut result, &mut peri_delta);

            result.events.push(OrchestratorEvent::PublishFsmTransition {
                from_phase: SystemPhase::MimoDosing,
                to_phase: SystemPhase::Cooldown,
                // TODO: replace with TransitionReason::HardTimeout if shared telemetry adds it.
                reason: hydragrow_shared::telemetry::transition::TransitionReason::Manual {
                    description: "MimoDosing hard timeout".to_string(),
                },
                phase_duration_ms: Some(
                    uptime.saturating_sub(ctx.phase_start_ms.unwrap_or(uptime)),
                ),
            });

            result.delta.phase = Some(SystemPhase::Cooldown);
            result.delta.phase_finish_ms =
                Some(Some(uptime + config.cooldown_sec.max(30) as u64 * 1000));
            result.delta.peripherals = Some(peri_delta);
            return result;
        }

        // 3. Tick DosingActor & xử lý sự kiện
        // SỬA: Truyền `uptime` vào DosingActor để các bộ đếm xung PWM bên trong không bị lỗi khi mất/có Wi-Fi
        let (dosing_event, hardware_events) = ctx.dosing.tick(uptime, config);
        result.events.extend(hardware_events);

        let is_water_active = matches!(
            ctx.water.sub_state,
            WaterSubState::Filling { .. } | WaterSubState::Draining { .. }
        );

        let water_event = if is_water_active {
            let (event, hw_events, sys_log) = ctx.water.tick(uptime, sensors, config);
            result.events.extend(hw_events);
            result
                .events
                .extend(sys_log.into_iter().filter_map(|mut log| {
                    log.timestamp_ms = now_ms;
                    serde_json::to_string(&log)
                        .ok()
                        .map(|payload_json| OrchestratorEvent::PublishSystemLog { payload_json })
                }));
            Some(event)
        } else {
            None
        };

        match dosing_event {
            DosingEvent::Pending => {
                if ctx.dosing.is_idle() {
                    if let Some(w_event) = water_event {
                        match w_event {
                            WaterEvent::Done { duration_sec, .. } => {
                                let (water_in_spent, water_out_spent) =
                                    if ctx.peripherals.pump_status.water_pump_in {
                                        (duration_sec as f32, 0.0)
                                    } else {
                                        (0.0, duration_sec as f32)
                                    };

                                stop_water_and_misting(ctx, &mut result, &mut peri_delta);

                                let sample = build_calibration_sample(
                                    format!("water-{now_ms}"), // GIỮ NGUYÊN: Dùng now_ms để tạo ID dễ đọc cho người dùng
                                    "water_only_cycle".to_string(),
                                    (0.0, 0.0, 0.0, 0.0),
                                    (water_in_spent, water_out_spent),
                                    uptime, // SỬA: Dùng uptime cho logic tracking bên trong
                                    sensors,
                                    config,
                                    ctx,
                                );
                                transition_to_active_mixing(uptime, sample, ctx, &mut result);
                            }
                            WaterEvent::Pending => {
                                // Chu kỳ bơm nước đang chạy — tiếp tục ở MimoDosing
                            }
                        }
                    } else if elapsed_ms >= 500 {
                        // Không có nước chạy và dosing đã hoàn tất
                        stop_water_and_misting(ctx, &mut result, &mut peri_delta);

                        let sample = build_calibration_sample(
                            format!("idle-{now_ms}"),
                            "idle_cycle".to_string(),
                            (0.0, 0.0, 0.0, 0.0),
                            (0.0, 0.0),
                            uptime,
                            sensors,
                            config,
                            ctx,
                        );
                        transition_to_active_mixing(uptime, sample, ctx, &mut result);
                    }
                }
            }
            DosingEvent::SoftStartDone | DosingEvent::PhaseTransition => {}
            DosingEvent::PulseToggle { pump, pulse_on } => {
                let target_pump = match pump {
                    PumpTarget::NutrientA { .. } => DosingPumpTarget::NutrientA,
                    PumpTarget::NutrientB => DosingPumpTarget::NutrientB,
                    PumpTarget::PhUp => DosingPumpTarget::PhUp,
                    PumpTarget::PhDown => DosingPumpTarget::PhDown,
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

                stop_water_and_misting(ctx, &mut result, &mut peri_delta);

                // Tắt trạng thái ảo
                peri_delta.pump_a = Some(false);
                peri_delta.pump_b = Some(false);
                peri_delta.ph_up = Some(false);
                peri_delta.ph_down = Some(false);

                let sample = build_calibration_sample(
                    format!("mimo-{now_ms}"), // GIỮ NGUYÊN: ID dùng mốc giờ thực tế
                    "mimo_matrix_control".to_string(),
                    (dose_a_ml, dose_b_ml, ph_up_ml, ph_down_ml),
                    (water_in_spent, water_out_spent),
                    uptime, // SỬA: Logic tracking dùng thời gian chip (uptime)
                    sensors,
                    config,
                    ctx,
                );
                transition_to_active_mixing(uptime, sample, ctx, &mut result); // SỬA: Dùng uptime
            }
            DosingEvent::Failed(code) => {
                result.delta.phase = Some(SystemPhase::Fault(code));
            }
        }

        result.delta.peripherals = Some(peri_delta);
        result
    }
}

impl MimoDosingPhase {
    /// Kiểm tra và ngắt bơm nước nếu chạy quá thời gian cấu hình tối đa
    fn check_water_pump_timeouts(
        &self,
        uptime: u64,
        config: &ControllerConfig,
        ctx: &SystemContext,
        result: &mut TickResult,
        peri_delta: &mut PeripheralDelta,
    ) {
        let pump_elapsed_ms = uptime.saturating_sub(
            ctx.peripherals
                .water_pump_started_uptime_ms
                .unwrap_or(uptime),
        );
        if ctx.peripherals.pump_status.water_pump_in
            && pump_elapsed_ms >= (config.max_refill_duration_sec as u64 * 1000)
        {
            result.events.push(OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop,
            });
            peri_delta.water_pump_in = Some(false);
            peri_delta.water_pump_started_uptime_ms = Some(None);
        }
        if ctx.peripherals.pump_status.water_pump_out
            && pump_elapsed_ms >= (config.max_drain_duration_sec as u64 * 1000)
        {
            result.events.push(OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop,
            });
            peri_delta.water_pump_out = Some(false);
            peri_delta.water_pump_started_uptime_ms = Some(None);
        }
    }
}

// --- Helper Functions Thuần Tuý Cho Module ---

/// Tắt bơm nước và van phun sương (nếu được bật bởi dosing)
fn stop_water_and_misting(
    ctx: &SystemContext,
    result: &mut TickResult,
    peri_delta: &mut PeripheralDelta,
) {
    if !result.events.iter().any(|e| {
        matches!(
            e,
            OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop
            }
        )
    }) {
        result.events.push(OrchestratorEvent::SetWaterPump {
            direction: WaterDirection::Stop,
        });
    }
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
    peri_delta.water_pump_started_uptime_ms = Some(None);
}

/// Khởi tạo mẫu Calibration DTO
#[allow(clippy::too_many_arguments)]
fn build_calibration_sample(
    cycle_id: String,
    trigger: String,
    doses_ml: (f32, f32, f32, f32), // (A, B, pH Up, pH Down)
    water_sec: (f32, f32),          // (In, Out)
    uptime: u64,                    // SỬA: Đổi tham số now_ms thành uptime
    sensors: &SensorData,
    config: &ControllerConfig,
    ctx: &SystemContext,
) -> PendingCalibrationSample {
    let (dose_a_ml, dose_b_ml, dose_ph_up_ml, dose_ph_down_ml) = doses_ml;
    let (water_in_sec, water_out_sec) = water_sec;

    PendingCalibrationSample {
        cycle_id,
        trigger,
        start_ec: ctx.safety.last_ec_before_dose.unwrap_or(sensors.ec),
        start_ph: ctx.safety.last_ph_before_dose.unwrap_or(sensors.ph),
        start_water_level: sensors.water_level,
        start_temp: sensors.temp,
        target_ec: config.ec_target,
        target_ph: config.ph_target,
        dose_a_ml,
        dose_b_ml,
        dose_ph_up_ml,
        dose_ph_down_ml,
        water_in_sec,
        water_out_sec,
        post_mixing_ec: 0.0,
        post_mixing_ph: 0.0,
        start_ms: ctx.phase_start_ms.unwrap_or(uptime), // SỬA: Dùng uptime
        active_mixing_finish_ms: uptime + (ctx.diagnostic.adaptive_mixing_sec as u64 * 1000), // SỬA: Dùng uptime
        stabilizing_start_ms: None,
        stabilizing_finish_ms: None,
        invalid_by_noise: false,
        invalid_by_water_change: false,
    }
}

/// Cập nhật kết quả chuyển phase sang ActiveMixing
fn transition_to_active_mixing(
    uptime: u64, // SỬA: Đổi tham số now_ms thành uptime
    sample: PendingCalibrationSample,
    ctx: &SystemContext,
    result: &mut TickResult,
) {
    result
        .events
        .push(OrchestratorEvent::SetMixValve { on: true });

    let mut peri_delta = result.delta.peripherals.take().unwrap_or_default();
    peri_delta.mix_valve = Some(true);
    peri_delta.mix_valve_started_by_dosing = Some(true);
    result.delta.peripherals = Some(peri_delta);

    result.delta.calibration = Some(CalibrationDelta::Start(sample));
    result.delta.phase = Some(SystemPhase::ActiveMixing);
    result.delta.phase_start_ms = Some(Some(uptime)); // SỬA: Dùng uptime
    result.delta.phase_finish_ms = Some(Some(
        uptime + ctx.diagnostic.adaptive_mixing_sec as u64 * 1000, // SỬA: Dùng uptime
    ));
    result.delta.reset_stabilizer = true;
}

#[cfg(test)]
mod tests {
    use crate::core::fsm::context::SystemContext;

    fn make_ctx_no_finish_ms() -> SystemContext {
        let mut ctx = SystemContext::default();
        ctx.phase_start_ms = Some(0);
        ctx.phase_finish_ms = None;
        ctx
    }

    #[test]
    fn hard_timeout_does_not_trigger_when_finish_ms_is_none() {
        let ctx = make_ctx_no_finish_ms();
        let uptime: u64 = 1_000;
        let timed_out = uptime
            >= ctx
                .phase_finish_ms
                .unwrap_or(u64::MAX)
                .saturating_add(5_000);
        assert!(!timed_out, "Không nên timeout khi phase_finish_ms là None");
    }

    #[test]
    fn hard_timeout_triggers_5s_after_finish_ms() {
        let mut ctx = SystemContext::default();
        ctx.phase_finish_ms = Some(10_000);
        let not_yet = 14_999u64
            >= ctx
                .phase_finish_ms
                .unwrap_or(u64::MAX)
                .saturating_add(5_000);
        assert!(!not_yet);
        let at_limit = 15_000u64
            >= ctx
                .phase_finish_ms
                .unwrap_or(u64::MAX)
                .saturating_add(5_000);
        assert!(at_limit);
    }
}
