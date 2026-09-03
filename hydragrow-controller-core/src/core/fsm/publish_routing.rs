//! Pure topic-resolution logic for the firmware's main health/publish loop.
//! Extracted so `continue` is never needed inside the caller's outer loop —
//! see ESP32-C3-CONTROLLER-NODE/src/runtime/health.rs.

use serde_json::Value;

/// Where an FSM-originated payload should be published, and what bytes to
/// send. `None` topic override means "use the caller's default fsm-state
/// routing based on payload shape" — the caller still does that part, this
/// function only handles the override case so both call sites share one
/// implementation instead of duplicating the override-detection logic.
#[derive(Debug, PartialEq, Eq)]
pub struct ResolvedPublish {
    pub topic: String,
    pub payload_json: Value,
}

/// Mirrors the `_mqtt_topic_override` / `_payload` convention used by both
/// the fsm_rx and dosing_report_rx channels in health.rs.
pub fn resolve_override_publish_target(value: &Value) -> Option<ResolvedPublish> {
    let topic = value.get("_mqtt_topic_override").and_then(|t| t.as_str())?;
    let payload_json = value
        .get("_payload")
        .cloned()
        .unwrap_or_else(|| value.clone());
    Some(ResolvedPublish {
        topic: topic.to_string(),
        payload_json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_override_target_with_payload() {
        let value = json!({
            "_mqtt_topic_override": "AGITECH/dev1/recipe_events",
            "_payload": {"status": "accepted"}
        });
        let resolved = resolve_override_publish_target(&value).unwrap();
        assert_eq!(resolved.topic, "AGITECH/dev1/recipe_events");
        assert_eq!(resolved.payload_json, json!({"status": "accepted"}));
    }

    #[test]
    fn resolves_override_target_falls_back_to_whole_value() {
        let value = json!({
            "_mqtt_topic_override": "AGITECH/dev1/dosing_report"
        });
        let resolved = resolve_override_publish_target(&value).unwrap();
        assert_eq!(resolved.payload_json, value);
    }

    #[test]
    fn returns_none_when_no_override_present() {
        let value = json!({"current_phase": "Monitoring"});
        assert!(resolve_override_publish_target(&value).is_none());
    }
}
