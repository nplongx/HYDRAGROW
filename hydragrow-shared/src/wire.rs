use crate::{
    CommandLifecycle,
    log::{LogCategory, LogLevel},
    telemetry::OperationalState,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Version of the cross-system JSON contract owned by `hydragrow-shared`.
pub const CANONICAL_SCHEMA_VERSION: u16 = 1;

pub type DeviceId = String;
pub type ResourceId = String;
pub type EventId = i32;
pub type UserId = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKind {
    User,
    Service,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    pub kind: PrincipalKind,
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub service_key_label: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    pub details: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorEnvelope {
    pub error: ApiErrorBody,
}

/// Canonical durable command projection. Persistence may contain additional
/// retry/idempotency columns; those remain backend storage concerns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandWireRecord {
    pub schema_version: u16,
    pub command_id: String,
    pub device_id: DeviceId,
    pub action: String,
    pub pump_id: Option<String>,
    pub requested_state: Option<bool>,
    pub requested_pwm: Option<u32>,
    pub lifecycle: CommandLifecycle,
    pub created_at_ms: i64,
    pub authorized_at_ms: i64,
    pub sent_at_ms: Option<i64>,
    pub acknowledged_at_ms: Option<i64>,
    pub confirmed_at_ms: Option<i64>,
    pub terminal_at_ms: Option<i64>,
    pub next_retry_at_ms: Option<i64>,
    pub attempt_count: i32,
    pub last_error: Option<String>,
    pub last_observed_at_ms: Option<i64>,
    pub version: i64,
    pub retry_safe: bool,
}

/// Canonical Journal read model. `event_id` is the server/database identity;
/// `resource_id` and `correlation_id` are optional domain references.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JournalEventEnvelope {
    pub id: EventId,
    pub device_id: DeviceId,
    pub level: LogLevel,
    pub category: LogCategory,
    pub event_type: String,
    pub title: String,
    pub message: String,
    pub occurred_at: String,
    pub received_at: String,
    pub source: String,
    pub reason: Option<String>,
    pub reason_code: Option<String>,
    pub actor: Option<Principal>,
    pub resource_id: Option<ResourceId>,
    pub correlation_id: Option<String>,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
    pub metadata: Option<Value>,
}

/// Canonical operational read model keeps telemetry semantics in one place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalTelemetryEnvelope {
    pub schema_version: u16,
    pub snapshot: crate::telemetry::AuthoritativeTelemetrySnapshot,
    pub operational_state: OperationalState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CommandLifecycle,
        log::{LogCategory, LogLevel},
    };

    #[test]
    fn canonical_command_wire_round_trips_and_preserves_terminal_state() {
        let command = CommandWireRecord {
            schema_version: CANONICAL_SCHEMA_VERSION,
            command_id: "cmd-1".into(),
            device_id: "device-1".into(),
            action: "set_pwm".into(),
            pump_id: Some("PUMP_A".into()),
            requested_state: None,
            requested_pwm: Some(70),
            lifecycle: CommandLifecycle::Confirmed,
            created_at_ms: 100,
            authorized_at_ms: 101,
            sent_at_ms: Some(102),
            acknowledged_at_ms: Some(103),
            confirmed_at_ms: Some(104),
            terminal_at_ms: Some(104),
            next_retry_at_ms: None,
            attempt_count: 1,
            last_error: None,
            last_observed_at_ms: Some(104),
            version: 4,
            retry_safe: true,
        };
        let json = serde_json::to_string(&command).unwrap();
        let decoded: CommandWireRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, command);
        assert!(json.contains("\"lifecycle\":\"CONFIRMED\""));
    }

    #[test]
    fn principal_serialization_is_explicit_and_stable() {
        let principal = Principal {
            kind: PrincipalKind::User,
            id: Some("42".into()),
            session_id: Some("firebase-session".into()),
            service_key_label: None,
            scopes: vec!["read:telemetry".into()],
        };
        let value = serde_json::to_value(&principal).unwrap();
        assert_eq!(value["kind"], "user");
        assert_eq!(value["id"], "42");
        assert_eq!(value["scopes"][0], "read:telemetry");
    }

    #[test]
    fn api_error_envelope_has_single_nested_error_shape() {
        let envelope = ApiErrorEnvelope {
            error: ApiErrorBody {
                code: "forbidden".into(),
                message: "Missing required scope".into(),
                details: serde_json::json!({"required_scope":"read:telemetry"}),
                request_id: Some("req-1".into()),
            },
        };
        let value = serde_json::to_value(&envelope).unwrap();
        assert!(value.get("error").unwrap().is_object());
        assert_eq!(value["error"]["code"], "forbidden");
        assert_eq!(
            value["error"]["details"]["required_scope"],
            "read:telemetry"
        );
    }

    #[test]
    fn journal_keeps_occurred_and_received_times_distinct() {
        let event = JournalEventEnvelope {
            id: 7,
            device_id: "device-1".into(),
            level: LogLevel::Info,
            category: LogCategory::Device,
            event_type: "device_status".into(),
            title: "status".into(),
            message: "online".into(),
            occurred_at: "2026-09-16T10:00:00Z".into(),
            received_at: "2026-09-16T10:00:01Z".into(),
            source: "controller".into(),
            reason: None,
            reason_code: None,
            actor: None,
            resource_id: Some("device-1".into()),
            correlation_id: None,
            resolved_at: None,
            resolved_by: None,
            metadata: None,
        };
        let decoded: JournalEventEnvelope =
            serde_json::from_value(serde_json::to_value(&event).unwrap()).unwrap();
        assert_eq!(decoded.id, 7);
        assert_eq!(decoded.occurred_at, "2026-09-16T10:00:00Z");
        assert_eq!(decoded.received_at, "2026-09-16T10:00:01Z");
    }
}
