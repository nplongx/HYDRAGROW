// src/fsm/orchestrator.rs
//! Orchestrator — Pure router, không chứa logic phase.
//! Nhận input → chọn PhaseTick impl → delegate → trả TickResult.

use hydragrow_shared::fsm::{FaultCode, SystemPhase};
use hydragrow_shared::{ControllerConfig, SensorData};

use crate::fsm::events::OrchestratorEvent;
use crate::fsm::phase_impls::{
    ActiveMixingPhase, CooldownPhase, MimoDosingPhase, MonitoringPhase, StabilizingPhase,
    WaterDrainingPhase, WaterRefillingPhase,
};
use crate::fsm::phase_tick::PhaseTick;
use crate::fsm::system_context::SystemContext;
use crate::fsm::tick_result::{PeripheralDelta, TickResult};
use crate::pump::WaterDirection;

pub fn tick(
    now_ms: u64,
    config: &ControllerConfig,
    sensors: &SensorData,
    sensor_last_update_ms: u64,
    ctx: &mut SystemContext,
) -> TickResult {
    let mut result = TickResult::default();

    // Kiểm tra Sensor timeout
    if now_ms.saturating_sub(sensor_last_update_ms) > 90_000 {
        if !matches!(ctx.phase, SystemPhase::Fault(_)) {
            log::error!("🚨 [SENSOR TIMEOUT] Quá 90s không nhận được gói tin cảm biến mới.");
            result.delta.phase = Some(SystemPhase::Fault(FaultCode::SensorTimeout));
            result.events.push(OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop,
            });
            result
                .events
                .push(OrchestratorEvent::SetMistValve { on: false });
            result
                .events
                .push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
            let mut peri_delta = PeripheralDelta::default();
            peri_delta.osaka_pump = Some(false);
            peri_delta.osaka_pwm = Some(0);
            peri_delta.is_misting_active = Some(false);
            peri_delta.is_scheduled_mixing_active = Some(false);
            result.delta.peripherals = Some(peri_delta);
        }
        return result;
    }

    // 2. Sensor noise check
    if check_sensor_noise(ctx, sensors, config, &mut result.delta) {
        return result;
    }

    // 3. Stabilizer tracker push (chỉ trong các phase cần theo dõi)
    if matches!(
        ctx.phase,
        SystemPhase::ActiveMixing | SystemPhase::Stabilizing
    ) {
        ctx.stabilizer_tracker.push(sensors.ec, sensors.ph);
    }

    // 4. Delegate sang Phase struct tương ứng
    let phase_result = match &ctx.phase {
        SystemPhase::Monitoring => MonitoringPhase.tick(now_ms, config, sensors, ctx),
        SystemPhase::MimoDosing => MimoDosingPhase.tick(now_ms, config, sensors, ctx),
        SystemPhase::ActiveMixing => ActiveMixingPhase.tick(now_ms, config, sensors, ctx),
        SystemPhase::Stabilizing => StabilizingPhase.tick(now_ms, config, sensors, ctx),
        SystemPhase::Cooldown => CooldownPhase.tick(now_ms, config, sensors, ctx),
        SystemPhase::WaterRefilling => WaterRefillingPhase.tick(now_ms, config, sensors, ctx),
        SystemPhase::WaterDraining => WaterDrainingPhase.tick(now_ms, config, sensors, ctx),
        // Các phase chưa có impl → idle
        _ => TickResult::default(),
    };

    // 5. Peripheral tick (osaka, misting) — chỉ khi không phải Fault/Emergency
    if !matches!(
        ctx.phase,
        SystemPhase::Fault(_) | SystemPhase::EmergencyStop(_)
    ) {
        let is_dosing_active = matches!(
            ctx.phase,
            SystemPhase::MimoDosing | SystemPhase::ActiveMixing | SystemPhase::Stabilizing
        );
        result = merge_tick_results(result, phase_result);
        result = tick_peripheral_systems(result, ctx, sensors, now_ms, config, is_dosing_active);
    } else {
        result = merge_tick_results(result, phase_result);
    }

    result
}

/// Merge 2 TickResult — events nối đuôi, delta sau override delta trước nếu Some.
fn merge_tick_results(mut base: TickResult, addition: TickResult) -> TickResult {
    base.events.extend(addition.events);
    // Phase delta từ addition override base nếu có
    if addition.delta.phase.is_some() {
        base.delta.phase = addition.delta.phase;
    }
    if addition.delta.phase_start_ms.is_some() {
        base.delta.phase_start_ms = addition.delta.phase_start_ms;
    }
    if addition.delta.phase_finish_ms.is_some() {
        base.delta.phase_finish_ms = addition.delta.phase_finish_ms;
    }
    if addition.delta.peripherals.is_some() {
        base.delta.peripherals = addition.delta.peripherals;
    }
    if addition.delta.calibration.is_some() {
        base.delta.calibration = addition.delta.calibration;
    }
    if addition.delta.dosing_cycle_count_increment {
        base.delta.dosing_cycle_count_increment = true;
    }
    if addition.delta.reset_stabilizer {
        base.delta.reset_stabilizer = true;
    }
    if addition.delta.last_water_change_sec.is_some() {
        base.delta.last_water_change_sec = addition.delta.last_water_change_sec;
    }
    if addition.delta.next_water_change_trigger_sec.is_some() {
        base.delta.next_water_change_trigger_sec = addition.delta.next_water_change_trigger_sec;
    }
    if addition.delta.water_change_cron.is_some() {
        base.delta.water_change_cron = addition.delta.water_change_cron;
    }
    if addition.delta.reset_safety_budget {
        base.delta.reset_safety_budget = true;
    }
    if addition.delta.safety_override_until.is_some() {
        base.delta.safety_override_until = addition.delta.safety_override_until;
    }
    if addition.delta.manual_pump_timeout.is_some() {
        base.delta.manual_pump_timeout = addition.delta.manual_pump_timeout;
    }
    if addition.delta.manual_pump_timeout_clear.is_some() {
        base.delta.manual_pump_timeout_clear = addition.delta.manual_pump_timeout_clear;
    }
    base
}

fn tick_peripheral_systems(
    mut result: TickResult,
    ctx: &SystemContext,
    sensors: &SensorData,
    now_ms: u64,
    config: &ControllerConfig,
    is_dosing_active: bool,
) -> TickResult {
    use crate::fsm::peripheral::PeripheralController;

    let (osaka_delta, osaka_events) =
        PeripheralController::tick_osaka(&ctx.peripherals, is_dosing_active, config);
    result.events.extend(osaka_events);

    // Misting chỉ tick khi không đang trong dosing cycle
    if !ctx.peripherals.misting_started_by_dosing {
        let (mist_delta, mist_events) =
            PeripheralController::tick_misting(&ctx.peripherals, sensors, now_ms, config);
        result.events.extend(mist_events);
        // Merge mist_delta vào osaka_delta
        let combined_peri = merge_peripheral_deltas(osaka_delta, mist_delta);
        if let Some(existing) = result.delta.peripherals.take() {
            result.delta.peripherals = Some(merge_peripheral_deltas(existing, combined_peri));
        } else {
            result.delta.peripherals = Some(combined_peri);
        }
    } else {
        if let Some(existing) = result.delta.peripherals.take() {
            result.delta.peripherals = Some(merge_peripheral_deltas(existing, osaka_delta));
        } else {
            result.delta.peripherals = Some(osaka_delta);
        }
    }

    let mixing_delta =
        PeripheralController::tick_scheduled_mixing(&ctx.peripherals, now_ms / 1000, config);
    if let Some(existing) = result.delta.peripherals.take() {
        result.delta.peripherals = Some(merge_peripheral_deltas(existing, mixing_delta));
    } else {
        result.delta.peripherals = Some(mixing_delta);
    }

    result
}

fn merge_peripheral_deltas(
    mut base: PeripheralDelta,
    addition: PeripheralDelta,
) -> PeripheralDelta {
    if addition.osaka_pump.is_some() {
        base.osaka_pump = addition.osaka_pump;
    }
    if addition.osaka_pwm.is_some() {
        base.osaka_pwm = addition.osaka_pwm;
    }
    if addition.is_misting_active.is_some() {
        base.is_misting_active = addition.is_misting_active;
    }
    if addition.last_mist_toggle_time.is_some() {
        base.last_mist_toggle_time = addition.last_mist_toggle_time;
    }
    if addition.mist_valve.is_some() {
        base.mist_valve = addition.mist_valve;
    }
    if addition.is_scheduled_mixing_active.is_some() {
        base.is_scheduled_mixing_active = addition.is_scheduled_mixing_active;
    }
    if addition.last_mixing_start_sec.is_some() {
        base.last_mixing_start_sec = addition.last_mixing_start_sec;
    }
    if addition.water_pump_in.is_some() {
        base.water_pump_in = addition.water_pump_in;
    }
    if addition.water_pump_out.is_some() {
        base.water_pump_out = addition.water_pump_out;
    }
    if addition.pump_a.is_some() {
        base.pump_a = addition.pump_a;
    }
    if addition.pump_b.is_some() {
        base.pump_b = addition.pump_b;
    }
    if addition.ph_up.is_some() {
        base.ph_up = addition.ph_up;
    }
    if addition.ph_down.is_some() {
        base.ph_down = addition.ph_down;
    }
    if addition.last_ec_before_dose.is_some() {
        base.last_ec_before_dose = addition.last_ec_before_dose;
    }
    if addition.last_ph_before_dose.is_some() {
        base.last_ph_before_dose = addition.last_ph_before_dose;
    }
    if addition.misting_started_by_dosing.is_some() {
        base.misting_started_by_dosing = addition.misting_started_by_dosing;
    }
    base
}

/// Kiểm tra nhiễu pH và EC có vược qua dung sai thiết lập
/// Ghi đè ContextDelta
/// Cập nhật lại previous_ph/ec nếu xảy ra nhiễu
fn check_sensor_noise(
    ctx: &SystemContext,
    sensors: &SensorData,
    config: &ControllerConfig,
    delta: &mut crate::fsm::tick_result::ContextDelta,
) -> bool {
    let mut is_noisy = false;
    let mut peri_delta = PeripheralDelta::default();

    if config.enable_ec_sensor && !sensors.err_ec.unwrap_or(false) {
        if let Some(prev_ec) = ctx.peripherals.previous_ec {
            if (sensors.ec - prev_ec).abs() > config.max_ec_delta {
                is_noisy = true;
            }
        }
        peri_delta.previous_ec = Some(Some(sensors.ec));
    }
    if config.enable_ph_sensor && !sensors.err_ph.unwrap_or(false) {
        if let Some(prev_ph) = ctx.peripherals.previous_ph {
            if (sensors.ph - prev_ph).abs() > config.max_ph_delta {
                is_noisy = true;
            }
        }
        peri_delta.previous_ph = Some(Some(sensors.ph));
    }

    delta.peripherals = Some(peri_delta);
    is_noisy
}
