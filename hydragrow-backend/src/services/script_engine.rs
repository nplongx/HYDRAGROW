use anyhow::{Context, Result};
use rhai::{AST, Dynamic, Engine, Map, Scope};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub use crate::models::script::{
    ActionCommandOutput, AlertOutput, RecipeOverrideOutput, ScriptActionInput, ScriptFsmInput,
    ScriptSensorInput, StageOverride,
};

/// Wrapper quanh Rhai Engine, configure một lần khi khởi động.
/// Clone-safe vì Engine implement Clone + Send + Sync (với feature "sync").
pub struct ScriptEngine {
    engine: Engine,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Giới hạn để ngăn script vòng lặp vô hạn hoặc dùng quá RAM
        engine.set_max_operations(50_000);
        engine.set_max_string_size(1024);
        engine.set_max_map_size(64);

        // Tắt print/debug để tránh output noise
        engine.on_print(|_| {});
        engine.on_debug(|_, _, _| {});

        Self { engine }
    }

    /// Compile source thành AST. Lưu AST vào cache, không compile lại mỗi lần eval.
    pub fn compile(&self, source: &str) -> Result<AST> {
        self.engine.compile(source).context("Rhai compile error")
    }

    /// Eval một alert script với sensor input.
    /// Script phải define `fn main(input)` và return Map hoặc () (unit = no alert).
    pub fn eval_alert(&self, ast: &AST, input: &ScriptSensorInput) -> Result<Option<AlertOutput>> {
        let mut map = Map::new();
        map.insert("ph".into(), Dynamic::from_float(input.ph));
        map.insert("ec".into(), Dynamic::from_float(input.ec));
        map.insert("temp".into(), Dynamic::from_float(input.temp));
        map.insert("water_level".into(), Dynamic::from_float(input.water_level));
        map.insert("device_id".into(), Dynamic::from(input.device_id.clone()));
        map.insert("timestamp_ms".into(), Dynamic::from_int(input.timestamp_ms));

        let result: Dynamic = self
            .engine
            .call_fn(&mut Scope::new(), ast, "main", (Dynamic::from_map(map),))
            .context("Rhai eval error in alert script")?;

        if result.is_unit() || result.is::<()>() {
            return Ok(None);
        }

        let map = result
            .try_cast::<Map>()
            .context("Alert script must return a Map or () (unit)")?;

        let level = map
            .get("level")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "info".to_string());

        let title = map.get("title").map(|v| v.to_string()).unwrap_or_default();

        let message = map
            .get("message")
            .map(|v| v.to_string())
            .unwrap_or_default();

        if title.is_empty() {
            return Ok(None);
        }

        Ok(Some(AlertOutput {
            level,
            title,
            message,
        }))
    }

    /// Eval một recipe_override script với FSM state.
    pub fn eval_recipe_override(
        &self,
        ast: &AST,
        input: &ScriptFsmInput,
    ) -> Result<Option<RecipeOverrideOutput>> {
        let mut map = Map::new();
        map.insert("phase".into(), Dynamic::from(input.phase.clone()));
        map.insert("stage_index".into(), Dynamic::from_int(input.stage_index));
        map.insert("ec".into(), Dynamic::from_float(input.ec));
        map.insert("ph".into(), Dynamic::from_float(input.ph));
        map.insert("elapsed_sec".into(), Dynamic::from_int(input.elapsed_sec));

        let result: Dynamic = self
            .engine
            .call_fn(&mut Scope::new(), ast, "main", (Dynamic::from_map(map),))
            .context("Rhai eval error in recipe_override script")?;

        if result.is_unit() || result.is::<()>() {
            return Ok(None);
        }

        let map = result
            .try_cast::<Map>()
            .context("Recipe override script must return a Map or () (unit)")?;

        let action = map
            .get("action")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "advance_stage".to_string());

        let reason = map.get("reason").map(|v| v.to_string()).unwrap_or_default();

        if action == "end_season" {
            return Ok(Some(RecipeOverrideOutput::EndSeason { reason }));
        }

        let target_stage_index = map
            .get("target_stage_index")
            .and_then(|v| v.clone().try_cast::<i64>())
            .context("Missing target_stage_index for advance_stage action")?;

        Ok(Some(RecipeOverrideOutput::AdvanceStage(StageOverride {
            target_stage_index,
            reason,
        })))
    }

    pub fn eval_action_command(
        &self,
        ast: &AST,
        input: &ScriptActionInput,
    ) -> Result<Option<ActionCommandOutput>> {
        self.eval_action_command_with_range_stat(ast, input, |_, _, _| 0.0)
    }

    pub fn eval_action_command_with_range_stat(
        &self,
        ast: &AST,
        input: &ScriptActionInput,
        range_stat_fetcher: impl Fn(String, String, i64) -> f64 + Send + Sync + 'static,
    ) -> Result<Option<ActionCommandOutput>> {
        // We must preserve configuration limits while mutating it to register the function.
        let mut engine = Engine::new();
        engine.set_max_operations(50_000);
        engine.set_max_string_size(1024);
        engine.set_max_map_size(64);
        engine.on_print(|_| {});
        engine.on_debug(|_, _, _| {});

        engine.register_fn("fetch_range_stat", range_stat_fetcher);

        let mut map = Map::new();
        map.insert("ph".into(), Dynamic::from_float(input.ph));
        map.insert("ec".into(), Dynamic::from_float(input.ec));
        map.insert("temp".into(), Dynamic::from_float(input.temp));
        map.insert("water_level".into(), Dynamic::from_float(input.water_level));
        map.insert("phase".into(), Dynamic::from(input.phase.clone()));
        map.insert("device_id".into(), Dynamic::from(input.device_id.clone()));
        map.insert("timestamp_ms".into(), Dynamic::from_int(input.timestamp_ms));

        let result: Dynamic = engine
            .call_fn(&mut Scope::new(), ast, "main", (Dynamic::from_map(map),))
            .context("Rhai eval error in action_command script")?;

        if result.is_unit() || result.is::<()>() {
            return Ok(None);
        }

        let map = result
            .try_cast::<Map>()
            .context("Action command script must return a Map or () (unit)")?;

        let action = map
            .get("action")
            .map(|v: &Dynamic| v.to_string())
            .context("Missing 'action' in action_command result")?;
        let pump = map.get("pump").map(|v: &Dynamic| v.to_string());
        let dose_ml = map.get("dose_ml").and_then(|v: &Dynamic| {
            v.clone()
                .try_cast::<f32>()
                .or_else(|| v.clone().try_cast::<f64>().map(|f| f as f32))
        });
        let duration_sec = map
            .get("duration_sec")
            .and_then(|v: &Dynamic| v.clone().try_cast::<i64>())
            .map(|i| i as u64);
        let pwm = map
            .get("pwm")
            .and_then(|v: &Dynamic| v.clone().try_cast::<i64>())
            .map(|i| i as u32);

        Ok(Some(ActionCommandOutput {
            action,
            pump,
            dose_ml,
            pwm,
            duration_sec,
        }))
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookFieldMapping {
    pub body_path: String,
    pub target_field: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookMode {
    Flow,
    Direct,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerConfig {
    Sensor,
    Fsm,
    Cron {
        expression: String,
        timezone: String,
    },
    Webhook {
        mode: WebhookMode,
        field_mappings: Vec<WebhookFieldMapping>,
    },
}

impl TriggerConfig {
    /// Parse `ir_json.trigger` một cách khoan dung: bất kỳ hình dạng lạ/thiếu field
    /// nào cũng fallback về `Sensor` thay vì lỗi — script cũ (ir_json = NULL, hoặc
    /// ir_json không có key "trigger") vẫn phải chạy đúng như trước Phase 4/5.
    pub fn from_ir_json(ir_json: Option<&serde_json::Value>) -> Self {
        let Some(trigger) = ir_json.and_then(|ir| ir.get("trigger")) else {
            return TriggerConfig::Sensor;
        };
        match trigger.get("type").and_then(|v| v.as_str()) {
            Some("fsm") => TriggerConfig::Fsm,
            Some("cron") => {
                let expression = trigger.get("expression").and_then(|v| v.as_str());
                let timezone = trigger
                    .get("timezone")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Asia/Ho_Chi_Minh");
                match expression {
                    Some(expr) if !expr.trim().is_empty() => TriggerConfig::Cron {
                        expression: expr.to_string(),
                        timezone: timezone.to_string(),
                    },
                    _ => TriggerConfig::Sensor,
                }
            }
            Some("webhook") => {
                let mode = match trigger.get("mode").and_then(|v| v.as_str()) {
                    Some("direct") => WebhookMode::Direct,
                    _ => WebhookMode::Flow,
                };
                let field_mappings = trigger
                    .get("fieldMappings")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|m| {
                                let body_path = m.get("bodyPath")?.as_str()?.to_string();
                                let target_field = m.get("targetField")?.as_str()?.to_string();
                                Some(WebhookFieldMapping {
                                    body_path,
                                    target_field,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                TriggerConfig::Webhook {
                    mode,
                    field_mappings,
                }
            }
            _ => TriggerConfig::Sensor,
        }
    }
}

/// Một script đã compile, ready to eval
#[derive(Clone)]
pub struct CachedScript {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub ast: AST,
    /// Danh sách script IDs (dạng String) sẽ được eval tiếp sau khi script này
    /// fire thành công. Copy trực tiếp từ `UserScript::next_flow_ids` lúc load —
    /// xem `ScriptEval::eval_alert_scripts_chained` (script_eval.rs) cho logic dùng nó.
    pub next_flow_ids: Vec<String>,
    /// Mới (AUTOMATION-002): loại trigger parse từ `ir_json.trigger`.
    /// Dùng bởi scheduler cron (AUTOMATION-005) và router webhook (AUTOMATION-006).
    pub trigger: TriggerConfig,
}

/// Thread-safe cache: device_id → Vec<CachedScript>
/// Chia hai entry: "device_id:alert" và "device_id:recipe_override"
#[derive(Clone)]
pub struct ScriptCache {
    engine: Arc<ScriptEngine>,
    // key: "device_id:kind"
    inner: Arc<RwLock<HashMap<String, Vec<CachedScript>>>>,
}

impl ScriptCache {
    pub fn new(engine: Arc<ScriptEngine>) -> Self {
        Self {
            engine,
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Upsert compiled scripts cho một device, thay thế toàn bộ list.
    pub async fn upsert(&self, device_id: &str, scripts: Vec<CachedScript>) {
        let mut map = self.inner.write().await;
        // Group theo kind
        let mut alert_scripts = Vec::new();
        let mut override_scripts = Vec::new();
        let mut action_command_scripts = Vec::new();
        for s in scripts {
            match s.kind.as_str() {
                "alert" => alert_scripts.push(s),
                "recipe_override" => override_scripts.push(s),
                "action_command" => action_command_scripts.push(s),
                _ => {}
            }
        }
        map.insert(format!("{}:alert", device_id), alert_scripts);
        map.insert(format!("{}:recipe_override", device_id), override_scripts);
        map.insert(
            format!("{}:action_command", device_id),
            action_command_scripts,
        );
    }

    pub async fn get_alert_scripts(&self, device_id: &str) -> Vec<CachedScript> {
        let map = self.inner.read().await;
        map.get(&format!("{}:alert", device_id))
            .cloned()
            .unwrap_or_default()
    }

    pub async fn get_action_command_scripts(&self, device_id: &str) -> Vec<CachedScript> {
        let map = self.inner.read().await;
        map.get(&format!("{}:action_command", device_id))
            .cloned()
            .unwrap_or_default()
    }

    pub async fn get_recipe_override_scripts(&self, device_id: &str) -> Vec<CachedScript> {
        let map = self.inner.read().await;
        map.get(&format!("{}:recipe_override", device_id))
            .cloned()
            .unwrap_or_default()
    }

    /// Trả về (device_id, script) cho MỌI script có trigger Cron, xuyên toàn bộ device.
    /// Dùng bởi scheduler cron (AUTOMATION-005) — quét mỗi tick thay vì query DB.
    pub async fn list_cron_scripts(&self) -> Vec<(String, CachedScript)> {
        let map = self.inner.read().await;
        map.iter()
            .flat_map(|(key, scripts)| {
                let device_id = key
                    .rsplit_once(':')
                    .map(|(d, _)| d)
                    .unwrap_or(key)
                    .to_string();
                scripts
                    .iter()
                    .filter(|s| matches!(s.trigger, TriggerConfig::Cron { .. }))
                    .map(move |s| (device_id.clone(), s.clone()))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Trả về mọi script có trigger Webhook của 1 device. Dùng bởi
    /// AUTOMATION-006 khi nhận payload webhook.
    pub async fn get_webhook_scripts(&self, device_id: &str) -> Vec<CachedScript> {
        let map = self.inner.read().await;
        [
            format!("{}:alert", device_id),
            format!("{}:action_command", device_id),
        ]
        .iter()
        .flat_map(|key| map.get(key).cloned().unwrap_or_default())
        .filter(|s| matches!(s.trigger, TriggerConfig::Webhook { .. }))
        .collect()
    }

    /// Load tất cả enabled scripts cho device từ DB, compile, upsert vào cache.
    pub async fn reload_device(&self, pool: &PgPool, device_id: &str) -> Result<usize> {
        let rows = sqlx::query_as::<_, crate::models::script::UserScript>(
            "SELECT * FROM user_scripts WHERE device_id = $1 AND enabled = TRUE ORDER BY created_at",
        )
        .bind(device_id)
        .fetch_all(pool)
        .await
        .context("Failed to load scripts from DB")?;

        let count = rows.len();
        let mut compiled = Vec::with_capacity(count);
        for row in rows {
            match self.engine.compile(&row.source) {
                Ok(ast) => compiled.push(CachedScript {
                    id: row.id,
                    kind: row.kind,
                    name: row.name,
                    ast,
                    next_flow_ids: row.next_flow_ids.clone(),
                    trigger: TriggerConfig::from_ir_json(row.ir_json.as_ref()),
                }),
                Err(e) => {
                    tracing::warn!(
                        script_id = %row.id,
                        script_name = %row.name,
                        device_id,
                        error = %e,
                        "Skipping script with compile error"
                    );
                }
            }
        }
        self.upsert(device_id, compiled).await;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn eval_recipe_override_defaults_to_advance_stage_when_action_key_absent() {
        // Backward-compat: script cũ (trước Phase 3) không có key "action" — vẫn phải
        // hiểu là advance_stage như trước, không được coi là lỗi hay bỏ qua.
        let engine = ScriptEngine::new();
        let src = r#"fn main(input) { #{ target_stage_index: 2, reason: "Đủ 24h" } }"#;
        let ast = engine.compile(src).unwrap();
        let input = ScriptFsmInput {
            phase: "Monitoring".into(),
            stage_index: 1,
            ec: 1.5,
            ph: 6.5,
            elapsed_sec: 90000,
        };
        let result = engine.eval_recipe_override(&ast, &input).unwrap().unwrap();
        match result {
            RecipeOverrideOutput::AdvanceStage(s) => {
                assert_eq!(s.target_stage_index, 2);
                assert_eq!(s.reason, "Đủ 24h");
            }
            RecipeOverrideOutput::EndSeason { .. } => panic!("expected AdvanceStage"),
        }
    }

    #[test]
    fn eval_recipe_override_reads_explicit_end_season_action() {
        let engine = ScriptEngine::new();
        let src = r#"fn main(input) { #{ action: "end_season", reason: "Hết vụ" } }"#;
        let ast = engine.compile(src).unwrap();
        let input = ScriptFsmInput {
            phase: "Monitoring".into(),
            stage_index: 3,
            ec: 1.5,
            ph: 6.5,
            elapsed_sec: 90000,
        };
        let result = engine.eval_recipe_override(&ast, &input).unwrap().unwrap();
        match result {
            RecipeOverrideOutput::EndSeason { reason } => assert_eq!(reason, "Hết vụ"),
            RecipeOverrideOutput::AdvanceStage(_) => panic!("expected EndSeason"),
        }
    }

    #[test]
    fn compiles_valid_alert_script() {
        let engine = ScriptEngine::new();
        let src =
            r#"fn main(input) { #{level: "warning", title: "pH cao", message: "pH vượt 7.5"} }"#;
        assert!(engine.compile(src).is_ok());
    }

    #[test]
    fn eval_alert_script_with_nested_condition_group_compiled_rhai() {
        let engine = ScriptEngine::new();
        let src = r#"
fn main(input) {
 if !((input.ph < 5.5 || input.ph > 7.5) && input.ec > 3.0) { return (); }
 #{
  "level": "warning",
  "title": "pH bất thường",
  "message": "pH bất thường"
 }
}
"#;
        let ast = engine.compile(src).unwrap();
        let input_matching = ScriptSensorInput {
            ph: 8.0,
            ec: 3.5,
            temp: 25.0,
            water_level: 80.0,
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        let alert = engine.eval_alert(&ast, &input_matching).unwrap().unwrap();
        assert_eq!(alert.title, "pH bất thường");

        let input_non_matching = ScriptSensorInput {
            ph: 6.5,
            ec: 3.5,
            temp: 25.0,
            water_level: 80.0,
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        assert!(
            engine
                .eval_alert(&ast, &input_non_matching)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn compile_returns_error_for_syntax_error() {
        let engine = ScriptEngine::new();
        let src = r#"fn main(input { }"#; // missing closing paren
        assert!(engine.compile(src).is_err());
    }

    #[test]
    fn eval_alert_script_returns_none_when_condition_not_met() {
        let engine = ScriptEngine::new();
        let src = r#"
fn main(input) {
    if input.ph < 7.5 { return (); }
    #{ level: "warning", title: "pH cao", message: `pH = ${input.ph}` }
}
"#;
        let ast = engine.compile(src).unwrap();
        let input = ScriptSensorInput {
            ph: 6.8,
            ec: 1.5,
            temp: 25.0,
            water_level: 80.0,
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        let result = engine.eval_alert(&ast, &input).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn eval_alert_script_returns_alert_when_condition_met() {
        let engine = ScriptEngine::new();
        let src = r#"
fn main(input) {
    if input.ph < 7.5 { return (); }
    #{ level: "warning", title: "pH cao", message: `pH = ${input.ph}` }
}
"#;
        let ast = engine.compile(src).unwrap();
        let input = ScriptSensorInput {
            ph: 7.8,
            ec: 1.5,
            temp: 25.0,
            water_level: 80.0,
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        let result = engine.eval_alert(&ast, &input).unwrap();
        assert!(result.is_some());
        let alert = result.unwrap();
        assert_eq!(alert.level, "warning");
    }

    #[test]
    fn eval_recipe_override_returns_none_when_no_advance() {
        let engine = ScriptEngine::new();
        let src = r#"
fn main(input) {
    if input.elapsed_sec < 86400 { return (); }
    #{ target_stage_index: input.stage_index + 1, reason: "Đủ 24h" }
}
"#;
        let ast = engine.compile(src).unwrap();
        let input = ScriptFsmInput {
            phase: "Monitoring".into(),
            stage_index: 0,
            ec: 1.5,
            ph: 6.5,
            elapsed_sec: 3600,
        };
        let result = engine.eval_recipe_override(&ast, &input).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn eval_action_command_returns_none_when_condition_not_met() {
        let engine = ScriptEngine::new();
        let src = r#"
fn main(input) {
    if input.ph < 7.5 { return (); }
    #{ action: "dose", pump: "ph_down", dose_ml: 3.0 }
}
"#;
        let ast = engine.compile(src).unwrap();
        let input = ScriptActionInput {
            ph: 6.8,
            ec: 1.5,
            temp: 25.0,
            water_level: 80.0,
            phase: "Monitoring".into(),
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        let result = engine.eval_action_command(&ast, &input).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn eval_action_command_returns_command_when_condition_met() {
        let engine = ScriptEngine::new();
        let src = r#"
fn main(input) {
    if input.ph < 7.5 { return (); }
    #{ action: "dose", pump: "ph_down", dose_ml: 3.0 }
}
"#;
        let ast = engine.compile(src).unwrap();
        let input = ScriptActionInput {
            ph: 8.0,
            ec: 1.5,
            temp: 25.0,
            water_level: 80.0,
            phase: "Monitoring".into(),
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        let result = engine.eval_action_command(&ast, &input).unwrap().unwrap();
        assert_eq!(result.action, "dose");
        assert_eq!(result.pump.as_deref(), Some("ph_down"));
        assert_eq!(result.dose_ml, Some(3.0));
    }

    #[test]
    fn eval_action_command_reads_pwm_field() {
        let engine = ScriptEngine::new();
        let src = r#"
        fn main(input) {
            #{ action: "dose", pump: "ph_down", dose_ml: 3.0, pwm: 80 }
        }
        "#;
        let ast = engine.compile(src).unwrap();
        let input = ScriptActionInput {
            ph: 8.0,
            ec: 1.5,
            temp: 25.0,
            water_level: 80.0,
            phase: "Monitoring".into(),
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        let result = engine.eval_action_command(&ast, &input).unwrap().unwrap();
        assert_eq!(result.pwm, Some(80));
    }

    #[test]
    fn eval_action_command_pwm_is_none_when_absent() {
        let engine = ScriptEngine::new();
        let src = r#"fn main(input) { #{ action: "water_on", pump: "WATER_PUMP_IN", duration_sec: 10 } }"#;
        let ast = engine.compile(src).unwrap();
        let input = ScriptActionInput {
            ph: 6.5,
            ec: 1.5,
            temp: 25.0,
            water_level: 80.0,
            phase: "Monitoring".into(),
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        let result = engine.eval_action_command(&ast, &input).unwrap().unwrap();
        assert_eq!(result.pwm, None);
    }

    #[test]
    fn trigger_config_defaults_to_sensor_when_ir_json_is_none() {
        let cfg = TriggerConfig::from_ir_json(None);
        assert_eq!(cfg, TriggerConfig::Sensor);
    }

    #[test]
    fn trigger_config_defaults_to_sensor_when_trigger_field_missing() {
        let ir = serde_json::json!({ "kind": "alert", "conditions": [], "actions": [] });
        let cfg = TriggerConfig::from_ir_json(Some(&ir));
        assert_eq!(cfg, TriggerConfig::Sensor);
    }

    #[test]
    fn trigger_config_parses_fsm() {
        let ir = serde_json::json!({ "trigger": { "type": "fsm" } });
        assert_eq!(TriggerConfig::from_ir_json(Some(&ir)), TriggerConfig::Fsm);
    }

    #[test]
    fn trigger_config_parses_cron_with_expression_and_timezone() {
        let ir = serde_json::json!({
            "trigger": { "type": "cron", "expression": "0 7 * * *", "timezone": "Asia/Ho_Chi_Minh" }
        });
        assert_eq!(
            TriggerConfig::from_ir_json(Some(&ir)),
            TriggerConfig::Cron {
                expression: "0 7 * * *".to_string(),
                timezone: "Asia/Ho_Chi_Minh".to_string(),
            }
        );
    }

    #[test]
    fn trigger_config_falls_back_to_sensor_on_malformed_cron() {
        let ir = serde_json::json!({ "trigger": { "type": "cron" } });
        assert_eq!(
            TriggerConfig::from_ir_json(Some(&ir)),
            TriggerConfig::Sensor
        );
    }

    #[test]
    fn trigger_config_parses_webhook_with_mappings() {
        let ir = serde_json::json!({
            "trigger": {
                "type": "webhook",
                "mode": "flow",
                "fieldMappings": [{ "bodyPath": "external_alarm", "targetField": "external_alarm" }]
            }
        });
        match TriggerConfig::from_ir_json(Some(&ir)) {
            TriggerConfig::Webhook {
                mode,
                field_mappings,
            } => {
                assert_eq!(mode, WebhookMode::Flow);
                assert_eq!(field_mappings.len(), 1);
                assert_eq!(field_mappings[0].body_path, "external_alarm");
            }
            other => panic!("expected Webhook, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_cron_scripts_returns_only_cron_triggered_across_devices() {
        let engine = Arc::new(ScriptEngine::new());
        let cache = ScriptCache::new(engine.clone());
        let cron_script = CachedScript {
            id: Uuid::new_v4(),
            kind: "alert".to_string(),
            name: "Tưới sáng".to_string(),
            ast: engine.compile("fn main(input) { () }").unwrap(),
            next_flow_ids: vec![],
            trigger: TriggerConfig::Cron {
                expression: "0 7 * * *".into(),
                timezone: "Asia/Ho_Chi_Minh".into(),
            },
        };
        let sensor_script = CachedScript {
            trigger: TriggerConfig::Sensor,
            ..cron_script.clone()
        };
        cache
            .upsert("device-a", vec![cron_script.clone(), sensor_script])
            .await;

        let result = cache.list_cron_scripts().await;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "device-a");
        assert_eq!(result[0].1.id, cron_script.id);
    }

    #[tokio::test]
    async fn script_cache_preserves_next_flow_ids() {
        let engine = Arc::new(ScriptEngine::new());
        let cache = ScriptCache::new(engine.clone());
        let script = CachedScript {
            id: uuid::Uuid::new_v4(),
            kind: "alert".to_string(),
            name: "root".to_string(),
            ast: engine.compile(r#"fn main(input) { () }"#).unwrap(),
            next_flow_ids: vec!["child-id".to_string()],
            trigger: TriggerConfig::Sensor,
        };
        cache.upsert("device_001", vec![script]).await;

        let scripts = cache.get_alert_scripts("device_001").await;
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].next_flow_ids, vec!["child-id".to_string()]);
    }

    #[tokio::test]
    async fn script_cache_compiles_and_retrieves_by_device() {
        let engine = Arc::new(ScriptEngine::new());
        let cache = ScriptCache::new(engine.clone());

        let script = CachedScript {
            id: uuid::Uuid::new_v4(),
            kind: "alert".to_string(),
            name: "test".to_string(),
            ast: engine.compile(r#"fn main(input) { () }"#).unwrap(),
            next_flow_ids: vec![],
            trigger: TriggerConfig::Sensor,
        };
        cache.upsert("device_001", vec![script]).await;

        let scripts = cache.get_alert_scripts("device_001").await;
        assert_eq!(scripts.len(), 1);

        let scripts_other = cache.get_alert_scripts("device_999").await;
        assert_eq!(scripts_other.len(), 0);
    }
}
