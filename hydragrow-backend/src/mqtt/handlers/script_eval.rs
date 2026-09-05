//! Eval Rhai scripts khi nhận sensor data từ MQTT.
//! Không blocking: eval là CPU-bound nhưng nhẹ (< 1ms per script với giới hạn 50k ops).
//! Fire-and-forget: lỗi trong script được log nhưng không làm drop sensor message.

use std::collections::HashMap;
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
    pub ir_json: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct WebhookChainNode {
    pub id: Uuid,
    pub kind: ScriptKind,
    pub next_flow_ids: Vec<String>,
    pub ast: rhai::AST,
}

pub fn eval_webhook_chain(
    engine: &Arc<ScriptEngine>,
    all_scripts: &[WebhookChainNode],
    payload: &serde_json::Map<String, serde_json::Value>,
) -> Vec<(Uuid, ChainFireResult)> {
    let mut fired = Vec::new();
    let mut seen = Vec::new();

    let child_ids: Vec<&str> = all_scripts
        .iter()
        .flat_map(|s| s.next_flow_ids.iter().map(|id| id.as_str()))
        .collect();

    let roots: Vec<&WebhookChainNode> = all_scripts
        .iter()
        .filter(|s| !child_ids.contains(&s.id.to_string().as_str()))
        .collect();

    let start = if roots.is_empty() {
        all_scripts.iter().collect()
    } else {
        roots
    };

    for script in start {
        eval_webhook_chain_from(
            script,
            all_scripts,
            engine,
            payload,
            0,
            &mut seen,
            &mut fired,
        );
    }

    fired
}

fn eval_webhook_chain_from(
    node: &WebhookChainNode,
    all_scripts: &[WebhookChainNode],
    engine: &Arc<ScriptEngine>,
    payload: &serde_json::Map<String, serde_json::Value>,
    depth: usize,
    seen: &mut Vec<Uuid>,
    fired: &mut Vec<(Uuid, ChainFireResult)>,
) {
    if depth >= MAX_CHAIN_DEPTH || seen.contains(&node.id) {
        if depth >= MAX_CHAIN_DEPTH {
            warn!(script_id = %node.id, depth, "Webhook flow chain depth limit reached — skipping");
        }
        return;
    }
    seen.push(node.id);

    let fired_result = engine
        .eval_with_dynamic_map(&node.ast, node.kind.clone(), payload)
        .ok()
        .flatten();

    if let Some(res) = fired_result {
        fired.push((node.id, res));
        for next_id in &node.next_flow_ids {
            if let Some(next) = all_scripts.iter().find(|s| s.id.to_string() == *next_id) {
                eval_webhook_chain_from(next, all_scripts, engine, payload, depth + 1, seen, fired);
            }
        }
    }
}

/// Key duy nhất cho 1 lần gọi fetch_range_stat: (sensor, mode, window_sec).
pub type RangeStatKey = (String, String, i64);

/// Quét đệ quy conditions[] trong ir_json (`ConditionOrGroup[]`) tìm mọi leaf
/// có mode != "instant", trả về danh sách key duy nhất cần prefetch.
/// KHÔNG gọi network — hàm thuần, test bằng JSON tĩnh.
pub fn collect_range_stat_keys(ir_json: &serde_json::Value) -> Vec<RangeStatKey> {
    fn walk(node: &serde_json::Value, out: &mut Vec<RangeStatKey>) {
        if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
            for child in children {
                walk(child, out);
            }
            return;
        }
        let mode = node
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("instant");
        if mode == "instant" {
            return;
        }
        let (Some(sensor), Some(window_sec)) = (
            node.get("sensor").and_then(|v| v.as_str()),
            node.get("windowSec").and_then(|v| v.as_i64()),
        ) else {
            return;
        };
        out.push((sensor.to_string(), mode.to_string(), window_sec));
    }

    let mut out = Vec::new();
    if let Some(conditions) = ir_json.get("conditions").and_then(|c| c.as_array()) {
        for c in conditions {
            walk(c, &mut out);
        }
    }
    out.sort();
    out.dedup();
    out
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

/// Biến thể sync, nhận sẵn 1 fetcher tra bảng (không I/O) — dùng để unit-test
/// không cần Influx thật, và để `eval_flow_chain` (async) gọi sau khi prefetch xong.
/// Biến thể sync, nhận sẵn 1 fetcher tra bảng (không I/O) — dùng để unit-test
/// không cần Influx thật, và để `eval_flow_chain` (async) gọi sau khi prefetch xong.
pub fn eval_flow_chain_with_fetcher<F>(
    engine: &Arc<ScriptEngine>,
    all_scripts: &[ChainNode],
    snapshot: &SensorSnapshot,
    device_id: &str,
    fetcher: F,
) -> Vec<(Uuid, ChainFireResult)>
where
    F: Fn(&str, &RangeStatKey) -> f64 + Send + Sync + 'static,
{
    eval_flow_chain_with_fetcher_and_context(
        engine,
        all_scripts,
        snapshot,
        device_id,
        fetcher,
        &HashMap::new(),
    )
}

/// Như `eval_flow_chain_with_fetcher`, cộng thêm `resolved_context_by_node`
/// (context Config·Read đã phân giải sẵn cho từng node — xem
/// `eval_flow_chain` trong Task 8) và cho phép mỗi flow cấu hình
/// `chainConfig.iterationLimit`/`passContextVariables` riêng qua `ir_json`.
/// Node không có `ir_json` hoặc không set các field này giữ nguyên hành vi cũ
/// (MAX_CHAIN_DEPTH, context rỗng).
pub fn eval_flow_chain_with_fetcher_and_context<F>(
    engine: &Arc<ScriptEngine>,
    all_scripts: &[ChainNode],
    snapshot: &SensorSnapshot,
    device_id: &str,
    fetcher: F,
    resolved_context_by_node: &HashMap<Uuid, HashMap<String, f64>>,
) -> Vec<(Uuid, ChainFireResult)>
where
    F: Fn(&str, &RangeStatKey) -> f64 + Send + Sync + 'static,
{
    let fetcher_arc = Arc::new(fetcher);
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
        let max_depth = script
            .ir_json
            .as_ref()
            .and_then(|j| j.get("chainConfig"))
            .and_then(|c| c.get("iterationLimit"))
            .and_then(|v| v.as_u64())
            .map(|n| (n as usize).min(MAX_CHAIN_DEPTH))
            .unwrap_or(MAX_CHAIN_DEPTH);
        eval_flow_chain_from(
            script,
            all_scripts,
            engine,
            snapshot,
            device_id,
            &fetcher_arc,
            0,
            max_depth,
            HashMap::new(),
            resolved_context_by_node,
            &mut seen,
            &mut fired,
        );
    }

    fired
}

/// Entry point thật — prefetch mọi (sensor, mode, window_sec) cần dùng trong TOÀN
/// BỘ chain (không chỉ root) trước khi eval, rồi cấp cho Rhai 1 closure tra bảng.
#[allow(clippy::too_many_arguments)]
pub async fn eval_flow_chain(
    engine: &Arc<ScriptEngine>,
    all_scripts: &[ChainNode],
    snapshot: &SensorSnapshot,
    device_id: &str,
    influx_client: &influxdb2::Client,
    influx_bucket: &str,
    pool: &sqlx::PgPool,
    condition_state_cache: &Arc<tokio::sync::RwLock<HashMap<Uuid, bool>>>,
) -> Vec<(Uuid, ChainFireResult)> {
    let mut keys: Vec<RangeStatKey> = all_scripts
        .iter()
        .filter_map(|s| s.ir_json.as_ref())
        .flat_map(collect_range_stat_keys)
        .collect();
    keys.sort();
    keys.dedup();

    let mut cache: HashMap<RangeStatKey, f64> = HashMap::new();
    if !keys.is_empty() {
        let fetches = keys.iter().map(|(sensor, mode, window_sec)| {
            crate::db::influx::query_range_stat(
                influx_client,
                influx_bucket,
                device_id,
                sensor,
                mode,
                *window_sec,
            )
        });
        let results = futures_util::future::join_all(fetches).await;
        for (key, result) in keys.iter().zip(results) {
            match result {
                Ok(value) => {
                    cache.insert(key.clone(), value);
                }
                Err(e) => {
                    warn!(
                        device_id,
                        sensor = %key.0,
                        mode = %key.1,
                        window_sec = key.2,
                        error = %e,
                        "fetch_range_stat prefetch failed — condition sẽ dùng 0.0, có thể không fire đúng"
                    );
                }
            }
        }
    }

    let fetcher =
        move |_dev_id: &str, key: &RangeStatKey| -> f64 { cache.get(key).copied().unwrap_or(0.0) };

    // Nạp context Config·Read cho toàn bộ chain trên CÙNG 1 thiết bị — chỉ 1
    // lượt query DeviceConfig, không phải 1 lượt/node.
    let mut resolved_context_by_node: HashMap<Uuid, HashMap<String, f64>> = HashMap::new();
    if let Ok(config) = crate::db::postgres::get_device_config(pool, device_id).await {
        for s in all_scripts {
            let Some(ir_json) = &s.ir_json else { continue };
            let reads = crate::services::config_context::parse_context_reads(ir_json);
            if reads.is_empty() {
                continue;
            }
            let ctx =
                crate::services::config_context::resolve_context_reads_from_config(&config, &reads);
            if !ctx.is_empty() {
                resolved_context_by_node.insert(s.id, ctx);
            }
        }
    }

    let fired = eval_flow_chain_with_fetcher_and_context(
        engine,
        all_scripts,
        snapshot,
        device_id,
        fetcher,
        &resolved_context_by_node,
    );

    // Config·Overwrite apply/restore — độc lập với việc action chính có fire
    // hay không (đây là 1 loại "action" khác, xem
    // hydragrow-backend/src/services/config_override.rs).
    for s in all_scripts {
        let Some(ir_json) = &s.ir_json else { continue };
        let Some(directive) = crate::services::config_context::parse_config_overwrite(ir_json)
        else {
            continue;
        };
        let empty = HashMap::new();
        let ctx = resolved_context_by_node.get(&s.id).unwrap_or(&empty);
        let mut sample: HashMap<String, crate::models::script::SampleValue> = [
            (
                "ph".to_string(),
                crate::models::script::SampleValue::Value(snapshot.ph as f64),
            ),
            (
                "ec".to_string(),
                crate::models::script::SampleValue::Value(snapshot.ec as f64),
            ),
            (
                "temp".to_string(),
                crate::models::script::SampleValue::Value(snapshot.temp as f64),
            ),
            (
                "water_level".to_string(),
                crate::models::script::SampleValue::Value(snapshot.water_level as f64),
            ),
        ]
        .into_iter()
        .collect();
        for (k, v) in ctx {
            sample.insert(k.clone(), crate::models::script::SampleValue::Value(*v));
        }
        let conditions = ir_json
            .get("conditions")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        let mut trace = Vec::new();
        let condition_state = conditions
            .iter()
            .all(|c| crate::api::script::eval_condition_tree(c, &sample, &mut trace));

        let previous_state = { condition_state_cache.read().await.get(&s.id).copied() };
        if let Err(e) = crate::services::config_override::apply_config_overwrite_transition(
            pool,
            s.id,
            device_id,
            &directive,
            ctx,
            previous_state,
            condition_state,
        )
        .await
        {
            warn!(
                script_id = %s.id,
                device_id,
                error = %e,
                "config overwrite apply/restore failed"
            );
        }
        condition_state_cache
            .write()
            .await
            .insert(s.id, condition_state);
    }

    fired
}

#[allow(clippy::too_many_arguments)]
fn eval_flow_chain_from<F>(
    node: &ChainNode,
    all_scripts: &[ChainNode],
    engine: &Arc<ScriptEngine>,
    snapshot: &SensorSnapshot,
    device_id: &str,
    fetcher: &Arc<F>,
    depth: usize,
    max_depth: usize,
    parent_context: HashMap<String, f64>,
    resolved_context_by_node: &HashMap<Uuid, HashMap<String, f64>>,
    seen: &mut Vec<Uuid>,
    fired: &mut Vec<(Uuid, ChainFireResult)>,
) where
    F: Fn(&str, &RangeStatKey) -> f64 + Send + Sync + 'static,
{
    if depth >= max_depth || seen.contains(&node.id) {
        if depth >= max_depth {
            warn!(
                script_id = %node.id,
                depth,
                max_depth,
                "Flow chain iteration limit reached — skipping"
            );
        }
        return;
    }
    seen.push(node.id);

    let mut effective_context = parent_context.clone();
    if let Some(own) = resolved_context_by_node.get(&node.id) {
        effective_context.extend(own.iter().map(|(k, v)| (k.clone(), *v)));
    }

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
            match engine.eval_alert_with_context(&node.ast, &input, &effective_context) {
                Ok(Some(mut alert)) => {
                    let vars = template_vars(&effective_context, snapshot);
                    alert.message =
                        crate::services::template::render_alert_template(&alert.message, &vars);
                    alert.title =
                        crate::services::template::render_alert_template(&alert.title, &vars);
                    Some(ChainFireResult::Alert(alert))
                }
                Ok(None) => None,
                Err(e) => {
                    warn!(
                        script_id = %node.id,
                        device_id = %snapshot.device_id,
                        error = %e,
                        "Alert script runtime error — skipping"
                    );
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
            let range_stat_fetcher = {
                let device_id = device_id.to_string();
                let fetcher_clone = fetcher.clone();
                move |sensor: String, mode: String, window_sec: i64| -> f64 {
                    fetcher_clone(&device_id, &(sensor, mode, window_sec))
                }
            };
            match engine.eval_action_command_with_context(
                &node.ast,
                &input,
                range_stat_fetcher,
                &effective_context,
            ) {
                Ok(Some(cmd)) => Some(ChainFireResult::ActionCommand(cmd)),
                Ok(None) => None,
                Err(e) => {
                    warn!(
                        script_id = %node.id,
                        device_id = %snapshot.device_id,
                        error = %e,
                        "ActionCommand script runtime error — skipping"
                    );
                    None
                }
            }
        }
        ScriptKind::RecipeOverride => {
            warn!(
                script_id = %node.id,
                "RecipeOverride script in sensor flow chain — skipping"
            );
            None
        }
        ScriptKind::ConfigOverride => {
            warn!(
                script_id = %node.id,
                "ConfigOverride script in sensor flow chain — handled via config service"
            );
            None
        }
    };

    if let Some(res) = fired_result {
        fired.push((node.id, res));
        let pass_context = node
            .ir_json
            .as_ref()
            .and_then(|j| j.get("chainConfig"))
            .and_then(|c| c.get("passContextVariables"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let next_parent_context = if pass_context {
            effective_context.clone()
        } else {
            HashMap::new()
        };

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
                device_id,
                fetcher,
                depth + 1,
                max_depth,
                next_parent_context.clone(),
                resolved_context_by_node,
                seen,
                fired,
            );
        }
    }
}

/// Xây map biến -> chuỗi hiển thị cho `{{var}}` trong Action·Alert — cộng gộp
/// context đã phân giải (Config·Read/Chain) với snapshot cảm biến + "time".
fn template_vars(
    context: &HashMap<String, f64>,
    snapshot: &SensorSnapshot,
) -> HashMap<String, String> {
    use chrono::TimeZone;
    let mut vars: HashMap<String, String> = context
        .iter()
        .map(|(k, v)| (k.clone(), format!("{v}")))
        .collect();
    vars.insert("ec".to_string(), format!("{:.2}", snapshot.ec));
    vars.insert("ph".to_string(), format!("{:.2}", snapshot.ph));
    vars.insert("temp".to_string(), format!("{:.1}", snapshot.temp));
    vars.insert(
        "water_level".to_string(),
        format!("{:.1}", snapshot.water_level),
    );
    let time = chrono::Utc
        .timestamp_millis_opt(snapshot.timestamp_ms)
        .single()
        .map(|dt| dt.format("%H:%M:%S UTC").to_string())
        .unwrap_or_default();
    vars.insert("time".to_string(), time);
    vars
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
            ir_json: s.ir_json.clone(),
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

    let fetcher = |_dev_id: &str, _key: &RangeStatKey| -> f64 { 0.0 };
    let results =
        eval_flow_chain_with_fetcher(engine, &chain_nodes, &snapshot, &input.device_id, fetcher);

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
            ir_json: None,
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
            ir_json: None,
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
    fn eval_webhook_chain_fires_when_mapped_field_matches_condition() {
        let engine = Arc::new(ScriptEngine::new());
        let source = r#"fn main(input) { if input.external_alarm == 1 { return #{ "level": "warning", "title": "Cảnh báo ngoài", "message": "external_alarm=1" }; } () }"#;
        let ast = engine.compile(source).expect("compile succeeds");
        let node = WebhookChainNode {
            id: Uuid::new_v4(),
            kind: ScriptKind::Alert,
            next_flow_ids: vec![],
            ast,
        };
        let mut payload = serde_json::Map::new();
        payload.insert("external_alarm".to_string(), serde_json::json!(1));
        let results = eval_webhook_chain(&engine, &[node], &payload);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn eval_webhook_chain_does_not_fire_when_condition_false() {
        let engine = Arc::new(ScriptEngine::new());
        let source = r#"fn main(input) { if input.external_alarm == 1 { return #{ "level": "warning", "title": "Cảnh báo ngoài", "message": "external_alarm=1" }; } () }"#;
        let ast = engine.compile(source).expect("compile succeeds");
        let node = WebhookChainNode {
            id: Uuid::new_v4(),
            kind: ScriptKind::Alert,
            next_flow_ids: vec![],
            ast,
        };
        let mut payload = serde_json::Map::new();
        payload.insert("external_alarm".to_string(), serde_json::json!(0));
        let results = eval_webhook_chain(&engine, &[node], &payload);
        assert!(results.is_empty());
    }

    #[test]
    fn eval_webhook_chain_respects_next_flow_ids_cross_kind() {
        let engine = Arc::new(ScriptEngine::new());
        let action_source =
            r#"fn main(input) { #{ "action": "dose", "pump": "PH_DOWN", "dose_ml": 5.0 } }"#;
        let action_ast = engine.compile(action_source).expect("compile succeeds");
        let action_node = WebhookChainNode {
            id: Uuid::new_v4(),
            kind: ScriptKind::ActionCommand,
            next_flow_ids: vec![],
            ast: action_ast,
        };

        let alert_source = r#"fn main(input) { if input.trigger == 1 { return #{ "level": "warning", "title": "Trigger", "message": "Fired" }; } () }"#;
        let alert_ast = engine.compile(alert_source).expect("compile succeeds");
        let alert_node = WebhookChainNode {
            id: Uuid::new_v4(),
            kind: ScriptKind::Alert,
            next_flow_ids: vec![action_node.id.to_string()],
            ast: alert_ast,
        };

        let mut payload = serde_json::Map::new();
        payload.insert("trigger".to_string(), serde_json::json!(1));

        let results = eval_webhook_chain(&engine, &[alert_node, action_node], &payload);
        assert_eq!(results.len(), 2);
        assert!(matches!(results[0].1, ChainFireResult::Alert(_)));
        assert!(matches!(results[1].1, ChainFireResult::ActionCommand(_)));
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
            ir_json: None,
        };
        let b = CachedScript {
            id: b_id,
            kind: "alert".to_string(),
            name: "b".to_string(),
            ast: engine
                .compile(FIRING_SCRIPT)
                .expect("test script B compiles"),
            next_flow_ids: vec![a_id.to_string()],
            ir_json: None,
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
                ir_json: None,
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
            ir_json: None,
        };

        let dummy_fetcher = |_dev: &str, _key: &RangeStatKey| -> f64 { 0.0 };
        let results = eval_flow_chain_with_fetcher(
            &engine,
            &[alert_root.clone(), action_child.clone()],
            &make_snapshot(),
            "d1",
            dummy_fetcher,
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
            ir_json: None,
        };

        let dummy_fetcher = |_dev: &str, _key: &RangeStatKey| -> f64 { 0.0 };
        let results = eval_flow_chain_with_fetcher(
            &engine,
            &[alert_root, action_child],
            &make_snapshot(),
            "d1",
            dummy_fetcher,
        );

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
            ir_json: None,
        };
        let b = ChainNode {
            id: b_id,
            kind: ScriptKind::ActionCommand,
            next_flow_ids: vec![a_id.to_string()],
            ast: engine.compile(DOSE_SCRIPT).expect("compiles"),
            ir_json: None,
        };

        let dummy_fetcher = |_dev: &str, _key: &RangeStatKey| -> f64 { 0.0 };
        let results =
            eval_flow_chain_with_fetcher(&engine, &[a, b], &make_snapshot(), "d1", dummy_fetcher);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn returns_empty_for_all_instant_conditions() {
        let ir = serde_json::json!({ "conditions": [{ "sensor": "ph", "operator": ">", "value": 7.5, "mode": "instant" }] });
        assert!(collect_range_stat_keys(&ir).is_empty());
    }

    #[test]
    fn collects_flat_time_window_condition() {
        let ir = serde_json::json!({ "conditions": [{ "sensor": "ph", "operator": ">", "value": 6.5, "mode": "mean", "windowSec": 900 }] });
        assert_eq!(
            collect_range_stat_keys(&ir),
            vec![("ph".to_string(), "mean".to_string(), 900)]
        );
    }

    #[test]
    fn collects_nested_inside_condition_group() {
        let ir = serde_json::json!({
            "conditions": [{
                "op": "and",
                "children": [
                    { "sensor": "ph", "operator": ">", "value": 6.5, "mode": "mean", "windowSec": 900 },
                    { "sensor": "ec", "operator": "<", "value": 1.0, "mode": "instant" }
                ]
            }]
        });
        assert_eq!(
            collect_range_stat_keys(&ir),
            vec![("ph".to_string(), "mean".to_string(), 900)]
        );
    }

    #[test]
    fn dedupes_identical_keys_across_multiple_scripts_worth_of_conditions() {
        let ir = serde_json::json!({
            "conditions": [
                { "sensor": "ph", "operator": ">", "value": 6.5, "mode": "mean", "windowSec": 900 },
                { "sensor": "ph", "operator": "<", "value": 8.0, "mode": "mean", "windowSec": 900 }
            ]
        });
        assert_eq!(collect_range_stat_keys(&ir).len(), 1);
    }

    #[test]
    fn eval_flow_chain_uses_real_range_stat_fetcher_not_zero_stub() {
        let engine = Arc::new(ScriptEngine::new());
        let source = r#"
fn main(input) {
    if fetch_range_stat("ph", "mean", 900) > 6.5 {
        return #{ "action": "dose", "pump": "PH_DOWN", "dose_ml": 5, "pwm": 100 };
    }
    ()
}
"#;
        let ir_json = serde_json::json!({
            "conditions": [{ "sensor": "ph", "operator": ">", "value": 6.5, "mode": "mean", "windowSec": 900 }]
        });
        let node = ChainNode {
            id: Uuid::new_v4(),
            kind: ScriptKind::ActionCommand,
            next_flow_ids: vec![],
            ast: engine.compile(source).expect("compiles"),
            ir_json: Some(ir_json),
        };
        let fetcher = |_device_id: &str, _key: &RangeStatKey| -> f64 { 7.0 };
        let result =
            eval_flow_chain_with_fetcher(&engine, &[node], &make_snapshot(), "device-1", fetcher);
        assert_eq!(
            result.len(),
            1,
            "phải fire vì 7.0 > 6.5, không phải 0.0 > 6.5 (stub cũ luôn fail)"
        );
    }

    #[test]
    fn eval_flow_chain_with_fetcher_and_context_injects_resolved_context_into_the_guard() {
        let engine = Arc::new(ScriptEngine::new());
        let ast = engine
            .compile(r#"fn main(input) { if input.ph > input.ph_target_now { return #{ "level": "warning", "title": "t", "message": "m" }; } () }"#)
            .expect("compiles");
        let node = ChainNode {
            id: Uuid::new_v4(),
            kind: ScriptKind::Alert,
            next_flow_ids: vec![],
            ast,
            ir_json: None,
        };
        let snapshot = SensorSnapshot {
            ph: 7.4,
            ec: 1.0,
            temp: 24.0,
            water_level: 80.0,
            phase: "Monitoring".into(),
            device_id: "d".into(),
            timestamp_ms: 0,
        };

        let mut ctx_by_node = std::collections::HashMap::new();
        ctx_by_node.insert(
            node.id,
            [("ph_target_now".to_string(), 7.2)].into_iter().collect(),
        );

        let fired = eval_flow_chain_with_fetcher_and_context(
            &engine,
            &[node],
            &snapshot,
            "d",
            |_, _| 0.0,
            &ctx_by_node,
        );
        assert_eq!(fired.len(), 1);
    }

    #[test]
    fn eval_flow_chain_with_fetcher_respects_a_lower_configured_iteration_limit() {
        let engine = Arc::new(ScriptEngine::new());
        let ast = engine
            .compile(r#"fn main(input) { #{ "level": "info", "title": "t", "message": "m" } }"#)
            .expect("compiles");
        // 3 script nối tiếp a -> b -> c, nhưng chainConfig.iterationLimit của a = 1
        // nên chỉ a được eval (b, c bị cắt).
        let c_id = Uuid::new_v4();
        let b_id = Uuid::new_v4();
        let a_id = Uuid::new_v4();
        let c = ChainNode {
            id: c_id,
            kind: ScriptKind::Alert,
            next_flow_ids: vec![],
            ast: ast.clone(),
            ir_json: None,
        };
        let b = ChainNode {
            id: b_id,
            kind: ScriptKind::Alert,
            next_flow_ids: vec![c_id.to_string()],
            ast: ast.clone(),
            ir_json: None,
        };
        let a = ChainNode {
            id: a_id,
            kind: ScriptKind::Alert,
            next_flow_ids: vec![b_id.to_string()],
            ast,
            ir_json: Some(serde_json::json!({ "chainConfig": { "iterationLimit": 1 } })),
        };
        let snapshot = SensorSnapshot {
            ph: 7.0,
            ec: 1.0,
            temp: 24.0,
            water_level: 80.0,
            phase: "Monitoring".into(),
            device_id: "d".into(),
            timestamp_ms: 0,
        };

        let fired = eval_flow_chain_with_fetcher(&engine, &[a, b, c], &snapshot, "d", |_, _| 0.0);
        assert_eq!(
            fired.len(),
            1,
            "expected only the root to fire before the iteration limit stops the chain"
        );
    }

    #[test]
    fn eval_flow_chain_with_fetcher_default_behavior_is_unchanged_without_ir_json() {
        // Guards against regressions: existing ChainNodes with ir_json: None must
        // keep using MAX_CHAIN_DEPTH and an empty context exactly as before.
        let engine = Arc::new(ScriptEngine::new());
        let ast = engine
            .compile(r#"fn main(input) { #{ "level": "info", "title": "t", "message": "m" } }"#)
            .expect("compiles");
        let node = ChainNode {
            id: Uuid::new_v4(),
            kind: ScriptKind::Alert,
            next_flow_ids: vec![],
            ast,
            ir_json: None,
        };
        let snapshot = SensorSnapshot {
            ph: 7.0,
            ec: 1.0,
            temp: 24.0,
            water_level: 80.0,
            phase: "Monitoring".into(),
            device_id: "d".into(),
            timestamp_ms: 0,
        };
        let fired = eval_flow_chain_with_fetcher(&engine, &[node], &snapshot, "d", |_, _| 0.0);
        assert_eq!(fired.len(), 1);
    }
}
