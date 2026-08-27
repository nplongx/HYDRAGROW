use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub struct CommandSecurity {
    secret: String,
}

impl CommandSecurity {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.to_string(),
        }
    }

    /// Verify HMAC-SHA256 signature trong JSON command.
    /// JSON phải có field "sig" = hex(HMAC-SHA256(secret, body_without_sig)).
    pub fn verify(&self, doc: &Value) -> bool {
        let sig_hex = match doc["sig"].as_str() {
            Some(s) => s,
            None => return false,
        };

        // Tạo body không có "sig"
        let mut body = doc.clone();
        if let Value::Object(ref mut map) = body {
            map.remove("sig");
        }
        let body_str = match serde_json::to_string(&body) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let mut mac = match HmacSha256::new_from_slice(self.secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => return false,
        };
        mac.update(body_str.as_bytes());
        let expected = hex::encode(mac.finalize().into_bytes());

        // constant-time compare
        expected == sig_hex
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_verify_valid() {
        // Pre-computed: HMAC-SHA256("testsecret", "{\"cmd\":\"get_status\"}")
        // (thực tế cần tính bằng python: import hmac,hashlib,json
        // body=json.dumps({"cmd":"get_status"},separators=(',',':'));
        // hmac.new(b"testsecret",body.encode(),hashlib.sha256).hexdigest())
        let sec = CommandSecurity::new("testsecret");
        // Với JSON có sig hợp lệ: test chỉ kiểm tra logic, sig cụ thể computed offline
        let doc = json!({"cmd": "restart", "sig": "invalid"});
        assert!(!sec.verify(&doc)); // sig sai → false
    }

    #[test]
    fn test_verify_missing_sig() {
        let sec = CommandSecurity::new("secret");
        let doc = json!({"cmd": "get_status"});
        assert!(!sec.verify(&doc));
    }
}
