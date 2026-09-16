use hydragrow_shared::{
    ApiErrorEnvelope, CANONICAL_SCHEMA_VERSION, CommandWireRecord, JournalEventEnvelope, Principal,
    PrincipalKind,
};
use serde_json::Value;

use crate::{
    api::{error::normalize_error_json, middleware::auth::AuthContext},
    models::config::{
        DeviceConfig, DosingCalibration, SafetyConfig, SensorCalibration, WaterConfig, from_db_rows,
    },
    services::durable_command::DurableCommand,
};

/// Backend-to-wire adapter. Persistence fields not exposed by the canonical
/// contract stay in the durable model.
pub fn durable_command_to_wire(command: &DurableCommand) -> CommandWireRecord {
    CommandWireRecord {
        schema_version: CANONICAL_SCHEMA_VERSION,
        command_id: command.command_id.clone(),
        device_id: command.device_id.clone(),
        action: command.action.clone(),
        pump_id: command.pump_id.clone(),
        requested_state: command.requested_state,
        requested_pwm: command.requested_pwm,
        lifecycle: command.lifecycle,
        created_at_ms: command.created_at.timestamp_millis(),
        authorized_at_ms: command.authorized_at.timestamp_millis(),
        sent_at_ms: command.sent_at.map(|v| v.timestamp_millis()),
        acknowledged_at_ms: command.acknowledged_at.map(|v| v.timestamp_millis()),
        confirmed_at_ms: command.confirmed_at.map(|v| v.timestamp_millis()),
        terminal_at_ms: command.terminal_at.map(|v| v.timestamp_millis()),
        next_retry_at_ms: command.next_retry_at.map(|v| v.timestamp_millis()),
        attempt_count: command.attempt_count,
        last_error: command.last_error.clone(),
        last_observed_at_ms: command.last_observed_at.map(|v| v.timestamp_millis()),
        version: command.version,
        retry_safe: command.retry_safe,
    }
}

pub fn auth_context_to_principal(auth: &AuthContext) -> Principal {
    Principal {
        kind: if auth.user_id.is_some() {
            PrincipalKind::User
        } else {
            PrincipalKind::Service
        },
        id: auth.user_id.clone(),
        session_id: auth.session_id.clone(),
        service_key_label: auth.service_key_label.clone(),
        scopes: auth.scopes.clone(),
    }
}

pub fn system_event_to_journal(
    event: &crate::db::postgres::SystemEventRecord,
) -> JournalEventEnvelope {
    let metadata = event.metadata.clone();
    JournalEventEnvelope {
        id: event.id,
        device_id: event.device_id.clone(),
        level: hydragrow_shared::log::LogLevel::from_field(&event.level)
            .unwrap_or(hydragrow_shared::log::LogLevel::Info),
        category: hydragrow_shared::log::LogCategory::from_field(&event.category)
            .unwrap_or(hydragrow_shared::log::LogCategory::System),
        event_type: event.event_type.clone(),
        title: event.title.clone(),
        message: event.message.clone(),
        occurred_at: chrono::DateTime::from_timestamp_millis(event.timestamp)
            .unwrap_or(event.received_at)
            .to_rfc3339(),
        received_at: event.received_at.to_rfc3339(),
        source: event.source.clone(),
        reason: event.reason.clone(),
        reason_code: event.primary_reason_code.clone(),
        actor: event.actor_id.clone().map(|id| Principal {
            kind: match event.actor_kind.as_str() {
                "user" => PrincipalKind::User,
                _ => PrincipalKind::Service,
            },
            id: Some(id),
            session_id: None,
            service_key_label: None,
            scopes: Vec::new(),
        }),
        resource_id: metadata
            .as_ref()
            .and_then(|v| v.get("resource_id"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        correlation_id: metadata.as_ref().and_then(|v| {
            v.get("correlation_id")
                .or_else(|| v.get("cycle_id"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        }),
        resolved_at: event.resolved_at.map(|v| v.to_rfc3339()),
        resolved_by: event.resolved_by.clone(),
        metadata,
    }
}

pub fn normalize_api_error(
    status: actix_web::http::StatusCode,
    value: Value,
    request_id: Option<&str>,
) -> Result<ApiErrorEnvelope, serde_json::Error> {
    serde_json::from_value(normalize_error_json(status, value, request_id))
}

/// Canonical configuration adapter. Each SQL row remains an explicit storage
/// projection; the shared `ControllerConfig` is the wire/domain representation.
pub fn config_rows_to_wire(
    dev: &DeviceConfig,
    water: &WaterConfig,
    safe: &SafetyConfig,
    dose: &DosingCalibration,
    sensor: &SensorCalibration,
) -> hydragrow_shared::ControllerConfig {
    from_db_rows(dev, water, safe, dose, sensor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use hydragrow_shared::CommandLifecycle;

    fn command() -> DurableCommand {
        let t = DateTime::parse_from_rfc3339("2026-09-16T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        DurableCommand {
            command_id: "cmd-1".into(),
            device_id: "device-1".into(),
            action: "set_pwm".into(),
            pump_id: Some("PUMP_A".into()),
            requested_state: None,
            requested_pwm: Some(70),
            lifecycle: CommandLifecycle::Acknowledged,
            created_at: t,
            authorized_at: t,
            sent_at: Some(t + chrono::Duration::seconds(1)),
            acknowledged_at: Some(t + chrono::Duration::seconds(2)),
            confirmed_at: None,
            terminal_at: None,
            next_retry_at: None,
            attempt_count: 1,
            last_error: None,
            last_observed_at: Some(t + chrono::Duration::seconds(2)),
            version: 3,
            retry_safe: true,
        }
    }

    #[test]
    fn durable_command_adapter_preserves_lifecycle_and_all_timestamps() {
        let source = command();
        let wire = durable_command_to_wire(&source);
        assert_eq!(wire.schema_version, CANONICAL_SCHEMA_VERSION);
        assert_eq!(wire.lifecycle, CommandLifecycle::Acknowledged);
        assert_eq!(
            wire.sent_at_ms,
            Some(source.sent_at.unwrap().timestamp_millis())
        );
        assert_eq!(
            wire.acknowledged_at_ms,
            Some(source.acknowledged_at.unwrap().timestamp_millis())
        );
        assert_eq!(wire.confirmed_at_ms, None);
        assert_eq!(
            wire.last_observed_at_ms,
            Some(source.last_observed_at.unwrap().timestamp_millis())
        );
    }

    #[test]
    fn auth_adapter_preserves_user_scope_and_session_identity() {
        let auth = AuthContext {
            scopes: vec!["read:telemetry".into(), "write:config".into()],
            user_id: Some("42".into()),
            session_id: Some("firebase-uid".into()),
            service_key_label: None,
        };
        let principal = auth_context_to_principal(&auth);
        assert_eq!(principal.kind, PrincipalKind::User);
        assert_eq!(principal.id.as_deref(), Some("42"));
        assert_eq!(principal.session_id.as_deref(), Some("firebase-uid"));
        assert_eq!(principal.scopes.len(), 2);
    }

    #[test]
    fn error_adapter_always_returns_canonical_nested_envelope() {
        let value = normalize_api_error(
            actix_web::http::StatusCode::FORBIDDEN,
            serde_json::json!({"error":"Missing required scope","required_scope":"read:telemetry"}),
            Some("req-1"),
        )
        .unwrap();
        assert_eq!(value.error.code, "forbidden");
        assert_eq!(value.error.request_id.as_deref(), Some("req-1"));
        assert_eq!(value.error.details["required_scope"], "read:telemetry");
    }
}
