use rhai::{AST, Dynamic, Engine, EvalAltResult, Map, Scope};
use uuid::Uuid;

use crate::models::script::AlertOutput;

pub struct ScriptEngine {
    engine: Engine,
}

#[derive(Debug, Clone)]
pub struct CachedScript {
    pub id: Uuid,
    pub kind: String,
    pub name: String,
    pub ast: AST,
}

#[derive(Debug, Clone)]
pub struct ScriptSensorInput {
    pub ph: f32,
    pub ec: f32,
    pub temp: f32,
    pub water_level: f32,
    pub device_id: String,
    pub timestamp_ms: i64,
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        engine.set_max_operations(50_000);
        Self { engine }
    }

    pub fn compile(&self, source: &str) -> Result<AST, rhai::ParseError> {
        self.engine.compile(source)
    }

    pub fn eval_alert(
        &self,
        ast: &AST,
        input: &ScriptSensorInput,
    ) -> Result<Option<AlertOutput>, Box<EvalAltResult>> {
        let mut scope = Scope::new();
        let mut map = Map::new();
        map.insert("ph".into(), Dynamic::from(input.ph));
        map.insert("ec".into(), Dynamic::from(input.ec));
        map.insert("temp".into(), Dynamic::from(input.temp));
        map.insert("water_level".into(), Dynamic::from(input.water_level));
        map.insert("device_id".into(), Dynamic::from(input.device_id.clone()));
        map.insert("timestamp_ms".into(), Dynamic::from(input.timestamp_ms));

        let result: Dynamic = self.engine.call_fn(&mut scope, ast, "main", (map,))?;

        if result.is_unit() {
            return Ok(None);
        }

        if let Some(alert_map) = result.try_cast::<Map>() {
            let level = alert_map
                .get("level")
                .and_then(|v| v.clone().try_cast::<String>())
                .unwrap_or_else(|| "info".to_string());
            let title = alert_map
                .get("title")
                .and_then(|v| v.clone().try_cast::<String>())
                .unwrap_or_default();
            let message = alert_map
                .get("message")
                .and_then(|v| v.clone().try_cast::<String>())
                .unwrap_or_default();

            Ok(Some(AlertOutput {
                level,
                title,
                message,
            }))
        } else {
            Ok(None)
        }
    }
}
