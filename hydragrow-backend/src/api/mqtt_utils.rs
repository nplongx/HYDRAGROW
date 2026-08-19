use anyhow::{Result, anyhow};
use chrono::Utc;
use hmac::{Hmac, Mac};
use hydragrow_shared::{MqttCommandOut, topics::topic_controller_command};
use rumqttc::QoS;
use serde::Serialize;
use serde_json::Value;
use sha2::Sha256;
use uuid::Uuid;

use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

fn command_secret(device_id: &str) -> Result<String> {
    let device_key = format!(
        "MQTT_COMMAND_SECRET_{}",
        device_id.to_ascii_uppercase().replace('-', "_")
    );
    std::env::var(&device_key)
        .or_else(|_| std::env::var("MQTT_COMMAND_SECRET"))
        .map_err(|_| {
            anyhow!(
                "missing MQTT command signing secret for device {}",
                device_id
            )
        })
}

fn canonical_payload(value: &Value) -> Result<Vec<u8>> {
    let mut unsigned = value.clone();
    if let Some(object) = unsigned.as_object_mut() {
        object.remove("signature");
    }
    Ok(serde_json::to_vec(&unsigned)?)
}

pub fn sign_command_value(device_id: &str, mut value: Value) -> Result<Value> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| anyhow!("MQTT command payload must be a JSON object"))?;
    object.insert("ts".to_string(), Value::from(Utc::now().timestamp()));
    object.insert("nonce".to_string(), Value::from(Uuid::new_v4().to_string()));
    object.remove("signature");

    let secret = command_secret(device_id)?;
    let canonical = canonical_payload(&value)?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
    mac.update(&canonical);
    let signature = hex::encode(mac.finalize().into_bytes());
    value
        .as_object_mut()
        .unwrap()
        .insert("signature".to_string(), Value::from(signature));
    Ok(value)
}

pub fn sign_command<T: Serialize>(device_id: &str, payload: &T) -> Result<Value> {
    sign_command_value(device_id, serde_json::to_value(payload)?)
}

pub async fn publish_signed_payload<T: Serialize>(
    app_state: &AppState,
    device_id: &str,
    topic: impl Into<String>,
    payload: &T,
) -> Result<Value> {
    let signed_payload = sign_command(device_id, payload)?;
    let payload_bytes = serde_json::to_vec(&signed_payload)?;

    app_state
        .mqtt_client
        .publish(topic.into(), QoS::AtLeastOnce, false, payload_bytes)
        .await?;

    Ok(signed_payload)
}

pub async fn publish_signed_json_value(
    app_state: &AppState,
    device_id: &str,
    topic: impl Into<String>,
    payload: Value,
) -> Result<Value> {
    let signed_payload = sign_command_value(device_id, payload)?;
    let payload_bytes = serde_json::to_vec(&signed_payload)?;

    app_state
        .mqtt_client
        .publish(topic.into(), QoS::AtLeastOnce, false, payload_bytes)
        .await?;

    Ok(signed_payload)
}

// Cập nhật lại hàm publish_command cũ
pub async fn publish_command(
    app_state: &AppState,
    device_id: &str,
    payload: &MqttCommandOut,
) -> Result<()> {
    let topic = topic_controller_command(device_id);
    publish_signed_payload(app_state, device_id, topic, payload).await?;
    Ok(())
}
