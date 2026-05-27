// fsm/mod.rs – điểm vào chính của module FSM
//
// Re-export các kiểu public để code bên ngoài chỉ cần `use crate::fsm::*`.

pub mod actors;
pub mod commands;
pub mod events;
pub mod matrix;
pub mod optimizer;
pub mod orchestrator;
pub mod peripheral;
pub mod system_context;
pub mod types;
pub mod utils;
pub use events::OrchestratorEvent;
pub mod tick_result;
use hydragrow_shared::fsm::{FsmBudgets, FsmSnapshot, SystemPhase};
use hydragrow_shared::log::{BasicSystemLogMetadata, LogCategory, LogLevel, SystemLogEvent};
use hydragrow_shared::telemetry::transition::TransitionReason;
use hydragrow_shared::MqttCommandIn;
pub use tick_result::ContextDelta;
pub mod dispatcher;
pub mod observer_set;
pub mod observers;
pub mod phase_impls;
pub mod phase_tick;
pub mod solver;

pub use system_context::SystemContext;
pub use types::SharedSensorData;

use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};
use tracing::{debug, info, warn};

use crate::config::SharedConfig;
use crate::fsm::matrix::InteractionMatrix;
use crate::pump::PumpController;

use commands::process_mqtt_commands;
use system_context::{NvsSnapshot, TunerState};
use utils::{get_current_time_ms, get_current_time_sec};

const INTERACTION_MATRIX_MIN: f32 = -10.0;
const INTERACTION_MATRIX_MAX: f32 = 10.0;

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
    _current_time_sec: u64,
) {
    let mut new_ctx = SystemContext::default();
    debug!("FSM tick: phase={:?}", new_ctx.phase.as_str());
    let mut last_reported_state = String::new();

    let mut nvs = EspNvs::new(nvs_partition, "agitech", true).ok();
    let current_time_on_boot = get_current_time_sec();

    let mut last_sensor_time_str = String::new();
    let mut sensor_last_update_ms = get_current_time_ms();

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
        if let Ok(Some(raw)) = flash.get_str("runtime_snap", &mut [0; 2048]) {
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

                if snapshot.ec_variance_baseline.is_finite() && snapshot.ec_variance_baseline >= 0.0
                {
                    new_ctx.tuner.ec_variance_baseline = snapshot.ec_variance_baseline;
                }
                if snapshot.ph_variance_baseline.is_finite() && snapshot.ph_variance_baseline >= 0.0
                {
                    new_ctx.tuner.ph_variance_baseline = snapshot.ph_variance_baseline;
                }
                if let Some(interaction_matrix) = snapshot.interaction_matrix {
                    let matrix_values_are_valid = interaction_matrix.iter().all(|value| {
                        value.is_finite()
                            && (*value >= INTERACTION_MATRIX_MIN)
                            && (*value <= INTERACTION_MATRIX_MAX)
                    });

                    let matrix_diagonal_gain_is_valid =
                        interaction_matrix[0] > 0.0 && interaction_matrix[6] > 0.0;
                    let matrix_is_valid = matrix_values_are_valid && matrix_diagonal_gain_is_valid;

                    if matrix_is_valid {
                        new_ctx.tuner.interaction_matrix =
                            InteractionMatrix::from_flat(interaction_matrix);
                        new_ctx.tuner.matrix_update_count = snapshot.matrix_update_count;
                        new_ctx.tuner.matrix_is_warm = if snapshot.matrix_is_warm {
                            true
                        } else {
                            snapshot.matrix_update_count >= 10
                        };
                    } else {
                        warn!(
                            "⚠️ Bỏ qua interaction_matrix từ snapshot do invalid. \
values_valid={}, diagonal_valid={}, m00={}, m12={}, update_count={}, snapshot_warm={}",
                            matrix_values_are_valid,
                            matrix_diagonal_gain_is_valid,
                            interaction_matrix[0],
                            interaction_matrix[6],
                            snapshot.matrix_update_count,
                            snapshot.matrix_is_warm
                        );
                    }
                }
                new_ctx.tuner.matrix_update_count = snapshot.matrix_update_count;
                new_ctx.tuner.matrix_is_warm =
                    snapshot.matrix_is_warm || new_ctx.tuner.matrix_update_count >= 10;
                new_ctx.tuner.state = TunerState::from_u8(snapshot.tuner_state);
                new_ctx.last_water_change_sec = snapshot.last_water_change_sec;
                new_ctx.dosing.retry_ec = snapshot.retry_ec;
                new_ctx.dosing.retry_ph = snapshot.retry_ph;
                new_ctx.dosing_cycle_count = snapshot.dosing_cycle_count;
            }
        }
    }

    if !new_ctx.tuner.matrix_is_warm {
        let config = shared_config.read().unwrap().clone();
        if config.ec_gain_per_ml > 0.0 && config.ph_shift_up_per_ml > 0.0 {
            new_ctx.tuner.interaction_matrix = InteractionMatrix::from_scalar(
                config.ec_gain_per_ml,
                config.ph_shift_up_per_ml,
                config.ph_shift_down_per_ml,
                0.05,
                0.04,
            );
            info!(
                "🧊 Cold matrix boot: re-seeded interaction matrix from shared_config (ec_gain_per_ml={}, ph_shift_up_per_ml={})",
                config.ec_gain_per_ml,
                config.ph_shift_up_per_ml
            );
        }
    } else {
        info!("🔥 Warm restore: keeping restored interaction matrix from NVS snapshot");
    }

    info!("🚀 Bắt đầu chạy Máy trạng thái (FSM) Đa luồng Hợp nhất...");

    // Giai đoạn khởi động 3 giây
    let boot_start_ms = get_current_time_ms();
    loop {
        let current_time_sec = get_current_time_sec();
        if get_current_time_ms() - boot_start_ms > 3000 {
            new_ctx.phase = SystemPhase::Monitoring;
            break;
        }
        if report_phase_if_changed(&new_ctx.phase, &mut last_reported_state) {
            let _ = fsm_mqtt_tx.send(build_status_msg(&new_ctx, current_time_sec));
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let mut observer_set = crate::fsm::observer_set::ObserverSet::new();

    // Vòng lặp chính
    loop {
        let config = shared_config.read().unwrap().clone();
        let sensors = shared_sensors.read().unwrap().clone();
        let current_time_ms = get_current_time_ms();
        let current_time_sec = current_time_ms / 1000;

        if sensors.time != last_sensor_time_str {
            last_sensor_time_str = sensors.time.clone();
            sensor_last_update_ms = current_time_ms;
        }

        let (mut cmd_delta, cmd_events) =
            process_mqtt_commands(&cmd_rx, &config, &new_ctx, current_time_ms, &fsm_mqtt_tx);
        let force_sync = cmd_delta.phase.is_some() || !cmd_events.is_empty();

        new_ctx.apply_delta(&mut cmd_delta);

        // Phát tán các command events phần cứng ngay lập tức để đáp ứng thời gian thực (Manual Mode)
        if !cmd_events.is_empty() {
            let mut manual_dc = crate::fsm::dispatcher::DispatchContext {
                pumps: &mut pump_ctrl,
                nvs: &mut nvs,
                mqtt_tx: &fsm_mqtt_tx,
                dosing_report_tx: &dosing_report_tx,
                sensor_cmd_tx: &sensor_cmd_tx,
                ctx: &new_ctx,
                now_sec: current_time_sec,
                device_id: &config.device_id,
                config: &config,
                observers: &mut observer_set,
            };
            crate::fsm::dispatcher::EventDispatcher::dispatch(cmd_events, &mut manual_dc);
        }

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
                &config.device_id,
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
                    cycle_id: None,
                }),
            );

            let mut delta = ContextDelta::default();
            let timeout_events = crate::fsm::commands::build_stop_pump_events(&pump, &mut delta);
            new_ctx.apply_delta(&mut delta);

            // Dispatch phần cứng ngay lập tức — giống manual command handling ở trên
            if !timeout_events.is_empty() {
                let mut timeout_dc = crate::fsm::dispatcher::DispatchContext {
                    pumps: &mut pump_ctrl,
                    nvs: &mut nvs,
                    mqtt_tx: &fsm_mqtt_tx,
                    dosing_report_tx: &dosing_report_tx,
                    sensor_cmd_tx: &sensor_cmd_tx,
                    ctx: &new_ctx,
                    now_sec: current_time_sec,
                    device_id: &config.device_id,
                    config: &config,
                    observers: &mut observer_set,
                };
                crate::fsm::dispatcher::EventDispatcher::dispatch(timeout_events, &mut timeout_dc);
            }
        }

        if let Ok(mut s) = shared_sensors.write() {
            s.pump_status = new_ctx.peripherals.pump_status.clone();
        }

        if force_sync {
            let mut sync_dc = crate::fsm::dispatcher::DispatchContext {
                pumps: &mut pump_ctrl,
                nvs: &mut nvs,
                mqtt_tx: &fsm_mqtt_tx,
                dosing_report_tx: &dosing_report_tx,
                sensor_cmd_tx: &sensor_cmd_tx,
                ctx: &new_ctx,
                now_sec: current_time_sec,
                device_id: &config.device_id,
                config: &config,
                observers: &mut observer_set,
            };
            crate::fsm::dispatcher::EventDispatcher::dispatch(
                vec![OrchestratorEvent::RequestSensorForcePublish],
                &mut sync_dc,
            );
            let _ = fsm_mqtt_tx.send(build_status_msg(&new_ctx, current_time_sec));
            last_reported_state.clear();
            info!("⚡ Đã ép publish trạng thái bơm mới nhất lên App!");
        }

        let is_safety_overridden = current_time_ms < new_ctx.safety.safety_override_until;
        if !is_safety_overridden {
            let has_sensor_fault = (config.enable_water_level_sensor
                && sensors.err_water.unwrap_or(false))
                || (config.enable_ec_sensor && sensors.err_ec.unwrap_or(false))
                || (config.enable_ph_sensor && sensors.err_ph.unwrap_or(false))
                || (config.enable_temp_sensor && sensors.err_temp.unwrap_or(false));

            if !has_sensor_fault {
                let mut tick_result = orchestrator::tick(
                    current_time_ms,
                    &config,
                    &sensors,
                    sensor_last_update_ms,
                    &mut new_ctx,
                );

                // Áp dụng những thay đổi state vào SystemContext thông qua ContextDelta
                new_ctx.apply_delta(&mut tick_result.delta);

                let dispatch_events =
                    build_tick_dispatch_events(tick_result, &new_ctx, current_time_ms);
                if !dispatch_events.is_empty() {
                    let mut dc = crate::fsm::dispatcher::DispatchContext {
                        pumps: &mut pump_ctrl,
                        nvs: &mut nvs,
                        mqtt_tx: &fsm_mqtt_tx,
                        dosing_report_tx: &dosing_report_tx,
                        sensor_cmd_tx: &sensor_cmd_tx,
                        ctx: &new_ctx,
                        now_sec: current_time_sec,
                        device_id: &config.device_id,
                        config: &config,
                        observers: &mut observer_set,
                    };
                    crate::fsm::dispatcher::EventDispatcher::dispatch(dispatch_events, &mut dc);
                }
            }
        }

        let needs_continuous = matches!(
            new_ctx.phase,
            SystemPhase::WaterRefilling | SystemPhase::WaterDraining
        );
        if needs_continuous != new_ctx.peripherals.last_continuous_level {
            let mut sensor_mode_dc = crate::fsm::dispatcher::DispatchContext {
                pumps: &mut pump_ctrl,
                nvs: &mut nvs,
                mqtt_tx: &fsm_mqtt_tx,
                dosing_report_tx: &dosing_report_tx,
                sensor_cmd_tx: &sensor_cmd_tx,
                ctx: &new_ctx,
                now_sec: current_time_sec,
                device_id: &config.device_id,
                config: &config,
                observers: &mut observer_set,
            };
            crate::fsm::dispatcher::EventDispatcher::dispatch(
                vec![OrchestratorEvent::SetSensorContinuousMode {
                    enabled: needs_continuous,
                }],
                &mut sensor_mode_dc,
            );
            new_ctx.peripherals.last_continuous_level = needs_continuous;
        }

        if let Ok(mut s) = shared_sensors.write() {
            s.pump_status = new_ctx.peripherals.pump_status.clone();
        }

        let state_changed = report_phase_if_changed(&new_ctx.phase, &mut last_reported_state);
        if state_changed {
            let _ = fsm_mqtt_tx.send(build_status_msg(&new_ctx, current_time_sec));
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

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

fn build_tick_dispatch_events(
    tick_result: tick_result::TickResult,
    ctx: &SystemContext,
    current_time_ms: u64,
) -> Vec<OrchestratorEvent> {
    let mut events = tick_result.events;

    if let Some(prev_phase) = tick_result.delta.previous_phase {
        let duration_ms = tick_result
            .delta
            .phase_start_before
            .map(|start| current_time_ms.saturating_sub(start));

        let reason =
            infer_transition_reason(Some(&prev_phase), &ctx.phase, ctx, duration_ms.unwrap_or(0));

        events.push(OrchestratorEvent::PublishFsmTransition {
            from_phase: prev_phase,
            to_phase: ctx.phase.clone(),
            reason,
            phase_duration_ms: duration_ms,
        });
    }

    events
}

fn build_status_msg(ctx: &SystemContext, now_sec: u64) -> String {
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

    // Populate log_drop_count from the global atomic counter
    let mut diagnostics_snapshot = ctx.diagnostic.clone();
    diagnostics_snapshot.log_drop_count = crate::fsm::utils::get_log_drop_count();

    let payload = FsmSnapshot {
        online: true,
        current_phase: ctx.phase.clone(),
        previous_phase: ctx.previous_phase.clone(),
        pump_status: ctx.peripherals.pump_status.clone(),
        budgets: FsmBudgets {
            ec_ml: sum_ml("NutrientA") + sum_ml("NutrientB"),
            ph_ml: sum_ml("PhUp") + sum_ml("PhDown"),
            refill_count: refill_count as u32,
            drain_count: drain_count as u32,
        },
        diagnostics: Some(diagnostics_snapshot),
    };

    serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
}

fn infer_transition_reason(
    from: Option<&SystemPhase>,
    to: &SystemPhase,
    ctx: &SystemContext,
    duration_rs: u64,
) -> TransitionReason {
    use hydragrow_shared::fsm::FaultCode;
    use hydragrow_shared::telemetry::transition::TransitionReason;

    match (from, to) {
        // --- 1. Chuỗi Khởi động ---
        (Some(SystemPhase::Booting), SystemPhase::Monitoring) => TransitionReason::BootComplete,

        // --- 2. Chuỗi Châm phân & Hòa trộn (Dosing Pipeline) ---
        (Some(SystemPhase::Monitoring), SystemPhase::MimoDosing) => TransitionReason::Manual {
            description: "Bắt đầu châm phân".to_string(),
        },

        (Some(SystemPhase::MimoDosing), SystemPhase::ActiveMixing) => {
            let cycle = ctx.dosing.cycle_ctx.as_ref();
            TransitionReason::DosingComplete {
                dose_a_ml: cycle.map_or(0.0, |c| c.dose_a_delivered_ml),
                dose_b_ml: cycle.map_or(0.0, |c| c.dose_b_delivered_ml),
                ph_up_ml: cycle.map_or(0.0, |c| c.ph_up_delivered_ml),
                ph_down_ml: cycle.map_or(0.0, |c| c.ph_down_delivered_ml),
            }
        }

        (Some(SystemPhase::ActiveMixing), SystemPhase::Stabilizing) => {
            TransitionReason::MixingComplete {
                actual_mixing_ms: duration_rs,
            }
        }

        (Some(SystemPhase::Stabilizing), SystemPhase::Cooldown) => {
            TransitionReason::StabilizingComplete {
                final_ec: 0.0,
                final_ph: 0.0,
                actual_stabilize_ms: duration_rs,
            }
        }

        (Some(SystemPhase::Cooldown), SystemPhase::Monitoring) => TransitionReason::CooldownExpired,

        // --- 3. Chuỗi Bơm/Xả Nước (Water Hydraulics) ---
        (Some(SystemPhase::Monitoring), SystemPhase::WaterRefilling) => TransitionReason::Manual {
            description: "Bắt đầu cấp nước".to_string(),
        },

        (Some(SystemPhase::WaterRefilling), SystemPhase::Monitoring)
        | (Some(SystemPhase::WaterRefilling), SystemPhase::ActiveMixing) => {
            TransitionReason::WaterRefillComplete {
                success: true,
                duration_sec: duration_rs / 1000,
                final_level: 0.0,
            }
        }

        (Some(SystemPhase::Monitoring), SystemPhase::WaterDraining) => TransitionReason::Manual {
            description: "Bắt đầu xả nước".to_string(),
        },

        (Some(SystemPhase::WaterDraining), SystemPhase::Monitoring) => {
            TransitionReason::WaterDrainComplete {
                success: true,
                duration_sec: duration_rs / 1000,
                final_level: 0.0,
            }
        }

        // --- 4. Chuỗi Hiệu chuẩn Cảm biến (Sensor Calibration) ---
        (Some(SystemPhase::Monitoring), SystemPhase::SensorCalibration) => {
            TransitionReason::EnterCalibration
        }
        (Some(SystemPhase::SensorCalibration), SystemPhase::Monitoring) => {
            TransitionReason::ExitCalibration
        }

        // --- 5. Chế độ Thủ công (Manual Mode) ---
        (_, SystemPhase::ManualMode) => TransitionReason::Manual {
            description: "Vào chế độ thủ công".to_string(),
        },
        (Some(SystemPhase::ManualMode), SystemPhase::Monitoring) => TransitionReason::Manual {
            description: "Thoát chế độ thủ công".to_string(),
        },

        // --- 6. Chuỗi Lỗi & Khẩn cấp (Faults & Emergency) ---
        (_, SystemPhase::Fault(code)) => {
            // Map linh hoạt số lần thất bại liên tiếp (retry/failures) tùy theo FaultCode
            let consecutive_failures = match code {
                FaultCode::EcDosingFailed => ctx.dosing.retry_ec as u32,
                FaultCode::PhDosingFailed => ctx.dosing.retry_ph as u32,
                FaultCode::WaterRefillFailed | FaultCode::TooManyRefills => ctx.water.retry_refill,
                // Các lỗi mang tính tức thời hoặc an toàn (MaxHourlyDose, SensorTimeout, v.v.)
                // mặc định là 1 lần vì chúng kích hoạt Fault ngay lập tức.
                _ => 1,
            };

            TransitionReason::FaultDetected {
                fault_code: code.clone(),
                consecutive_failures,
            }
        }

        (_, SystemPhase::EmergencyStop(reason)) => TransitionReason::Manual {
            description: format!("Dừng khẩn cấp: {}", reason),
        },

        (Some(SystemPhase::Fault(_)), SystemPhase::Monitoring)
        | (Some(SystemPhase::EmergencyStop(_)), SystemPhase::Monitoring) => {
            TransitionReason::FaultReset
        }

        // --- 7. Fallback cho các ngoại lệ chưa lường trước ---
        _ => TransitionReason::Manual {
            description: format!(
                "Chuyển trạng thái: {} -> {}",
                from.map(|p| p.as_str()).unwrap_or("None"),
                to.as_str()
            ),
        },
    }
}
