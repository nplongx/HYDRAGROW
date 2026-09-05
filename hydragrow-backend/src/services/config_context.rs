use std::collections::HashMap;

use crate::models::config::DeviceConfig;
use crate::services::config_override::{read_numeric_field, ConfigOverwriteDirective};

/// Đọc `ir_json.contextReads` (mảng {configKey, saveToVariable} do frontend
/// tính sẵn từ canvas Config·Read nodes — xem
/// hydragrow-frontend/src/lib/automation/configDirectives.ts). Trả về danh
/// sách rỗng nếu field vắng mặt (flow cũ, chưa dùng Config·Read).
pub fn parse_context_reads(ir_json: &serde_json::Value) -> Vec<(String, String)> {
    ir_json
        .get("contextReads")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    let config_key = entry.get("configKey")?.as_str()?.to_string();
                    let save_to_variable = entry.get("saveToVariable")?.as_str()?.to_string();
                    Some((config_key, save_to_variable))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Đọc `ir_json.configOverwrite` thành 1 `ConfigOverwriteDirective`, hoặc
/// `None` nếu flow này không có Config·Overwrite.
pub fn parse_config_overwrite(ir_json: &serde_json::Value) -> Option<ConfigOverwriteDirective> {
    let node = ir_json.get("configOverwrite")?;
    Some(ConfigOverwriteDirective {
        config_key: node.get("configKey")?.as_str()?.to_string(),
        value: node.get("value")?.as_str()?.to_string(),
        read_original_before_write: node
            .get("readOriginalBeforeWrite")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// Phân giải `reads` thành map tên biến -> giá trị số hiện tại, dùng 1
/// `DeviceConfig` đã fetch sẵn (để chain nhiều flow trên cùng thiết bị chỉ
/// cần 1 lượt query DB — xem eval_flow_chain trong Task 8). Field không phải
/// số (control_mode, is_enabled) bị bỏ qua vì Condition chỉ so sánh số.
pub fn resolve_context_reads_from_config(
    config: &DeviceConfig,
    reads: &[(String, String)],
) -> HashMap<String, f64> {
    reads
        .iter()
        .filter_map(|(config_key, save_to_variable)| {
            read_numeric_field(config, config_key).map(|v| (save_to_variable.clone(), v))
        })
        .collect()
}

/// Biến thể async tiện dụng — fetch DeviceConfig rồi phân giải luôn. Dùng ở
/// nơi chỉ cần context của 1 flow đơn lẻ (vd. test); trong `eval_flow_chain`
/// (nhiều flow cùng thiết bị), fetch 1 lần rồi gọi
/// `resolve_context_reads_from_config` trực tiếp để tránh N lượt query.
pub async fn resolve_context_reads(
    pool: &sqlx::PgPool,
    device_id: &str,
    reads: &[(String, String)],
) -> anyhow::Result<HashMap<String, f64>> {
    if reads.is_empty() {
        return Ok(HashMap::new());
    }
    let config = crate::db::postgres::get_device_config(pool, device_id).await?;
    Ok(resolve_context_reads_from_config(&config, reads))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_context_reads_extracts_configkey_savetovariable_pairs() {
        let ir_json = serde_json::json!({
            "contextReads": [
                { "configKey": "ph_target", "saveToVariable": "ph_target_now" },
                { "configKey": "ec_target", "saveToVariable": "ec_target_now" },
            ]
        });
        assert_eq!(
            parse_context_reads(&ir_json),
            vec![
                ("ph_target".to_string(), "ph_target_now".to_string()),
                ("ec_target".to_string(), "ec_target_now".to_string()),
            ]
        );
    }

    #[test]
    fn parse_context_reads_returns_empty_when_field_absent() {
        let ir_json = serde_json::json!({ "conditions": [] });
        assert_eq!(parse_context_reads(&ir_json), Vec::<(String, String)>::new());
    }

    #[test]
    fn parse_config_overwrite_extracts_the_directive() {
        let ir_json = serde_json::json!({
            "configOverwrite": {
                "configKey": "ec_target",
                "value": "1.8",
                "readOriginalBeforeWrite": true,
                "restoreMode": "on_condition_false"
            }
        });
        let directive = parse_config_overwrite(&ir_json).unwrap();
        assert_eq!(directive.config_key, "ec_target");
        assert_eq!(directive.value, "1.8");
        assert!(directive.read_original_before_write);
    }

    #[test]
    fn parse_config_overwrite_returns_none_when_field_absent() {
        let ir_json = serde_json::json!({ "conditions": [] });
        assert!(parse_config_overwrite(&ir_json).is_none());
    }

    #[test]
    fn resolve_context_reads_returns_empty_map_for_empty_reads() {
        use crate::models::config::DeviceConfig;
        let cfg = DeviceConfig {
            device_id: "d".to_string(),
            ec_target: 1.0,
            ec_tolerance: 0.1,
            ph_target: 6.0,
            ph_tolerance: 0.1,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 1,
            last_updated: chrono::Utc::now(),
        };
        assert_eq!(
            resolve_context_reads_from_config(&cfg, &[]),
            std::collections::HashMap::new()
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn resolve_context_reads_maps_config_values_to_their_saveToVariable_name() {
        use crate::models::config::DeviceConfig;
        let cfg = DeviceConfig {
            device_id: "d".to_string(),
            ec_target: 1.0,
            ec_tolerance: 0.1,
            ph_target: 6.5,
            ph_tolerance: 0.1,
            control_mode: "auto".to_string(),
            is_enabled: true,
            delay_between_a_and_b_sec: 1,
            last_updated: chrono::Utc::now(),
        };
        let reads = vec![("ph_target".to_string(), "ph_target_now".to_string())];
        let resolved = resolve_context_reads_from_config(&cfg, &reads);
        assert_eq!(resolved.get("ph_target_now"), Some(&(6.5f32 as f64)));
    }
}
