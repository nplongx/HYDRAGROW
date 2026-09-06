// src/runtime/fsm_loop.rs
//! FSM Control Loop Thread Runtime.

use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};
use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::MqttCommandIn;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use tracing::info;

use crate::config::SharedConfig;
use crate::hw::mqtt_client::get_uptime_ms; // SỬA: Import đúng module chứa get_uptime_ms
use crate::hw::pump_controller::PumpController;
use crate::hw::NvsStore;
use crate::runtime::command_handler::{build_stop_pump_events, process_mqtt_commands};
use crate::runtime::dispatcher::{DispatchContext, EventDispatcher};
use crate::runtime::health::build_status_msg;
use crate::runtime::observers::ObserverSet;
use hydragrow_controller_core::core::fsm::context::SystemContext;
use hydragrow_controller_core::core::fsm::orchestrator;
use hydragrow_controller_core::core::fsm::types::SharedSensorData;
use hydragrow_controller_core::utils::{get_current_time_ms, read_or_recover, write_or_recover};

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
    int_rx: Receiver<()>, // <-- Nhận channel ngắt
    _current_time_sec: u64,
) {
    let mut ctx = SystemContext::default();
    let mut nvs_store = NvsStore::new(nvs_partition.clone());
    nvs_store.load_runtime_snapshot(&mut ctx);
    // Safety budget (hourly_doses, refill_history, drain_history) KHÔNG được restore sau reboot.
    // Đây là thiết kế có chủ ý: uptime-based timestamps không tương thích với wall-clock của NVS.
    // Budget sẽ tích lũy lại từ đầu sau mỗi reboot.
    info!("ℹ️ [BOOT] Safety budget được reset (uptime-based, không restore từ NVS).");

    ctx.phase = SystemPhase::Monitoring;

    let mut nvs = EspNvs::new(nvs_partition, "agitech", true).ok();
    let mut observer_set = ObserverSet::new();
    info!("  [RUNTIME] FSM Loop đã chạy...");

    let mut last_reported_state = String::new();
    let mut sensor_last_update_ms = get_uptime_ms();
    let mut last_controller_recieved_ms = None;
    let mut last_tank_alert = crate::hw::pcf857x::TankAlert::default();
    let mut last_int_handled_uptime_ms: u64 = 0;
    const INT_DEBOUNCE_MS: u64 = 80;

    loop {
        let config = read_or_recover(&shared_config).effective_config;
        let sensors = read_or_recover(&shared_sensors);

        let current_wall_time_ms = get_current_time_ms();
        let current_uptime_ms = get_uptime_ms();

        if sensors.controller_received_ms != last_controller_recieved_ms {
            last_controller_recieved_ms = sensors.controller_received_ms;
            sensor_last_update_ms = current_uptime_ms;
        }

        // --- 0. XỬ LÝ NGẮT TỪ EXPANDER PIN (CẢNH BÁO MỨC DUNG DỊCH) ---
        if int_rx.try_recv().is_ok() {
            while int_rx.try_recv().is_ok() {}

            if current_uptime_ms.saturating_sub(last_int_handled_uptime_ms) >= INT_DEBOUNCE_MS {
                last_int_handled_uptime_ms = current_uptime_ms;
                match pump_ctrl.check_tank_alert() {
                    Ok(alert) => {
                        if alert != last_tank_alert {
                            last_tank_alert = alert;
                            info!("  [ALERT] Trạng thái bình dung dịch thay đổi: {:?}", alert);

                            let alert_payload = serde_json::json!({
                                "type": "system_alert",
                                "device_id": config.device_id,
                                "level": if alert.has_alert() { "Warning" } else { "Info" },
                                "category": "Dosing",
                                "title": "Cảnh báo mức dung dịch bình chứa",
                                "details": {
                                    "tank_a_low": alert.tank_a_low,
                                    "tank_b_low": alert.tank_b_low,
                                    "tank_ph_down_low": alert.tank_ph_down_low,
                                    "tank_ph_up_low": alert.tank_ph_up_low,
                                },
                                "timestamp_ms": current_wall_time_ms
                            });

                            // Gửi thẳng vào kênh fsm_mqtt_tx để đẩy lên topic fsm_events qua health_loop
                            let _ = fsm_mqtt_tx.send(alert_payload.to_string());
                        }
                    }
                    Err(e) => tracing::warn!("  [ALERT] Không thể đọc I2C Expander: {:?}", e),
                }
            }
        }

        // 1. Parse lệnh MQTT
        let (mut cmd_delta, cmd_events) = process_mqtt_commands(
            &cmd_rx,
            &config,
            &ctx,
            current_uptime_ms,
            current_wall_time_ms,
            &fsm_mqtt_tx,
        );
        ctx.apply_delta(&mut cmd_delta);
        if !cmd_events.is_empty() {
            let fault = {
                let mut dc = DispatchContext {
                    pumps: &mut pump_ctrl,
                    nvs: &mut nvs,
                    mqtt_tx: &fsm_mqtt_tx,
                    dosing_report_tx: &dosing_report_tx,
                    sensor_cmd_tx: &sensor_cmd_tx,
                    ctx: &ctx,
                    now_sec: current_wall_time_ms / 1000,
                    device_id: &config.device_id,
                    config: &config,
                    observers: &mut observer_set,
                };
                EventDispatcher::dispatch(cmd_events, &mut dc)
            };
            if let Some(fault) = fault {
                apply_dispatch_fault(
                    fault,
                    &mut ctx,
                    &mut pump_ctrl,
                    &mut nvs,
                    &fsm_mqtt_tx,
                    &dosing_report_tx,
                    &sensor_cmd_tx,
                    current_wall_time_ms / 1000,
                    &config.device_id,
                    &config,
                    &mut observer_set,
                );
            }

            let _ = fsm_mqtt_tx.send(build_status_msg(
                &ctx,
                current_wall_time_ms / 1000,
                current_uptime_ms / 1000,
            ));
        }

        let expired_pumps: Vec<String> = ctx
            .safety
            .manual_timeouts
            .iter()
            .filter(|(_, &finish_ms)| current_uptime_ms >= finish_ms)
            .map(|(name, _)| name.clone())
            .collect();

        for pump_name in expired_pumps {
            info!(
                "⏱️ [MANUAL TIMEOUT] Hết thời gian hẹn giờ cho {}, tự động ngắt!",
                pump_name
            );
            let mut timeout_delta = hydragrow_controller_core::core::fsm::ContextDelta::default();

            // Tạo các event ngắt rơ-le / PWM tương ứng cho thiết bị
            let stop_events = build_stop_pump_events(&pump_name, &mut timeout_delta, &ctx);

            // Xóa thiết bị khỏi danh sách chờ timeout
            timeout_delta.manual_pump_timeout_clear = Some(pump_name);
            ctx.apply_delta(&mut timeout_delta);

            if !stop_events.is_empty() {
                let fault = {
                    let mut dc = DispatchContext {
                        pumps: &mut pump_ctrl,
                        nvs: &mut nvs,
                        mqtt_tx: &fsm_mqtt_tx,
                        dosing_report_tx: &dosing_report_tx,
                        sensor_cmd_tx: &sensor_cmd_tx,
                        ctx: &ctx,
                        now_sec: current_wall_time_ms / 1000,
                        device_id: &config.device_id,
                        config: &config,
                        observers: &mut observer_set,
                    };
                    EventDispatcher::dispatch(stop_events, &mut dc)
                };
                if let Some(fault) = fault {
                    apply_dispatch_fault(
                        fault,
                        &mut ctx,
                        &mut pump_ctrl,
                        &mut nvs,
                        &fsm_mqtt_tx,
                        &dosing_report_tx,
                        &sensor_cmd_tx,
                        current_wall_time_ms / 1000,
                        &config.device_id,
                        &config,
                        &mut observer_set,
                    );
                }

                // Đồng bộ ngay lập tức trạng thái Tắt lên MQTT để UI Web/App cập nhật tức thì
                let _ = fsm_mqtt_tx.send(build_status_msg(
                    &ctx,
                    current_wall_time_ms / 1000,
                    current_uptime_ms / 1000,
                ));
            }
        }

        // 2. Chạy Recipe Engine trước FSM để stage override có hiệu lực trong tick hiện tại
        let mut recipe_result;
        let updated_config;
        {
            let mut state = write_or_recover(&shared_config);

            // Cho phép Recipe Engine can thiệp thẳng vào effective_config của hệ thống
            recipe_result =
                hydragrow_controller_core::core::fsm::recipe_manager::tick_recipe_engine(
                    &mut state.effective_config,
                    &ctx,
                    current_wall_time_ms / 1000,
                );

            // Cập nhật stage override hoặc khôi phục base config khi hoàn thành recipe
            state.apply_recipe_tick_result(&recipe_result);

            // Lấy ra bản clone MỚI NHẤT (đã được override) để truyền xuống Orchestrator
            updated_config = state.effective_config.clone();
        }

        ctx.apply_delta(&mut recipe_result.delta);

        if !recipe_result.events.is_empty() {
            let fault = {
                let mut dc = DispatchContext {
                    pumps: &mut pump_ctrl,
                    nvs: &mut nvs,
                    mqtt_tx: &fsm_mqtt_tx,
                    dosing_report_tx: &dosing_report_tx,
                    sensor_cmd_tx: &sensor_cmd_tx,
                    ctx: &ctx,
                    now_sec: current_wall_time_ms / 1000,
                    device_id: &updated_config.device_id, // Sử dụng updated_config
                    config: &updated_config,              // Sử dụng updated_config
                    observers: &mut observer_set,
                };
                EventDispatcher::dispatch(recipe_result.events, &mut dc)
            };
            if let Some(fault) = fault {
                apply_dispatch_fault(
                    fault,
                    &mut ctx,
                    &mut pump_ctrl,
                    &mut nvs,
                    &fsm_mqtt_tx,
                    &dosing_report_tx,
                    &sensor_cmd_tx,
                    current_wall_time_ms / 1000,
                    &updated_config.device_id,
                    &updated_config,
                    &mut observer_set,
                );
            }
        }

        // 3. Chạy FSM Tick Decision Engine
        let mut tick_result = orchestrator::tick(
            current_wall_time_ms,
            current_uptime_ms,
            &updated_config,
            &sensors,
            sensor_last_update_ms,
            &mut ctx,
        );
        ctx.apply_delta(&mut tick_result.delta);

        // 4. Thực thi Side Effects
        if !tick_result.events.is_empty() {
            let fault = {
                let mut dc = DispatchContext {
                    pumps: &mut pump_ctrl,
                    nvs: &mut nvs,
                    mqtt_tx: &fsm_mqtt_tx,
                    dosing_report_tx: &dosing_report_tx,
                    sensor_cmd_tx: &sensor_cmd_tx,
                    ctx: &ctx,
                    now_sec: current_wall_time_ms / 1000,
                    device_id: &updated_config.device_id,
                    config: &updated_config,
                    observers: &mut observer_set,
                };
                EventDispatcher::dispatch(tick_result.events, &mut dc)
            };
            if let Some(fault) = fault {
                apply_dispatch_fault(
                    fault,
                    &mut ctx,
                    &mut pump_ctrl,
                    &mut nvs,
                    &fsm_mqtt_tx,
                    &dosing_report_tx,
                    &sensor_cmd_tx,
                    current_wall_time_ms / 1000,
                    &updated_config.device_id,
                    &updated_config,
                    &mut observer_set,
                );
            } else if let Some(tx) = &tick_result.safety_transaction {
                ctx.commit_safety_transaction(tx);
            }
        } else if let Some(tx) = &tick_result.safety_transaction {
            ctx.commit_safety_transaction(tx);
        }

        // 5. Báo trạng thái chuyển Phase
        let state_str = ctx.phase.as_str().to_string();
        if state_str != last_reported_state {
            info!("  [FSM] Phase thay đổi: [{}]", state_str);
            last_reported_state = state_str;
            let _ = fsm_mqtt_tx.send(build_status_msg(
                &ctx,
                current_wall_time_ms / 1000,
                current_uptime_ms / 1000,
            ));
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_dispatch_fault(
    fault: hydragrow_shared::fsm::FaultCode,
    ctx: &mut SystemContext,
    pump_ctrl: &mut PumpController<'_>,
    nvs: &mut Option<esp_idf_svc::nvs::EspDefaultNvs>,
    mqtt_tx: &Sender<String>,
    dosing_report_tx: &Sender<String>,
    sensor_cmd_tx: &Sender<String>,
    now_sec: u64,
    device_id: &str,
    config: &hydragrow_shared::ControllerConfig,
    observers: &mut ObserverSet,
) {
    tracing::error!(
        "🚨 [DISPATCHER] Physical actuator failure: {:?}. Forcing Fault phase and physical ALL-OFF!",
        fault
    );
    let mut tick_result = hydragrow_controller_core::core::fsm::TickResult::default();
    tick_result.delta.phase = Some(hydragrow_shared::fsm::SystemPhase::Fault(fault));
    orchestrator::fault_all_outputs_off(&mut tick_result);
    ctx.apply_delta(&mut tick_result.delta);

    let mut dc = DispatchContext {
        pumps: pump_ctrl,
        nvs,
        mqtt_tx,
        dosing_report_tx,
        sensor_cmd_tx,
        ctx: &*ctx,
        now_sec,
        device_id,
        config,
        observers,
    };
    if let Some(shutdown_fault) =
        EventDispatcher::dispatch_best_effort_all_off(tick_result.events, &mut dc)
    {
        tracing::error!(
            "🚨 [DISPATCHER] Secondary actuator failure during emergency ALL-OFF: {:?} (primary fault latched: {:?})",
            shutdown_fault,
            fault
        );
    }
}
