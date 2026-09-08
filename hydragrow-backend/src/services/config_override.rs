use anyhow::{Context, Result};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::config::DeviceConfig;
use crate::services::config_registry::{self, ValueType};

/// Các config key mà Config·Read / Config·Overwrite được phép nhắm tới.
/// Key số tra từ config_registry (nguồn duy nhất); control_mode/is_enabled
/// giữ ngoài registry vì không phải số có min/max.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigFieldKind {
    Numeric,
    Integer,
    Text,
    Bool,
}

fn config_field_kind(key: &str) -> Option<ConfigFieldKind> {
    match key {
        "control_mode" => return Some(ConfigFieldKind::Text),
        "is_enabled" => return Some(ConfigFieldKind::Bool),
        _ => {}
    }
    let def = config_registry::lookup(key)?;
    match def.value_type {
        ValueType::Float => Some(ConfigFieldKind::Numeric),
        ValueType::Integer => Some(ConfigFieldKind::Integer),
    }
}

/// Đọc 1 field số hiện tại của `config` — CHỈ field số (dùng để nạp execution
/// context cho Condition.valueVariable). Field không phải số trả về `None`.
pub fn read_numeric_field(config: &DeviceConfig, key: &str) -> Option<f64> {
    let def = config_registry::lookup(key)?;
    match def.value_type {
        ValueType::Float | ValueType::Integer => match key {
            "ec_target" => Some(config.ec_target as f64),
            "ec_tolerance" => Some(config.ec_tolerance as f64),
            "ph_target" => Some(config.ph_target as f64),
            "ph_tolerance" => Some(config.ph_tolerance as f64),
            "delay_between_a_and_b_sec" => Some(config.delay_between_a_and_b_sec as f64),
            _ => None,
        },
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
/// Giá trị số được kẹp (clamp) theo [min, max] của registry.
/// Trả về `Ok(true)` nếu đã clamp, `Ok(false)` nếu ghi nguyên văn.
pub fn write_field(
    config: &mut DeviceConfig,
    key: &str,
    raw: &str,
    context: &HashMap<String, f64>,
) -> Result<bool> {
    let kind = config_field_kind(key).context(format!("Unknown config key: {key}"))?;
    let resolved_numeric = || -> Result<f64> {
        if let Some(v) = context.get(raw) {
            return Ok(*v);
        }
        raw.parse::<f64>().context(format!(
            "'{raw}' is neither a known context variable nor a number"
        ))
    };
    match kind {
        ConfigFieldKind::Numeric => {
            let (clamped, did_clamp) = config_registry::clamp(key, resolved_numeric()?);
            let v = clamped as f32;
            match key {
                "ec_target" => config.ec_target = v,
                "ec_tolerance" => config.ec_tolerance = v,
                "ph_target" => config.ph_target = v,
                "ph_tolerance" => config.ph_tolerance = v,
                _ => unreachable!("config_field_kind and this match must stay in sync"),
            }
            Ok(did_clamp)
        }
        ConfigFieldKind::Integer => {
            let (clamped, did_clamp) = config_registry::clamp(key, resolved_numeric()?);
            config.delay_between_a_and_b_sec = clamped as i32;
            Ok(did_clamp)
        }
        ConfigFieldKind::Text => {
            config.control_mode = raw.to_string();
            Ok(false)
        }
        ConfigFieldKind::Bool => {
            config.is_enabled = raw
                .parse::<bool>()
                .context(format!("'{raw}' is not a valid bool for is_enabled"))?;
            Ok(false)
        }
    }
}

/// 1 chỉ thị Config·Overwrite đã được phân giải từ `ir_json.configOverwrite`
/// (xem services/config_context.rs::parse_config_overwrite).
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigOverwriteDirective {
    pub config_key: String,
    pub value: String,
    pub read_original_before_write: bool,
    pub priority: i32,
}

/// Một Flow đang "ứng cử" để giữ quyền ghi đè `config_key` tại tick hiện tại.
#[derive(Debug, Clone)]
pub struct OverwriteContender {
    pub script_id: Uuid,
    pub directive: ConfigOverwriteDirective,
    /// Kết quả eval điều kiện của Flow này tại tick hiện tại (đã tính sẵn ở
    /// caller — xem eval_flow_chain trong script_eval.rs).
    pub condition_state: bool,
    /// Execution context (Config·Read / Chain) của riêng Flow này, dùng khi
    /// `directive.value` là tên biến thay vì literal.
    pub context: HashMap<String, f64>,
}

/// Chọn Flow "thắng" quyền giữ `config_key` trong số các `contenders` có
/// `condition_state == true`, theo priority CAO HƠN thắng. Hoà priority ->
/// Flow xuất hiện TRƯỚC trong `contenders` thắng (thứ tự ổn định, khớp hành
/// vi cũ trước khi có priority tường minh — xem trang 08 đặc tả Figma).
/// Không có ai `condition_state == true` -> `None` (không ai nên giữ key này).
pub fn pick_winner(contenders: &[OverwriteContender]) -> Option<Uuid> {
    let mut winner: Option<&OverwriteContender> = None;
    for c in contenders {
        if !c.condition_state {
            continue;
        }
        winner = match winner {
            None => Some(c),
            Some(w) if c.directive.priority > w.directive.priority => Some(c),
            Some(w) => Some(w),
        };
    }
    winner.map(|c| c.script_id)
}

/// Đối chiếu (reconcile) 1 nhóm Flow cùng nhắm `config_key` trên `device_id`:
/// đọc AI đang thực sự giữ key này trong DB, so với AI NÊN giữ (pick_winner),
/// rồi thực hiện đúng 1 trong 4 chuyển đổi cần thiết. Gọi lại mỗi tick với
/// TOÀN BỘ contenders hiện có cho key đó — hàm này "level-triggered" (tính
/// lại từ đầu mỗi lần), không cần cache trạng thái trước đó ở caller.
pub async fn reconcile_config_overwrite_group(
    pool: &PgPool,
    device_id: &str,
    config_key: &str,
    contenders: &[OverwriteContender],
) -> Result<()> {
    let winner_id = pick_winner(contenders);

    let current_holder: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT script_id, original_value FROM flow_config_overrides \
        WHERE device_id = $1 AND config_key = $2 AND restored_at IS NULL \
        ORDER BY applied_at DESC LIMIT 1",
    )
    .bind(device_id)
    .bind(config_key)
    .fetch_optional(pool)
    .await
    .context("failed to look up current config override holder")?;

    match (current_holder, winner_id) {
        (None, None) => Ok(()),
        (Some((holder_id, _)), Some(w)) if holder_id == w => Ok(()),
        (Some((holder_id, chained_original)), Some(w)) => {
            // Đổi người giữ: Flow ưu tiên cao hơn giành quyền. KHÔNG ghi giá
            // trị gốc về thiết bị ở bước đóng — người thắng mới sẽ ghi đè
            // ngay sau, tránh nhấp nháy về baseline rồi lại đổi ngay.
            let winner = contenders
                .iter()
                .find(|c| c.script_id == w)
                .context("winner id not found among contenders")?;
            close_holder_row(pool, holder_id, device_id, config_key).await?;
            become_holder(pool, w, device_id, winner, Some(chained_original)).await
        }
        (Some((holder_id, original_value)), None) => {
            // Không còn Flow nào muốn giữ -> khôi phục thật về baseline.
            release_holder(pool, holder_id, device_id, config_key, &original_value).await
        }
        (None, Some(w)) => {
            // Chưa ai giữ, có người thắng mới -> đọc giá trị gốc THẬT từ
            // device_config hiện tại (chưa bị Flow nào ghi đè).
            let winner = contenders
                .iter()
                .find(|c| c.script_id == w)
                .context("winner id not found among contenders")?;
            become_holder(pool, w, device_id, winner, None).await
        }
    }
}

async fn close_holder_row(
    pool: &PgPool,
    script_id: Uuid,
    device_id: &str,
    config_key: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE flow_config_overrides SET restored_at = NOW() \
        WHERE script_id = $1 AND device_id = $2 AND config_key = $3 AND restored_at IS NULL",
    )
    .bind(script_id)
    .bind(device_id)
    .bind(config_key)
    .execute(pool)
    .await
    .context("failed to close superseded config override row")?;
    Ok(())
}

/// `chained_original`: khi Flow này ĐANG kế thừa quyền giữ từ 1 Flow ưu tiên
/// thấp hơn vừa bị đóng (xem nhánh (Some, Some) ở trên), dùng LẠI baseline
/// thật của Flow trước đó thay vì đọc device_config hiện tại (lúc này đã bị
/// Flow trước ghi đè, không còn là baseline thật nữa).
async fn become_holder(
    pool: &PgPool,
    script_id: Uuid,
    device_id: &str,
    contender: &OverwriteContender,
    chained_original: Option<String>,
) -> Result<()> {
    let mut config = crate::db::postgres::get_device_config(pool, device_id).await?;
    let directive = &contender.directive;

    if directive.read_original_before_write {
        let original = match chained_original {
            Some(v) => v,
            None => read_field_as_string(&config, &directive.config_key)
                .context("cannot back up an unknown config key")?,
        };
        sqlx::query(
            "INSERT INTO flow_config_overrides \
            (script_id, device_id, config_key, original_value, override_value, priority, clamped) \
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(script_id)
        .bind(device_id)
        .bind(&directive.config_key)
        .bind(&original)
        .bind(&directive.value)
        .bind(directive.priority)
        .bind(false)
        .execute(pool)
        .await
        .context("failed to persist config override backup")?;
    }

    let clamped = write_field(
        &mut config,
        &directive.config_key,
        &directive.value,
        &contender.context,
    )?;
    if clamped && directive.read_original_before_write {
        sqlx::query(
            "UPDATE flow_config_overrides SET clamped = true \
            WHERE script_id = $1 AND device_id = $2 AND config_key = $3 AND restored_at IS NULL",
        )
        .bind(script_id)
        .bind(device_id)
        .bind(&directive.config_key)
        .execute(pool)
        .await
        .context("failed to flag clamped override")?;
    }
    crate::db::postgres::upsert_device_config(pool, &config).await
}

async fn release_holder(
    pool: &PgPool,
    script_id: Uuid,
    device_id: &str,
    config_key: &str,
    original_value: &str,
) -> Result<()> {
    let mut config = crate::db::postgres::get_device_config(pool, device_id).await?;
    write_field(&mut config, config_key, original_value, &HashMap::new())?;
    crate::db::postgres::upsert_device_config(pool, &config).await?;

    sqlx::query(
        "UPDATE flow_config_overrides SET restored_at = NOW() \
        WHERE script_id = $1 AND device_id = $2 AND config_key = $3 AND restored_at IS NULL",
    )
    .bind(script_id)
    .bind(device_id)
    .bind(config_key)
    .execute(pool)
    .await
    .context("failed to mark config override restored")?;
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
        tracing::warn!(
            device_id,
            config_key,
            "orphan override recovery: restored un-restored config override from a previous run"
        );
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
        assert_eq!(
            read_field_as_string(&cfg, "control_mode"),
            Some("auto".to_string())
        );
        assert_eq!(
            read_field_as_string(&cfg, "is_enabled"),
            Some("true".to_string())
        );
        assert_eq!(
            read_field_as_string(&cfg, "delay_between_a_and_b_sec"),
            Some("5".to_string())
        );
        assert_eq!(read_field_as_string(&cfg, "unknown_key"), None);
    }

    #[test]
    fn write_field_parses_a_literal_number_for_numeric_fields() {
        let mut cfg = sample_config();
        let _ = write_field(&mut cfg, "ec_target", "2.4", &HashMap::new()).unwrap();
        assert!((cfg.ec_target - 2.4).abs() < 0.001);
    }

    #[test]
    fn write_field_resolves_a_context_variable_name_before_parsing_as_literal() {
        let mut cfg = sample_config();
        let ctx: HashMap<String, f64> = [("ph_target_now".to_string(), 6.4)].into_iter().collect();
        let _ = write_field(&mut cfg, "ph_target", "ph_target_now", &ctx).unwrap();
        assert!((cfg.ph_target - 6.4).abs() < 0.001);
    }

    #[test]
    fn write_field_parses_bool_and_text_fields() {
        let mut cfg = sample_config();
        let _ = write_field(&mut cfg, "is_enabled", "false", &HashMap::new()).unwrap();
        assert!(!cfg.is_enabled);
        let _ = write_field(&mut cfg, "control_mode", "manual", &HashMap::new()).unwrap();
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

    #[test]
    fn write_field_clamps_a_too_high_value_to_registry_max() {
        let mut cfg = sample_config();
        let clamped =
            write_field(&mut cfg, "ec_target", "99.0", &HashMap::new()).unwrap();
        assert!(clamped, "expected clamp flag for out-of-range high value");
        assert!((cfg.ec_target - 3.2).abs() < 0.001);
    }

    #[test]
    fn write_field_clamps_a_too_low_value_to_registry_min() {
        let mut cfg = sample_config();
        let clamped =
            write_field(&mut cfg, "ec_target", "0.1", &HashMap::new()).unwrap();
        assert!(clamped, "expected clamp flag for out-of-range low value");
        assert!((cfg.ec_target - 0.8).abs() < 0.001);
    }

    #[test]
    fn write_field_reports_no_clamp_for_an_in_range_value() {
        let mut cfg = sample_config();
        let clamped =
            write_field(&mut cfg, "ec_target", "2.4", &HashMap::new()).unwrap();
        assert!(!clamped, "in-range value must not report clamping");
        assert!((cfg.ec_target - 2.4).abs() < 0.001);
    }

    #[test]
    fn write_field_still_errors_on_dose_max_ml() {
        let mut cfg = sample_config();
        assert!(write_field(&mut cfg, "dose_max_ml", "10", &HashMap::new()).is_err());
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
            priority: 0,
        }
    }

    fn contender(script_id: Uuid, priority: i32, condition_state: bool) -> OverwriteContender {
        OverwriteContender {
            script_id,
            directive: ConfigOverwriteDirective {
                config_key: "ec_target".to_string(),
                value: "1.8".to_string(),
                read_original_before_write: true,
                priority,
            },
            condition_state,
            context: HashMap::new(),
        }
    }

    #[test]
    fn pick_winner_returns_none_when_nobody_is_currently_true() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        let contenders = vec![contender(a, 0, false), contender(b, 5, false)];
        assert_eq!(pick_winner(&contenders), None);
    }

    #[test]
    fn pick_winner_picks_the_only_true_contender() {
        let a = uuid::Uuid::new_v4();
        let contenders = vec![contender(a, 0, true)];
        assert_eq!(pick_winner(&contenders), Some(a));
    }

    #[test]
    fn pick_winner_picks_the_highest_priority_among_true_contenders() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        let contenders = vec![contender(a, 0, true), contender(b, 5, true)];
        assert_eq!(pick_winner(&contenders), Some(b));
    }

    #[test]
    fn pick_winner_ignores_a_higher_priority_contender_whose_condition_is_false() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        let contenders = vec![contender(a, 0, true), contender(b, 5, false)];
        assert_eq!(pick_winner(&contenders), Some(a));
    }

    #[test]
    fn pick_winner_breaks_a_priority_tie_in_favor_of_the_earlier_contender() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        let contenders = vec![contender(a, 3, true), contender(b, 3, true)];
        assert_eq!(pick_winner(&contenders), Some(a));
    }

    fn single_contender(script_id: Uuid, condition_state: bool) -> Vec<OverwriteContender> {
        vec![OverwriteContender {
            script_id,
            directive: directive(),
            condition_state,
            context: HashMap::new(),
        }]
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reconcile_applies_the_single_winner_and_backs_up_the_original(pool: sqlx::PgPool) {
        seed_device(&pool, "dev-a").await;
        let script_id = uuid::Uuid::new_v4();
        reconcile_config_overwrite_group(&pool, "dev-a", "ec_target", &single_contender(script_id, true))
            .await
            .unwrap();

        let cfg = get_device_config(&pool, "dev-a").await.unwrap();
        assert!((cfg.ec_target - 2.4).abs() < 0.001);

        let row: (String, String, i32) = sqlx::query_as(
            "SELECT original_value, override_value, priority FROM flow_config_overrides \
            WHERE script_id = $1 AND restored_at IS NULL",
        )
        .bind(script_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "1.8");
        assert_eq!(row.1, "2.4");
        assert_eq!(row.2, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reconcile_restores_the_original_once_nobody_wants_the_key_anymore(pool: sqlx::PgPool) {
        seed_device(&pool, "dev-b").await;
        let script_id = uuid::Uuid::new_v4();
        reconcile_config_overwrite_group(&pool, "dev-b", "ec_target", &single_contender(script_id, true))
            .await
            .unwrap();
        reconcile_config_overwrite_group(&pool, "dev-b", "ec_target", &single_contender(script_id, false))
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
    async fn reconcile_does_not_create_a_second_backup_while_the_same_winner_stays_true(pool: sqlx::PgPool) {
        seed_device(&pool, "dev-c").await;
        let script_id = uuid::Uuid::new_v4();
        reconcile_config_overwrite_group(&pool, "dev-c", "ec_target", &single_contender(script_id, true))
            .await
            .unwrap();
        reconcile_config_overwrite_group(&pool, "dev-c", "ec_target", &single_contender(script_id, true))
            .await
            .unwrap();

        let backup_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM flow_config_overrides WHERE script_id = $1")
                .bind(script_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(backup_count, 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reconcile_hands_the_key_to_a_higher_priority_flow_and_back_when_it_yields(pool: sqlx::PgPool) {
        seed_device(&pool, "dev-e").await;
        let low = uuid::Uuid::new_v4();
        let high = uuid::Uuid::new_v4();
        let low_directive = ConfigOverwriteDirective {
            config_key: "ec_target".to_string(),
            value: "2.0".to_string(),
            read_original_before_write: true,
            priority: 1,
        };
        let high_directive = ConfigOverwriteDirective {
            config_key: "ec_target".to_string(),
            value: "2.8".to_string(),
            read_original_before_write: true,
            priority: 10,
        };

        // Tick 1: chỉ low đúng điều kiện -> low giữ key, backup baseline thật (1.8).
        reconcile_config_overwrite_group(
            &pool,
            "dev-e",
            "ec_target",
            &[OverwriteContender { script_id: low, directive: low_directive.clone(), condition_state: true, context: HashMap::new() }],
        )
        .await
        .unwrap();
        assert!((get_device_config(&pool, "dev-e").await.unwrap().ec_target - 2.0).abs() < 0.001);

        // Tick 2: cả 2 đúng điều kiện -> high (priority cao hơn) giành quyền.
        reconcile_config_overwrite_group(
            &pool,
            "dev-e",
            "ec_target",
            &[
                OverwriteContender { script_id: low, directive: low_directive.clone(), condition_state: true, context: HashMap::new() },
                OverwriteContender { script_id: high, directive: high_directive.clone(), condition_state: true, context: HashMap::new() },
            ],
        )
        .await
        .unwrap();
        assert!((get_device_config(&pool, "dev-e").await.unwrap().ec_target - 2.8).abs() < 0.001);

        // high phải kế thừa baseline THẬT (1.8), không phải giá trị low vừa ghi (2.0).
        let high_original: String = sqlx::query_scalar(
            "SELECT original_value FROM flow_config_overrides WHERE script_id = $1 AND restored_at IS NULL",
        )
        .bind(high)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(high_original, "1.8");

        // Tick 3: high không còn đúng điều kiện -> low tự động lấy lại quyền,
        // KHÔNG rơi về baseline 1.8 (vì low vẫn muốn giữ key).
        reconcile_config_overwrite_group(
            &pool,
            "dev-e",
            "ec_target",
            &[
                OverwriteContender { script_id: low, directive: low_directive, condition_state: true, context: HashMap::new() },
                OverwriteContender { script_id: high, directive: high_directive, condition_state: false, context: HashMap::new() },
            ],
        )
        .await
        .unwrap();
        assert!(
            (get_device_config(&pool, "dev-e").await.unwrap().ec_target - 2.0).abs() < 0.001,
            "expected low-priority flow to resume holding the key at 2.0"
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn recover_orphan_overrides_restores_un_restored_rows_and_marks_them_restored(
        pool: sqlx::PgPool,
    ) {
        seed_device(&pool, "dev-d").await;
        let script_id = uuid::Uuid::new_v4();
        reconcile_config_overwrite_group(&pool, "dev-d", "ec_target", &single_contender(script_id, true))
            .await
            .unwrap();
        // Không gọi reconcile lần 2 — mô phỏng flow dừng đột ngột (crash/mất điện).
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
