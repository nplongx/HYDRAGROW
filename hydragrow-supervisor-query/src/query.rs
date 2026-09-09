use serde::{Deserialize, Serialize};

/// Design spec §8.1: these six caps are the hard bounds every query is
/// clamped to before it's allowed to run — a request for more is honored up to the cap,
/// never rejected outright (§8.4: "a model retrying after a clamp shouldn't
/// need special-case error handling").
pub const MAX_SENSOR_HISTORY_MINUTES: u32 = 60;
/// Dosing history is fetched by time window (the real backend endpoint,
/// `analytics.rs`'s `get_dosing_history_range`, takes `start`/`end`, not a
/// row limit) — found while implementing this plan. 24h is generous enough
/// to reliably contain the last few dosing events even on a slow-cycling
/// device; the row-count cap below is what actually bounds what reaches the
/// model.
pub const MAX_DOSING_HISTORY_WINDOW_MINUTES: u32 = 1440;
pub const MAX_DOSING_HISTORY_COUNT: usize = 5;
pub const MAX_FSM_EVENTS_COUNT: usize = 10;
pub const MAX_OUTPUT_BYTES: usize = 16_384;
pub const QUERY_TIMEOUT_SECS: u64 = 20;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SensorHistoryQuery {
    pub device_id: String,
    #[serde(default = "default_sensor_minutes")]
    pub minutes: u32,
}
fn default_sensor_minutes() -> u32 {
    MAX_SENSOR_HISTORY_MINUTES
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DosingHistoryQuery {
    pub device_id: String,
    #[serde(default = "default_dosing_minutes")]
    pub minutes: u32,
}
fn default_dosing_minutes() -> u32 {
    MAX_DOSING_HISTORY_WINDOW_MINUTES
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct FsmEventsQuery {
    pub device_id: String,
    #[serde(default = "default_fsm_limit")]
    pub limit: u32,
}
fn default_fsm_limit() -> u32 {
    MAX_FSM_EVENTS_COUNT as u32
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct HealthTopicsQuery {
    pub device_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct HestiaSnapshotQuery {
    pub device_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CropTargetQuery {
    pub device_id: String,
}

/// The allowlist. Closed by construction: there is no variant for control,
/// dosing commands, acknowledgment, reset, or any other write — adding one
/// would require editing this enum, which is the point. `#[serde(tag =
/// "query_type")]` means any tool-calling provider's JSON tool-call
/// arguments deserialize straight into this without this crate knowing
/// anything about which provider sent them (design spec §8.2's
/// provider-agnostic requirement, extended to the query layer itself).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "query_type", rename_all = "snake_case")]
pub enum SupervisorQuery {
    SensorHistory(SensorHistoryQuery),
    DosingHistory(DosingHistoryQuery),
    FsmEvents(FsmEventsQuery),
    HealthTopics(HealthTopicsQuery),
    HestiaSnapshot(HestiaSnapshotQuery),
    CropTarget(CropTargetQuery),
}

impl SupervisorQuery {
    pub fn device_id(&self) -> &str {
        match self {
            Self::SensorHistory(q) => &q.device_id,
            Self::DosingHistory(q) => &q.device_id,
            Self::FsmEvents(q) => &q.device_id,
            Self::HealthTopics(q) => &q.device_id,
            Self::HestiaSnapshot(q) => &q.device_id,
            Self::CropTarget(q) => &q.device_id,
        }
    }

    pub fn tag(&self) -> &'static str {
        match self {
            Self::SensorHistory(_) => "sensor_history",
            Self::DosingHistory(_) => "dosing_history",
            Self::FsmEvents(_) => "fsm_events",
            Self::HealthTopics(_) => "health_topics",
            Self::HestiaSnapshot(_) => "hestia_snapshot",
            Self::CropTarget(_) => "crop_target",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("device_id must not be empty")]
    EmptyDeviceId,
    #[error("query targets device '{requested}' but this session is scoped to '{expected}'")]
    DeviceScopeMismatch { expected: String, requested: String },
    #[error("backend request failed: {0}")]
    Backend(String),
    #[error("backend returned a response that could not be parsed: {0}")]
    InvalidResponse(String),
}

/// Clamps every numeric bound to its cap and rejects structurally invalid
/// queries (an empty device_id). Never rejects a query solely for asking
/// for *more* than the cap — it clamps instead (§8.4).
pub fn validate_and_clamp(query: SupervisorQuery) -> Result<SupervisorQuery, QueryError> {
    if query.device_id().trim().is_empty() {
        return Err(QueryError::EmptyDeviceId);
    }

    Ok(match query {
        SupervisorQuery::SensorHistory(mut q) => {
            q.minutes = q.minutes.min(MAX_SENSOR_HISTORY_MINUTES);
            SupervisorQuery::SensorHistory(q)
        }
        SupervisorQuery::DosingHistory(mut q) => {
            q.minutes = q.minutes.min(MAX_DOSING_HISTORY_WINDOW_MINUTES);
            SupervisorQuery::DosingHistory(q)
        }
        SupervisorQuery::FsmEvents(mut q) => {
            q.limit = q.limit.min(MAX_FSM_EVENTS_COUNT as u32);
            SupervisorQuery::FsmEvents(q)
        }
        other => other, // HealthTopics/HestiaSnapshot/CropTarget carry no bounded fields
    })
}

/// The other half of "device scope" enforcement (design spec §8.4): a query
/// that's well-formed and within its numeric limits can still be asking
/// about the wrong device. Callers that bind a query to a specific
/// diagnosis or CLI invocation call this before `QueryBackend::execute`;
/// this crate provides the check, not the policy of when to call it.
pub fn validate_device_scope(
    query: &SupervisorQuery,
    expected_device_id: &str,
) -> Result<(), QueryError> {
    if query.device_id() == expected_device_id {
        Ok(())
    } else {
        Err(QueryError::DeviceScopeMismatch {
            expected: expected_device_id.to_string(),
            requested: query.device_id().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_tagged_json_for_every_variant() {
        let cases = [
            (
                r#"{"query_type": "sensor_history", "device_id": "d1", "minutes": 30}"#,
                "sensor_history",
            ),
            (
                r#"{"query_type": "dosing_history", "device_id": "d1", "minutes": 1440}"#,
                "dosing_history",
            ),
            (
                r#"{"query_type": "fsm_events", "device_id": "d1", "limit": 10}"#,
                "fsm_events",
            ),
            (
                r#"{"query_type": "health_topics", "device_id": "d1"}"#,
                "health_topics",
            ),
            (
                r#"{"query_type": "hestia_snapshot", "device_id": "d1"}"#,
                "hestia_snapshot",
            ),
            (
                r#"{"query_type": "crop_target", "device_id": "d1"}"#,
                "crop_target",
            ),
        ];
        for (json, expected_tag) in cases {
            let query: SupervisorQuery = serde_json::from_str(json).unwrap();
            assert_eq!(query.device_id(), "d1");
            assert_eq!(query.tag(), expected_tag);
        }
    }

    #[test]
    fn deserialize_rejects_unknown_query_type() {
        let json = r#"{"query_type": "control_pump", "device_id": "d1"}"#;
        let result: Result<SupervisorQuery, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn validate_and_clamp_caps_sensor_history_minutes() {
        let query = SupervisorQuery::SensorHistory(SensorHistoryQuery {
            device_id: "d1".to_string(),
            minutes: 10_000,
        });
        let clamped = validate_and_clamp(query).unwrap();
        match clamped {
            SupervisorQuery::SensorHistory(q) => assert_eq!(q.minutes, MAX_SENSOR_HISTORY_MINUTES),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn validate_and_clamp_leaves_smaller_values_untouched() {
        let query = SupervisorQuery::SensorHistory(SensorHistoryQuery {
            device_id: "d1".to_string(),
            minutes: 15,
        });
        let clamped = validate_and_clamp(query).unwrap();
        match clamped {
            SupervisorQuery::SensorHistory(q) => assert_eq!(q.minutes, 15),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn validate_and_clamp_caps_fsm_events_limit() {
        let query = SupervisorQuery::FsmEvents(FsmEventsQuery {
            device_id: "d1".to_string(),
            limit: 999,
        });
        let clamped = validate_and_clamp(query).unwrap();
        match clamped {
            SupervisorQuery::FsmEvents(q) => assert_eq!(q.limit, MAX_FSM_EVENTS_COUNT as u32),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn validate_and_clamp_rejects_empty_device_id() {
        let query = SupervisorQuery::HealthTopics(HealthTopicsQuery {
            device_id: String::new(),
        });
        assert!(validate_and_clamp(query).is_err());
    }

    #[test]
    fn validate_device_scope_accepts_matching_device() {
        let query = SupervisorQuery::CropTarget(CropTargetQuery {
            device_id: "d1".to_string(),
        });
        assert!(validate_device_scope(&query, "d1").is_ok());
    }

    #[test]
    fn validate_device_scope_rejects_mismatched_device() {
        let query = SupervisorQuery::CropTarget(CropTargetQuery {
            device_id: "some-other-device".to_string(),
        });
        assert!(validate_device_scope(&query, "d1").is_err());
    }
}
