use actix_web::web;
use hydragrow_shared::CommandLifecycleEvent;
use serde_json::json;
use tracing::{debug, warn};

use crate::AppState;
use crate::services::durable_command::{TransitionResult, get_command, transition_with_state};

pub async fn handle(device_id: String, payload: &[u8], app_state: web::Data<AppState>) {
    let event: CommandLifecycleEvent = match serde_json::from_slice(payload) {
        Ok(event) => event,
        Err(error) => {
            warn!(device_id = %device_id, ?error, "Invalid command lifecycle payload");
            return;
        }
    };

    if event.device_id != device_id || event.command_id.trim().is_empty() {
        warn!(device_id = %device_id, command_id = %event.command_id, "Rejected cross-device or empty command lifecycle event");
        return;
    }

    let command = match get_command(&app_state.pg_pool, &event.command_id).await {
        Ok(command) if command.device_id == device_id => command,
        Ok(_) => {
            warn!(device_id = %device_id, command_id = %event.command_id, "Rejected command lifecycle event for another device");
            return;
        }
        Err(_) => {
            warn!(device_id = %device_id, command_id = %event.command_id, "Ignoring lifecycle event for unknown command");
            return;
        }
    };

    let result = transition_with_state(
        &app_state,
        &event.command_id,
        &device_id,
        event.lifecycle,
        event.reason.clone(),
        "controller",
        json!({"controller_event_timestamp_ms": event.timestamp_ms}),
    )
    .await;

    match result {
        Ok(TransitionResult::Applied) => {}
        Ok(TransitionResult::Duplicate) => {
            debug!(device_id = %device_id, command_id = %event.command_id, lifecycle = ?event.lifecycle, "Duplicate command lifecycle event");
        }
        Err(error) => {
            debug!(device_id = %device_id, command_id = %event.command_id, from = ?command.lifecycle, to = ?event.lifecycle, ?error, "Rejected command lifecycle transition");
        }
    }
}
