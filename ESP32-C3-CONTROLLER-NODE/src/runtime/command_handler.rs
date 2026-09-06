// src/runtime/command_handler.rs
//! CommandHandler — Chuyển đổi lệnh MQTT thành ContextDelta và OrchestratorEvent.
#![allow(clippy::field_reassign_with_default)]

use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::log::{LogCategory, LogLevel, UnifiedSystemLog};
use hydragrow_shared::{ControlMode, ControllerConfig, MqttCommandIn};
use log::{info, warn};
use std::sync::mpsc::{Receiver, Sender};

use hydragrow_controller_core::core::fsm::context::SystemContext;
use hydragrow_controller_core::core::fsm::events::{DosingPumpTarget, OrchestratorEvent};
use hydragrow_controller_core::core::fsm::tick_result::{ContextDelta, PeripheralDelta};
use hydragrow_controller_core::WaterDirection;

fn merge_delta(acc: &mut ContextDelta, step: ContextDelta) {
    if step.phase.is_some() {
        acc.phase = step.phase;
    }
    if step.phase_start_ms.is_some() {
        acc.phase_start_ms = step.phase_start_ms;
    }
    if step.phase_finish_ms.is_some() {
        acc.phase_finish_ms = step.phase_finish_ms;
    }
    if step.dosing_cycle_count_increment {
        acc.dosing_cycle_count_increment = true;
    }
    if step.reset_stabilizer {
        acc.reset_stabilizer = true;
    }
    if step.last_water_change_sec.is_some() {
        acc.last_water_change_sec = step.last_water_change_sec;
    }
    if step.next_water_change_trigger_sec.is_some() {
        acc.next_water_change_trigger_sec = step.next_water_change_trigger_sec;
    }
    if step.water_change_cron.is_some() {
        acc.water_change_cron = step.water_change_cron;
    }
    if step.current_stage_index.is_some() {
        acc.current_stage_index = step.current_stage_index;
    }
    if step.recipe_completed.is_some() {
        acc.recipe_completed = step.recipe_completed;
    }
    if step.last_recipe_check_sec.is_some() {
        acc.last_recipe_check_sec = step.last_recipe_check_sec;
    }
    if step.reset_safety_budget {
        acc.reset_safety_budget = true;
    }
    if step.safety_override_until.is_some() {
        acc.safety_override_until = step.safety_override_until;
    }
    if step.manual_pump_timeout.is_some() {
        acc.manual_pump_timeout = step.manual_pump_timeout;
    }
    if step.manual_pump_timeout_clear.is_some() {
        acc.manual_pump_timeout_clear = step.manual_pump_timeout_clear;
    }
    if step.calibration.is_some() {
        acc.calibration = step.calibration;
    }
    if let Some(step_peri) = step.peripherals {
        let acc_peri = acc.peripherals.get_or_insert_with(Default::default);
        if step_peri.pump_a.is_some() {
            acc_peri.pump_a = step_peri.pump_a;
        }
        if step_peri.pump_b.is_some() {
            acc_peri.pump_b = step_peri.pump_b;
        }
        if step_peri.ph_up.is_some() {
            acc_peri.ph_up = step_peri.ph_up;
        }
        if step_peri.ph_down.is_some() {
            acc_peri.ph_down = step_peri.ph_down;
        }
        if step_peri.water_pump_in.is_some() {
            acc_peri.water_pump_in = step_peri.water_pump_in;
        }
        if step_peri.water_pump_out.is_some() {
            acc_peri.water_pump_out = step_peri.water_pump_out;
        }
        if step_peri.mist_valve.is_some() {
            acc_peri.mist_valve = step_peri.mist_valve;
        }
        if step_peri.mix_valve.is_some() {
            acc_peri.mix_valve = step_peri.mix_valve;
        }
        if step_peri.osaka_pump.is_some() {
            acc_peri.osaka_pump = step_peri.osaka_pump;
        }
        if step_peri.osaka_pwm.is_some() {
            acc_peri.osaka_pwm = step_peri.osaka_pwm;
        }
        if step_peri.is_misting_active.is_some() {
            acc_peri.is_misting_active = step_peri.is_misting_active;
        }
        if step_peri.is_scheduled_mixing_active.is_some() {
            acc_peri.is_scheduled_mixing_active = step_peri.is_scheduled_mixing_active;
        }
        if step_peri.misting_started_by_dosing.is_some() {
            acc_peri.misting_started_by_dosing = step_peri.misting_started_by_dosing;
        }
        if step_peri.mix_valve_started_by_dosing.is_some() {
            acc_peri.mix_valve_started_by_dosing = step_peri.mix_valve_started_by_dosing;
        }
        if step_peri.last_mist_toggle_time.is_some() {
            acc_peri.last_mist_toggle_time = step_peri.last_mist_toggle_time;
        }
        if step_peri.last_mixing_start_sec.is_some() {
            acc_peri.last_mixing_start_sec = step_peri.last_mixing_start_sec;
        }
        if step_peri.last_ec_before_dose.is_some() {
            acc_peri.last_ec_before_dose = step_peri.last_ec_before_dose;
        }
        if step_peri.last_ph_before_dose.is_some() {
            acc_peri.last_ph_before_dose = step_peri.last_ph_before_dose;
        }
        if step_peri.previous_ec.is_some() {
            acc_peri.previous_ec = step_peri.previous_ec;
        }
        if step_peri.previous_ph.is_some() {
            acc_peri.previous_ph = step_peri.previous_ph;
        }
        if step_peri.last_continuous_level.is_some() {
            acc_peri.last_continuous_level = step_peri.last_continuous_level;
        }
        if step_peri.water_pump_started_uptime_ms.is_some() {
            acc_peri.water_pump_started_uptime_ms = step_peri.water_pump_started_uptime_ms;
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandStateSnapshot {
    pub phase: SystemPhase,
    pub pump_status: hydragrow_shared::PumpStatus,
}

impl CommandStateSnapshot {
    pub fn new(ctx: &SystemContext) -> Self {
        Self {
            phase: ctx.phase.clone(),
            pump_status: ctx.peripherals.pump_status.clone(),
        }
    }

    pub fn apply_step_delta(&mut self, delta: &ContextDelta) {
        if let Some(ref p) = delta.phase {
            self.phase = p.clone();
        }
        if let Some(ref pd) = delta.peripherals {
            if let Some(v) = pd.pump_a {
                self.pump_status.pump_a = v;
            }
            if let Some(v) = pd.pump_b {
                self.pump_status.pump_b = v;
            }
            if let Some(v) = pd.ph_up {
                self.pump_status.ph_up = v;
            }
            if let Some(v) = pd.ph_down {
                self.pump_status.ph_down = v;
            }
            if let Some(v) = pd.water_pump_in {
                self.pump_status.water_pump_in = v;
            }
            if let Some(v) = pd.water_pump_out {
                self.pump_status.water_pump_out = v;
            }
            if let Some(v) = pd.mist_valve {
                self.pump_status.mist_valve = v;
            }
            if let Some(v) = pd.mix_valve {
                self.pump_status.mix_valve = v;
            }
            if let Some(v) = pd.osaka_pump {
                self.pump_status.osaka_pump = v;
            }
            if let Some(v) = pd.osaka_pwm {
                self.pump_status.osaka_pwm = Some(v);
            }
        }
    }
}

pub fn process_mqtt_commands(
    cmd_rx: &Receiver<MqttCommandIn>,
    config: &ControllerConfig,
    ctx: &SystemContext,
    now_uptime_ms: u64,
    now_wall_time_ms: u64,
    _fsm_mqtt_tx: &Sender<String>,
) -> (ContextDelta, Vec<OrchestratorEvent>) {
    let mut accumulated_delta = ContextDelta::default();
    let mut all_events = Vec::new();
    let mut temp_state = CommandStateSnapshot::new(ctx);

    while let Ok(cmd) = cmd_rx.try_recv() {
        let action_lower = cmd.action.to_lowercase();
        let mut step_delta = ContextDelta::default();
        let mut step_events = Vec::new();
        let is_emergency_state = matches!(
            temp_state.phase,
            SystemPhase::EmergencyStop(_) | SystemPhase::Fault(_) | SystemPhase::SensorCalibration
        );

        // --- 1. Lệnh hiệu chuẩn cảm biến ---
        if action_lower == "enter_calibration" {
            info!("🛠️ Bắt đầu chu kỳ hiệu chuẩn cảm biến!");
            stop_all_hardware(&mut step_events);
            step_delta.phase = Some(SystemPhase::SensorCalibration);
            step_delta.phase_finish_ms = Some(Some(now_uptime_ms + 3_600_000));
            step_delta.reset_active_actors = true;

            let mut peri_delta = PeripheralDelta::default();
            peri_delta.osaka_pump = Some(false);
            peri_delta.osaka_pwm = Some(0);
            peri_delta.is_misting_active = Some(false);
            peri_delta.mist_valve = Some(false);
            peri_delta.mix_valve = Some(false);
            peri_delta.is_scheduled_mixing_active = Some(false);
            peri_delta.pump_a = Some(false);
            peri_delta.pump_b = Some(false);
            peri_delta.ph_up = Some(false);
            peri_delta.ph_down = Some(false);
            peri_delta.water_pump_in = Some(false);
            peri_delta.water_pump_out = Some(false);
            step_delta.peripherals = Some(peri_delta);

            step_events.push(OrchestratorEvent::SaveNvsSnapshot);

            temp_state.apply_step_delta(&step_delta);
            merge_delta(&mut accumulated_delta, step_delta);
            all_events.append(&mut step_events);
            continue;
        }

        if action_lower == "exit_calibration" {
            if matches!(temp_state.phase, SystemPhase::SensorCalibration) {
                info!("✅ Thoát chế độ hiệu chuẩn, quay về Monitoring.");
                step_delta.phase = Some(SystemPhase::Monitoring);
                step_delta.phase_finish_ms = Some(None);
                step_delta.reset_active_actors = true;

                temp_state.apply_step_delta(&step_delta);
                merge_delta(&mut accumulated_delta, step_delta);
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
            stop_all_hardware(&mut step_events);

            step_delta.phase = Some(SystemPhase::Monitoring);
            step_delta.phase_start_ms = Some(None);
            step_delta.phase_finish_ms = Some(None);
            step_delta.reset_stabilizer = true;
            step_delta.reset_safety_budget = true;

            let mut peri_delta = PeripheralDelta::default();
            peri_delta.pump_a = Some(false);
            peri_delta.pump_b = Some(false);
            peri_delta.ph_up = Some(false);
            peri_delta.ph_down = Some(false);
            peri_delta.osaka_pump = Some(false);
            peri_delta.osaka_pwm = Some(0);
            peri_delta.is_misting_active = Some(false);
            peri_delta.mist_valve = Some(false);
            peri_delta.mix_valve = Some(false);
            peri_delta.water_pump_in = Some(false);
            peri_delta.water_pump_out = Some(false);
            step_delta.peripherals = Some(peri_delta);

            step_events.push(OrchestratorEvent::SaveNvsSnapshot);

            temp_state.apply_step_delta(&step_delta);
            merge_delta(&mut accumulated_delta, step_delta);
            all_events.append(&mut step_events);
            continue;
        }

        // --- 4. Lệnh cập nhật OTA ---
        if action_lower == "trigger_ota" {
            info!("⚠️ Nhận lệnh OTA Update! Dừng hệ thống để chuẩn bị flash...");
            stop_all_hardware(&mut step_events);

            // Ép hệ thống vào Phase Fault/Emergency để không bị trigger các logic khác
            step_delta.phase = Some(SystemPhase::Fault(
                hydragrow_shared::fsm::FaultCode::EmergencyStop,
            ));

            let mut peri_delta = PeripheralDelta::default();
            peri_delta.osaka_pump = Some(false);
            peri_delta.mist_valve = Some(false);
            peri_delta.mix_valve = Some(false);
            step_delta.peripherals = Some(peri_delta);

            // Yêu cầu Dispatcher gọi hàm cập nhật
            step_events.push(OrchestratorEvent::TriggerOtaUpdate);

            temp_state.apply_step_delta(&step_delta);
            merge_delta(&mut accumulated_delta, step_delta);
            all_events.append(&mut step_events);
            continue;
        }

        // HMAC/replay validation happens in mqtt_client.rs before cmd_tx.send —
        // see verify_signed_json_payload. Do not remove that check assuming it's
        // redundant; commands reaching this function are already verified.
        if action_lower == "update_wifi_list" {
            let candidates = cmd
                .params
                .as_ref()
                .and_then(|params| params.candidates.clone());
            match candidates {
                Some(candidates)
                    if candidates
                        .iter()
                        .any(|candidate| !candidate.ssid.trim().is_empty()) =>
                {
                    all_events.push(OrchestratorEvent::UpdateWifiList {
                        list: hydragrow_shared::WifiCredentialList { candidates },
                    });
                }
                _ => warn!("⚠️ [CMD] Ignoring update_wifi_list without a valid SSID."),
            }
            continue;
        }

        if action_lower == "reboot_device" {
            info!("🔄 [CMD] Nhận lệnh reboot_device. Dừng hardware...");
            stop_all_hardware(&mut step_events);
            step_events.push(OrchestratorEvent::RebootDevice);
            all_events.append(&mut step_events);
            break;
        }

        if action_lower == "factory_reset" {
            info!("⚠️ [CMD] Nhận lệnh factory_reset. Xoá NVS và reboot...");
            stop_all_hardware(&mut step_events);
            step_delta.phase = Some(hydragrow_shared::fsm::SystemPhase::Fault(
                hydragrow_shared::fsm::FaultCode::EmergencyStop,
            ));
            step_events.push(OrchestratorEvent::FactoryReset);

            temp_state.apply_step_delta(&step_delta);
            merge_delta(&mut accumulated_delta, step_delta);
            all_events.append(&mut step_events);
            break;
        }

        // Nếu đang ở chế độ AUTO thì bỏ qua lệnh điều khiển tay đơn lẻ
        if config.control_mode == ControlMode::Auto {
            warn!("⚠️ Bỏ qua lệnh thủ công vì hệ thống đang ở chế độ AUTO.");
            continue;
        }

        match cmd.target.as_deref() {
            Some("all") => {}
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

        if is_emergency_state && is_on {
            warn!(
                "⛔ BLOCKED: Không thể điều khiển {} trong trạng thái khẩn cấp (kể cả force_on).",
                pump_name
            );
            continue;
        }

        if is_force_on {
            let duration = duration_sec.unwrap_or(120);
            let pwm_val = pwm.unwrap_or(100);

            if let Some(norm_pump) =
                hydragrow_shared::dosing::normalize_dosing_pump_name(&pump_name)
            {
                let cap = hydragrow_shared::dosing::capacity_ml_per_sec_for_pump(
                    config.pump_a_capacity_ml_per_sec,
                    config.pump_b_capacity_ml_per_sec,
                    config.pump_ph_up_capacity_ml_per_sec,
                    config.pump_ph_down_capacity_ml_per_sec,
                    norm_pump,
                );
                let estimated_ml = hydragrow_shared::dosing::estimate_ml(cap, pwm_val, duration);
                if estimated_ml > config.max_dose_per_cycle {
                    warn!(
                        "⛔ BLOCKED: Lệnh force_on cho {} vượt ngưỡng an toàn liều lượng ({:.2}ml > max_dose_per_cycle {:.2}ml)",
                        pump_name, estimated_ml, config.max_dose_per_cycle
                    );
                    continue;
                }
            }

            step_delta.safety_override_until = Some(now_uptime_ms + (duration * 1000));
            let log_payload = UnifiedSystemLog::build_basic_log_json_with_ts(
                &config.device_id,
                LogLevel::Warning,
                LogCategory::UserAction,
                "Can thiệp cưỡng chế",
                format!("Kích hoạt FORCE ON {} trong {}s.", pump_name, duration),
                None,
                "fsm_command",
                now_wall_time_ms,
            );
            step_events.push(OrchestratorEvent::PublishSystemLog {
                payload_json: log_payload,
            });
        }

        if is_on {
            if let Some(duration) = duration_sec {
                if duration > 0 {
                    step_delta.manual_pump_timeout =
                        Some((pump_name.clone(), now_uptime_ms + (duration * 1000)));
                }
            }
        } else {
            step_delta.manual_pump_timeout_clear = Some(pump_name.clone());
        }

        let pwm_val = pwm.unwrap_or(if is_on { 100 } else { 0 });
        let mut pump_events = build_pump_events_with_status(
            &pump_name,
            is_on,
            pwm_val,
            &mut step_delta,
            &temp_state.pump_status,
        );
        step_events.append(&mut pump_events);

        temp_state.apply_step_delta(&step_delta);
        merge_delta(&mut accumulated_delta, step_delta);
        all_events.append(&mut step_events);
    }

    (accumulated_delta, all_events)
}

fn stop_all_hardware(events: &mut Vec<OrchestratorEvent>) {
    events.push(OrchestratorEvent::SetWaterPump {
        direction: WaterDirection::Stop,
    });
    events.push(OrchestratorEvent::SetMistValve { on: false });
    events.push(OrchestratorEvent::SetMixValve { on: false });
    events.push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
    events.push(OrchestratorEvent::SetDosingPump {
        pump: DosingPumpTarget::NutrientA,
        on: false,
        pwm_percent: 0,
    });
    events.push(OrchestratorEvent::SetDosingPump {
        pump: DosingPumpTarget::NutrientB,
        on: false,
        pwm_percent: 0,
    });
    events.push(OrchestratorEvent::SetDosingPump {
        pump: DosingPumpTarget::PhUp,
        on: false,
        pwm_percent: 0,
    });
    events.push(OrchestratorEvent::SetDosingPump {
        pump: DosingPumpTarget::PhDown,
        on: false,
        pwm_percent: 0,
    });
}

pub fn build_pump_events_with_status(
    pump_name: &str,
    is_on: bool,
    pwm_val: u32,
    delta: &mut ContextDelta,
    pump_status: &hydragrow_shared::PumpStatus,
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
            let mist_valve_is_open = peri_delta.mist_valve.unwrap_or(pump_status.mist_valve);
            let mix_valve_is_open = peri_delta.mix_valve.unwrap_or(pump_status.mix_valve);
            if is_on && !(mist_valve_is_open || mix_valve_is_open) {
                events.push(OrchestratorEvent::PublishCommandRejected {
                    reason: "no_valve_open".to_string(),
                    requested: is_on,
                });
            } else if is_on {
                peri_delta.osaka_pump = Some(is_on);
                peri_delta.osaka_pwm = Some(if is_on { pwm_val } else { 0 });
                events.push(OrchestratorEvent::StartOsakaSoft {
                    target_pwm_percent: pwm_val,
                });
            } else {
                peri_delta.osaka_pump = Some(false);
                peri_delta.osaka_pwm = Some(0);
                events.push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
            }
        }
        "MIST_VALVE" | "MIST" => {
            peri_delta.mist_valve = Some(is_on);
            peri_delta.is_misting_active = Some(is_on);
            events.push(OrchestratorEvent::SetMistValve { on: is_on });

            // [NPL-9] Tắt valve -> nếu Osaka đang chạy và không còn valve nào mở -> Tắt Osaka
            if !is_on {
                let mix_valve_is_open = peri_delta.mix_valve.unwrap_or(pump_status.mix_valve);
                let osaka_running = peri_delta.osaka_pump.unwrap_or(pump_status.osaka_pump);
                if osaka_running && !mix_valve_is_open {
                    peri_delta.osaka_pump = Some(false);
                    peri_delta.osaka_pwm = Some(0);
                    events.push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
                }
            }
        }
        "MIX_VALVE" | "MIX" => {
            peri_delta.mix_valve = Some(is_on);
            events.push(OrchestratorEvent::SetMixValve { on: is_on });

            // [NPL-9] Tắt valve -> nếu Osaka đang chạy và không còn valve nào mở -> Tắt Osaka
            if !is_on {
                let mist_valve_is_open = peri_delta.mist_valve.unwrap_or(pump_status.mist_valve);
                let osaka_running = peri_delta.osaka_pump.unwrap_or(pump_status.osaka_pump);
                if osaka_running && !mist_valve_is_open {
                    peri_delta.osaka_pump = Some(false);
                    peri_delta.osaka_pwm = Some(0);
                    events.push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
                }
            }
        }
        "WATER_PUMP" | "WATER_PUMP_IN" | "PUMP_IN" => {
            peri_delta.water_pump_in = Some(is_on);
            if is_on {
                peri_delta.water_pump_out = Some(false);
            }
            events.push(OrchestratorEvent::SetWaterPump {
                direction: if is_on {
                    WaterDirection::In
                } else {
                    WaterDirection::Stop
                },
            });
        }
        "DRAIN_PUMP" | "WATER_PUMP_OUT" | "PUMP_OUT" => {
            peri_delta.water_pump_out = Some(is_on);
            if is_on {
                peri_delta.water_pump_in = Some(false);
            }
            events.push(OrchestratorEvent::SetWaterPump {
                direction: if is_on {
                    WaterDirection::Out
                } else {
                    WaterDirection::Stop
                },
            });
        }
        _ => {}
    }

    delta.peripherals = Some(peri_delta);
    events
}

pub fn build_pump_events(
    pump_name: &str,
    is_on: bool,
    pwm_val: u32,
    delta: &mut ContextDelta,
    ctx: &SystemContext,
) -> Vec<OrchestratorEvent> {
    build_pump_events_with_status(
        pump_name,
        is_on,
        pwm_val,
        delta,
        &ctx.peripherals.pump_status,
    )
}

pub fn build_stop_pump_events(
    pump_name: &str,
    delta: &mut ContextDelta,
    ctx: &SystemContext,
) -> Vec<OrchestratorEvent> {
    build_pump_events(pump_name, false, 0, delta, ctx)
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::*;
    use hydragrow_shared::fsm::{FaultCode, SystemPhase};
    use hydragrow_shared::MqttCommandInParams;
    use std::sync::mpsc::channel;

    #[test]
    fn reset_fault_followed_by_pump_on_evaluates_against_updated_phase() {
        let (cmd_tx, cmd_rx) = channel();
        let (mqtt_tx, _mqtt_rx) = channel();
        let mut config = ControllerConfig::default();
        config.control_mode = ControlMode::Manual;

        let mut ctx = SystemContext::default();
        ctx.phase = SystemPhase::Fault(FaultCode::EmergencyStop);

        // Queue reset_fault then pump_on
        cmd_tx
            .send(MqttCommandIn {
                action: "reset_fault".to_string(),
                target: Some("all".to_string()),
                params: None,
                pump: None,
                duration_sec: None,
                pwm: None,
            })
            .unwrap();

        cmd_tx
            .send(MqttCommandIn {
                action: "pump_on".to_string(),
                target: Some("all".to_string()),
                params: Some(MqttCommandInParams {
                    pump_id: Some("A".to_string()),
                    duration_sec: Some(10),
                    pwm: Some(80),
                    state: Some(true),
                    ota_url: None,
                    candidates: None,
                }),
                pump: None,
                duration_sec: None,
                pwm: None,
            })
            .unwrap();

        let (delta, events) = process_mqtt_commands(&cmd_rx, &config, &ctx, 1000, 1000, &mqtt_tx);

        let pump_a_event = events.iter().any(|e| {
            matches!(
                e,
                OrchestratorEvent::SetDosingPump {
                    pump: DosingPumpTarget::NutrientA,
                    on: true,
                    ..
                }
            )
        });
        assert!(
            pump_a_event,
            "pump_on immediately following reset_fault must evaluate against the Monitoring phase and be allowed"
        );
        assert_eq!(
            delta.peripherals.as_ref().and_then(|p| p.pump_a),
            Some(true)
        );
    }

    #[test]
    fn emergency_followed_by_pump_on_evaluates_against_updated_phase() {
        let (cmd_tx, cmd_rx) = channel();
        let (mqtt_tx, _mqtt_rx) = channel();
        let mut config = ControllerConfig::default();
        config.control_mode = ControlMode::Manual;

        let mut ctx = SystemContext::default();
        ctx.phase = SystemPhase::Monitoring;

        // Queue trigger_ota (which transitions to Fault/Emergency) then pump_on
        cmd_tx
            .send(MqttCommandIn {
                action: "trigger_ota".to_string(),
                target: Some("all".to_string()),
                params: None,
                pump: None,
                duration_sec: None,
                pwm: None,
            })
            .unwrap();

        cmd_tx
            .send(MqttCommandIn {
                action: "pump_on".to_string(),
                target: Some("all".to_string()),
                params: Some(MqttCommandInParams {
                    pump_id: Some("A".to_string()),
                    duration_sec: Some(10),
                    pwm: Some(80),
                    state: Some(true),
                    ota_url: None,
                    candidates: None,
                }),
                pump: None,
                duration_sec: None,
                pwm: None,
            })
            .unwrap();

        let (delta, events) = process_mqtt_commands(&cmd_rx, &config, &ctx, 1000, 1000, &mqtt_tx);

        let pump_a_event = events.iter().any(|e| {
            matches!(
                e,
                OrchestratorEvent::SetDosingPump {
                    pump: DosingPumpTarget::NutrientA,
                    on: true,
                    ..
                }
            )
        });
        assert!(
            !pump_a_event,
            "pump_on immediately following trigger_ota must evaluate against the Fault phase and be blocked"
        );
        assert_ne!(
            delta.peripherals.as_ref().and_then(|p| p.pump_a),
            Some(true)
        );
    }

    #[test]
    fn manual_pump_timeout_set_and_clear() {
        let (cmd_tx, cmd_rx) = channel();
        let (mqtt_tx, _mqtt_rx) = channel();
        let mut config = ControllerConfig::default();
        config.control_mode = ControlMode::Manual;

        let ctx = SystemContext::default();

        // Queue pump A on with 15s duration
        cmd_tx
            .send(MqttCommandIn {
                action: "pump_on".to_string(),
                target: Some("all".to_string()),
                params: Some(MqttCommandInParams {
                    pump_id: Some("A".to_string()),
                    duration_sec: Some(15),
                    pwm: Some(100),
                    state: Some(true),
                    ota_url: None,
                    candidates: None,
                }),
                pump: None,
                duration_sec: None,
                pwm: None,
            })
            .unwrap();

        let (delta, _) = process_mqtt_commands(&cmd_rx, &config, &ctx, 2000, 1000, &mqtt_tx);
        assert_eq!(
            delta.manual_pump_timeout,
            Some(("A".to_string(), 2000 + 15_000))
        );

        // Queue pump A off
        cmd_tx
            .send(MqttCommandIn {
                action: "pump_off".to_string(),
                target: Some("all".to_string()),
                params: Some(MqttCommandInParams {
                    pump_id: Some("A".to_string()),
                    duration_sec: None,
                    pwm: None,
                    state: Some(false),
                    ota_url: None,
                    candidates: None,
                }),
                pump: None,
                duration_sec: None,
                pwm: None,
            })
            .unwrap();

        let (delta_off, _) = process_mqtt_commands(&cmd_rx, &config, &ctx, 5000, 1000, &mqtt_tx);
        assert_eq!(delta_off.manual_pump_timeout_clear, Some("A".to_string()));
    }

    #[test]
    fn valve_open_followed_by_osaka_in_same_batch_is_allowed() {
        let (cmd_tx, cmd_rx) = channel();
        let (mqtt_tx, _mqtt_rx) = channel();
        let mut config = ControllerConfig::default();
        config.control_mode = ControlMode::Manual;

        let ctx = SystemContext::default();
        assert!(!ctx.peripherals.pump_status.mix_valve);
        assert!(!ctx.peripherals.pump_status.mist_valve);

        // Batch: open mix valve, then turn osaka pump on
        cmd_tx
            .send(MqttCommandIn {
                action: "on".to_string(),
                target: Some("all".to_string()),
                params: Some(MqttCommandInParams {
                    pump_id: Some("MIX_VALVE".to_string()),
                    duration_sec: None,
                    pwm: None,
                    state: Some(true),
                    ota_url: None,
                    candidates: None,
                }),
                pump: None,
                duration_sec: None,
                pwm: None,
            })
            .unwrap();

        cmd_tx
            .send(MqttCommandIn {
                action: "on".to_string(),
                target: Some("all".to_string()),
                params: Some(MqttCommandInParams {
                    pump_id: Some("OSAKA".to_string()),
                    duration_sec: None,
                    pwm: Some(85),
                    state: Some(true),
                    ota_url: None,
                    candidates: None,
                }),
                pump: None,
                duration_sec: None,
                pwm: None,
            })
            .unwrap();

        let (delta, events) = process_mqtt_commands(&cmd_rx, &config, &ctx, 1000, 1000, &mqtt_tx);

        let osaka_soft_started = events.iter().any(|e| {
            matches!(
                e,
                OrchestratorEvent::StartOsakaSoft {
                    target_pwm_percent: 85
                }
            )
        });
        let command_rejected = events
            .iter()
            .any(|e| matches!(e, OrchestratorEvent::PublishCommandRejected { .. }));

        assert!(
            osaka_soft_started,
            "Osaka pump must start when mix_valve was opened earlier in the same batch"
        );
        assert!(!command_rejected, "Osaka command must not be rejected");
        assert_eq!(
            delta.peripherals.as_ref().and_then(|p| p.osaka_pump),
            Some(true)
        );
    }

    #[test]
    fn osaka_without_valve_in_same_batch_is_rejected() {
        let (cmd_tx, cmd_rx) = channel();
        let (mqtt_tx, _mqtt_rx) = channel();
        let mut config = ControllerConfig::default();
        config.control_mode = ControlMode::Manual;

        let ctx = SystemContext::default();
        assert!(!ctx.peripherals.pump_status.mix_valve);
        assert!(!ctx.peripherals.pump_status.mist_valve);

        // Command: osaka on without any valve open
        cmd_tx
            .send(MqttCommandIn {
                action: "on".to_string(),
                target: Some("all".to_string()),
                params: Some(MqttCommandInParams {
                    pump_id: Some("OSAKA".to_string()),
                    duration_sec: None,
                    pwm: Some(85),
                    state: Some(true),
                    ota_url: None,
                    candidates: None,
                }),
                pump: None,
                duration_sec: None,
                pwm: None,
            })
            .unwrap();

        let (_delta, events) = process_mqtt_commands(&cmd_rx, &config, &ctx, 1000, 1000, &mqtt_tx);

        let command_rejected = events.iter().any(|e| {
            matches!(
                e,
                OrchestratorEvent::PublishCommandRejected {
                    reason,
                    ..
                } if reason == "no_valve_open"
            )
        });
        assert!(
            command_rejected,
            "Osaka without open valve must be rejected"
        );
    }
}
