use hydragrow_shared::{ControlMode, ControllerConfig};
use log::{info, warn};
use std::sync::mpsc::Receiver;

use super::context::ControlContext;
use super::types::SystemState;
use crate::mqtt::MqttCommandPayload;
use crate::pump::{PumpController, PumpType, WaterDirection};

// ---------------------------------------------------------------------------
// process_mqtt_commands
//
// Xử lý tất cả lệnh đến từ MQTT trong một tick FSM.
// Trả về `true` nếu cần force-publish trạng thái ngay lập tức.
// ---------------------------------------------------------------------------
pub fn process_mqtt_commands(
    cmd_rx: &Receiver<MqttCommandPayload>,
    config: &ControllerConfig,
    pump_ctrl: &mut PumpController,
    ctx: &mut ControlContext,
    current_time_ms: u64,
) -> bool {
    let mut force_sync = false;

    let is_emergency_state = matches!(
        ctx.current_state,
        SystemState::EmergencyStop(_)
            | SystemState::SystemFault(_)
            | SystemState::SensorCalibration { .. }
    );

    while let Ok(cmd) = cmd_rx.try_recv() {
        let action_lower = cmd.action.to_lowercase();

        // --- Lệnh hệ thống (không phụ thuộc mode) ---
        if action_lower == "enter_calibration" {
            info!("🛠️ Bắt đầu chế độ Hiệu chuẩn Cảm biến! Khóa chéo an toàn.");
            ctx.stop_all_pumps(pump_ctrl);
            let step = cmd.target.clone().unwrap_or_else(|| "IDLE".to_string());
            ctx.current_state = SystemState::SensorCalibration {
                step,
                finish_time: current_time_ms + 3_600_000,
            };
            force_sync = true;
            continue;
        }

        if action_lower == "exit_calibration" {
            if matches!(ctx.current_state, SystemState::SensorCalibration { .. }) {
                info!("✅ Thoát chế độ Hiệu chuẩn, quay về Monitoring.");
                ctx.current_state = SystemState::Monitoring;
                force_sync = true;
            }
            continue;
        }

        if action_lower == "sync_status" {
            force_sync = true;
            continue;
        }

        if action_lower == "reset_fault" {
            info!("🔄 Nhận lệnh Reset. Khôi phục hệ thống...");
            ctx.stop_all_pumps(pump_ctrl);
            ctx.reset_faults();
            force_sync = true;
            continue;
        }

        // --- Lệnh bơm thủ công chỉ cho phép khi ở MANUAL mode ---
        if config.control_mode == ControlMode::Auto {
            warn!("Bỏ qua lệnh thủ công vì đang ở AUTO.");
            continue;
        }

        if let Some(target) = &cmd.target {
            let target_lower = target.to_lowercase();
            if target_lower != "pump" && target_lower != "all" {
                continue;
            }
        }

        let pump_name = cmd
            .params
            .as_ref()
            .and_then(|p| p.pump_id.as_ref())
            .map(|p| p.to_uppercase())
            .or_else(|| cmd.pump.as_ref().map(|p| p.to_uppercase()))
            .unwrap_or_else(|| "ALL".to_string());

        let is_force_on = action_lower == "force_on";
        let is_set_pwm = action_lower == "set_pwm";
        let pwm = cmd.params.as_ref().and_then(|p| p.pwm).or(cmd.pwm);
        let duration_sec = cmd
            .params
            .as_ref()
            .and_then(|p| p.duration_sec)
            .or(cmd.duration_sec);
        let explicit_state = cmd.params.as_ref().and_then(|p| p.state);

        let mut is_on = is_force_on
            || matches!(action_lower.as_str(), "pump_on" | "on" | "true" | "1")
            || (is_set_pwm && pwm.unwrap_or(0) > 0);

        if let Some(state) = explicit_state {
            is_on = state;
        }

        if is_emergency_state && is_on && !is_force_on {
            warn!(
                "❌ BLOCKED: Không thể điều khiển {} bình thường vì hệ thống đang Lỗi / Hiệu chuẩn / EmergencyStop. Vui lòng dùng FORCE.",
                pump_name
            );
            continue;
        }

        if is_force_on {
            info!("⚠️ NGƯỜI DÙNG CƯỠNG CHẾ BẬT {}!", pump_name);
            let duration = duration_sec.unwrap_or(120);
            ctx.safety_override_until = current_time_ms + (duration as u64 * 1000);
        }

        // Ghi timeout thủ công
        if is_on {
            if let Some(duration) = duration_sec {
                if duration > 0 {
                    let finish_time = current_time_ms + (duration as u64 * 1000);
                    ctx.manual_timeouts.insert(pump_name.clone(), finish_time);
                }
            }
        } else {
            ctx.manual_timeouts.remove(&pump_name);
        }

        let pwm_val = pwm.unwrap_or(if is_on { 100 } else { 0 });

        apply_pump_command(
            ctx,
            pump_ctrl,
            &pump_name,
            is_on,
            is_set_pwm,
            pwm,
            pwm_val,
            current_time_ms,
        );
        force_sync = true;
    }

    force_sync
}

// ---------------------------------------------------------------------------
// apply_pump_command – áp dụng lệnh bơm cụ thể lên phần cứng + trạng thái
// ---------------------------------------------------------------------------
fn apply_pump_command(
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    pump_name: &str,
    is_on: bool,
    is_set_pwm: bool,
    pwm: Option<u32>,
    pwm_val: u32,
    current_time_ms: u64,
) {
    let _ = match pump_name {
        "A" | "PUMP_A" => {
            ctx.pump_status.pump_a = is_on;
            ctx.pump_status.pump_a_pwm = Some(if is_on { pwm_val } else { 0 });
            if pwm.is_some() || is_set_pwm {
                pump_ctrl.set_dosing_pump_pwm(PumpType::NutrientA, is_on, pwm_val)
            } else {
                pump_ctrl.set_pump_state(PumpType::NutrientA, is_on)
            }
        }
        "B" | "PUMP_B" => {
            ctx.pump_status.pump_b = is_on;
            ctx.pump_status.pump_b_pwm = Some(if is_on { pwm_val } else { 0 });
            if pwm.is_some() || is_set_pwm {
                pump_ctrl.set_dosing_pump_pwm(PumpType::NutrientB, is_on, pwm_val)
            } else {
                pump_ctrl.set_pump_state(PumpType::NutrientB, is_on)
            }
        }
        "PH_UP" | "PUMP_PH_UP" => {
            ctx.pump_status.ph_up = is_on;
            ctx.pump_status.ph_up_pwm = Some(if is_on { pwm_val } else { 0 });
            if pwm.is_some() || is_set_pwm {
                pump_ctrl.set_dosing_pump_pwm(PumpType::PhUp, is_on, pwm_val)
            } else {
                pump_ctrl.set_pump_state(PumpType::PhUp, is_on)
            }
        }
        "PH_DOWN" | "PUMP_PH_DOWN" => {
            ctx.pump_status.ph_down = is_on;
            ctx.pump_status.ph_down_pwm = Some(if is_on { pwm_val } else { 0 });
            if pwm.is_some() || is_set_pwm {
                pump_ctrl.set_dosing_pump_pwm(PumpType::PhDown, is_on, pwm_val)
            } else {
                pump_ctrl.set_pump_state(PumpType::PhDown, is_on)
            }
        }
        "OSAKA_PUMP" | "OSAKA" => {
            ctx.pump_status.osaka_pump = is_on;
            ctx.pump_status.osaka_pwm = Some(if is_on { pwm_val } else { 0 });
            if pwm.is_some() || is_set_pwm {
                pump_ctrl.set_osaka_pump_pwm(pwm_val)
            } else {
                pump_ctrl.set_osaka_pump(is_on)
            }
        }
        "MIST_VALVE" | "MIST" => {
            ctx.pump_status.mist_valve = is_on;
            ctx.is_misting_active = is_on;
            if is_on {
                ctx.last_mist_toggle_time = current_time_ms;
            }
            pump_ctrl.set_mist_valve(is_on)
        }
        "WATER_PUMP" | "WATER_PUMP_IN" | "PUMP_IN" => {
            ctx.pump_status.water_pump_in = is_on;
            if is_on {
                ctx.pump_status.water_pump_out = false;
            }
            pump_ctrl.set_water_pump(if is_on {
                WaterDirection::In
            } else {
                WaterDirection::Stop
            })
        }
        "DRAIN_PUMP" | "WATER_PUMP_OUT" | "PUMP_OUT" => {
            ctx.pump_status.water_pump_out = is_on;
            if is_on {
                ctx.pump_status.water_pump_in = false;
            }
            pump_ctrl.set_water_pump(if is_on {
                WaterDirection::Out
            } else {
                WaterDirection::Stop
            })
        }
        _ => Ok(()),
    };
}
