//! ESP32 adapter for the shared MQTT command semantics.
//!
//! The command state machine lives in `hydragrow-controller-core` so the
//! Digital Twin and physical controller execute the same decision logic.
//! This module retains only ESP/NVS-specific persistent command dedupe.

use hydragrow_shared::MqttCommandIn;
use std::sync::mpsc::{self, Receiver, Sender};

pub use hydragrow_controller_core::runtime::command_handler::build_stop_pump_events;

pub fn process_mqtt_commands_with_dedupe(
    cmd_rx: &Receiver<MqttCommandIn>,
    config: &hydragrow_shared::ControllerConfig,
    ctx: &hydragrow_controller_core::core::fsm::context::SystemContext,
    now_uptime_ms: u64,
    now_wall_time_ms: u64,
    fsm_mqtt_tx: &Sender<String>,
    nvs: Option<&mut esp_idf_svc::nvs::EspDefaultNvs>,
) -> (
    hydragrow_controller_core::core::fsm::tick_result::ContextDelta,
    Vec<hydragrow_controller_core::core::fsm::events::OrchestratorEvent>,
) {
    let (filtered_tx, filtered_rx) = mpsc::channel();
    let nvs = nvs;
    while let Ok(cmd) = cmd_rx.try_recv() {
        if !is_duplicate_persistent_command(&cmd, &nvs) {
            let _ = filtered_tx.send(cmd);
        }
    }
    hydragrow_controller_core::runtime::command_handler::process_mqtt_commands(
        &filtered_rx,
        config,
        ctx,
        now_uptime_ms,
        now_wall_time_ms,
        fsm_mqtt_tx,
    )
}

const PROCESSED_COMMAND_IDS_KEY: &str = "processed_cmd_ids";
const MAX_PROCESSED_COMMAND_IDS: usize = 32;

fn is_duplicate_persistent_command(
    cmd: &MqttCommandIn,
    nvs: &Option<&mut esp_idf_svc::nvs::EspDefaultNvs>,
) -> bool {
    let Some(command_id) = cmd.metadata.as_ref().and_then(|m| m.command_id.as_deref()) else {
        return false;
    };
    let Some(nvs) = nvs.as_deref() else {
        return false;
    };
    let mut buffer = [0u8; 4096];
    nvs.get_str(PROCESSED_COMMAND_IDS_KEY, &mut buffer)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .is_some_and(|ids| ids.iter().any(|id| id == command_id))
}

/// Persist a command ID only after its lifecycle event reaches the dispatcher.
pub(crate) fn mark_persistent_command_processed(
    command_id: &str,
    nvs: &mut Option<esp_idf_svc::nvs::EspDefaultNvs>,
) {
    let Some(nvs) = nvs.as_mut() else {
        return;
    };
    let mut buffer = [0u8; 4096];
    let mut ids: Vec<String> = nvs
        .get_str(PROCESSED_COMMAND_IDS_KEY, &mut buffer)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .unwrap_or_default();
    if ids.iter().any(|id| id == command_id) {
        return;
    }
    ids.push(command_id.to_string());
    if ids.len() > MAX_PROCESSED_COMMAND_IDS {
        let drop_count = ids.len() - MAX_PROCESSED_COMMAND_IDS;
        ids.drain(..drop_count);
    }
    if let Ok(serialized) = serde_json::to_string(&ids) {
        let _ = nvs.set_str(PROCESSED_COMMAND_IDS_KEY, &serialized);
    }
}
