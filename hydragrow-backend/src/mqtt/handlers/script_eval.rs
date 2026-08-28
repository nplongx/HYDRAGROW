//! Eval Rhai scripts khi nhận sensor data từ MQTT.
//! Không blocking: eval là CPU-bound nhưng nhẹ (< 1ms per script với giới hạn 50k ops).
//! Fire-and-forget: lỗi trong script được log nhưng không làm drop sensor message.

use std::sync::Arc;
use tracing::warn;

use crate::models::script::AlertOutput;
use crate::services::script_engine::{CachedScript, ScriptEngine, ScriptSensorInput};

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
            ast: engine.compile(source).unwrap(),
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
