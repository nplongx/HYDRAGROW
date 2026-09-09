use sha2::{Digest, Sha256};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudinaryConfig {
    pub cloud_name: String,
    pub api_key: String,
    pub api_secret: String,
}

impl CloudinaryConfig {
    pub fn from_env() -> Option<Self> {
        let cloud_name = std::env::var("CLOUDINARY_CLOUD_NAME").ok()?;
        let api_key = std::env::var("CLOUDINARY_API_KEY").ok()?;
        let api_secret = std::env::var("CLOUDINARY_API_SECRET").ok()?;
        if cloud_name.is_empty() || api_key.is_empty() || api_secret.is_empty() {
            return None;
        }
        Some(Self {
            cloud_name,
            api_key,
            api_secret,
        })
    }

    /// Ký chữ ký upload theo chuẩn Cloudinary (signature_algorithm=sha256):
    /// sha256("param1=val1&param2=val2&...&api_secret={secret}") với tham số sắp theo alphabet.
    /// Ở đây chỉ ký "folder" và "timestamp" (đã đúng thứ tự alphabet: folder < timestamp).
    pub fn sign_upload(&self, folder: &str, timestamp: i64) -> String {
        let to_sign = format!("folder={}&timestamp={}{}", folder, timestamp, self.api_secret);
        let mut hasher = Sha256::new();
        hasher.update(to_sign.as_bytes());
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CloudinaryConfig {
        CloudinaryConfig {
            cloud_name: "demo".to_string(),
            api_key: "123456789012345".to_string(),
            api_secret: "abcdEFGH12345678".to_string(),
        }
    }

    #[test]
    fn sign_upload_is_deterministic_for_same_inputs() {
        let config = test_config();
        let sig1 = config.sign_upload("hydragrow/dev-1/season-1", 1700000000);
        let sig2 = config.sign_upload("hydragrow/dev-1/season-1", 1700000000);
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn sign_upload_changes_with_secret() {
        let config_a = test_config();
        let mut config_b = test_config();
        config_b.api_secret = "different_secret".to_string();
        assert_ne!(
            config_a.sign_upload("folder", 1700000000),
            config_b.sign_upload("folder", 1700000000)
        );
    }

    #[test]
    fn sign_upload_changes_with_timestamp() {
        let config = test_config();
        assert_ne!(
            config.sign_upload("folder", 1700000000),
            config.sign_upload("folder", 1700000001)
        );
    }

    #[test]
    fn sign_upload_returns_64_char_hex() {
        let config = test_config();
        let sig = config.sign_upload("folder", 1700000000);
        assert_eq!(sig.len(), 64);
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn from_env_returns_none_when_unset() {
        unsafe {
            std::env::remove_var("CLOUDINARY_CLOUD_NAME");
            std::env::remove_var("CLOUDINARY_API_KEY");
            std::env::remove_var("CLOUDINARY_API_SECRET");
        }
        assert!(CloudinaryConfig::from_env().is_none());
    }

    #[test]
    fn from_env_returns_some_when_all_three_set() {
        unsafe {
            std::env::set_var("CLOUDINARY_CLOUD_NAME", "demo");
            std::env::set_var("CLOUDINARY_API_KEY", "key123");
            std::env::set_var("CLOUDINARY_API_SECRET", "secret123");
        }
        let config = CloudinaryConfig::from_env();
        assert!(config.is_some());
        unsafe {
            std::env::remove_var("CLOUDINARY_CLOUD_NAME");
            std::env::remove_var("CLOUDINARY_API_KEY");
            std::env::remove_var("CLOUDINARY_API_SECRET");
        }
    }
}
