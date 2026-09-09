#[derive(Debug, Clone)]
pub struct WatchdogConfig {
    pub backend_url: String,
    pub api_key: String,
    pub poll_interval_secs: u64,
    pub stale_threshold_secs: u64,
}

impl WatchdogConfig {
    /// Testable independent of real process env vars — `from_env` (below)
    /// is a thin wrapper over this that reads `std::env::var` for each key.
    pub fn from_pairs(pairs: &[(&str, &str)]) -> anyhow::Result<Self> {
        let get = |key: &str| pairs.iter().find(|(k, _)| *k == key).map(|(_, v)| v.to_string());

        let backend_url = get("WATCHDOG_BACKEND_URL")
            .ok_or_else(|| anyhow::anyhow!("WATCHDOG_BACKEND_URL is required"))?;
        let api_key = get("WATCHDOG_API_KEY")
            .ok_or_else(|| anyhow::anyhow!("WATCHDOG_API_KEY is required"))?;
        let poll_interval_secs = get("WATCHDOG_POLL_INTERVAL_SECS")
            .map(|v| v.parse())
            .transpose()?
            .unwrap_or(20); // design spec §4.2
        let stale_threshold_secs = get("WATCHDOG_STALE_THRESHOLD_SECS")
            .map(|v| v.parse())
            .transpose()?
            .unwrap_or(60); // design spec §4.2

        Ok(Self {
            backend_url,
            api_key,
            poll_interval_secs,
            stale_threshold_secs,
        })
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let keys = [
            "WATCHDOG_BACKEND_URL",
            "WATCHDOG_API_KEY",
            "WATCHDOG_POLL_INTERVAL_SECS",
            "WATCHDOG_STALE_THRESHOLD_SECS",
        ];
        let owned: Vec<(String, String)> = keys
            .iter()
            .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), v)))
            .collect();
        let pairs: Vec<(&str, &str)> =
            owned.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        Self::from_pairs(&pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_reads_all_required_vars() {
        let vars = [
            ("WATCHDOG_BACKEND_URL", "https://api.example.com"),
            ("WATCHDOG_API_KEY", "svc_test123"),
            ("WATCHDOG_POLL_INTERVAL_SECS", "20"),
            ("WATCHDOG_STALE_THRESHOLD_SECS", "60"),
        ];
        let config = WatchdogConfig::from_pairs(&vars).unwrap();
        assert_eq!(config.backend_url, "https://api.example.com");
        assert_eq!(config.api_key, "svc_test123");
        assert_eq!(config.poll_interval_secs, 20);
        assert_eq!(config.stale_threshold_secs, 60);
    }

    #[test]
    fn from_pairs_applies_defaults_for_optional_vars() {
        let vars = [
            ("WATCHDOG_BACKEND_URL", "https://api.example.com"),
            ("WATCHDOG_API_KEY", "svc_test123"),
        ];
        let config = WatchdogConfig::from_pairs(&vars).unwrap();
        assert_eq!(config.poll_interval_secs, 20); // design spec §4.2 default
        assert_eq!(config.stale_threshold_secs, 60); // design spec §4.2 default
    }

    #[test]
    fn from_pairs_errors_on_missing_backend_url() {
        let vars = [("WATCHDOG_API_KEY", "svc_test123")];
        assert!(WatchdogConfig::from_pairs(&vars).is_err());
    }

    #[test]
    fn from_pairs_errors_on_missing_api_key() {
        let vars = [("WATCHDOG_BACKEND_URL", "https://api.example.com")];
        assert!(WatchdogConfig::from_pairs(&vars).is_err());
    }
}
