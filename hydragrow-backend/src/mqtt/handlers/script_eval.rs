//! Eval Rhai scripts khi nhận sensor data từ MQTT.
//! Không blocking: eval là CPU-bound nhưng nhẹ (< 1ms per script với giới hạn 50k ops).
//! Fire-and-forget: lỗi trong script được log nhưng không làm drop sensor message.

use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

use crate::models::script::{
    ActionCommandOutput, AlertOutput, RecipeOverrideOutput, ScriptActionInput, ScriptKind,
    SensorSnapshot,
};
use crate::services::script_engine::{CachedScript, ScriptEngine, ScriptSensorInput};

const MAX_CHAIN_DEPTH: usize = 5;

/// Node trong đồ thị flow chain đa-kind
#[derive(Clone)]
pub struct ChainNode {
    pub id: Uuid,
    pub kind: ScriptKind,
    pub next_flow_ids: Vec<String>,
    pub ast: rhai::AST,
}

/// Kết quả eval 1 node trong chain, đủ đa hình để caller (sensors.rs) biết làm gì tiếp:
/// alert -> ghi event bus/DB/FCM
/// action_command -> dispatch_action_command
/// recipe_override -> logic ở fsm.rs
#[derive(Debug, Clone, PartialEq)]
pub enum ChainFireResult {
    Alert(AlertOutput),
    ActionCommand(ActionCommandOutput),
    RecipeOverride(RecipeOverrideOutput),
}

/// Eval mọi root node của đồ thị Flow chain (toàn bộ script enabled thuộc device).
/// Khi một node fire, tiếp tục eval các script trong `next_flow_ids` của nó (kể cả cross-kind).
/// Dedupe theo script id: một script chỉ góp mặt tối đa 1 lần trong kết quả.
pub fn eval_flow_chain(
    engine: &Arc<ScriptEngine>,
    all_scripts: &[ChainNode],
    snapshot: &SensorSnapshot,
) -> Vec<(Uuid, ChainFireResult)> {
    let mut fired: Vec<(Uuid, ChainFireResult)> = Vec::new();
    let mut seen: Vec<Uuid> = Vec::new();

    let child_ids: Vec<&str> = all_scripts
        .iter()
        .flat_map(|s| s.next_flow_ids.iter().map(|id| id.as_str()))
        .collect();

    let roots: Vec<&ChainNode> = all_scripts
        .iter()
        .filter(|s| !child_ids.contains(&s.id.to_string().as_str()))
        .collect();

    let start_scripts = if roots.is_empty() {
        all_scripts.iter().collect::<Vec<&ChainNode>>()
    } else {
        roots
    };

    for script in start_scripts {
        eval_flow_chain_from(
            script,
            all_scripts,
            engine,
            snapshot,
            0,
            &mut seen,
            &mut fired,
        );
    }

    fired
}

fn eval_flow_chain_from(
    node: &ChainNode,
    all_scripts: &[ChainNode],
    engine: &Arc<ScriptEngine>,
    snapshot: &SensorSnapshot,
    depth: usize,
    seen: &mut Vec<Uuid>,
    fired: &mut Vec<(Uuid, ChainFireResult)>,
) {
    if depth >= MAX_CHAIN_DEPTH || seen.contains(&node.id) {
        if depth >= MAX_CHAIN_DEPTH {
            warn!(script_id = %node.id, depth, "Flow chain depth limit reached — skipping");
        }
        return;
    }
    seen.push(node.id);

    let fired_result = match node.kind {
        ScriptKind::Alert => {
            let input = ScriptSensorInput {
                ph: snapshot.ph,
                ec: snapshot.ec,
                temp: snapshot.temp,
                water_level: snapshot.water_level,
                device_id: snapshot.device_id.clone(),
                timestamp_ms: snapshot.timestamp_ms,
            };
            match engine.eval_alert(&node.ast, &input) {
                Ok(Some(alert)) => Some(ChainFireResult::Alert(alert)),
                Ok(None) => None,
                Err(e) => {
                    warn!(script_id = %node.id, device_id = %snapshot.device_id, error = %e, "Alert script runtime error — skipping");
                    None
                }
            }
        }
        ScriptKind::ActionCommand => {
            let input = ScriptActionInput {
                ph: snapshot.ph,
                ec: snapshot.ec,
                temp: snapshot.temp,
                water_level: snapshot.water_level,
                phase: snapshot.phase.clone(),
                device_id: snapshot.device_id.clone(),
                timestamp_ms: snapshot.timestamp_ms,
            };
            match engine.eval_action_command(&node.ast, &input) {
                Ok(Some(cmd)) => Some(ChainFireResult::ActionCommand(cmd)),
                Ok(None) => None,
                Err(e) => {
                    warn!(script_id = %node.id, device_id = %snapshot.device_id, error = %e, "ActionCommand script runtime error — skipping");
                    None
                }
            }
        }
        ScriptKind::RecipeOverride => {
            warn!(script_id = %node.id, "RecipeOverride script in sensor flow chain — skipping");
            None
        }
    };

    if let Some(res) = fired_result {
        fired.push((node.id, res));
        for next_id in &node.next_flow_ids {
            let Some(next_script) = all_scripts.iter().find(|s| s.id.to_string() == *next_id)
            else {
                warn!(
                    root_script_id = %node.id,
                    next_flow_id = %next_id,
                    "next_flow_ids trỏ tới script không tồn tại — bỏ qua"
                );
                continue;
            };
            eval_flow_chain_from(
                next_script,
                all_scripts,
                engine,
                snapshot,
                depth + 1,
                seen,
                fired,
            );
        }
    }
}

/// Backward-compatible wrapper cho eval_alert_scripts_chained
pub fn eval_alert_scripts_chained(
    engine: &Arc<ScriptEngine>,
    scripts: &[CachedScript],
    input: &ScriptSensorInput,
) -> Vec<(Uuid, AlertOutput)> {
    let chain_nodes: Vec<ChainNode> = scripts
        .iter()
        .map(|s| ChainNode {
            id: s.id,
            kind: match s.kind.as_str() {
                "action_command" => ScriptKind::ActionCommand,
                "recipe_override" => ScriptKind::RecipeOverride,
                _ => ScriptKind::Alert,
            },
            next_flow_ids: s.next_flow_ids.clone(),
            ast: s.ast.clone(),
        })
        .collect();

    let snapshot = SensorSnapshot {
        ph: input.ph,
        ec: input.ec,
        temp: input.temp,
        water_level: input.water_level,
        phase: "Monitoring".to_string(),
        device_id: input.device_id.clone(),
        timestamp_ms: input.timestamp_ms,
    };

    let results = eval_flow_chain(engine, &chain_nodes, &snapshot);

    results
        .into_iter()
        .filter_map(|(id, res)| match res {
            ChainFireResult::Alert(alert) => Some((id, alert)),
            _ => None,
        })
        .collect()
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
            trigger: crate::services::script_engine::TriggerConfig::Sensor,
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

    const DOSE_SCRIPT: &str = r#"
fn main(input) {
    if input.ph <= 7.5 { return (); }
    #{ action: "dose", pump: "ph_down", dose_ml: 5.0 }
}
"#;

    fn make_action_command_script(source: &str) -> ChainNode {
        let engine = ScriptEngine::new();
        ChainNode {
            id: uuid::Uuid::new_v4(),
            kind: ScriptKind::ActionCommand,
            next_flow_ids: vec![],
            ast: engine
                .compile(source)
                .expect("Failed to compile test Rhai action command script source"),
        }
    }

    fn make_snapshot() -> SensorSnapshot {
        SensorSnapshot {
            ph: 8.1,
            ec: 1.4,
            temp: 25.0,
            water_level: 80.0,
            phase: "Monitoring".into(),
            device_id: "d1".into(),
            timestamp_ms: 0,
        }
    }

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
            trigger: crate::services::script_engine::TriggerConfig::Sensor,
        };
        let b = CachedScript {
            id: b_id,
            kind: "alert".to_string(),
            name: "b".to_string(),
            ast: engine
                .compile(FIRING_SCRIPT)
                .expect("test script B compiles"),
            next_flow_ids: vec![a_id.to_string()],
            trigger: crate::services::script_engine::TriggerConfig::Sensor,
        };
        let alerts = eval_alert_scripts_chained(&engine, &[a, b], &make_input());
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
                trigger: crate::services::script_engine::TriggerConfig::Sensor,
            })
            .collect();
        let alerts = eval_alert_scripts_chained(&engine, &scripts, &make_input());
        for id in &ids[0..5] {
            assert!(
                alerts.iter().any(|(fid, _)| fid == id),
                "script {id} should fire"
            );
        }
    }

    #[test]
    fn alert_chains_into_action_command_cross_kind() {
        let engine = Arc::new(ScriptEngine::new());
        let action_child = make_action_command_script(DOSE_SCRIPT);
        let alert_root_ast = engine.compile(FIRING_SCRIPT).expect("compiles");
        let alert_root = ChainNode {
            id: uuid::Uuid::new_v4(),
            kind: ScriptKind::Alert,
            next_flow_ids: vec![action_child.id.to_string()],
            ast: alert_root_ast,
        };

        let results = eval_flow_chain(
            &engine,
            &[alert_root.clone(), action_child.clone()],
            &make_snapshot(),
        );

        assert_eq!(results.len(), 2);
        let child_res = results
            .iter()
            .find(|(id, _)| *id == action_child.id)
            .map(|(_, r)| r)
            .expect("action_child result present");
        assert!(matches!(child_res, ChainFireResult::ActionCommand(_)));
    }

    #[test]
    fn non_firing_alert_root_does_not_chain_into_action_command() {
        let engine = Arc::new(ScriptEngine::new());
        let action_child = make_action_command_script(DOSE_SCRIPT);
        let alert_root_ast = engine.compile(NON_FIRING_SCRIPT).expect("compiles");
        let alert_root = ChainNode {
            id: uuid::Uuid::new_v4(),
            kind: ScriptKind::Alert,
            next_flow_ids: vec![action_child.id.to_string()],
            ast: alert_root_ast,
        };

        let results = eval_flow_chain(&engine, &[alert_root, action_child], &make_snapshot());

        assert!(results.is_empty());
    }

    #[test]
    fn cross_kind_cycle_still_detected_at_runtime() {
        let engine = Arc::new(ScriptEngine::new());
        let a_id = uuid::Uuid::new_v4();
        let b_id = uuid::Uuid::new_v4();

        let a = ChainNode {
            id: a_id,
            kind: ScriptKind::Alert,
            next_flow_ids: vec![b_id.to_string()],
            ast: engine.compile(FIRING_SCRIPT).expect("compiles"),
        };
        let b = ChainNode {
            id: b_id,
            kind: ScriptKind::ActionCommand,
            next_flow_ids: vec![a_id.to_string()],
            ast: engine.compile(DOSE_SCRIPT).expect("compiles"),
        };

        let results = eval_flow_chain(&engine, &[a, b], &make_snapshot());
        assert_eq!(results.len(), 2);
    }
}
