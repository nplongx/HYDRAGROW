use hydragrow_shared::{
    BasicSystemLogMetadata, ControlMode, ControllerConfig, LogCategory, LogLevel, MqttCommandIn,
    SystemLogEvent,
};
use log::{info, warn};
use std::sync::mpsc::{Receiver, Sender};

use super::phases::SystemPhase;
use super::system_context::SystemContext;
use crate::fsm::events::{DosingPumpTarget, OrchestratorEvent};
use crate::pump::WaterDirection;

// ---------------------------------------------------------------------------
// process_mqtt_commands
//
// Xử lý tất cả lệnh đến từ MQTT trong một tick FSM qua cơ chế sinh Event.
// Trả về tuple `(bool, Vec<OrchestratorEvent>)`.
// ---------------------------------------------------------------------------
pub fn process_mqtt_commands(
    cmd_rx: &Receiver<MqttCommandIn>,
    config: &ControllerConfig,
    ctx: &mut SystemContext,
    current_time_ms: u64,
    fsm_mqtt_tx: &Sender<String>,
) -> (bool, Vec<OrchestratorEvent>) {
    let mut force_sync = false;
    let mut all_events = Vec::new();

    let is_emergency_state = matches!(
        ctx.phase,
        SystemPhase::EmergencyStop(_)
            | SystemPhase::Fault(_)
            | SystemPhase::SensorCalibration { .. }
    );

    while let Ok(cmd) = cmd_rx.try_recv() {
        let action_lower = cmd.action.to_lowercase();

        // --- Lệnh hệ thống (không phụ thuộc mode) ---
        if action_lower == "enter_calibration" {
            info!("🛠️ Bắt đầu chế độ Hiệu chuẩn Cảm biến! Khóa chéo an toàn.");

            // Ép phần cứng hạ toàn bộ chân cờ điều khiển bơm vật lý
            all_events.push(OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop,
            });
            all_events.push(OrchestratorEvent::SetMistValve { on: false });
            all_events.push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
            all_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: false,
                pwm_percent: 0,
            });
            all_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientB,
                on: false,
                pwm_percent: 0,
            });
            all_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhUp,
                on: false,
                pwm_percent: 0,
            });
            all_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhDown,
                on: false,
                pwm_percent: 0,
            });

            // Cập nhật trạng thái Context tương ứng
            ctx.peripherals.reset(current_time_ms / 1000);
            let step = cmd.target.clone().unwrap_or_else(|| "default".to_string());
            ctx.phase = SystemPhase::SensorCalibration { step };
            ctx.phase_finish_ms = Some(current_time_ms + 3_600_000);
            force_sync = true;
            continue;
        }

        if action_lower == "exit_calibration" {
            if matches!(ctx.phase, SystemPhase::SensorCalibration { .. }) {
                info!("✅ Thoát chế độ Hiệu chuẩn, quay về Monitoring.");
                ctx.phase = SystemPhase::Monitoring;
                ctx.phase_finish_ms = None;
                force_sync = true;
            }
            continue;
        }

        if action_lower == "sync_status" {
            force_sync = true;
            all_events.push(OrchestratorEvent::PublishFsmState);
            continue;
        }

        if action_lower == "ota_update" {
            let ota_url = cmd
                .params
                .as_ref()
                .and_then(|p| p.ota_url.as_deref())
                .unwrap_or("");

            let log_payload = serde_json::json!(BasicSystemLogMetadata {
                source: "fsm_command".to_string(),
                message: format!(
                    "Nhận lệnh OTA từ MQTT. URL: {}. Firmware sẽ chuyển giao cho OTA task.",
                    if ota_url.is_empty() {
                        "<missing>"
                    } else {
                        ota_url
                    }
                ),
                skip_reason: None,
                cycle_id: None,
            })
            .to_string();

            all_events.push(OrchestratorEvent::PublishSystemLog {
                payload_json: log_payload,
            });
            info!("📦 OTA trigger received: {}", ota_url);
            force_sync = true;
            continue;
        }

        if action_lower == "reset_fault" {
            info!("🔄 Nhận lệnh Reset. Khôi phục hệ thống...");

            // Phát dọn dẹp tắt hết bơm cứng phần cứng
            all_events.push(OrchestratorEvent::SetWaterPump {
                direction: WaterDirection::Stop,
            });
            all_events.push(OrchestratorEvent::SetMistValve { on: false });
            all_events.push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
            all_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: false,
                pwm_percent: 0,
            });
            all_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientB,
                on: false,
                pwm_percent: 0,
            });
            all_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhUp,
                on: false,
                pwm_percent: 0,
            });
            all_events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhDown,
                on: false,
                pwm_percent: 0,
            });

            // Xóa sạch cờ trạng thái Context
            ctx.peripherals.reset(current_time_ms / 1000);
            ctx.stabilizer_tracker.reset();

            if let Some(sample) = ctx.calibration.pending_sample.as_mut() {
                sample.invalid_by_noise = true;
            }

            ctx.phase_start_ms = None;
            ctx.phase_finish_ms = None;
            ctx.phase = SystemPhase::Monitoring;

            // Đồng bộ bộ nhớ flash
            all_events.push(OrchestratorEvent::SaveNvsSnapshot);

            force_sync = true;
            continue;
        }

        // --- Lệnh bơm thủ công chỉ cho phép khi ở MANUAL mode ---
        if config.control_mode == ControlMode::Auto {
            warn!("Bỏ qua lệnh thủ công vì đang ở AUTO.");
            continue;
        }

        let target_lower = cmd.target.as_deref().unwrap_or("pump").to_lowercase();
        if target_lower != "pump" && target_lower != "all" {
            continue;
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
            ctx.safety.safety_override_until = current_time_ms + (duration as u64 * 1000);

            let log_payload = serde_json::json!(BasicSystemLogMetadata {
                source: "fsm_command".to_string(),
                message: format!(
                    "Người dùng đã dùng lệnh FORCE ON để ép bật {} trong {} giây, vượt qua các lớp bảo vệ an toàn.",
                    pump_name, duration
                ),
                skip_reason: None,
                cycle_id: None,
            })
            .to_string();

            all_events.push(OrchestratorEvent::PublishSystemLog {
                payload_json: log_payload,
            });
        } else if is_on {
            let log_payload = serde_json::json!(BasicSystemLogMetadata {
                source: "fsm_command".to_string(),
                message: format!("Người dùng đã bật bơm thủ công {}.", pump_name),
                skip_reason: None,
                cycle_id: None,
            })
            .to_string();

            all_events.push(OrchestratorEvent::PublishSystemLog {
                payload_json: log_payload,
            });
        }

        // Ghi timeout thủ công vào bộ nhớ Context
        if is_on {
            match duration_sec {
                Some(duration) if duration > 0 => {
                    let finish_time = current_time_ms + (duration as u64 * 1000);
                    ctx.safety
                        .manual_timeouts
                        .insert(pump_name.clone(), finish_time);
                }
                _ => {
                    ctx.safety.manual_timeouts.remove(&pump_name);
                }
            }
        } else {
            ctx.safety.manual_timeouts.remove(&pump_name);
        }

        let pwm_val = pwm.unwrap_or(if is_on { 100 } else { 0 });

        let mut pump_events = apply_pump_command(ctx, &pump_name, is_on, pwm_val, current_time_ms);
        all_events.append(&mut pump_events);
        force_sync = true;
    }

    (force_sync, all_events)
}

// ---------------------------------------------------------------------------
// apply_pump_command – Trích xuất và xuất sinh Hardware Events tương ứng
// ---------------------------------------------------------------------------
fn apply_pump_command(
    ctx: &mut SystemContext,
    pump_name: &str,
    is_on: bool,
    pwm_val: u32,
    current_time_ms: u64,
) -> Vec<OrchestratorEvent> {
    let mut events = Vec::new();

    match pump_name {
        "A" | "PUMP_A" => {
            ctx.peripherals.pump_status.pump_a = is_on;
            ctx.peripherals.pump_status.pump_a_pwm = Some(if is_on { pwm_val } else { 0 });
            events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientA,
                on: is_on,
                pwm_percent: if is_on { pwm_val } else { 0 },
            });
        }
        "B" | "PUMP_B" => {
            ctx.peripherals.pump_status.pump_b = is_on;
            ctx.peripherals.pump_status.pump_b_pwm = Some(if is_on { pwm_val } else { 0 });
            events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::NutrientB,
                on: is_on,
                pwm_percent: if is_on { pwm_val } else { 0 },
            });
        }
        "PH_UP" | "PUMP_PH_UP" => {
            ctx.peripherals.pump_status.ph_up = is_on;
            ctx.peripherals.pump_status.ph_up_pwm = Some(if is_on { pwm_val } else { 0 });
            events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhUp,
                on: is_on,
                pwm_percent: if is_on { pwm_val } else { 0 },
            });
        }
        "PH_DOWN" | "PUMP_PH_DOWN" => {
            ctx.peripherals.pump_status.ph_down = is_on;
            ctx.peripherals.pump_status.ph_down_pwm = Some(if is_on { pwm_val } else { 0 });
            events.push(OrchestratorEvent::SetDosingPump {
                pump: DosingPumpTarget::PhDown,
                on: is_on,
                pwm_percent: if is_on { pwm_val } else { 0 },
            });
        }
        "OSAKA_PUMP" | "OSAKA" => {
            ctx.peripherals.pump_status.osaka_pump = is_on;
            ctx.peripherals.pump_status.osaka_pwm = Some(if is_on { pwm_val } else { 0 });
            if is_on {
                events.push(OrchestratorEvent::StartOsakaSoft {
                    target_pwm_percent: pwm_val,
                });
            } else {
                events.push(OrchestratorEvent::SetOsakaPump { pwm_percent: 0 });
            }
        }
        "MIST_VALVE" | "MIST" => {
            ctx.peripherals.pump_status.mist_valve = is_on;
            ctx.peripherals.is_misting_active = is_on;
            if is_on {
                ctx.peripherals.last_mist_toggle_time = current_time_ms;
            }
            events.push(OrchestratorEvent::SetMistValve { on: is_on });
        }
        "WATER_PUMP" | "WATER_PUMP_IN" | "PUMP_IN" => {
            ctx.peripherals.pump_status.water_pump_in = is_on;
            if is_on {
                ctx.peripherals.pump_status.water_pump_out = false;
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
            ctx.peripherals.pump_status.water_pump_out = is_on;
            if is_on {
                ctx.peripherals.pump_status.water_pump_in = false;
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

    events
}

