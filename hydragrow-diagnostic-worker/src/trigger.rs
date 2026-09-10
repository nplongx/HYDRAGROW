use serde::{Deserialize, Serialize};

/// Design spec §4.1: an extensible set of trigger sources, not a single
/// boolean gate. Two variants today; a future trigger type (e.g. a
/// cross-signal pattern independent of Hestia) can be added as a third
/// without restructuring evaluate_triggers's signature or callers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SupervisorTrigger {
    HestiaState { state: String, reasons: Vec<String> },
    WatchdogBreach,
}

/// `hestia` is the raw JSON from `/api/health/hestia` for one device (may be
/// absent if it's never reported); `has_recent_watchdog_breach` comes from
/// lookback check.
pub fn evaluate_triggers(
    hestia: Option<&serde_json::Value>,
    has_recent_watchdog_breach: bool,
) -> Option<SupervisorTrigger> {
    if let Some(hestia) = hestia {
        let state = hestia.get("state").and_then(|s| s.as_str()).unwrap_or("");
        if state == "WARNING" || state == "CRITICAL" {
            let reasons = hestia
                .get("reasons")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            return Some(SupervisorTrigger::HestiaState {
                state: state.to_string(),
                reasons,
            });
        }
    }

    if has_recent_watchdog_breach {
        return Some(SupervisorTrigger::WatchdogBreach);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_state_produces_hestia_trigger() {
        let hestia = serde_json::json!({"state": "WARNING", "reasons": ["ec_out_of_range"]});
        let trigger = evaluate_triggers(Some(&hestia), false);
        assert!(matches!(
            trigger,
            Some(SupervisorTrigger::HestiaState { .. })
        ));
    }

    #[test]
    fn critical_state_produces_hestia_trigger() {
        let hestia = serde_json::json!({"state": "CRITICAL", "reasons": []});
        assert!(evaluate_triggers(Some(&hestia), false).is_some());
    }

    #[test]
    fn comfortable_state_does_not_trigger_on_its_own() {
        // Design spec §4.1: Comfortable is not an absolute stop for *all*
        // trigger types — it just means no Hestia-sourced trigger fired.
        let hestia = serde_json::json!({"state": "COMFORTABLE", "reasons": []});
        assert!(evaluate_triggers(Some(&hestia), false).is_none());
    }

    #[test]
    fn recovery_state_does_not_trigger() {
        let hestia = serde_json::json!({"state": "RECOVERY", "reasons": []});
        assert!(evaluate_triggers(Some(&hestia), false).is_none());
    }

    #[test]
    fn watchdog_breach_triggers_even_when_hestia_is_comfortable() {
        // The second, independent trigger source (§4.1) — deliberately not
        // an else-if off the Hestia check, since either check firing alone
        // is sufficient.
        let hestia = serde_json::json!({"state": "COMFORTABLE", "reasons": []});
        let trigger = evaluate_triggers(Some(&hestia), true);
        assert!(matches!(trigger, Some(SupervisorTrigger::WatchdogBreach)));
    }

    #[test]
    fn hestia_trigger_takes_precedence_when_both_fire() {
        // Not a correctness requirement either way, but a diagnosis needs
        // exactly one SupervisorTrigger to build DiagnosticContext around
        // — Hestia wins because it carries richer detail (reasons)
        // than a bare watchdog breach.
        let hestia = serde_json::json!({"state": "WARNING", "reasons": ["ec_degrading"]});
        let trigger = evaluate_triggers(Some(&hestia), true);
        assert!(matches!(
            trigger,
            Some(SupervisorTrigger::HestiaState { .. })
        ));
    }

    #[test]
    fn no_hestia_snapshot_and_no_breach_means_no_trigger() {
        // A device that's never published controller/status yet.
        assert!(evaluate_triggers(None, false).is_none());
    }
}
