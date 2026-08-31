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

/// Một script đã compile, ready to eval
#[derive(Clone)]
pub struct CachedScript {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub ast: AST,
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

    #[tokio::test]
    async fn script_cache_compiles_and_retrieves_by_device() {
        let engine = Arc::new(ScriptEngine::new());
        let cache = ScriptCache::new(engine.clone());

        let script = CachedScript {
            id: uuid::Uuid::new_v4(),
            kind: "alert".to_string(),
            name: "test".to_string(),
            ast: engine.compile(r#"fn main(input) { () }"#).unwrap(),
        };
        cache.upsert("device_001", vec![script]).await;

        let scripts = cache.get_alert_scripts("device_001").await;
        assert_eq!(scripts.len(), 1);

        let scripts_other = cache.get_alert_scripts("device_999").await;
        assert_eq!(scripts_other.len(), 0);
    }
}
