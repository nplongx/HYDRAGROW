// src/core/fsm/orchestrator.rs
//! Orchestrator — Pure Router điều phối FSM Tick sang các Phase độc lập.

use hydragrow_shared::fsm::{FaultCode, SystemPhase};
use hydragrow_shared::{ControlMode, ControllerConfig, SensorData};
use tracing::error;

use crate::WaterDirection;
use crate::core::fsm::ContextDelta;
use crate::core::fsm::context::SystemContext;
use crate::core::fsm::events::OrchestratorEvent;
use crate::core::fsm::peripheral::PeripheralController;
use crate::core::fsm::phase_tick::PhaseTick;
use crate::core::fsm::phases::{
    ActiveMixingPhase, CooldownPhase, MimoDosingPhase, MonitoringPhase, StabilizingPhase,
    WaterDrainingPhase, WaterRefillingPhase,
};
use crate::core::fsm::tick_result::{PeripheralDelta, TickResult};

pub fn tick(
    now_ms: u64,
    uptime_ms: u64, // SỬA: Nhận thêm uptime_ms
    config: &ControllerConfig,
    sensors: &SensorData,
    sensor_last_update_ms: u64, // (Biến này ở fsm_loop đã được nạp bằng uptime_ms)
    ctx: &mut SystemContext,
) -> TickResult {
    let mut result = TickResult::default();

    // Điều này đảm bảo rằng mỗi khi user lưu cấu hình mới, AI sẽ phản hồi ngay lập tức
    ctx.tuner.sync_with_config(config);

    // [NPL-9] Safety watchdog — chạy bất kể mode (Auto/Manual/Fault)
    let osaka_running = ctx.peripherals.pump_status.osaka_pump;
    let mist_valve_open = ctx.peripherals.pump_status.mist_valve;
    let mix_valve_open = ctx.peripherals.pump_status.mix_valve;
    if osaka_running && !mist_valve_open && !mix_valve_open {
        result
            .events
            .push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
        let mut peri_delta = result.delta.peripherals.take().unwrap_or_default();
        peri_delta.osaka_pump = Some(false);
        peri_delta.osaka_pwm = Some(0);
        result.delta.peripherals = Some(peri_delta);

        result.delta.phase = Some(SystemPhase::Fault(FaultCode::OsakaRunningWithoutValve));
        return result;
    }

    // 1. Kiểm tra Sensor Timeout
    // SỬA: Dùng uptime_ms để kiểm tra timeout, an toàn tuyệt đối trước Time Jump
    if uptime_ms.saturating_sub(sensor_last_update_ms) > 90_000 {
        if !matches!(ctx.phase, SystemPhase::Fault(_)) {
            error!("🚨 [SENSOR TIMEOUT] Quá 90s không nhận được gói tin cảm biến mới.");
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
    } else if matches!(ctx.phase, SystemPhase::Fault(FaultCode::SensorTimeout)) {
        tracing::info!(
            "✅ [SENSOR RECOVERED] Đã nhận dữ liệu cảm biến mới. Tự động thoát Fault chuyển về Monitoring."
        );
        result.delta.phase = Some(SystemPhase::Monitoring);
        result.delta.phase_start_ms = Some(None);
        result.delta.phase_finish_ms = Some(None);
        result.delta.reset_stabilizer = true;
    }

    // 2. Sensor noise check
    if check_sensor_noise(ctx, sensors, config, &mut result.delta) {
        return result;
    }

    if config.control_mode != ControlMode::Auto || !config.is_enabled {
        return stop_automation_if_needed(result, ctx);
    }

    if matches!(
        ctx.phase,
        SystemPhase::ActiveMixing | SystemPhase::Stabilizing
    ) {
        ctx.stabilizer_tracker.push(sensors.ec, sensors.ph);
    }

    // 4. Delegate sang Phase Handler tương ứng (Truyền cả 2 tham số thời gian)
    let phase_result = match &ctx.phase {
        SystemPhase::Monitoring => MonitoringPhase.tick(now_ms, uptime_ms, config, sensors, ctx),
        SystemPhase::MimoDosing => MimoDosingPhase.tick(now_ms, uptime_ms, config, sensors, ctx),
        SystemPhase::ActiveMixing => {
            ActiveMixingPhase.tick(now_ms, uptime_ms, config, sensors, ctx)
        }
        SystemPhase::Stabilizing => StabilizingPhase.tick(now_ms, uptime_ms, config, sensors, ctx),
        SystemPhase::Cooldown => CooldownPhase.tick(now_ms, uptime_ms, config, sensors, ctx),
        SystemPhase::WaterRefilling => {
            WaterRefillingPhase.tick(now_ms, uptime_ms, config, sensors, ctx)
        }
        SystemPhase::WaterDraining => {
            WaterDrainingPhase.tick(now_ms, uptime_ms, config, sensors, ctx)
        }
        _ => TickResult::default(),
    };

    // 5. Tick Peripheral Controllers
    if !matches!(
        ctx.phase,
        SystemPhase::Fault(_) | SystemPhase::EmergencyStop(_)
    ) {
        let is_dosing_active = matches!(
            ctx.phase,
            SystemPhase::MimoDosing | SystemPhase::ActiveMixing | SystemPhase::Stabilizing
        );
        result = merge_tick_results(result, phase_result);

        // SỬA: Truyền thêm uptime_ms vào
        result = tick_peripheral_systems(
            result,
            ctx,
            sensors,
            now_ms,
            uptime_ms,
            config,
            is_dosing_active,
        );
    } else {
        result = merge_tick_results(result, phase_result);
    }

    result
}

fn merge_tick_results(mut base: TickResult, addition: TickResult) -> TickResult {
    base.events.extend(addition.events);

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
    if addition.delta.current_stage_index.is_some() {
        base.delta.current_stage_index = addition.delta.current_stage_index;
    }
    if addition.delta.recipe_completed.is_some() {
        base.delta.recipe_completed = addition.delta.recipe_completed;
    }
    if addition.delta.last_recipe_check_sec.is_some() {
        base.delta.last_recipe_check_sec = addition.delta.last_recipe_check_sec;
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
    _now_ms: u64,
    uptime_ms: u64, // SỬA: Nhận uptime_ms
    config: &ControllerConfig,
    _is_dosing_active: bool,
) -> TickResult {
    // 1. Rút cái `peripherals` delta hiện tại ra (nếu các pha trước đã có thay đổi thì giữ lại, không thì tạo mới)
    let mut current_peri_delta = result.delta.peripherals.take().unwrap_or_default();

    // 2. Tính toán Van Phun Sương (Mist)
    let mut mist_delta = PeripheralDelta::default();
    if !ctx.peripherals.misting_started_by_dosing {
        let (delta, mist_events) =
            PeripheralController::tick_misting(&ctx.peripherals, sensors, uptime_ms, config);
        mist_delta = delta;
        result.events.extend(mist_events);
    }

    // 3. Tính toán Van Trộn (Mix)
    let mut mix_delta = PeripheralDelta::default();
    if !ctx.peripherals.mix_valve_started_by_dosing {
        let (delta, mix_events) =
            PeripheralController::tick_scheduled_mixing(&ctx.peripherals, uptime_ms / 1000, config);
        mix_delta = delta;
        result.events.extend(mix_events);
    }

    // ---> Gộp state của 2 van vào current_peri_delta
    current_peri_delta = merge_peripheral_deltas(current_peri_delta, mist_delta);
    current_peri_delta = merge_peripheral_deltas(current_peri_delta, mix_delta);

    let mist_valve_is_open = if current_peri_delta.mist_valve.is_some() {
        current_peri_delta.mist_valve
    } else {
        None
    };

    let mix_valve_is_open = if current_peri_delta.mix_valve.is_some() {
        current_peri_delta.mix_valve
    } else {
        None
    };

    // 4. Tính toán Bơm Osaka (Thực thi SAU CÙNG như đã bàn)
    let (osaka_delta, osaka_events) = PeripheralController::tick_osaka(
        &ctx.peripherals,
        &mist_valve_is_open,
        &mix_valve_is_open,
        config,
    ); // dù đã đặt tick_osaka
    result.events.extend(osaka_events);

    // ---> Gộp nốt state của Osaka vào current_peri_delta
    current_peri_delta = merge_peripheral_deltas(current_peri_delta, osaka_delta);

    // 5. Đóng gói lại và trả về
    result.delta.peripherals = Some(current_peri_delta);

    result
}

fn stop_automation_if_needed(mut result: TickResult, ctx: &SystemContext) -> TickResult {
    if matches!(
        ctx.phase,
        SystemPhase::ManualMode | SystemPhase::Fault(_) | SystemPhase::EmergencyStop(_)
    ) {
        return result;
    }

    let automation_was_active = matches!(
        ctx.phase,
        SystemPhase::MimoDosing
            | SystemPhase::ActiveMixing
            | SystemPhase::Stabilizing
            | SystemPhase::Cooldown
            | SystemPhase::WaterRefilling
            | SystemPhase::WaterDraining
    );

    if automation_was_active {
        result.events.push(OrchestratorEvent::SetWaterPump {
            direction: WaterDirection::Stop,
        });
        result
            .events
            .push(OrchestratorEvent::SetMistValve { on: false });
        result
            .events
            .push(OrchestratorEvent::SetMixValve { on: false });
        result
            .events
            .push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
        result.events.push(OrchestratorEvent::SetDosingPump {
            pump: crate::core::fsm::events::DosingPumpTarget::NutrientA,
            on: false,
            pwm_percent: 0,
        });
        result.events.push(OrchestratorEvent::SetDosingPump {
            pump: crate::core::fsm::events::DosingPumpTarget::NutrientB,
            on: false,
            pwm_percent: 0,
        });
        result.events.push(OrchestratorEvent::SetDosingPump {
            pump: crate::core::fsm::events::DosingPumpTarget::PhUp,
            on: false,
            pwm_percent: 0,
        });
        result.events.push(OrchestratorEvent::SetDosingPump {
            pump: crate::core::fsm::events::DosingPumpTarget::PhDown,
            on: false,
            pwm_percent: 0,
        });

        let mut peri_delta = result.delta.peripherals.take().unwrap_or_default();
        peri_delta.water_pump_in = Some(false);
        peri_delta.water_pump_out = Some(false);
        peri_delta.mist_valve = Some(false);
        peri_delta.mix_valve = Some(false);
        peri_delta.is_misting_active = Some(false);
        peri_delta.is_scheduled_mixing_active = Some(false);
        peri_delta.misting_started_by_dosing = Some(false);
        peri_delta.mix_valve_started_by_dosing = Some(false);
        peri_delta.osaka_pump = Some(false);
        peri_delta.osaka_pwm = Some(0);
        peri_delta.pump_a = Some(false);
        peri_delta.pump_b = Some(false);
        peri_delta.ph_up = Some(false);
        peri_delta.ph_down = Some(false);
        result.delta.peripherals = Some(peri_delta);
    }

    result.delta.phase = Some(SystemPhase::ManualMode);
    result.delta.phase_start_ms = Some(None);
    result.delta.phase_finish_ms = Some(None);
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
    if addition.mix_valve_started_by_dosing.is_some() {
        base.mix_valve_started_by_dosing = addition.mix_valve_started_by_dosing;
    }
    base
}

fn check_sensor_noise(
    ctx: &SystemContext,
    sensors: &SensorData,
    config: &ControllerConfig,
    delta: &mut ContextDelta,
) -> bool {
    let mut is_noisy = false;
    let mut peri_delta = PeripheralDelta::default();

    if config.enable_ec_sensor && !sensors.err_ec.unwrap_or(false) {
        if let Some(prev_ec) = ctx.peripherals.previous_ec
            && (sensors.ec - prev_ec).abs() > config.max_ec_delta
        {
            is_noisy = true;
        }
        peri_delta.previous_ec = Some(Some(sensors.ec));
    }
    if config.enable_ph_sensor && !sensors.err_ph.unwrap_or(false) {
        if let Some(prev_ph) = ctx.peripherals.previous_ph
            && (sensors.ph - prev_ph).abs() > config.max_ph_delta
        {
            is_noisy = true;
        }
        peri_delta.previous_ph = Some(Some(sensors.ph));
    }

    delta.peripherals = Some(peri_delta);
    is_noisy
}
