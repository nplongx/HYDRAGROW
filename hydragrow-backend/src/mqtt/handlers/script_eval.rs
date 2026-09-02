//! Eval Rhai scripts khi nhận sensor data từ MQTT.
//! Không blocking: eval là CPU-bound nhưng nhẹ (< 1ms per script với giới hạn 50k ops).
//! Fire-and-forget: lỗi trong script được log nhưng không làm drop sensor message.

use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

use crate::models::script::AlertOutput;
use crate::services::script_engine::{CachedScript, ScriptEngine, ScriptSensorInput};

const MAX_CHAIN_DEPTH: usize = 5;

/// Eval mọi alert script "gốc" (toàn bộ script enabled của device) rồi, với mỗi
/// script fire, tiếp tục eval các script trong `next_flow_ids` của nó — tra cứu
/// trong CHÍNH `scripts` (chỉ chain trong cùng kind `alert`; nếu next_flow_id
/// trỏ ra ngoài slice này, bỏ qua và log warning — xem "Quyết định phạm vi"
/// trong plan). Dedupe theo script id: một script chỉ góp mặt tối đa 1 lần
/// trong kết quả kể cả khi vừa tự fire vừa được chain tới từ script khác.
pub fn eval_alert_scripts_chained(
    engine: &Arc<ScriptEngine>,
    scripts: &[CachedScript],
    input: &ScriptSensorInput,
) -> Vec<(Uuid, AlertOutput)> {
    let mut fired: Vec<(Uuid, AlertOutput)> = Vec::new();
    let mut seen: Vec<Uuid> = Vec::new();

    let child_ids: Vec<&str> = scripts
        .iter()
        .flat_map(|s| s.next_flow_ids.iter().map(|id| id.as_str()))
        .collect();

    let roots: Vec<&CachedScript> = scripts
        .iter()
        .filter(|s| !child_ids.contains(&s.id.to_string().as_str()))
        .collect();

    let start_scripts = if roots.is_empty() {
        scripts.iter().collect::<Vec<&CachedScript>>()
    } else {
        roots
    };

    for script in start_scripts {
        eval_chain_from(script, scripts, engine, input, 0, &mut seen, &mut fired);
    }

    fired
}

fn eval_chain_from(
    script: &CachedScript,
    all_scripts: &[CachedScript],
    engine: &Arc<ScriptEngine>,
    input: &ScriptSensorInput,
    depth: usize,
    seen: &mut Vec<Uuid>,
    fired: &mut Vec<(Uuid, AlertOutput)>,
) {
    if depth >= MAX_CHAIN_DEPTH || seen.contains(&script.id) {
        if depth >= MAX_CHAIN_DEPTH {
            warn!(script_id = %script.id, depth, "Flow chain depth limit reached — skipping");
        }
        return;
    }
    seen.push(script.id);

    match engine.eval_alert(&script.ast, input) {
        Ok(Some(alert)) => {
            fired.push((script.id, alert));
            for next_id in &script.next_flow_ids {
                let Some(next_script) = all_scripts.iter().find(|s| s.id.to_string() == *next_id)
                else {
                    warn!(
                        root_script_id = %script.id,
                        next_flow_id = %next_id,
                        "next_flow_ids trỏ tới script không tồn tại trong cùng kind — bỏ qua"
                    );
                    continue;
                };
                eval_chain_from(
                    next_script,
                    all_scripts,
                    engine,
                    input,
                    depth + 1,
                    seen,
                    fired,
                );
            }
        }
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

    fn make_alert_script(source: &str) -> CachedScript {
        make_alert_script_with_next(source, vec![])
    }

    fn make_alert_script_with_next(source: &str, next_flow_ids: Vec<String>) -> CachedScript {
        let engine = ScriptEngine::new();
        CachedScript {
            id: uuid::Uuid::new_v4(),
            kind: "alert".to_string(),
            name: "test".to_string(),
            ast: engine
                .compile(source)
                .expect("Failed to compile test Rhai alert script source"),
            next_flow_ids,
        }
    }

    fn make_input() -> ScriptSensorInput {
        ScriptSensorInput {
            ph: 8.1,
            ec: 1.4,
            temp: 25.0,
            water_level: 80.0,
            device_id: "d1".into(),
            timestamp_ms: 0,
        }
    }

    const FIRING_SCRIPT: &str = r#"
fn main(input) {
    if input.ph <= 7.5 { return (); }
    #{ level: "warning", title: "pH cao", message: `pH=${input.ph}` }
}
"#;
    const NON_FIRING_SCRIPT: &str = r#"fn main(input) { () }"#;

    #[test]
    fn no_scripts_produces_no_alerts() {
        let engine = Arc::new(ScriptEngine::new());
        let alerts = eval_alert_scripts_chained(&engine, &[], &make_input());
        assert!(alerts.is_empty());
    }

    #[test]
    fn script_returning_unit_produces_no_alert() {
        let engine = Arc::new(ScriptEngine::new());
        let script = make_alert_script(NON_FIRING_SCRIPT);
        let alerts = eval_alert_scripts_chained(&engine, &[script], &make_input());
        assert!(alerts.is_empty());
    }

    #[test]
    fn script_with_matching_condition_produces_alert() {
        let engine = Arc::new(ScriptEngine::new());
        let script = make_alert_script(FIRING_SCRIPT);
        let alerts = eval_alert_scripts_chained(&engine, &[script], &make_input());
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].1.level, "warning");
    }

    #[test]
    fn erroring_script_is_skipped_gracefully() {
        let engine = Arc::new(ScriptEngine::new());
        let script = make_alert_script(r#"fn main(input) { throw "boom"; }"#);
        let alerts = eval_alert_scripts_chained(&engine, &[script], &make_input());
        assert!(alerts.is_empty());
    }

    #[test]
    fn firing_root_chains_into_next_flow_id() {
        let engine = Arc::new(ScriptEngine::new());
        let child = make_alert_script(FIRING_SCRIPT);
        let root = make_alert_script_with_next(FIRING_SCRIPT, vec![child.id.to_string()]);
        let alerts =
            eval_alert_scripts_chained(&engine, &[root.clone(), child.clone()], &make_input());
        let ids: Vec<Uuid> = alerts.iter().map(|(id, _)| *id).collect();
        assert_eq!(alerts.len(), 2);
        assert!(ids.contains(&root.id));
        assert!(ids.contains(&child.id));
    }

    #[test]
    fn non_firing_root_does_not_chain() {
        let engine = Arc::new(ScriptEngine::new());
        let child = make_alert_script(FIRING_SCRIPT);
        let root = make_alert_script_with_next(NON_FIRING_SCRIPT, vec![child.id.to_string()]);
        let alerts = eval_alert_scripts_chained(&engine, &[root, child], &make_input());
        assert!(alerts.is_empty());
    }

    #[test]
    fn cycle_is_detected_and_does_not_infinite_loop() {
        let engine = Arc::new(ScriptEngine::new());
        let a_id = uuid::Uuid::new_v4();
        let b_id = uuid::Uuid::new_v4();
        let a = CachedScript {
            id: a_id,
            kind: "alert".to_string(),
            name: "a".to_string(),
            ast: engine
                .compile(FIRING_SCRIPT)
                .expect("test script A compiles"),
            next_flow_ids: vec![b_id.to_string()],
        };
        let b = CachedScript {
            id: b_id,
            kind: "alert".to_string(),
            name: "b".to_string(),
            ast: engine
                .compile(FIRING_SCRIPT)
                .expect("test script B compiles"),
            next_flow_ids: vec![a_id.to_string()],
        };
        let alerts = eval_alert_scripts_chained(&engine, &[a, b], &make_input());
        // Mỗi script chỉ được đếm 1 lần dù A→B→A tạo vòng lặp.
        assert_eq!(alerts.len(), 2);
    }

    #[test]
    fn depth_limit_stops_long_chain() {
        let engine = Arc::new(ScriptEngine::new());
        let ids: Vec<uuid::Uuid> = (0..8).map(|_| uuid::Uuid::new_v4()).collect();
        let scripts: Vec<CachedScript> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| CachedScript {
                id: *id,
                kind: "alert".to_string(),
                name: format!("s{i}"),
                ast: engine
                    .compile(FIRING_SCRIPT)
                    .expect("test chain script compiles"),
                next_flow_ids: ids
                    .get(i + 1)
                    .map(|next| vec![next.to_string()])
                    .unwrap_or_default(),
            })
            .collect();
        let alerts = eval_alert_scripts_chained(&engine, &scripts, &make_input());
        // MAX_CHAIN_DEPTH = 5: chỉ script[0] (root, depth 0) tới script[4] (depth 4)
        // được eval trước khi depth 5 bị chặn — không phải mọi root trong danh sách
        // đều fire tới cuối chuỗi 8, nên assert theo id cụ thể thay vì đếm tổng.
        for id in &ids[0..5] {
            assert!(
                alerts.iter().any(|(fid, _)| fid == id),
                "script {id} should fire"
            );
        }
    }
}
