// fsm/mod.rs – điểm vào chính của module FSM
//
// Re-export các kiểu public để code bên ngoài chỉ cần `use crate::fsm::*`.

pub mod auto_fsm;
pub mod calibration;
pub mod commands;
pub mod context;
pub mod types;
pub mod utils;

pub use context::ControlContext;
pub use types::{PendingCalibrationSample, PendingDose, SharedSensorData, SystemState};

use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};
use hydragrow_shared::ControlMode;
use log::info;

use crate::config::SharedConfig;
use crate::mqtt::MqttCommandPayload;
use crate::pump::PumpController;

use auto_fsm::run_auto_fsm;
use commands::process_mqtt_commands;
use utils::{get_current_time_ms, get_current_time_sec};

// ---------------------------------------------------------------------------
// start_fsm_control_loop
//
// Hàm khởi động vòng lặp FSM chạy trên thread riêng.
// Gọi một lần khi khởi động hệ thống.
// ---------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub fn start_fsm_control_loop(
    shared_config: SharedConfig,
    shared_sensors: SharedSensorData,
    mut pump_ctrl: PumpController,
    nvs_partition: EspDefaultNvsPartition,
    cmd_rx: Receiver<MqttCommandPayload>,
    fsm_mqtt_tx: Sender<String>,
    dosing_report_tx: Sender<String>,
    sensor_cmd_tx: Sender<String>,
) {
    let mut ctx = ControlContext::default();
    let mut last_reported_state = String::new();

    let mut nvs = EspNvs::new(nvs_partition, "agitech", true).ok();
    let current_time_on_boot = get_current_time_sec();

    // Khôi phục thời điểm thay nước & bơm định kỳ từ NVS flash
    ctx.last_water_change_time = nvs
        .as_mut()
        .and_then(|f| f.get_u64("last_w_change").unwrap_or(None))
        .unwrap_or_else(|| {
            if let Some(f) = nvs.as_mut() {
                let _ = f.set_u64("last_w_change", current_time_on_boot);
            }
            current_time_on_boot
        });

    ctx.last_scheduled_dose_time_sec = nvs
        .as_mut()
        .and_then(|f| f.get_u64("last_sched_dose").unwrap_or(None))
        .unwrap_or_else(|| {
            if let Some(f) = nvs.as_mut() {
                let _ = f.set_u64("last_sched_dose", current_time_on_boot);
            }
            current_time_on_boot
        });

    ctx.last_mixing_start_sec = current_time_on_boot;

    info!("🚀 Bắt đầu chạy Máy trạng thái (FSM) Đa luồng Hợp nhất...");

    // Giai đoạn khởi động 3 giây
    let boot_start_ms = get_current_time_ms();
    loop {
        if get_current_time_ms() - boot_start_ms > 3000 {
            ctx.current_state = SystemState::Monitoring;
            break;
        }
        if report_state_if_changed(&ctx.current_state, &mut last_reported_state) {
            let _ = fsm_mqtt_tx.send(build_status_msg(&ctx));
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Vòng lặp chính
    loop {
        let config = shared_config.read().unwrap().clone();
        let sensors = shared_sensors.read().unwrap().clone();
        let current_time_ms = get_current_time_ms();
        let current_time_sec = current_time_ms / 1000;

        ctx.sync_adaptive_ratios_from_config(&config);

        let force_sync =
            process_mqtt_commands(&cmd_rx, &config, &mut pump_ctrl, &mut ctx, current_time_ms);

        // --- Xử lý timeout bơm thủ công ---
        let expired: Vec<String> = ctx
            .manual_timeouts
            .iter()
            .filter(|(_, &t)| current_time_ms >= t)
            .map(|(k, _)| k.clone())
            .collect();
        for pump in expired {
            ctx.manual_timeouts.remove(&pump);
            info!("⏱️ HẾT GIỜ (SAFE TIMEOUT): Tự động tắt bơm {}!", pump);
            ctx.turn_off_pump(&pump, &mut pump_ctrl);
        }

        let is_safety_overridden = current_time_ms < ctx.safety_override_until;

        if !is_safety_overridden {
            let is_noisy_sample = ctx.check_and_update_noise(&sensors, &config);
            let has_sensor_fault = (config.enable_water_level_sensor && sensors.err_water)
                || (config.enable_ec_sensor && sensors.err_ec)
                || (config.enable_ph_sensor && sensors.err_ph)
                || (config.enable_temp_sensor && sensors.err_temp);

            ctx.update_auto_tune_health(is_noisy_sample || has_sensor_fault);

            if is_noisy_sample {
                ctx.mark_pending_sample_noise_violation();
            }

            if !(is_noisy_sample && config.control_mode == ControlMode::Auto) {
                tick_safety_and_control(
                    current_time_ms,
                    current_time_sec,
                    &config,
                    &sensors,
                    &mut ctx,
                    &mut pump_ctrl,
                    &shared_config,
                    &mut nvs,
                    &dosing_report_tx,
                    &fsm_mqtt_tx,
                );
            }
        }

        // --- Cập nhật chế độ đọc cảm biến liên tục khi đang bơm nước ---
        let needs_continuous = matches!(
            ctx.current_state,
            SystemState::WaterRefilling { .. } | SystemState::WaterDraining { .. }
        );
        if needs_continuous != ctx.last_continuous_level {
            let _ = sensor_cmd_tx.send(format!(
                r#"{{"target":"sensor","action":"set_continuous","params":{{"state":{}}}}}"#,
                needs_continuous
            ));
            ctx.last_continuous_level = needs_continuous;
        }

        // --- Đồng bộ pump_status ra shared sensor ---
        if let Ok(mut s) = shared_sensors.write() {
            s.pump_status = ctx.pump_status.clone();
        }

        // --- Publish trạng thái nếu thay đổi ---
        let state_changed = report_state_if_changed(&ctx.current_state, &mut last_reported_state);
        if state_changed || force_sync {
            let _ = fsm_mqtt_tx.send(build_status_msg(&ctx));
            if force_sync {
                last_reported_state.clear();
                let _ = sensor_cmd_tx.send(
                    r#"{"target":"sensor","action":"force_publish","params":{}}"#.to_string(),
                );
                info!("⚡ Đã ép luồng chính Publish trạng thái bơm mới nhất lên App!");
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

// ---------------------------------------------------------------------------
// tick_safety_and_control
// Kiểm tra điều kiện khẩn cấp rồi điều phối Auto/Manual FSM.
// ---------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
fn tick_safety_and_control(
    current_time_ms: u64,
    current_time_sec: u64,
    config: &hydragrow_shared::ControllerConfig,
    sensors: &crate::mqtt::SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
    shared_config: &SharedConfig,
    nvs: &mut Option<EspNvs<esp_idf_svc::nvs::NvsDefault>>,
    dosing_report_tx: &Sender<String>,
    fsm_mqtt_tx: &Sender<String>,
) {
    use crate::pump::WaterDirection;
    use log::error;

    // --- Xác định lý do dừng khẩn cấp ---
    let emergency_reason = detect_emergency(config, sensors);
    let should_emergency_stop = !emergency_reason.is_empty();

    if should_emergency_stop {
        if !matches!(ctx.current_state, SystemState::EmergencyStop(_)) {
            error!(
                "⚠️ DỪNG KHẨN CẤP Toàn bộ hệ thống! Lý do: {}",
                emergency_reason
            );
            ctx.stop_all_pumps(pump_ctrl);
            ctx.current_state = SystemState::EmergencyStop(emergency_reason);
        }
        return;
    }

    if !config.is_enabled {
        if ctx.current_state != SystemState::Monitoring {
            ctx.stop_all_pumps(pump_ctrl);
            ctx.current_state = SystemState::Monitoring;
        }
        return;
    }

    if matches!(ctx.current_state, SystemState::EmergencyStop(_)) {
        info!("✅ Hệ thống an toàn trở lại (hoặc đang Cưỡng chế).");
        ctx.current_state = SystemState::Monitoring;
        return;
    }

    if config.control_mode == ControlMode::Auto {
        // --- Misting ---
        tick_misting(current_time_ms, config, sensors, ctx, pump_ctrl);

        // --- Mixing định kỳ ---
        tick_scheduled_mixing(current_time_sec, config, ctx);

        if !matches!(ctx.current_state, SystemState::SystemFault(_)) {
            run_auto_fsm(
                current_time_ms,
                config,
                sensors,
                ctx,
                pump_ctrl,
                shared_config,
                nvs,
                dosing_report_tx,
                fsm_mqtt_tx,
            );
        }

        // --- Điều khiển bơm Osaka ---
        tick_osaka_pump(current_time_ms, config, ctx, pump_ctrl);
    } else {
        // Manual mode
        let is_auto_running_state = !matches!(
            ctx.current_state,
            SystemState::Monitoring
                | SystemState::SystemFault(_)
                | SystemState::EmergencyStop(_)
                | SystemState::SensorCalibration { .. }
                | SystemState::ManualMode
                | SystemState::DosingCycleComplete
        );
        if is_auto_running_state {
            info!("Chuyển sang chế độ MANUAL.");
            ctx.stop_all_pumps(pump_ctrl);
            ctx.current_state = SystemState::ManualMode;
        }
    }
}

/// Trả về lý do emergency nếu có, rỗng nếu hệ thống bình thường.
fn detect_emergency(
    config: &hydragrow_shared::ControllerConfig,
    sensors: &crate::mqtt::SensorData,
) -> String {
    if config.emergency_shutdown {
        return "MANUAL_STOP".to_string();
    }
    if config.enable_water_level_sensor && sensors.err_water {
        return "SENSOR_FAULT_WATER".to_string();
    }
    if config.enable_ec_sensor && sensors.err_ec {
        return "SENSOR_FAULT_EC".to_string();
    }
    if config.enable_ph_sensor && sensors.err_ph {
        return "SENSOR_FAULT_PH".to_string();
    }
    if config.enable_temp_sensor && sensors.err_temp {
        return "SENSOR_FAULT_TEMP".to_string();
    }
    if config.enable_water_level_sensor && sensors.water_level < config.water_level_critical_min {
        return "WATER_CRITICAL".to_string();
    }
    if config.enable_ec_sensor
        && (sensors.ec < config.min_ec_limit || sensors.ec > config.max_ec_limit)
    {
        return "EC_OUT_OF_BOUNDS".to_string();
    }
    if config.enable_ph_sensor
        && (sensors.ph < config.min_ph_limit || sensors.ph > config.max_ph_limit)
    {
        return "PH_OUT_OF_BOUNDS".to_string();
    }
    if config.enable_temp_sensor
        && (sensors.temp < config.min_temp_limit || sensors.temp > config.max_temp_limit)
    {
        return format!("TEMP_OUT_OF_BOUNDS: {:.1}°C", sensors.temp);
    }
    String::new()
}

fn tick_misting(
    current_time_ms: u64,
    config: &hydragrow_shared::ControllerConfig,
    sensors: &crate::mqtt::SensorData,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
) {
    let is_hot = config.enable_temp_sensor && sensors.temp >= config.misting_temp_threshold;
    let on_duration = if is_hot {
        config.high_temp_misting_on_duration_ms as u64
    } else {
        config.misting_on_duration_ms as u64
    };
    let off_duration = if is_hot {
        config.high_temp_misting_off_duration_ms as u64
    } else {
        config.misting_off_duration_ms as u64
    };

    if ctx.is_misting_active {
        if current_time_ms >= ctx.last_mist_toggle_time + on_duration {
            let _ = pump_ctrl.set_mist_valve(false);
            ctx.is_misting_active = false;
            ctx.last_mist_toggle_time = current_time_ms;
            ctx.pump_status.mist_valve = false;
        }
    } else if current_time_ms >= ctx.last_mist_toggle_time + off_duration {
        let _ = pump_ctrl.set_mist_valve(true);
        ctx.is_misting_active = true;
        ctx.last_mist_toggle_time = current_time_ms;
        ctx.pump_status.mist_valve = true;
    }
}

fn tick_scheduled_mixing(
    current_time_sec: u64,
    config: &hydragrow_shared::ControllerConfig,
    ctx: &mut ControlContext,
) {
    if config.scheduled_mixing_interval_sec > 0 && config.scheduled_mixing_duration_sec > 0 {
        if ctx.is_scheduled_mixing_active {
            if current_time_sec
                >= ctx.last_mixing_start_sec + config.scheduled_mixing_duration_sec as u64
            {
                ctx.is_scheduled_mixing_active = false;
            }
        } else if current_time_sec
            >= ctx.last_mixing_start_sec + config.scheduled_mixing_interval_sec as u64
        {
            ctx.is_scheduled_mixing_active = true;
            ctx.last_mixing_start_sec = current_time_sec;
        }
    } else {
        ctx.is_scheduled_mixing_active = false;
    }
}

fn tick_osaka_pump(
    current_time_ms: u64,
    config: &hydragrow_shared::ControllerConfig,
    ctx: &mut ControlContext,
    pump_ctrl: &mut PumpController,
) {
    let needs_osaka =
        ctx.fsm_osaka_active || ctx.is_misting_active || ctx.is_scheduled_mixing_active;

    if needs_osaka {
        let target_pwm = if ctx.is_misting_active {
            config.osaka_misting_pwm_percent as u32
        } else {
            config.osaka_mixing_pwm_percent as u32
        };
        if !ctx.pump_status.osaka_pump {
            let _ = pump_ctrl.start_osaka_pump_soft(target_pwm);
            ctx.pump_status.osaka_pump = true;
            ctx.pump_status.osaka_pwm = Some(target_pwm);
            ctx.current_osaka_pwm = target_pwm;
        } else if ctx.current_osaka_pwm != target_pwm {
            let _ = pump_ctrl.set_osaka_pump_pwm(target_pwm);
            ctx.pump_status.osaka_pwm = Some(target_pwm);
            ctx.current_osaka_pwm = target_pwm;
        }
    } else if ctx.pump_status.osaka_pump {
        let _ = pump_ctrl.set_osaka_pump_pwm(0);
        ctx.pump_status.osaka_pump = false;
        ctx.pump_status.osaka_pwm = Some(0);
        ctx.current_osaka_pwm = 0;
    }
}

// ---------------------------------------------------------------------------
// Helpers nhỏ dùng trong vòng lặp
// ---------------------------------------------------------------------------

fn report_state_if_changed(current_state: &SystemState, last_reported_state: &mut String) -> bool {
    let s = current_state.to_payload_string();
    if s != *last_reported_state {
        info!("📡 Trạng thái FSM: [{}]", s);
        *last_reported_state = s;
        true
    } else {
        false
    }
}

fn build_status_msg(ctx: &ControlContext) -> String {
    serde_json::json!({
        "online": true,
        "current_state": ctx.current_state.to_payload_string(),
        "pump_status": ctx.pump_status
    })
    .to_string()
}
