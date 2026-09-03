//! Pure HMAC verification for signed MQTT command/recipe payloads.
//! Host-testable — no esp-idf dependency. Mirrors the signing logic in
//! hydragrow-backend/src/api/mqtt_utils.rs::sign_command_value.

use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Verifies a signed JSON payload (must contain `signature`, `ts`, `nonce`
/// fields as added by `hydragrow-backend`'s `sign_command_value`).
/// Returns the original parsed value (with `signature` re-inserted) on
/// success, or an error describing which check failed.
pub fn verify_signed_json_payload(
    device_id: &str,
    data: &[u8],
    secret: &str,
) -> anyhow::Result<Value> {
    if secret.is_empty() {
        anyhow::bail!("missing MQTT command signing secret for {}", device_id);
    }

    let value: Value = serde_json::from_slice(data)?;
    let mut value_clone = value.clone();
    let object = value_clone
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("signed MQTT payload must be a JSON object"))?;

    let signature = object
        .get("signature")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing MQTT payload signature"))?
        .to_owned();

    if !object.get("ts").is_some_and(|v| v.is_i64() || v.is_u64()) {
        anyhow::bail!("missing MQTT payload timestamp");
    }
    if object
        .get("nonce")
        .and_then(|v| v.as_str())
        .is_none_or(|nonce| nonce.is_empty())
    {
        anyhow::bail!("missing MQTT payload nonce");
    }

    object.remove("signature");
    let canonical = serde_json::to_vec(&value_clone)?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
    mac.update(&canonical);
    let expected = hex::encode(mac.finalize().into_bytes());

    if signature != expected {
        anyhow::bail!("invalid MQTT payload signature");
    }

    let mut result = value_clone;
    result
        .as_object_mut()
        .expect("checked above")
        .insert("signature".to_string(), Value::String(signature));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sign(secret: &str, mut value: Value, ts: i64, nonce: &str) -> Value {
        {
            let obj = value.as_object_mut().unwrap();
            obj.insert("ts".to_string(), json!(ts));
            obj.insert("nonce".to_string(), json!(nonce));
            obj.remove("signature");
        }
        let canonical = serde_json::to_vec(&value).unwrap();
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(&canonical);
        let sig = hex::encode(mac.finalize().into_bytes());
        let obj = value.as_object_mut().unwrap();
        obj.insert("signature".to_string(), json!(sig));
        value
    }

    #[test]
    fn accepts_valid_signature() {
        let payload = sign("secret123", json!({"action": "force_on"}), 1000, "n1");
        let data = serde_json::to_vec(&payload).unwrap();
        let result = verify_signed_json_payload("dev1", &data, "secret123");
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_tampered_payload() {
        let mut payload = sign("secret123", json!({"action": "force_on"}), 1000, "n1");
        payload["action"] = json!("factory_reset");
        let data = serde_json::to_vec(&payload).unwrap();
        let result = verify_signed_json_payload("dev1", &data, "secret123");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_wrong_secret() {
        let payload = sign("secret123", json!({"action": "force_on"}), 1000, "n1");
        let data = serde_json::to_vec(&payload).unwrap();
        let result = verify_signed_json_payload("dev1", &data, "wrong_secret");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_signature() {
        let data =
            serde_json::to_vec(&json!({"action": "force_on", "ts": 1000, "nonce": "n1"})).unwrap();
        let result = verify_signed_json_payload("dev1", &data, "secret123");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_ts() {
        let data =
            serde_json::to_vec(&json!({"action": "force_on", "nonce": "n1", "signature": "x"}))
                .unwrap();
        let result = verify_signed_json_payload("dev1", &data, "secret123");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_empty_secret() {
        let payload = sign("secret123", json!({"action": "force_on"}), 1000, "n1");
        let data = serde_json::to_vec(&payload).unwrap();
        let result = verify_signed_json_payload("dev1", &data, "");
        assert!(result.is_err());
    }
}
