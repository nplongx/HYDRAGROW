use anyhow::{Context, Result};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

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

/// 1 chỉ thị Config·Overwrite đã được phân giải từ `ir_json.configOverwrite`
/// (xem services/config_context.rs::parse_config_overwrite).
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigOverwriteDirective {
    pub config_key: String,
    pub value: String,
    pub read_original_before_write: bool,
}

/// Áp dụng hoặc khôi phục Config·Overwrite dựa trên chuyển trạng thái điều kiện
/// giữa 2 lần eval liên tiếp (`previous_state` -> `condition_state`):
/// - None/false -> true: áp dụng (backup giá trị gốc nếu `read_original_before_write`).
/// - true -> true: no-op — đã áp dụng, không backup thêm lần 2.
/// - true -> false: khôi phục giá trị gốc từ backup gần nhất chưa restore.
/// - false/None -> false: no-op.
pub async fn apply_config_overwrite_transition(
    pool: &PgPool,
    script_id: Uuid,
    device_id: &str,
    directive: &ConfigOverwriteDirective,
    context: &HashMap<String, f64>,
    previous_state: Option<bool>,
    condition_state: bool,
) -> Result<()> {
    match (previous_state, condition_state) {
        (Some(true), true) | (Some(false), false) | (None, false) => Ok(()),
        (Some(true), false) => restore_override(pool, script_id, device_id, directive).await,
        (_, true) => apply_override(pool, script_id, device_id, directive, context).await,
    }
}

async fn apply_override(
    pool: &PgPool,
    script_id: Uuid,
    device_id: &str,
    directive: &ConfigOverwriteDirective,
    context: &HashMap<String, f64>,
) -> Result<()> {
    let mut config = crate::db::postgres::get_device_config(pool, device_id).await?;
    if directive.read_original_before_write {
        let original = read_field_as_string(&config, &directive.config_key)
            .context("cannot back up an unknown config key")?;
        sqlx::query(
            "INSERT INTO flow_config_overrides (script_id, device_id, config_key, original_value) \
            VALUES ($1, $2, $3, $4)",
        )
        .bind(script_id)
        .bind(device_id)
        .bind(&directive.config_key)
        .bind(&original)
        .execute(pool)
        .await
        .context("failed to persist config override backup")?;
    }
    write_field(&mut config, &directive.config_key, &directive.value, context)?;
    crate::db::postgres::upsert_device_config(pool, &config).await
}

async fn restore_override(
    pool: &PgPool,
    script_id: Uuid,
    device_id: &str,
    directive: &ConfigOverwriteDirective,
) -> Result<()> {
    let row: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, original_value FROM flow_config_overrides \
        WHERE script_id = $1 AND device_id = $2 AND config_key = $3 AND restored_at IS NULL \
        ORDER BY applied_at DESC LIMIT 1",
    )
    .bind(script_id)
    .bind(device_id)
    .bind(&directive.config_key)
    .fetch_optional(pool)
    .await
    .context("failed to look up config override backup")?;

    let Some((backup_id, original_value)) = row else {
        return Ok(()); // Không có gì để khôi phục — vd. read_original_before_write=false.
    };

    let mut config = crate::db::postgres::get_device_config(pool, device_id).await?;
    write_field(&mut config, &directive.config_key, &original_value, &HashMap::new())?;
    crate::db::postgres::upsert_device_config(pool, &config).await?;

    sqlx::query("UPDATE flow_config_overrides SET restored_at = NOW() WHERE id = $1")
        .bind(backup_id)
        .execute(pool)
        .await
        .context("failed to mark config override backup restored")?;
    Ok(())
}

/// Quét mọi override chưa được khôi phục (flow dừng đột ngột — mất điện/crash
/// trước khi condition chuyển về false) và khôi phục giá trị gốc. Gọi 1 lần
/// khi khởi động server — xem main.rs.
pub async fn recover_orphan_overrides(pool: &PgPool) -> Result<usize> {
    let rows: Vec<(Uuid, String, String, String)> = sqlx::query_as(
        "SELECT id, device_id, config_key, original_value FROM flow_config_overrides \
        WHERE restored_at IS NULL",
    )
    .fetch_all(pool)
    .await
    .context("failed to list orphan config overrides")?;

    let mut recovered = 0;
    for (backup_id, device_id, config_key, original_value) in rows {
        let mut config = match crate::db::postgres::get_device_config(pool, &device_id).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(device_id, error = %e, "orphan override recovery: device_config missing, skipping");
                continue;
            }
        };
        if let Err(e) = write_field(&mut config, &config_key, &original_value, &HashMap::new()) {
            tracing::warn!(device_id, config_key, error = %e, "orphan override recovery: failed to parse original value, skipping");
            continue;
        }
        if let Err(e) = crate::db::postgres::upsert_device_config(pool, &config).await {
            tracing::warn!(device_id, config_key, error = %e, "orphan override recovery: failed to write restored config");
            continue;
        }
        let _ = sqlx::query("UPDATE flow_config_overrides SET restored_at = NOW() WHERE id = $1")
            .bind(backup_id)
            .execute(pool)
            .await;
        tracing::warn!(device_id, config_key, "orphan override recovery: restored un-restored config override from a previous run");
        recovered += 1;
    }
    Ok(recovered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::postgres::{get_device_config, upsert_device_config};
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

    async fn seed_device(pool: &sqlx::PgPool, device_id: &str) {
        upsert_device_config(
            pool,
            &DeviceConfig {
                device_id: device_id.to_string(),
                ec_target: 1.8,
                ec_tolerance: 0.2,
                ph_target: 6.0,
                ph_tolerance: 0.3,
                control_mode: "auto".to_string(),
                is_enabled: true,
                delay_between_a_and_b_sec: 5,
                last_updated: chrono::Utc::now(),
            },
        )
        .await
        .unwrap();
    }

    fn directive() -> ConfigOverwriteDirective {
        ConfigOverwriteDirective {
            config_key: "ec_target".to_string(),
            value: "2.4".to_string(),
            read_original_before_write: true,
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn transition_to_true_applies_override_and_backs_up_original(pool: sqlx::PgPool) {
        seed_device(&pool, "dev-a").await;
        let script_id = uuid::Uuid::new_v4();
        apply_config_overwrite_transition(
            &pool,
            script_id,
            "dev-a",
            &directive(),
            &HashMap::new(),
            None,
            true,
        )
        .await
        .unwrap();
        let cfg = get_device_config(&pool, "dev-a").await.unwrap();
        assert!((cfg.ec_target - 2.4).abs() < 0.001);

        let backup_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM flow_config_overrides WHERE script_id = $1 AND restored_at IS NULL",
        )
        .bind(script_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(backup_count, 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn transition_from_true_to_false_restores_original_value(pool: sqlx::PgPool) {
        seed_device(&pool, "dev-b").await;
        let script_id = uuid::Uuid::new_v4();
        apply_config_overwrite_transition(
            &pool,
            script_id,
            "dev-b",
            &directive(),
            &HashMap::new(),
            None,
            true,
        )
        .await
        .unwrap();
        apply_config_overwrite_transition(
            &pool,
            script_id,
            "dev-b",
            &directive(),
            &HashMap::new(),
            Some(true),
            false,
        )
        .await
        .unwrap();

        let cfg = get_device_config(&pool, "dev-b").await.unwrap();
        assert!(
            (cfg.ec_target - 1.8).abs() < 0.001,
            "expected restore to original 1.8, got {}",
            cfg.ec_target
        );
        let unrestored: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM flow_config_overrides WHERE script_id = $1 AND restored_at IS NULL",
        )
        .bind(script_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(unrestored, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn staying_true_does_not_create_a_second_backup(pool: sqlx::PgPool) {
        seed_device(&pool, "dev-c").await;
        let script_id = uuid::Uuid::new_v4();
        apply_config_overwrite_transition(
            &pool,
            script_id,
            "dev-c",
            &directive(),
            &HashMap::new(),
            None,
            true,
        )
        .await
        .unwrap();
        apply_config_overwrite_transition(
            &pool,
            script_id,
            "dev-c",
            &directive(),
            &HashMap::new(),
            Some(true),
            true,
        )
        .await
        .unwrap();
        let backup_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM flow_config_overrides WHERE script_id = $1",
        )
        .bind(script_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(backup_count, 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn recover_orphan_overrides_restores_un_restored_rows_and_marks_them_restored(
        pool: sqlx::PgPool,
    ) {
        seed_device(&pool, "dev-d").await;
        let script_id = uuid::Uuid::new_v4();
        apply_config_overwrite_transition(
            &pool,
            script_id,
            "dev-d",
            &directive(),
            &HashMap::new(),
            None,
            true,
        )
        .await
        .unwrap();
        // Không gọi transition sang false — mô phỏng flow dừng đột ngột (crash/mất điện).
        let recovered = recover_orphan_overrides(&pool).await.unwrap();
        assert_eq!(recovered, 1);
        let cfg = get_device_config(&pool, "dev-d").await.unwrap();
        assert!((cfg.ec_target - 1.8).abs() < 0.001);
        let recovered_again = recover_orphan_overrides(&pool).await.unwrap();
        assert_eq!(
            recovered_again, 0,
            "already-recovered rows must not be recovered twice"
        );
    }
}
