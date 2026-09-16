use serde::{Deserialize, Serialize};

/// Canonical operational-state dimensions. These describe observed condition,
/// never command intent or transport connectivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ContactState {
    Unknown,
    Contacted,
    NotContacted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FreshnessState {
    Unknown,
    Fresh,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RuntimeReadiness {
    Unknown,
    Ready,
    NotReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ActuatorKnowledge {
    Unknown,
    Known,
    Stale,
    Contradictory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationalState {
    pub contact: ContactState,
    pub freshness: FreshnessState,
    pub readiness: RuntimeReadiness,
    pub actuator: ActuatorKnowledge,
    /// Backend receipt time of the observation used to classify this state.
    pub classified_at: Option<String>,
    /// Backend/application receipt time of the observation.
    pub received_at: Option<String>,
    /// Source observation time, if the source supplied one.
    pub observed_at: Option<String>,
}

impl Default for OperationalState {
    fn default() -> Self {
        Self::unknown()
    }
}

impl OperationalState {
    pub const fn unknown() -> Self {
        Self {
            contact: ContactState::Unknown,
            freshness: FreshnessState::Unknown,
            readiness: RuntimeReadiness::Unknown,
            actuator: ActuatorKnowledge::Unknown,
            classified_at: None,
            received_at: None,
            observed_at: None,
        }
    }

    /// A required operational prerequisite is usable only when its observation
    /// is current and internally known. Unknown/stale/contradictory is fail-closed.
    pub const fn safety_prerequisites_known(&self) -> bool {
        matches!(self.contact, ContactState::Contacted)
            && matches!(self.freshness, FreshnessState::Fresh)
            && matches!(self.readiness, RuntimeReadiness::Ready)
            && matches!(self.actuator, ActuatorKnowledge::Known)
    }

    /// Compute freshness from backend receipt time. The source observation
    /// timestamp remains separately preserved and is never silently substituted.
    pub fn classify_freshness(
        received_at: Option<&str>,
        now: chrono::DateTime<chrono::Utc>,
        threshold_secs: i64,
    ) -> FreshnessState {
        let Some(received_at) = received_at else {
            return FreshnessState::Unknown;
        };
        let Ok(received_at) = chrono::DateTime::parse_from_rfc3339(received_at) else {
            return FreshnessState::Unknown;
        };
        let age = now.signed_duration_since(received_at.with_timezone(&chrono::Utc));
        if age.num_seconds() < 0 {
            return FreshnessState::Unknown;
        }
        if age.num_seconds() >= threshold_secs {
            FreshnessState::Stale
        } else {
            FreshnessState::Fresh
        }
    }
}

/// Single source of truth for current operational freshness. Consumers should
/// pass this value rather than introducing their own magic timeout.
pub const OPERATIONAL_FRESHNESS_THRESHOLD_SECS: i64 = 30;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn empty_state_is_unknown_and_not_safe() {
        let state = OperationalState::default();
        assert_eq!(state.contact, ContactState::Unknown);
        assert_eq!(state.freshness, FreshnessState::Unknown);
        assert!(!state.safety_prerequisites_known());
    }

    #[test]
    fn recent_receipt_is_fresh() {
        let now = Utc::now();
        let received = (now - Duration::seconds(5)).to_rfc3339();
        assert_eq!(
            OperationalState::classify_freshness(
                Some(&received),
                now,
                OPERATIONAL_FRESHNESS_THRESHOLD_SECS
            ),
            FreshnessState::Fresh
        );
    }

    #[test]
    fn expired_receipt_is_stale() {
        let now = Utc::now();
        let received = (now - Duration::seconds(30)).to_rfc3339();
        assert_eq!(
            OperationalState::classify_freshness(
                Some(&received),
                now,
                OPERATIONAL_FRESHNESS_THRESHOLD_SECS
            ),
            FreshnessState::Stale
        );
    }

    #[test]
    fn missing_invalid_or_future_receipt_is_unknown() {
        let now = Utc::now();
        assert_eq!(
            OperationalState::classify_freshness(None, now, OPERATIONAL_FRESHNESS_THRESHOLD_SECS),
            FreshnessState::Unknown
        );
        assert_eq!(
            OperationalState::classify_freshness(
                Some("not-a-time"),
                now,
                OPERATIONAL_FRESHNESS_THRESHOLD_SECS
            ),
            FreshnessState::Unknown
        );
        let future = (now + Duration::seconds(5)).to_rfc3339();
        assert_eq!(
            OperationalState::classify_freshness(
                Some(&future),
                now,
                OPERATIONAL_FRESHNESS_THRESHOLD_SECS
            ),
            FreshnessState::Unknown
        );
    }
}
