// src/runtime/fsm_loop.rs
//! FSM Control Loop Thread Runtime.

use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};
use hydragrow_shared::fsm::SystemPhase;
use hydragrow_shared::MqttCommandIn;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use tracing::{debug, info};

use crate::config::SharedConfig;
use crate::core::fsm::context::SystemContext;
use crate::core::fsm::orchestrator;
use crate::core::fsm::types::SharedSensorData;
use crate::hw::mqtt_client::get_uptime_ms; // SỬA: Import đúng module chứa get_uptime_ms
use crate::hw::pump_controller::PumpController;
use crate::runtime::command_handler::process_mqtt_commands;
use crate::runtime::dispatcher::{DispatchContext, EventDispatcher};
use crate::runtime::health::build_status_msg;
use crate::runtime::observers::ObserverSet;
use crate::utils::{get_current_time_ms, get_current_time_sec};

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
    let mut ctx = SystemContext::default();
    let mut nvs = EspNvs::new(nvs_partition, "agitech", true).ok();
    let mut observer_set = ObserverSet::new();

    info!("🚀 [RUNTIME] FSM Loop đã khởi chạy...");

    let mut last_reported_state = String::new();
    let mut sensor_last_update_ms = get_current_time_ms();

    let mut last_controller_recieved_ms = None;

    loop {
        let config = shared_config.read().unwrap().clone();
        let sensors = shared_sensors.read().unwrap().clone();

        // SỬA: Lấy cả 2 loại thời gian
        let current_wall_time_ms = get_current_time_ms();
        let current_uptime_ms = get_uptime_ms();

        // Tracker mất tín hiệu cảm biến sử dụng Uptime để chống nhảy cóc
        if sensors.controller_received_ms != last_controller_recieved_ms {
            last_controller_recieved_ms = sensors.controller_received_ms;
            sensor_last_update_ms = current_uptime_ms;
        }

        // 1. Parse lệnh MQTT
        // Truyền uptime để đếm thời gian cho chức năng Manual Pump (bật bơm thủ công) an toàn
        let (mut cmd_delta, cmd_events) =
            process_mqtt_commands(&cmd_rx, &config, &ctx, current_uptime_ms, &fsm_mqtt_tx);
        ctx.apply_delta(&mut cmd_delta);

        if !cmd_events.is_empty() {
            let mut dc = DispatchContext {
                pumps: &mut pump_ctrl,
                nvs: &mut nvs,
                mqtt_tx: &fsm_mqtt_tx,
                dosing_report_tx: &dosing_report_tx,
                sensor_cmd_tx: &sensor_cmd_tx,
                ctx: &ctx,
                // SỬA: NVS và Log cần giờ thực tế chuẩn (wall_time), và phải chia 1000 để ra Giây
                now_sec: current_wall_time_ms / 1000,
                device_id: &config.device_id,
                config: &config,
                observers: &mut observer_set,
            };
            EventDispatcher::dispatch(cmd_events, &mut dc);
        }

        // 2. Chạy FSM Tick Decision Engine
        // SỬA: Cung cấp cả now_ms (để ghi log) và uptime_ms (để canh thời gian)
        let mut tick_result = orchestrator::tick(
            current_wall_time_ms,
            current_uptime_ms,
            &config,
            &sensors,
            sensor_last_update_ms,
            &mut ctx,
        );

        ctx.apply_delta(&mut tick_result.delta);

        // 3. Thực thi Side Effects
        if !tick_result.events.is_empty() {
            let mut dc = DispatchContext {
                pumps: &mut pump_ctrl,
                nvs: &mut nvs,
                mqtt_tx: &fsm_mqtt_tx,
                dosing_report_tx: &dosing_report_tx,
                sensor_cmd_tx: &sensor_cmd_tx,
                ctx: &ctx,
                now_sec: current_wall_time_ms / 1000, // SỬA
                device_id: &config.device_id,
                config: &config,
                observers: &mut observer_set,
            };
            EventDispatcher::dispatch(tick_result.events, &mut dc);
        }

        // 4. Báo cáo trạng thái nếu có chuyển Phase
        let state_str = ctx.phase.as_str().to_string();
        if state_str != last_reported_state {
            info!("🔄 [FSM] Phase thay đổi: [{}]", state_str);
            last_reported_state = state_str;
            let _ = fsm_mqtt_tx.send(build_status_msg(&ctx, current_wall_time_ms / 1000));
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}
