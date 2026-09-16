use serde::{Deserialize, Serialize};

pub type CommandId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandLifecycle {
    Requested,
    Sent,
    Acknowledged,
    Confirmed,
    Rejected,
    Failed,
    Timeout,
    Unknown,
}

impl CommandLifecycle {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Confirmed | Self::Rejected | Self::Failed | Self::Timeout | Self::Unknown
        )
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        match self {
            Self::Requested => matches!(next, Self::Sent | Self::Failed | Self::Unknown),
            Self::Sent => matches!(
                next,
                Self::Acknowledged | Self::Rejected | Self::Failed | Self::Timeout | Self::Unknown
            ),
            Self::Acknowledged => matches!(
                next,
                Self::Confirmed | Self::Rejected | Self::Failed | Self::Timeout | Self::Unknown
            ),
            Self::Confirmed | Self::Rejected | Self::Failed | Self::Timeout | Self::Unknown => {
                false
            }
        }
    }

    pub fn transition_to(self, next: Self) -> Result<Self, CommandLifecycleError> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(CommandLifecycleError::InvalidTransition {
                from: self,
                to: next,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLifecycleError {
    InvalidTransition {
        from: CommandLifecycle,
        to: CommandLifecycle,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CommandMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_id: Option<CommandId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandRecord {
    pub command_id: CommandId,
    pub device_id: String,
    pub action: String,
    pub pump_id: Option<String>,
    pub requested_state: Option<bool>,
    pub requested_pwm: Option<u32>,
    pub lifecycle: CommandLifecycle,
    pub requested_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandLifecycleEvent {
    pub command_id: CommandId,
    pub device_id: String,
    pub lifecycle: CommandLifecycle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub timestamp_ms: i64,
}

#[cfg(test)]
mod tests {
    use super::{CommandLifecycle, CommandLifecycleError, CommandMetadata};

    #[test]
    fn canonical_happy_path_is_requested_sent_acknowledged_confirmed() {
        assert!(CommandLifecycle::Requested.can_transition_to(CommandLifecycle::Sent));
        assert!(CommandLifecycle::Sent.can_transition_to(CommandLifecycle::Acknowledged));
        assert!(CommandLifecycle::Acknowledged.can_transition_to(CommandLifecycle::Confirmed));
    }

    #[test]
    fn sent_can_end_without_ack_when_outcome_is_known_or_unresolved() {
        for terminal in [
            CommandLifecycle::Rejected,
            CommandLifecycle::Failed,
            CommandLifecycle::Timeout,
            CommandLifecycle::Unknown,
        ] {
            assert!(CommandLifecycle::Sent.can_transition_to(terminal));
        }
    }

    #[test]
    fn acknowledged_can_end_without_confirmation_when_outcome_is_not_confirmed() {
        for terminal in [
            CommandLifecycle::Rejected,
            CommandLifecycle::Failed,
            CommandLifecycle::Timeout,
            CommandLifecycle::Unknown,
        ] {
            assert!(CommandLifecycle::Acknowledged.can_transition_to(terminal));
        }
    }

    #[test]
    fn timeout_is_distinct_from_failed() {
        assert_ne!(CommandLifecycle::Timeout, CommandLifecycle::Failed);
        assert!(CommandLifecycle::Timeout.is_terminal());
        assert!(CommandLifecycle::Failed.is_terminal());
    }

    #[test]
    fn terminal_states_cannot_transition_again() {
        for terminal in [
            CommandLifecycle::Confirmed,
            CommandLifecycle::Rejected,
            CommandLifecycle::Failed,
            CommandLifecycle::Timeout,
            CommandLifecycle::Unknown,
        ] {
            assert!(!terminal.can_transition_to(CommandLifecycle::Confirmed));
        }
    }

    #[test]
    fn invalid_transition_returns_error() {
        let result = CommandLifecycle::Requested.transition_to(CommandLifecycle::Confirmed);
        assert_eq!(
            result,
            Err(CommandLifecycleError::InvalidTransition {
                from: CommandLifecycle::Requested,
                to: CommandLifecycle::Confirmed,
            })
        );
    }

    #[test]
    fn lifecycle_serializes_with_canonical_names() {
        assert_eq!(
            serde_json::to_string(&CommandLifecycle::Acknowledged).unwrap(),
            "\"ACKNOWLEDGED\""
        );
        assert_eq!(
            serde_json::to_string(&CommandLifecycle::Unknown).unwrap(),
            "\"UNKNOWN\""
        );
    }

    #[test]
    fn command_metadata_round_trips_command_id() {
        let metadata = CommandMetadata {
            command_id: Some("cmd-123".to_string()),
        };
        let json = serde_json::to_string(&metadata).unwrap();
        assert_eq!(json, r#"{"command_id":"cmd-123"}"#);
        assert_eq!(
            serde_json::from_str::<CommandMetadata>(&json).unwrap(),
            metadata
        );
    }
}
