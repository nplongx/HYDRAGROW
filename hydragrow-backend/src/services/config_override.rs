use anyhow::{Context, Result};
use std::collections::HashMap;

use crate::models::config::DeviceConfig;

/// Các config key mà Config·Read / Config·Overwrite được phép nhắm tới — khớp
/// DEVICE_CONFIG_KEYS ở hydragrow-frontend/src/components/automation/reactflow/NodeEditorPanel.tsx.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigFieldKind {
    Numeric,
    Integer,
    Text,
    Bool,
}

fn config_field_kind(key: &str) -> Option<ConfigFieldKind> {
    match key {
        "ec_target" | "ec_tolerance" | "ph_target" | "ph_tolerance" => Some(ConfigFieldKind::Numeric),
        "delay_between_a_and_b_sec" => Some(ConfigFieldKind::Integer),
        "control_mode" => Some(ConfigFieldKind::Text),
        "is_enabled" => Some(ConfigFieldKind::Bool),
        _ => None,
    }
}

/// Đọc 1 field số hiện tại của `config` — CHỈ field số (dùng để nạp execution
/// context cho Condition.valueVariable). Field không phải số trả về `None`.
pub fn read_numeric_field(config: &DeviceConfig, key: &str) -> Option<f64> {
    match key {
        "ec_target" => Some(config.ec_target as f64),
        "ec_tolerance" => Some(config.ec_tolerance as f64),
        "ph_target" => Some(config.ph_target as f64),
        "ph_tolerance" => Some(config.ph_tolerance as f64),
        "delay_between_a_and_b_sec" => Some(config.delay_between_a_and_b_sec as f64),
        _ => None,
    }
}

/// Đọc 1 field bất kỳ (kể cả control_mode/is_enabled) dưới dạng String — dùng
/// để backup giá trị gốc trước khi Config·Overwrite ghi đè.
pub fn read_field_as_string(config: &DeviceConfig, key: &str) -> Option<String> {
    match key {
        "ec_target" => Some(config.ec_target.to_string()),
        "ec_tolerance" => Some(config.ec_tolerance.to_string()),
        "ph_target" => Some(config.ph_target.to_string()),
        "ph_tolerance" => Some(config.ph_tolerance.to_string()),
        "delay_between_a_and_b_sec" => Some(config.delay_between_a_and_b_sec.to_string()),
        "control_mode" => Some(config.control_mode.clone()),
        "is_enabled" => Some(config.is_enabled.to_string()),
        _ => None,
    }
}

/// Ghi `raw` vào đúng field của `config`, parse theo kiểu thật của field đó.
/// `raw` có thể là literal ("1.8", "true") hoặc, khi trùng tên 1 key trong
/// `context`, giá trị số được lấy từ `context` — khớp hành vi VariableCombobox
/// ở Config·Overwrite (người dùng có thể gõ số hoặc chọn 1 biến).
pub fn write_field(
    config: &mut DeviceConfig,
    key: &str,
    raw: &str,
    context: &HashMap<String, f64>,
) -> Result<()> {
    let kind = config_field_kind(key).context(format!("Unknown config key: {key}"))?;
    let resolved_numeric = || -> Result<f64> {
        if let Some(v) = context.get(raw) {
            return Ok(*v);
        }
        raw.parse::<f64>()
            .context(format!("'{raw}' is neither a known context variable nor a number"))
    };
    match kind {
        ConfigFieldKind::Numeric => {
            let v = resolved_numeric()? as f32;
            match key {
                "ec_target" => config.ec_target = v,
                "ec_tolerance" => config.ec_tolerance = v,
                "ph_target" => config.ph_target = v,
                "ph_tolerance" => config.ph_tolerance = v,
                _ => unreachable!("config_field_kind and this match must stay in sync"),
            }
        }
        ConfigFieldKind::Integer => {
            config.delay_between_a_and_b_sec = resolved_numeric()? as i32;
        }
        ConfigFieldKind::Text => {
            config.control_mode = raw.to_string();
        }
        ConfigFieldKind::Bool => {
            config.is_enabled = raw
                .parse::<bool>()
                .context(format!("'{raw}' is not a valid bool for is_enabled"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::DeviceConfig;
    use std::collections::HashMap;

    fn sample_config() -> DeviceConfig {
        DeviceConfig {
            device_id: "dev-1".to_string(),
            ec_target: 1.8,
            ec_tolerance: 0.2,
            ph_target: 6.0,
            ph_tolerance: 0.3,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 5,
            last_updated: chrono::Utc::now(),
        }
    }

    #[test]
    fn read_numeric_field_returns_none_for_non_numeric_keys() {
        let cfg = sample_config();
        assert_eq!(read_numeric_field(&cfg, "ec_target"), Some(1.8f32 as f64));
        assert_eq!(read_numeric_field(&cfg, "control_mode"), None);
        assert_eq!(read_numeric_field(&cfg, "is_enabled"), None);
    }

    #[test]
    fn read_field_as_string_covers_every_writable_key() {
        let cfg = sample_config();
        assert_eq!(read_field_as_string(&cfg, "control_mode"), Some("auto".to_string()));
        assert_eq!(read_field_as_string(&cfg, "is_enabled"), Some("true".to_string()));
        assert_eq!(read_field_as_string(&cfg, "delay_between_a_and_b_sec"), Some("5".to_string()));
        assert_eq!(read_field_as_string(&cfg, "unknown_key"), None);
    }

    #[test]
    fn write_field_parses_a_literal_number_for_numeric_fields() {
        let mut cfg = sample_config();
        write_field(&mut cfg, "ec_target", "2.4", &HashMap::new()).unwrap();
        assert!((cfg.ec_target - 2.4).abs() < 0.001);
    }

    #[test]
    fn write_field_resolves_a_context_variable_name_before_parsing_as_literal() {
        let mut cfg = sample_config();
        let ctx: HashMap<String, f64> = [("ph_target_now".to_string(), 6.4)].into_iter().collect();
        write_field(&mut cfg, "ph_target", "ph_target_now", &ctx).unwrap();
        assert!((cfg.ph_target - 6.4).abs() < 0.001);
    }

    #[test]
    fn write_field_parses_bool_and_text_fields() {
        let mut cfg = sample_config();
        write_field(&mut cfg, "is_enabled", "false", &HashMap::new()).unwrap();
        assert!(!cfg.is_enabled);
        write_field(&mut cfg, "control_mode", "manual", &HashMap::new()).unwrap();
        assert_eq!(cfg.control_mode, "manual");
    }

    #[test]
    fn write_field_errors_on_unknown_key() {
        let mut cfg = sample_config();
        assert!(write_field(&mut cfg, "not_a_real_key", "1", &HashMap::new()).is_err());
    }

    #[test]
    fn write_field_errors_when_literal_is_neither_a_number_nor_a_known_variable() {
        let mut cfg = sample_config();
        assert!(write_field(&mut cfg, "ec_target", "not_a_number", &HashMap::new()).is_err());
    }
}
