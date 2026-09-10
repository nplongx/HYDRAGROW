#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub backend_url: String,
    pub api_key: String,
    pub anthropic_api_key: String,
    pub llm_provider: String,
    pub llm_model: String,
    pub poll_interval_secs: u64,
    pub max_concurrent_diagnoses: usize,
    pub dedup_cooldown_minutes: i64,
    pub max_tool_round_trips: u32,
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub per_call_timeout_secs: u64,
    pub diagnosis_wall_clock_budget_secs: u64,
    pub watchdog_breach_lookback_minutes: i64,
}

impl WorkerConfig {
    pub fn from_pairs(pairs: &[(&str, &str)]) -> anyhow::Result<Self> {
        let get = |key: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        };
        let get_or = |key: &str, default: &str| get(key).unwrap_or_else(|| default.to_string());
        let get_num = |key: &str, default: u64| -> anyhow::Result<u64> {
            Ok(match get(key) {
                Some(v) => v.parse()?,
                None => default,
            })
        };

        let backend_url = get("DIAGNOSTIC_BACKEND_URL")
            .ok_or_else(|| anyhow::anyhow!("DIAGNOSTIC_BACKEND_URL is required"))?;
        let api_key = get("DIAGNOSTIC_API_KEY")
            .ok_or_else(|| anyhow::anyhow!("DIAGNOSTIC_API_KEY is required"))?;
        let anthropic_api_key = get("ANTHROPIC_API_KEY")
            .ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY is required"))?;

        Ok(Self {
            backend_url,
            api_key,
            anthropic_api_key,
            llm_provider: get_or("DIAGNOSTIC_LLM_PROVIDER", "anthropic"),
            llm_model: get_or("DIAGNOSTIC_LLM_MODEL", "claude-sonnet-5"),
            poll_interval_secs: get_num("DIAGNOSTIC_POLL_INTERVAL_SECS", 120)?,
            max_concurrent_diagnoses: get_num("DIAGNOSTIC_MAX_CONCURRENT", 5)? as usize,
            dedup_cooldown_minutes: get_num("DIAGNOSTIC_DEDUP_COOLDOWN_MINUTES", 20)? as i64,
            max_tool_round_trips: get_num("DIAGNOSTIC_MAX_TOOL_ROUND_TRIPS", 5)? as u32,
            max_input_tokens: get_num("DIAGNOSTIC_MAX_INPUT_TOKENS", 8000)? as u32,
            max_output_tokens: get_num("DIAGNOSTIC_MAX_OUTPUT_TOKENS", 1024)? as u32,
            per_call_timeout_secs: get_num("DIAGNOSTIC_PER_CALL_TIMEOUT_SECS", 25)?,
            diagnosis_wall_clock_budget_secs: get_num("DIAGNOSTIC_WALL_CLOCK_BUDGET_SECS", 60)?,
            watchdog_breach_lookback_minutes: get_num("DIAGNOSTIC_WATCHDOG_LOOKBACK_MINUTES", 10)?
                as i64,
        })
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let keys = [
            "DIAGNOSTIC_BACKEND_URL",
            "DIAGNOSTIC_API_KEY",
            "ANTHROPIC_API_KEY",
            "DIAGNOSTIC_LLM_PROVIDER",
            "DIAGNOSTIC_LLM_MODEL",
            "DIAGNOSTIC_POLL_INTERVAL_SECS",
            "DIAGNOSTIC_MAX_CONCURRENT",
            "DIAGNOSTIC_DEDUP_COOLDOWN_MINUTES",
            "DIAGNOSTIC_MAX_TOOL_ROUND_TRIPS",
            "DIAGNOSTIC_MAX_INPUT_TOKENS",
            "DIAGNOSTIC_MAX_OUTPUT_TOKENS",
            "DIAGNOSTIC_PER_CALL_TIMEOUT_SECS",
            "DIAGNOSTIC_WALL_CLOCK_BUDGET_SECS",
            "DIAGNOSTIC_WATCHDOG_LOOKBACK_MINUTES",
        ];
        let owned: Vec<(String, String)> = keys
            .iter()
            .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), v)))
            .collect();
        let pairs: Vec<(&str, &str)> = owned
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        Self::from_pairs(&pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_pairs_reads_required_and_applies_defaults() {
        let vars = [
            ("DIAGNOSTIC_BACKEND_URL", "https://api.example.com"),
            ("DIAGNOSTIC_API_KEY", "svc_test"),
            ("ANTHROPIC_API_KEY", "sk-ant-test"),
        ];
        let config = WorkerConfig::from_pairs(&vars).unwrap();
        assert_eq!(config.backend_url, "https://api.example.com");
        assert_eq!(config.poll_interval_secs, 120); // design spec: 1-5 min, default 2 min
        assert_eq!(config.max_concurrent_diagnoses, 5); // §8.7
        assert_eq!(config.dedup_cooldown_minutes, 20); // §5
        assert_eq!(config.max_tool_round_trips, 5); // §8.1/§8.5
        assert_eq!(config.max_input_tokens, 8000); // §8.5
        assert_eq!(config.max_output_tokens, 1024); // §8.5
        assert_eq!(config.per_call_timeout_secs, 25); // §8.1: 20-30s
        assert_eq!(config.diagnosis_wall_clock_budget_secs, 60); // §8.5
        assert_eq!(config.llm_provider, "anthropic");
        assert_eq!(config.llm_model, "claude-sonnet-5");
    }

    #[test]
    fn from_pairs_errors_without_anthropic_api_key() {
        let vars = [
            ("DIAGNOSTIC_BACKEND_URL", "https://api.example.com"),
            ("DIAGNOSTIC_API_KEY", "svc_test"),
        ];
        assert!(WorkerConfig::from_pairs(&vars).is_err());
    }
}
