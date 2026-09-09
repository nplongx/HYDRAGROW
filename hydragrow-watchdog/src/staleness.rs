use crate::backend_client::TopicStatus;
use chrono::{DateTime, Utc};

#[derive(Debug, PartialEq)]
pub struct StaleBreach {
    pub seconds_stale: i64,
}

/// Design spec §4.2: only `controller/status` is a standalone staleness
/// trigger, because it's the one topic that's genuinely periodic — Hestia
/// snapshots publish on a fixed cadence regardless of FSM activity, so a
/// fixed threshold is safe. `fsm/transition` is event-driven and is never
/// evaluated here at all, even if present in `topics` — a long gap there
/// can simply mean nothing needed to change, not that anything is stuck.
pub fn check_staleness(
    topics: &[TopicStatus],
    now: DateTime<Utc>,
    threshold_secs: u64,
) -> Option<StaleBreach> {
    let status = topics
        .iter()
        .find(|t| t.topic_category == "controller/status")?;

    let seconds_stale = (now - status.last_seen_at).num_seconds();
    if seconds_stale >= threshold_secs as i64 {
        Some(StaleBreach { seconds_stale })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_client::TopicStatus;
    use chrono::{Duration, Utc};

    #[test]
    fn flags_stale_when_controller_status_exceeds_threshold() {
        let now = Utc::now();
        let topics = vec![TopicStatus {
            topic_category: "controller/status".to_string(),
            last_seen_at: now - Duration::seconds(90),
        }];
        let breach = check_staleness(&topics, now, 60);
        assert!(breach.is_some());
        assert_eq!(breach.unwrap().seconds_stale, 90);
    }

    #[test]
    fn does_not_flag_when_within_threshold() {
        let now = Utc::now();
        let topics = vec![TopicStatus {
            topic_category: "controller/status".to_string(),
            last_seen_at: now - Duration::seconds(10),
        }];
        assert!(check_staleness(&topics, now, 60).is_none());
    }

    #[test]
    fn ignores_fsm_transition_staleness_entirely() {
        // Design spec §4.2: fsm/transition is event-driven, not periodic —
        // it must never be a standalone trigger, even if very stale, since
        // a long gap can simply mean nothing needed to change.
        let now = Utc::now();
        let topics = vec![TopicStatus {
            topic_category: "fsm/transition".to_string(),
            last_seen_at: now - Duration::hours(5),
        }];
        assert!(check_staleness(&topics, now, 60).is_none());
    }

    #[test]
    fn returns_none_when_controller_status_topic_is_absent() {
        // A device that has never published controller/status at all (e.g.
        // brand new, not yet provisioned) isn't a staleness breach — that's
        // a different problem the watchdog isn't trying to detect.
        let now = Utc::now();
        let topics = vec![TopicStatus {
            topic_category: "sensor/data".to_string(),
            last_seen_at: now,
        }];
        assert!(check_staleness(&topics, now, 60).is_none());
    }
}
