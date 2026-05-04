use anyhow::Result;
use hydragrow_shared::MqttCommandPayload;
use rumqttc::QoS;

use crate::AppState;

pub async fn publish_command(
    app_state: &AppState,
    device_id: &str,
    payload: &MqttCommandPayload,
) -> Result<()> {
    let topic = format!("AGITECH/{}/controller/command", device_id);
    let payload_bytes = serde_json::to_vec(payload)?;

    app_state
        .mqtt_client
        .publish(topic, QoS::AtLeastOnce, false, payload_bytes)
        .await?;

    Ok(())
}
