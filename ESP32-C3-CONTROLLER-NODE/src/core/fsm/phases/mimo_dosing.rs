// src/fsm/phase_impls/mimo_dosing.rs
use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::{ControllerConfig, SensorData};
use log::warn;

use crate::core::actors::dosing_actor::{DosingEvent, PumpTarget};
use crate::core::fsm::tick_result::CalibrationDelta;
use crate::core::fsm::types::PendingCalibrationSample;
use crate::core::fsm::{
    DosingPumpTarget, OrchestratorEvent, PeripheralDelta, PhaseTick, SystemContext, TickResult,
};
use crate::hw::WaterDirection;

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

        // 1. Kiểm tra Safety Timeout của bơm nước (elapsed_ms giờ đã an toàn tuyệt đối)
        self.check_water_pump_timeouts(elapsed_ms, config, ctx, &mut result, &mut peri_delta);

        // 2. Hard Timeout toàn Phase -> Chuyển Cooldown
        // SỬA: Dùng `uptime` để so sánh và thiết lập mốc thời gian tương lai
        if uptime >= ctx.phase_finish_ms.unwrap_or(u64::MAX) + 5_000 {
            warn!("⚠️ [FSM] Dosing phase timeout cứng! Chuyển về Cooldown.");
            stop_water_and_misting(ctx, &mut result, &mut peri_delta);

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

        match dosing_event {
            DosingEvent::Pending => {
                if ctx.dosing.is_idle() && elapsed_ms >= 500 {
                    // Chu kỳ chỉ bơm nước hoàn tất
                    let water_in_spent = if ctx.peripherals.pump_status.water_pump_in {
                        elapsed_ms.min(config.max_refill_duration_sec as u64 * 1000) as f32 / 1000.0
                    } else {
                        0.0
                    };
                    let water_out_spent = if ctx.peripherals.pump_status.water_pump_out {
                        elapsed_ms.min(config.max_drain_duration_sec as u64 * 1000) as f32 / 1000.0
                    } else {
                        0.0
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
        elapsed_ms: u64,
        config: &ControllerConfig,
        ctx: &SystemContext,
        result: &mut TickResult,
        peri_delta: &mut PeripheralDelta,
    ) {
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
    }
}

// --- Helper Functions Thuần Tuý Cho Module ---

/// Tắt bơm nước và van phun sương (nếu được bật bởi dosing)
fn stop_water_and_misting(
    ctx: &SystemContext,
    result: &mut TickResult,
    peri_delta: &mut PeripheralDelta,
) {
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
    result.delta.calibration = Some(CalibrationDelta::Start(sample));
    result.delta.phase = Some(SystemPhase::ActiveMixing);
    result.delta.phase_start_ms = Some(Some(uptime)); // SỬA: Dùng uptime
    result.delta.phase_finish_ms = Some(Some(
        uptime + ctx.diagnostic.adaptive_mixing_sec as u64 * 1000, // SỬA: Dùng uptime
    ));
    result.delta.reset_stabilizer = true;
}
