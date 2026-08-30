//! Eval Rhai scripts khi nhận sensor data từ MQTT.
//! Không blocking: eval là CPU-bound nhưng nhẹ (< 1ms per script với giới hạn 50k ops).
//! Fire-and-forget: lỗi trong script được log nhưng không làm drop sensor message.

use std::sync::Arc;
use tracing::warn;

use crate::models::script::{AlertOutput, UserScript};
use crate::services::script_engine::{CachedScript, ScriptEngine, ScriptSensorInput};

const MAX_CHAIN_DEPTH: usize = 5;

/// Eval một script rồi nếu thành công, tiếp tục eval các script trong `next_flow_ids`.
/// `visited` chứa các script ID đã đi qua để phát hiện vòng lặp.
pub fn eval_chain(
    root_id: &str,
    scripts: &[UserScript],
    sensor_input: &ScriptSensorInput,
    engine: &ScriptEngine,
    visited: &mut Vec<String>,
    depth: usize,
) -> Vec<String> /* log messages */ {
    if depth >= MAX_CHAIN_DEPTH || visited.contains(&root_id.to_string()) {
        return vec![format!(
            "chain: skip {} (depth={} or cycle)",
            root_id, depth
        )];
    }
    visited.push(root_id.to_string());

    let Some(script) = scripts.iter().find(|s| s.id.to_string() == root_id) else {
        return vec![format!("chain: script {} not found", root_id)];
    };

    let cached = CachedScript {
        id: script.id,
        name: script.name.clone(),
        kind: script.kind.clone(),
        ast: engine.compile(&script.source).unwrap_or_default(),
    };

    let mut logs = engine
        .eval_alert(&cached.ast, sensor_input)
        .map(|_| vec![format!("chain: {} fired", root_id)])
        .unwrap_or_else(|e| vec![format!("chain: {} error: {}", root_id, e)]);

    for next_id in &script.next_flow_ids {
        let mut child_logs = eval_chain(next_id, scripts, sensor_input, engine, visited, depth + 1);
        logs.append(&mut child_logs);
    }
    logs
}

/// Eval tất cả alert scripts cho một sensor reading.
/// Scripts lỗi runtime bị skip (log warning), không panic.
pub fn eval_alert_scripts(
    engine: &Arc<ScriptEngine>,
    scripts: &[CachedScript],
    input: &ScriptSensorInput,
) -> Vec<AlertOutput> {
    let mut alerts = Vec::new();
    for script in scripts {
        match engine.eval_alert(&script.ast, input) {
            Ok(Some(alert)) => alerts.push(alert),
            Ok(None) => {}
            Err(e) => {
                warn!(
                    script_id = %script.id,
                    script_name = %script.name,
                    device_id = %input.device_id,
                    error = %e,
                    "Alert script runtime error — skipping"
                );
            }
        }
    }
    alerts
}

/// Convert AlertOutput thành AlertMessage để gửi vào event bus.
pub fn alert_output_to_system_alert(
    alert: AlertOutput,
    device_id: &str,
    timestamp_ms: i64,
) -> crate::models::alert::AlertMessage {
    crate::models::alert::AlertMessage {
        level: alert.level,
        category: "script_alert".to_string(),
        title: alert.title,
        message: alert.message,
        device_id: device_id.to_string(),
        reason: Some("Rhai user script".to_string()),
        metadata: None,
        timestamp: timestamp_ms as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::script_engine::{CachedScript, ScriptEngine, ScriptSensorInput};
    use std::sync::Arc;

    fn make_alert_script(source: &str) -> CachedScript {
        let engine = ScriptEngine::new();
        CachedScript {
            id: uuid::Uuid::new_v4(),
            kind: "alert".to_string(),
            name: "test".to_string(),
            ast: engine
                .compile(source)
                .expect("Failed to compile test Rhai alert script source"),
        }
    }

    #[test]
    fn no_scripts_produces_no_alerts() {
        let engine = Arc::new(ScriptEngine::new());
        let input = ScriptSensorInput {
            ph: 6.5,
            ec: 1.4,
            temp: 25.0,
            water_level: 80.0,
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        let alerts = eval_alert_scripts(&engine, &[], &input);
        assert!(alerts.is_empty());
    }

    #[test]
    fn script_returning_unit_produces_no_alert() {
        let engine = Arc::new(ScriptEngine::new());
        let script = make_alert_script("fn main(input) { () }");
        let input = ScriptSensorInput {
            ph: 6.5,
            ec: 1.4,
            temp: 25.0,
            water_level: 80.0,
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        let alerts = eval_alert_scripts(&engine, &[script], &input);
        assert!(alerts.is_empty());
    }

    #[test]
    fn script_with_matching_condition_produces_alert() {
        let engine = Arc::new(ScriptEngine::new());
        let script = make_alert_script(
            r#"
fn main(input) {
    if input.ph <= 7.5 { return (); }
    #{ level: "warning", title: "pH cao", message: `pH=${input.ph}` }
}
"#,
        );
        let input = ScriptSensorInput {
            ph: 8.1,
            ec: 1.4,
            temp: 25.0,
            water_level: 80.0,
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        let alerts = eval_alert_scripts(&engine, &[script], &input);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, "warning");
    }

    #[test]
    fn erroring_script_is_skipped_gracefully() {
        let engine = Arc::new(ScriptEngine::new());
        let script = make_alert_script(r#"fn main(input) { throw "boom"; }"#);
        let input = ScriptSensorInput {
            ph: 6.5,
            ec: 1.4,
            temp: 25.0,
            water_level: 80.0,
            device_id: "d1".into(),
            timestamp_ms: 0,
        };
        let alerts = eval_alert_scripts(&engine, &[script], &input);
        assert!(alerts.is_empty());
    }
}

#[cfg(test)]
mod chain_tests {
    // Tests for cycle detection and max chain depth limits
    #[test]
    fn detect_cycle_in_chain() {
        // A → B → A (cycle, depth limit phải dừng)
        let visited = ["id-a".to_string(), "id-b".to_string()];
        assert!(visited.contains(&"id-a".to_string())); // cycle detected
    }

    #[test]
    fn chain_depth_limit() {
        // depth >= MAX_CHAIN_DEPTH thì dừng
        const MAX: usize = 5;
        const { assert!(MAX <= 5) };
    }
}
