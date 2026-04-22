use crate::AppState;
use anyhow::{Context, Result};
use rumqttc::QoS;
use serde::Deserialize;
use tracing::{info, instrument};

#[derive(serde::Serialize, Deserialize, Debug)]
pub struct CommandPayload {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pump: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_sec: Option<u64>,
}

#[instrument(skip(app_state))]
pub async fn send_command(
    app_state: &AppState,
    device_id: &str,
    payload: &CommandPayload,
) -> Result<()> {
    let topic = format!("AGITECH/{}/command", device_id);
    let payload_bytes = serde_json::to_vec(payload)?;

    app_state
        .mqtt_client
        .publish(&topic, QoS::AtLeastOnce, false, payload_bytes)
        .await
        .context("Failed to publish command via MQTT")?;

    info!(device_id, action = payload.action, "Command sent to device");
    Ok(())
}

pub async fn trigger_emergency_stop(app_state: &AppState, device_id: &str) -> Result<()> {
    let payload = CommandPayload {
        action: "emergency_stop".to_string(),
        pump: None,
        duration_sec: None,
    };
    send_command(app_state, device_id, &payload).await
}
