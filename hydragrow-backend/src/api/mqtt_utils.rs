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
    if let Some(obj) = value.as_object_mut() {
        obj.insert("signature".to_string(), Value::from(signature));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::env;
    use serial_test::serial;

    #[test]
    fn test_canonical_payload_removes_signature() {
        let value = json!({
            "a": 1,
            "b": "test",
            "signature": "some_sig"
        });
        let canonical = canonical_payload(&value).unwrap();
        let expected = serde_json::to_vec(&json!({
            "a": 1,
            "b": "test"
        })).unwrap();
        assert_eq!(canonical, expected);
    }

    #[test]
    #[serial]
    fn test_command_secret_resolution() {
        // Test with specific device secret
        unsafe { env::set_var("MQTT_COMMAND_SECRET_TEST_DEVICE_1", "device_specific_secret"); }
        assert_eq!(command_secret("test-device-1").unwrap(), "device_specific_secret");
        unsafe { env::remove_var("MQTT_COMMAND_SECRET_TEST_DEVICE_1"); }

        // Test with fallback secret
        unsafe { env::set_var("MQTT_COMMAND_SECRET", "fallback_secret"); }
        assert_eq!(command_secret("test-device-2").unwrap(), "fallback_secret");
        unsafe { env::remove_var("MQTT_COMMAND_SECRET");
        env::remove_var("MQTT_COMMAND_SECRET_TEST_DEVICE"); }

        // Test missing secret
        assert!(command_secret("test-device-3").is_err());
    }

    #[test]
    fn test_sign_command_value_rejects_non_object() {
        let value = json!("not an object");
        let result = sign_command_value("test-device", value);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "MQTT command payload must be a JSON object");
    }

    #[test]
    #[serial]
    fn test_sign_command_value_adds_fields_and_valid_signature() {
        unsafe { env::set_var("MQTT_COMMAND_SECRET", "test_secret");
        env::set_var("MQTT_COMMAND_SECRET_TEST_DEVICE", "test_secret"); }

        let value = json!({
            "action": "turn_on"
        });

        let signed = sign_command_value("test-device", value).unwrap();
        let obj = signed.as_object().unwrap();

        assert!(obj.contains_key("ts"));
        assert!(obj.contains_key("nonce"));
        assert!(obj.contains_key("signature"));
        assert!(obj.contains_key("action"));

        // Verify the signature
        let signature = obj.get("signature").unwrap().as_str().unwrap();
        let canonical = canonical_payload(&signed).unwrap();

        let mut mac = HmacSha256::new_from_slice(b"test_secret").unwrap();
        mac.update(&canonical);
        let expected_signature = hex::encode(mac.finalize().into_bytes());

        assert_eq!(signature, expected_signature);

        unsafe { env::remove_var("MQTT_COMMAND_SECRET");
        env::remove_var("MQTT_COMMAND_SECRET_TEST_DEVICE"); }
    }
}
