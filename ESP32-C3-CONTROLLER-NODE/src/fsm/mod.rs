// fsm/mod.rs – điểm vào chính của module FSM
//
// Re-export các kiểu public để code bên ngoài chỉ cần `use crate::fsm::*`.

pub mod actors;
pub mod commands;
pub mod orchestrator;
pub mod peripheral;
pub mod phases;
pub mod system_context;
pub mod types;
pub mod utils;

pub use phases::{FaultCode, SystemPhase};
pub use system_context::SystemContext;
pub use types::{PendingCalibrationSample, PendingDose, SharedSensorData};

use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition, EspNvs};
use hydragrow_shared::{
    BasicSystemLogMetadata, ControlMode, LogCategory, LogLevel, MqttCommandIn, SystemLogEvent,
};
use log::info;

use crate::config::SharedConfig;
use crate::pump::PumpController;

use commands::process_mqtt_commands;
use system_context::{NvsSnapshot, TunerState};
use utils::{get_current_time_ms, get_current_time_sec};

pub mod mod_helpers {
    use hydragrow_shared::PumpStatus;

    use crate::fsm::SystemContext;
    use crate::pump::{PumpController, PumpType, WaterDirection};
    use std::sync::mpsc::Sender;

    pub fn turn_off_pump_from_system_ctx(
        ctx: &mut SystemContext,
        pump_name: &str,
        pump_ctrl: &mut PumpController,
    ) {
        let _ = match pump_name {
            "A" | "PUMP_A" => {
                ctx.peripherals.pump_status.pump_a = false;
                pump_ctrl.set_pump_state(PumpType::NutrientA, false)
            }
            "B" | "PUMP_B" => {
                ctx.peripherals.pump_status.pump_b = false;
                pump_ctrl.set_pump_state(PumpType::NutrientB, false)
            }
            "PH_UP" | "PUMP_PH_UP" => {
                ctx.peripherals.pump_status.ph_up = false;
                pump_ctrl.set_pump_state(PumpType::PhUp, false)
            }
            "PH_DOWN" | "PUMP_PH_DOWN" => {
                ctx.peripherals.pump_status.ph_down = false;
                pump_ctrl.set_pump_state(PumpType::PhDown, false)
            }
            "MIST" | "MIST_VALVE" => {
                ctx.peripherals.pump_status.mist_valve = false;
                ctx.peripherals.is_misting_active = false;
                pump_ctrl.set_mist_valve(false)
            }
            "WATER_PUMP" | "WATER_PUMP_IN" | "PUMP_IN" => {
                ctx.peripherals.pump_status.water_pump_in = false;
                pump_ctrl.set_water_pump(WaterDirection::Stop)
            }
            "DRAIN_PUMP" | "WATER_PUMP_OUT" | "PUMP_OUT" => {
                ctx.peripherals.pump_status.water_pump_out = false;
                pump_ctrl.set_water_pump(WaterDirection::Stop)
            }
            _ => Ok(()),
        };
        // SỬA LỖI: Wrap bằng Some()
        ctx.peripherals.pump_status.dosing_pulse_active = Some(false);
        ctx.peripherals.pump_status.dosing_pulse_count = Some(0);
    }

    pub fn stop_all_pumps_from_system_ctx(ctx: &mut SystemContext, pump_ctrl: &mut PumpController) {
        let _ = pump_ctrl.stop_all();
        ctx.peripherals.pump_status = PumpStatus::default();
        ctx.peripherals.is_misting_active = false;
        ctx.peripherals.is_scheduled_mixing_active = false;
        ctx.peripherals.osaka_active = false;
        ctx.peripherals.osaka_pwm = 0;
        ctx.safety.manual_timeouts.clear();
    }

    pub fn reset_faults_from_system_ctx(
        ctx: &mut SystemContext,
        _device_id: &str,
        _tx: &Sender<String>,
    ) {
        ctx.tuner.on_manual_reset();
        ctx.phase = super::SystemPhase::Monitoring;
        ctx.phase_finish_ms = None;
        ctx.dosing.retry_ec = 0;
        ctx.dosing.retry_ph = 0;
        ctx.safety.flush_for_reset();
    }
}

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
    cmd_rx: Receiver<MqttCommandIn>,
    fsm_mqtt_tx: Sender<String>,
    dosing_report_tx: Sender<String>,
    sensor_cmd_tx: Sender<String>,
    current_time_sec: u64,
) {
    let mut new_ctx = SystemContext::default();
    let mut last_reported_state = String::new();

    let mut nvs = EspNvs::new(nvs_partition, "agitech", true).ok();
    let current_time_on_boot = get_current_time_sec();

    // Biến phụ để tracking timeout sensor dựa trên `time: String`
    let mut last_sensor_time_str = String::new();
    let mut sensor_last_update_ms = get_current_time_ms();

    // Khôi phục thời điểm thay nước & bơm định kỳ từ NVS flash
    new_ctx.last_water_change_sec = nvs
        .as_mut()
        .and_then(|f| f.get_u64("last_w_change").unwrap_or(None))
        .unwrap_or_else(|| {
            if let Some(f) = nvs.as_mut() {
                let _ = f.set_u64("last_w_change", current_time_on_boot);
            }
            current_time_on_boot
        });

    new_ctx.peripherals.last_mixing_start_sec = current_time_on_boot;
    if let Some(flash) = nvs.as_mut() {
        if let Ok(Some(raw)) = flash.get_str("runtime_snap", &mut [0; 1024]) {
            if let Ok(snapshot) = serde_json::from_str::<NvsSnapshot>(raw) {
                if snapshot.step_ratio_ec.is_finite() {
                    new_ctx.tuner.adaptive_ec_ratio = snapshot.step_ratio_ec.clamp(0.1, 2.0);
                }
                if snapshot.best_ec_ratio.is_finite() {
                    new_ctx.tuner.best_ec_ratio = snapshot.best_ec_ratio.clamp(0.1, 2.0);
                }
                if snapshot.step_ratio_ph.is_finite() {
                    new_ctx.tuner.adaptive_ph_ratio = snapshot.step_ratio_ph.clamp(0.1, 2.0);
                }
                if snapshot.best_ph_ratio.is_finite() {
                    new_ctx.tuner.best_ph_ratio = snapshot.best_ph_ratio.clamp(0.1, 2.0);
                }
                if snapshot.ema_ec_gain.is_finite() {
                    new_ctx.tuner.gain_learner.ec.ema = snapshot.ema_ec_gain;
                }
                if snapshot.ema_ph_up_gain.is_finite() {
                    new_ctx.tuner.gain_learner.ph_up.ema = snapshot.ema_ph_up_gain;
                }
                if snapshot.ema_ph_down_gain.is_finite() {
                    new_ctx.tuner.gain_learner.ph_down.ema = snapshot.ema_ph_down_gain;
                }
                new_ctx.tuner.gain_learner.ec.sample_count = snapshot.ec_sample_count;
                let ph_up_count = if snapshot.ph_up_sample_count > 0 {
                    snapshot.ph_up_sample_count
                } else {
                    snapshot.ph_sample_count
                };
                let ph_down_count = if snapshot.ph_down_sample_count > 0 {
                    snapshot.ph_down_sample_count
                } else {
                    snapshot.ph_sample_count
                };
                new_ctx.tuner.gain_learner.ph_up.sample_count = ph_up_count;
                new_ctx.tuner.gain_learner.ph_down.sample_count = ph_down_count;
                new_ctx.tuner.gain_learner.ec.recalculate_confidence();
                new_ctx.tuner.gain_learner.ph_up.recalculate_confidence();
                new_ctx.tuner.gain_learner.ph_down.recalculate_confidence();
                if snapshot.ec_variance_baseline.is_finite() && snapshot.ec_variance_baseline >= 0.0
                {
                    new_ctx.tuner.ec_variance_baseline = snapshot.ec_variance_baseline;
                }
                if snapshot.ph_variance_baseline.is_finite() && snapshot.ph_variance_baseline >= 0.0
                {
                    new_ctx.tuner.ph_variance_baseline = snapshot.ph_variance_baseline;
                }
                new_ctx.tuner.state = TunerState::from_u8(snapshot.tuner_state);
                new_ctx.last_water_change_sec = snapshot.last_water_change_sec;
                new_ctx.dosing.retry_ec = snapshot.retry_ec;
                new_ctx.dosing.retry_ph = snapshot.retry_ph;
                new_ctx.dosing_cycle_count = snapshot.dosing_cycle_count;
            }
        }
    }

    info!("🚀 Bắt đầu chạy Máy trạng thái (FSM) Đa luồng Hợp nhất...");

    // Giai đoạn khởi động 3 giây
    let boot_start_ms = get_current_time_ms();
    loop {
        if get_current_time_ms() - boot_start_ms > 3000 {
            new_ctx.phase = SystemPhase::Monitoring;
            break;
        }
        if report_phase_if_changed(&new_ctx.phase, &mut last_reported_state) {
            let _ = fsm_mqtt_tx.send(build_status_msg(&new_ctx, current_time_sec));
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Vòng lặp chính
    loop {
        let config = shared_config.read().unwrap().clone();
        let sensors = shared_sensors.read().unwrap().clone();
        let current_time_ms = get_current_time_ms();
        let current_time_sec = current_time_ms / 1000;

        // Xử lý kiểm tra timeout của sensor (Cập nhật thời điểm khi gói dữ liệu thay đổi)
        if sensors.time != last_sensor_time_str {
            last_sensor_time_str = sensors.time.clone();
            sensor_last_update_ms = current_time_ms;
        }

        let force_sync = process_mqtt_commands(
            &cmd_rx,
            &config,
            &mut pump_ctrl,
            &mut new_ctx,
            current_time_ms,
            &fsm_mqtt_tx,
        );

        // --- Xử lý timeout bơm thủ công (có thể tắt bơm) ---
        let expired: Vec<String> = new_ctx
            .safety
            .manual_timeouts
            .iter()
            .filter(|(_, &t)| current_time_ms >= t)
            .map(|(k, _)| k.clone())
            .collect();
        for pump in expired {
            new_ctx.safety.manual_timeouts.remove(&pump);
            info!("⏱️ HẾT GIỜ (SAFE TIMEOUT): Tự động tắt bơm {}!", pump);
            utils::send_system_log(
                &fsm_mqtt_tx,
                &config.device_id, // Đảm bảo config có chứa device_id
                LogLevel::Warning,
                LogCategory::UserAction,
                "Tự động tắt bơm (Safety Timeout)",
                SystemLogEvent::BasicSystemLog(BasicSystemLogMetadata {
                    source: "fsm_command".to_string(),
                    message: format!(
                        "Bơm {} đã tự động tắt do hết thời gian an toàn của chế độ thủ công.",
                        pump
                    ),
                    skip_reason: None,
                }),
            );
            mod_helpers::turn_off_pump_from_system_ctx(&mut new_ctx, &pump, &mut pump_ctrl);
        }

        // ✅ Cập nhật shared_sensors NGAY LẬP TỨC sau khi xử lý lệnh + timeout
        if let Ok(mut s) = shared_sensors.write() {
            s.pump_status = new_ctx.peripherals.pump_status.clone();
        }

        // ✅ Nếu có force_sync, publish trạng thái ngay lập tức
        if force_sync {
            // Gửi lệnh ép sensor task publish dữ liệu (bao gồm pump_status)
            let _ = sensor_cmd_tx
                .send(r#"{"target":"sensor","action":"force_publish","params":{}}"#.to_string());
            // Đồng thời publish trạng thái FSM để backend nhận ngay
            let _ = fsm_mqtt_tx.send(build_status_msg(&new_ctx, current_time_sec));
            last_reported_state.clear();
            info!("⚡ Đã ép publish trạng thái bơm mới nhất lên App!");
        }

        // --- Phần còn lại giữ nguyên (kiểm tra safety, auto FSM...) ---
        let is_safety_overridden = current_time_ms < new_ctx.safety.safety_override_until;
        if !is_safety_overridden {
            // SỬA LỖI: Sử dụng unwrap_or(false)
            let has_sensor_fault = (config.enable_water_level_sensor
                && sensors.err_water.unwrap_or(false))
                || (config.enable_ec_sensor && sensors.err_ec.unwrap_or(false))
                || (config.enable_ph_sensor && sensors.err_ph.unwrap_or(false))
                || (config.enable_temp_sensor && sensors.err_temp.unwrap_or(false));

            if !has_sensor_fault {
                // SỬA LỖI: Thêm tham số sensor_last_update_ms
                orchestrator::tick(
                    current_time_ms,
                    &config,
                    &sensors,
                    sensor_last_update_ms,
                    &mut new_ctx,
                    &mut pump_ctrl,
                    &mut nvs,
                    &dosing_report_tx,
                    &fsm_mqtt_tx,
                );
            }
        }

        // --- Cập nhật chế độ đọc cảm biến liên tục khi đang bơm nước ---
        let needs_continuous = matches!(
            new_ctx.phase,
            SystemPhase::WaterRefilling | SystemPhase::WaterDraining
        );
        if needs_continuous != new_ctx.peripherals.last_continuous_level {
            let _ = sensor_cmd_tx.send(format!(
                r#"{{"target":"sensor","action":"set_continuous","params":{{"state":{}}}}}"#,
                needs_continuous
            ));
            new_ctx.peripherals.last_continuous_level = needs_continuous;
        }

        // --- Đồng bộ pump_status ra shared_sensor (lần cuối sau khi FSM chạy) ---
        if let Ok(mut s) = shared_sensors.write() {
            s.pump_status = new_ctx.peripherals.pump_status.clone();
        }

        // --- Publish trạng thái nếu state FSM thay đổi ---
        let state_changed = report_phase_if_changed(&new_ctx.phase, &mut last_reported_state);
        if state_changed {
            let _ = fsm_mqtt_tx.send(build_status_msg(&new_ctx, current_time_sec));
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

// ---------------------------------------------------------------------------
// Helpers nhỏ dùng trong vòng lặp
// ---------------------------------------------------------------------------

fn report_phase_if_changed(current_phase: &SystemPhase, last_reported_state: &mut String) -> bool {
    let s = match current_phase {
        SystemPhase::Fault(code) => format!("Fault:{}", code.as_str()),
        SystemPhase::EmergencyStop(reason) => format!("EmergencyStop:{}", reason),
        _ => current_phase.as_str().to_string(),
    };
    if s != *last_reported_state {
        info!("📡 Trạng thái FSM: [{}]", s);
        *last_reported_state = s;
        true
    } else {
        false
    }
}

// Nhớ truyền thêm tham số now_sec vào hàm này
fn build_status_msg(ctx: &SystemContext, now_sec: u64) -> String {
    // Hàm closure tính tổng số ml đã bơm trong 1 giờ qua
    let sum_ml = |pump_name: &str| -> f32 {
        ctx.safety
            .hourly_doses()
            .get(pump_name)
            .map(|hist| {
                hist.iter()
                    .filter(|(ts, _)| now_sec.saturating_sub(*ts) <= 3600)
                    .map(|(_, ml)| ml)
                    .sum()
            })
            .unwrap_or(0.0)
    };

    // Đếm số lần bơm nước/xả nước trong 1 giờ
    let refill_count = ctx
        .safety
        .refill_history()
        .iter()
        .filter(|ts| now_sec.saturating_sub(**ts) <= 3600)
        .count();
    let drain_count = ctx
        .safety
        .drain_history()
        .iter()
        .filter(|ts| now_sec.saturating_sub(**ts) <= 3600)
        .count();

    serde_json::json!({
        "online": true,
        "current_state": match &ctx.phase {
            SystemPhase::Fault(code) => format!("Fault:{}", code.as_str()),
            SystemPhase::EmergencyStop(reason) => format!("EmergencyStop:{}", reason),
            _ => ctx.phase.as_str().to_string(),
        },
        "pump_status": ctx.peripherals.pump_status,
        "budgets": {
            "ec_ml": sum_ml("NutrientA") + sum_ml("NutrientB"),
            "ph_ml": sum_ml("PhUp") + sum_ml("PhDown"),
            "refill_count": refill_count,
            "drain_count": drain_count
        }
    })
    .to_string()
}

