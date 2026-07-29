// src/runtime/command_handler.rs
//! CommandHandler — Chuyển đổi lệnh MQTT thành ContextDelta và OrchestratorEvent.

use std::sync::mpsc::{Receiver, Sender};
use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::log::{LogCategory, LogLevel, UnifiedSystemLog};
use hydragrow_shared::{ControlMode, ControllerConfig, MqttCommandIn};
use log::{info, warn};

use crate::core::fsm::context::SystemContext;
use crate::core::fsm::events::{DosingPumpTarget, OrchestratorEvent};
use crate::core::fsm::tick_result::ContextDelta;
use crate::hw::pump_controller::WaterDirection;

pub fn process_mqtt_commands(
    cmd_rx: &Receiver<MqttCommandIn>,
    config: &ControllerConfig,
    ctx: &SystemContext,
    current_time_ms: u64,
    _fsm_mqtt_tx: &Sender<String>,
) -> (ContextDelta, Vec<OrchestratorEvent>) {
    let mut delta = ContextDelta::default();
    let mut all_events = Vec::new();
    let is_emergency_state = matches!(
        ctx.phase,
        SystemPhase::EmergencyStop(_) | SystemPhase::Fault(_) | SystemPhase::SensorCalibration
    );

    while let Ok(cmd) = cmd_rx.try_recv() {
        let action_lower = cmd.action.to_lowercase();

        // --- 1. Lệnh hiệu chuẩn cảm biến ---
        if action_lower == "enter_calibration" {
            info!("🛠️ Bắt đầu chu kỳ hiệu chuẩn cảm biến!");
            stop_all_hardware(&mut all_events);
            delta.phase = Some(SystemPhase::SensorCalibration);
            delta.phase_finish_ms = Some(Some(current_time_ms + 3_600_000));

            let mut peri_delta = delta.peripherals.take().unwrap_or_default();
            peri_delta.osaka_pump = Some(false);
            peri_delta.osaka_pwm = Some(0);
            peri_delta.is_misting_active = Some(false);
            peri_delta.mist_valve = Some(false);
            delta.peripherals = Some(peri_delta);

            all_events.push(OrchestratorEvent::SaveNvsSnapshot);
            continue;
        }

        if action_lower == "exit_calibration" {
            if matches!(ctx.phase, SystemPhase::SensorCalibration) {
                info!("✅ Thoát chế độ hiệu chuẩn, quay về Monitoring.");
                delta.phase = Some(SystemPhase::Monitoring);
                delta.phase_finish_ms = Some(None);
            }
            continue;
        }

        // --- 2. Lệnh đồng bộ trạng thái ---
        if action_lower == "sync_status" {
            all_events.push(OrchestratorEvent::PublishFsmState);
            continue;
        }

        // --- 3. Lệnh Reset Fault ---
        if action_lower == "reset_fault" {
            info!("🔄 Nhận lệnh Reset. Khôi phục hệ thống...");
            stop_all_hardware(&mut all_events);

            delta.phase = Some(SystemPhase::Monitoring);
            delta.phase_start_ms = Some(None);
            delta.phase_finish_ms = Some(None);
            delta.reset_stabilizer = true;
            delta.reset_safety_budget = true;

            let mut peri_delta = delta.peripherals.take().unwrap_or_default();
            peri_delta.osaka_pump = Some(false);
            peri_delta.osaka_pwm = Some(0);
            peri_delta.is_misting_active = Some(false);
            peri_delta.mist_valve = Some(false);
            peri_delta.water_pump_in = Some(false);
            peri_delta.water_pump_out = Some(false);
            delta.peripherals = Some(peri_delta);

            all_events.push(OrchestratorEvent::SaveNvsSnapshot);
            continue;
        }

        // Nếu đang ở chế độ AUTO thì bỏ qua lệnh điều khiển tay đơn lẻ
        if config.control_mode == ControlMode::Auto {
            warn!("⚠️ Bỏ qua lệnh thủ công vì hệ thống đang ở chế độ AUTO.");
            continue;
        }

        match cmd.target.as_deref() {
            Some(target_lower) if target_lower == "all" => {}
            _ => continue,
        }

        let pump_name = cmd
            .params
            .as_ref()
            .and_then(|p| p.pump_id.as_ref())
            .cloned()
            .or_else(|| cmd.pump.clone())
            .map(|p| p.to_uppercase())
            .unwrap_or_else(|| "ALL".to_string());

        let is_force_on = action_lower == "force_on";
        let pwm = cmd.params.as_ref().and_then(|p| p.pwm).or(cmd.pwm);
        let duration_sec = cmd
            .params
            .as_ref()
            .and_then(|p| p.duration_sec)
            .or(cmd.duration_sec);

        let mut is_on = is_force_on
            || matches!(action_lower.as_str(), "pump_on" | "on" | "true" | "1")
            || (action_lower == "set_pwm" && pwm.unwrap_or(0) > 0);

        if let Some(state) = cmd.params.as_ref().and_then(|p| p.state) {
            is_on = state;
        }

        if is_emergency_state && is_on && !is_force_on {
            warn!("⛔ BLOCKED: Không thể điều khiển {} trong trạng thái khẩn cấp.", pump_name);
            continue;
        }

        if is_force_on {
            let duration = duration_sec.unwrap_or(120);
            delta.safety_override_until = Some(current_time_ms + (duration as u64 * 1000));
            let log_payload = UnifiedSystemLog::build_basic_log_json_with_ts(
                &config.device_id,
                LogLevel::Warning,
                LogCategory::UserAction,
                "Can thiệp cưỡng chế",
                format!("Kích hoạt FORCE ON {} trong {}s.", pump_name, duration),
                None,
                "fsm_command",
                current_time_ms,
            );
            all_events.push(OrchestratorEvent::PublishSystemLog { payload_json: log_payload });
        }

        if is_on {
            if let Some(duration) = duration_sec {
                if duration > 0 {
                    delta.manual_pump_timeout = Some((
                        pump_name.clone(),
                        current_time_ms + (duration as u64 * 1000),
                    ));
                }
            }
        } else {
            delta.manual_pump_timeout_clear = Some(pump_name.clone());
        }

        let pwm_val = pwm.unwrap_or(if is_on { 100 } else { 0 });
        let mut pump_events = build_pump_events(&pump_name, is_on, pwm_val, &mut delta);
        all_events.append(&mut pump_events);
    }

    (delta, all_events)
}

fn stop_all_hardware(events: &mut Vec<OrchestratorEvent>) {
    events.push(OrchestratorEvent::SetWaterPump { direction: WaterDirection::Stop });
    events.push(OrchestratorEvent::SetMistValve { on: false });
    events.push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
    events.push(OrchestratorEvent::SetDosingPump { pump: DosingPumpTarget::NutrientA, on: false, pwm_percent: 0 });
    events.push(OrchestratorEvent::SetDosingPump { pump: DosingPumpTarget::NutrientB, on: false, pwm_percent: 0 });
    events.push(OrchestratorEvent::SetDosingPump { pump: DosingPumpTarget::PhUp, on: false, pwm_percent: 0 });
    events.push(OrchestratorEvent::SetDosingPump { pump: DosingPumpTarget::PhDown, on: false, pwm_percent: 0 });
}

pub fn build_pump_events(
    pump_name: &str,
    is_on: bool,
    pwm_val: u32,
    delta: &mut ContextDelta,
) -> Vec<OrchestratorEvent> {
    let mut events = Vec::new();
    let mut peri_delta = delta.peripherals.take().unwrap_or_default();

    match pump_name {
        "A" | "PUMP_A" => {
            peri_delta.pump_a = Some(is_on);
            events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: is_on,
                pwm_percent: if is_on { pwm_val } else { 0 },
            });
        }
        "B" | "PUMP_B" => {
            peri_delta.pump_b = Some(is_on);
            events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientB,
                on: is_on,
                pwm_percent: if is_on { pwm_val } else { 0 },
            });
        }
        "PH_UP" | "PUMP_PH_UP" => {
            peri_delta.ph_up = Some(is_on);
            events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhUp,
                on: is_on,
                pwm_percent: if is_on { pwm_val } else { 0 },
            });
        }
        "PH_DOWN" | "PUMP_PH_DOWN" => {
            peri_delta.ph_down = Some(is_on);
            events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhDown,
                on: is_on,
                pwm_percent: if is_on { pwm_val } else { 0 },
            });
        }
        "OSAKA_PUMP" | "OSAKA" => {
            peri_delta.osaka_pump = Some(is_on);
            peri_delta.osaka_pwm = Some(if is_on { pwm_val } else { 0 });
            if is_on {
                events.push(OrchestratorEvent::StartOsakaSoft { target_pwm_percent: pwm_val });
            } else {
                events.push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
            }
        }
        "MIST_VALVE" | "MIST" => {
            peri_delta.mist_valve = Some(is_on);
            peri_delta.is_misting_active = Some(is_on);
            events.push(OrchestratorEvent::SetMistValve { on: is_on });
        }
        "WATER_PUMP" | "WATER_PUMP_IN" | "PUMP_IN" => {
            peri_delta.water_pump_in = Some(is_on);
            if is_on { peri_delta.water_pump_out = Some(false); }
            events.push(OrchestratorEvent::SetWaterPump {
                direction: if is_on { WaterDirection::In } else { WaterDirection::Stop },
            });
        }
        "DRAIN_PUMP" | "WATER_PUMP_OUT" | "PUMP_OUT" => {
            peri_delta.water_pump_out = Some(is_on);
            if is_on { peri_delta.water_pump_in = Some(false); }
            events.push(OrchestratorEvent::SetWaterPump {
                direction: if is_on { WaterDirection::Out } else { WaterDirection::Stop },
            });
        }
        _ => {}
    }

    delta.peripherals = Some(peri_delta);
    events
}

pub fn build_stop_pump_events(pump_name: &str, delta: &mut ContextDelta) -> Vec<OrchestratorEvent> {
    build_pump_events(pump_name, false, 0, delta)
}