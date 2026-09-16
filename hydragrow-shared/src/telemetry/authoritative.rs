use crate::{
    PumpStatus, SensorData, sensors::IncomingSensorPayload, telemetry::health::DeviceHealthSnapshot,
};
use serde::{Deserialize, Serialize};

use super::operational::{
    ActuatorKnowledge, ContactState, FreshnessState, OPERATIONAL_FRESHNESS_THRESHOLD_SECS,
    OperationalState, RuntimeReadiness,
};

/// Quality of one observed telemetry axis. Unknown means no authoritative
/// observation is available; it must not be rendered as a numeric default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TelemetryQuality {
    Unknown,
    Valid,
    Stale,
    Error,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TelemetryAvailability {
    Unknown,
    Online,
    Degraded,
    Offline,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetrySource {
    ControllerSensor,
    BackendCache,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetryAxis {
    pub name: String,
    pub value: Option<f64>,
    pub unit: String,
    pub quality: TelemetryQuality,
    /// Source observation time. `None` means the source did not provide a
    /// trustworthy observation timestamp; receipt time must never replace it.
    pub observed_at: Option<String>,
    /// Backend/application receipt time, distinct from source observation time.
    pub received_at: Option<String>,
    pub source: TelemetrySource,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedActuatorState {
    pub pump_status: PumpStatus,
    pub observed_at: Option<String>,
    pub received_at: Option<String>,
    pub source: TelemetrySource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedFsmState {
    pub state: String,
    pub observed_at: Option<String>,
    pub received_at: Option<String>,
    pub source: TelemetrySource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthoritativeTelemetrySnapshot {
    pub device_id: String,
    pub observed_at: Option<String>,
    pub received_at: Option<String>,
    pub availability: TelemetryAvailability,
    pub axes: Vec<TelemetryAxis>,
    pub controller_health: Option<DeviceHealthSnapshot>,
    pub actuator: Option<ObservedActuatorState>,
    pub fsm: Option<ObservedFsmState>,
    /// Explicit runtime readiness observation; absence remains UNKNOWN.
    #[serde(default)]
    pub runtime_ready: Option<bool>,
    /// Set when equally ordered observations disagree; never auto-resolved.
    #[serde(default)]
    pub actuator_contradictory: bool,
    /// Canonical operational interpretation of the observations above.
    #[serde(default)]
    pub operational_state: OperationalState,
}

impl AuthoritativeTelemetrySnapshot {
    /// Adapt the existing SensorData wire shape into the canonical semantic
    /// model without inventing a missing observation timestamp.
    pub fn from_sensor_data(data: &SensorData, received_at: Option<String>) -> Self {
        let payload = IncomingSensorPayload {
            temp: Some(data.temp),
            ec: Some(data.ec),
            ph: Some(data.ph),
            water_level: Some(data.water_level),
            ph_voltage_mv: data.ph_voltage_mv.map(|v| v as f32),
            time: (!data.time.trim().is_empty()).then(|| data.time.clone()),
            rssi: data.rssi,
            free_heap: data.free_heap,
            uptime: data.uptime,
            is_continuous: data.is_continuous,
            err_water: data.err_water,
            err_temp: data.err_temp,
            err_ph: data.err_ph,
            err_ec: data.err_ec,
        };
        Self::from_incoming_payload(&data.device_id, &payload, received_at)
    }

    pub fn from_incoming_payload(
        device_id: &str,
        payload: &IncomingSensorPayload,
        received_at: Option<String>,
    ) -> Self {
        let observed_at = payload
            .time
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .cloned();

        Self {
            device_id: device_id.to_string(),
            observed_at: observed_at.clone(),
            received_at: received_at.clone(),
            availability: TelemetryAvailability::Online,
            axes: vec![
                axis(
                    "ph",
                    payload.ph.map(|v| v as f64),
                    "pH",
                    payload.err_ph,
                    &observed_at,
                    &received_at,
                ),
                axis(
                    "ec",
                    payload.ec.map(|v| v as f64),
                    "mS/cm",
                    payload.err_ec,
                    &observed_at,
                    &received_at,
                ),
                axis(
                    "temp",
                    payload.temp.map(|v| v as f64),
                    "°C",
                    payload.err_temp,
                    &observed_at,
                    &received_at,
                ),
                axis(
                    "water_level",
                    payload.water_level.map(|v| v as f64),
                    "cm",
                    payload.err_water,
                    &observed_at,
                    &received_at,
                ),
            ],
            controller_health: None,
            actuator: None,
            fsm: None,
            runtime_ready: None,
            actuator_contradictory: false,
            operational_state: OperationalState::unknown(),
        }
    }

    pub fn refresh_operational_state(&mut self, now: chrono::DateTime<chrono::Utc>) {
        let freshness = Self::classify_snapshot_freshness(self, now);
        let contact = match freshness {
            FreshnessState::Fresh => ContactState::Contacted,
            FreshnessState::Stale => ContactState::Contacted,
            FreshnessState::Unknown => ContactState::Unknown,
        };
        let actuator = if self.actuator_contradictory {
            ActuatorKnowledge::Contradictory
        } else {
            match (&self.actuator, freshness) {
                (Some(_), FreshnessState::Fresh) => ActuatorKnowledge::Known,
                (Some(_), FreshnessState::Stale) => ActuatorKnowledge::Stale,
                _ => ActuatorKnowledge::Unknown,
            }
        };
        let readiness = self
            .runtime_ready
            .map(|ready| {
                if ready {
                    RuntimeReadiness::Ready
                } else {
                    RuntimeReadiness::NotReady
                }
            })
            .unwrap_or(RuntimeReadiness::Unknown);
        self.operational_state = OperationalState {
            contact,
            freshness,
            readiness,
            actuator,
            classified_at: Some(now.to_rfc3339()),
            received_at: self.received_at.clone(),
            observed_at: self.observed_at.clone(),
        };
    }

    fn classify_snapshot_freshness(
        snapshot: &Self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> FreshnessState {
        OperationalState::classify_freshness(
            snapshot.received_at.as_deref(),
            now,
            OPERATIONAL_FRESHNESS_THRESHOLD_SECS,
        )
    }

    pub fn merge_into(&self, current: Option<&Self>) -> Self {
        let mut merged = current.cloned().unwrap_or_else(|| Self {
            device_id: self.device_id.clone(),
            observed_at: None,
            received_at: self.received_at.clone(),
            availability: TelemetryAvailability::Unknown,
            axes: self.axes.iter().map(TelemetryAxis::as_unknown).collect(),
            controller_health: None,
            actuator: None,
            fsm: None,
            runtime_ready: None,
            actuator_contradictory: false,
            operational_state: OperationalState::unknown(),
        });
        merged.device_id = self.device_id.clone();
        let mut accepted_observation = false;
        let incoming_controller_at = self.controller_health.as_ref().and_then(health_timestamp);
        let existing_controller_at = merged.controller_health.as_ref().and_then(health_timestamp);
        let controller_restarted = match (
            self.controller_health.as_ref(),
            merged.controller_health.as_ref(),
        ) {
            (Some(incoming), Some(existing)) => {
                incoming.uptime_sec < existing.uptime_sec
                    && newer_timestamp(
                        incoming_controller_at.as_deref(),
                        existing_controller_at.as_deref(),
                    )
            }
            _ => false,
        };
        if self.controller_health.is_some()
            && newer_timestamp(
                incoming_controller_at.as_deref(),
                existing_controller_at.as_deref(),
            )
        {
            merged.controller_health = self.controller_health.clone();
            if controller_restarted {
                // A controller reboot invalidates the previous runtime/actuator
                // observation. The new health packet proves contact/freshness,
                // but does not prove what survived or changed during reboot.
                merged.actuator = None;
                merged.fsm = None;
                merged.runtime_ready = None;
                merged.actuator_contradictory = false;
            }
            if newer_timestamp(self.observed_at.as_deref(), merged.observed_at.as_deref()) {
                merged.observed_at = self.observed_at.clone();
            }
            accepted_observation = true;
        }
        if let Some(actuator) = &self.actuator {
            let existing = merged.actuator.as_ref();
            let incoming_at = actuator.observed_at.as_deref();
            let existing_at = existing.and_then(|a| a.observed_at.as_deref());
            if incoming_at.is_some() && incoming_at == existing_at && existing != Some(actuator) {
                merged.actuator_contradictory = true;
            } else if existing.is_none() || newer_timestamp(incoming_at, existing_at) {
                merged.actuator = Some(actuator.clone());
                merged.actuator_contradictory = false;
                accepted_observation = true;
            }
        }
        if let Some(fsm) = &self.fsm
            && (merged.fsm.is_none()
                || newer_timestamp(
                    fsm.observed_at.as_deref(),
                    merged.fsm.as_ref().and_then(|f| f.observed_at.as_deref()),
                ))
        {
            merged.fsm = Some(fsm.clone());
            accepted_observation = true;
        }
        if self.runtime_ready.is_some()
            && newer_timestamp(
                incoming_controller_at.as_deref(),
                existing_controller_at.as_deref(),
            )
        {
            merged.runtime_ready = self.runtime_ready;
            accepted_observation = true;
        }

        for incoming in &self.axes {
            if incoming.quality == TelemetryQuality::Unknown {
                continue;
            }
            if let Some(existing) = merged
                .axes
                .iter_mut()
                .find(|axis| axis.name == incoming.name)
            {
                if is_newer_or_equal(incoming, existing) {
                    *existing = incoming.clone();
                    accepted_observation = true;
                }
            } else {
                merged.axes.push(incoming.clone());
                accepted_observation = true;
            }
        }
        if accepted_observation {
            merged.received_at = self.received_at.clone();
            merged.availability = self.availability;
        }
        if newer_timestamp(self.observed_at.as_deref(), merged.observed_at.as_deref()) {
            merged.observed_at = self.observed_at.clone();
        }
        merged
    }
}

impl TelemetryAxis {
    fn as_unknown(&self) -> Self {
        Self {
            name: self.name.clone(),
            value: None,
            unit: self.unit.clone(),
            quality: TelemetryQuality::Unknown,
            observed_at: None,
            received_at: None,
            source: self.source,
            error_code: None,
        }
    }
}

fn is_newer_or_equal(incoming: &TelemetryAxis, existing: &TelemetryAxis) -> bool {
    match (
        incoming.observed_at.as_deref(),
        existing.observed_at.as_deref(),
    ) {
        (Some(incoming), Some(existing)) => incoming >= existing,
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => true,
    }
}

fn health_timestamp(health: &DeviceHealthSnapshot) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(health.timestamp_ms as i64).map(|dt| dt.to_rfc3339())
}

fn newer_timestamp(incoming: Option<&str>, existing: Option<&str>) -> bool {
    let Some(incoming) = incoming else {
        return false;
    };
    let Ok(incoming) = chrono::DateTime::parse_from_rfc3339(incoming) else {
        return false;
    };
    let Some(existing) = existing else {
        return true;
    };
    let Ok(existing) = chrono::DateTime::parse_from_rfc3339(existing) else {
        return true;
    };
    incoming > existing
}

fn axis(
    name: &str,
    value: Option<f64>,
    unit: &str,
    error: Option<bool>,
    observed_at: &Option<String>,
    received_at: &Option<String>,
) -> TelemetryAxis {
    let quality = match error {
        Some(true) => TelemetryQuality::Error,
        _ if value.is_some() => TelemetryQuality::Valid,
        _ => TelemetryQuality::Unknown,
    };
    TelemetryAxis {
        name: name.to_string(),
        value: if quality == TelemetryQuality::Valid {
            value
        } else {
            None
        },
        unit: unit.to_string(),
        quality,
        observed_at: observed_at.clone(),
        received_at: received_at.clone(),
        source: TelemetrySource::ControllerSensor,
        error_code: error.filter(|v| *v).map(|_| format!("{name}_sensor_error")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SensorData {
        SensorData {
            device_id: "dev-1".into(),
            ec: 1.4,
            ph: 6.2,
            temp: 25.0,
            water_level: 20.0,
            pump_status: PumpStatus::default(),
            time: "2026-09-15T10:00:00Z".into(),
            controller_received_ms: None,
            rssi: None,
            free_heap: None,
            uptime: None,
            err_water: Some(true),
            err_temp: Some(false),
            err_ph: None,
            err_ec: None,
            is_continuous: None,
            ph_voltage_mv: None,
            ec_received_ms: None,
            ph_received_ms: None,
            temp_received_ms: None,
            water_received_ms: None,
        }
    }

    fn test_health(timestamp: &str, uptime_sec: u64) -> DeviceHealthSnapshot {
        DeviceHealthSnapshot {
            device_id: "dev-1".to_string(),
            free_heap: 100_000,
            uptime_sec,
            rssi: -40,
            health_score_percent: 100,
            fsm_state_display: "Monitoring".to_string(),
            log_drop_count: 0,
            firmware_version: "test".to_string(),
            kalman_confidence: None,
            matrix_update_count: 0,
            matrix_is_warm: false,
            hestia: None,
            timestamp_ms: chrono::DateTime::parse_from_rfc3339(timestamp)
                .unwrap()
                .timestamp_millis() as u64,
        }
    }

    fn test_actuator(observed_at: &str) -> ObservedActuatorState {
        ObservedActuatorState {
            pump_status: PumpStatus {
                pump_a: true,
                ..Default::default()
            },
            observed_at: Some(observed_at.to_string()),
            received_at: Some("2026-09-16T10:00:01Z".to_string()),
            source: TelemetrySource::ControllerSensor,
        }
    }

    #[test]
    fn preserves_source_observation_time_and_separate_receipt_time() {
        let snapshot = AuthoritativeTelemetrySnapshot::from_sensor_data(
            &sample(),
            Some("2026-09-15T10:01:00Z".to_string()),
        );
        assert_eq!(
            snapshot.observed_at.as_deref(),
            Some("2026-09-15T10:00:00Z")
        );
        assert_eq!(
            snapshot.received_at.as_deref(),
            Some("2026-09-15T10:01:00Z")
        );
        assert_eq!(
            snapshot.axes[0].observed_at.as_deref(),
            Some("2026-09-15T10:00:00Z")
        );
        assert_eq!(
            snapshot.axes[0].received_at.as_deref(),
            Some("2026-09-15T10:01:00Z")
        );
    }

    #[test]
    fn equally_ordered_conflicting_actuators_are_marked_contradictory() {
        let observed_at = Some("2026-09-15T10:00:00Z".to_string());
        let received_at = Some("2026-09-15T10:00:01Z".to_string());
        let mut first = AuthoritativeTelemetrySnapshot::from_incoming_payload(
            "dev-1",
            &IncomingSensorPayload::default(),
            received_at.clone(),
        );
        first.actuator = Some(ObservedActuatorState {
            pump_status: PumpStatus {
                pump_a: true,
                ..Default::default()
            },
            observed_at: observed_at.clone(),
            received_at: received_at.clone(),
            source: TelemetrySource::ControllerSensor,
        });
        let mut second = first.clone();
        second.actuator = Some(ObservedActuatorState {
            pump_status: PumpStatus::default(),
            ..first.actuator.clone().unwrap()
        });
        let mut merged = second.merge_into(Some(&first));
        merged.refresh_operational_state(chrono::Utc::now());
        assert!(merged.actuator_contradictory);
        assert_eq!(
            merged.operational_state.actuator,
            ActuatorKnowledge::Contradictory
        );
    }

    #[test]
    fn missing_runtime_ready_remains_unknown() {
        let mut snapshot = AuthoritativeTelemetrySnapshot::from_incoming_payload(
            "dev-1",
            &IncomingSensorPayload::default(),
            Some("2026-09-15T10:00:01Z".into()),
        );
        snapshot.refresh_operational_state(
            chrono::DateTime::parse_from_rfc3339("2026-09-15T10:00:02Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        );
        assert_eq!(
            snapshot.operational_state.readiness,
            RuntimeReadiness::Unknown
        );
        assert!(!snapshot.operational_state.safety_prerequisites_known());
    }

    #[test]
    fn sensor_error_is_error_not_zero_or_unknown() {
        let snapshot = AuthoritativeTelemetrySnapshot::from_sensor_data(
            &sample(),
            Some("2026-09-15T10:01:00Z".to_string()),
        );
        assert_eq!(snapshot.axes[3].quality, TelemetryQuality::Error);
        assert_eq!(snapshot.axes[3].value, None);
        assert_eq!(
            snapshot.axes[3].error_code.as_deref(),
            Some("water_level_sensor_error")
        );
    }

    #[test]
    fn missing_axis_is_unknown_and_has_no_value() {
        let payload = IncomingSensorPayload {
            ph: Some(6.2),
            ..Default::default()
        };
        let snapshot = AuthoritativeTelemetrySnapshot::from_incoming_payload(
            "dev-1",
            &payload,
            Some("2026-09-15T10:01:00Z".to_string()),
        );
        assert_eq!(snapshot.axes[0].quality, TelemetryQuality::Valid);
        assert_eq!(snapshot.axes[0].value, Some(6.2_f32 as f64));
        assert_eq!(snapshot.axes[1].quality, TelemetryQuality::Unknown);
        assert_eq!(snapshot.axes[1].value, None);
    }

    #[test]
    fn controller_restart_invalidates_previous_runtime_and_actuator_observations() {
        let first = AuthoritativeTelemetrySnapshot {
            device_id: "dev-1".to_string(),
            observed_at: Some("2026-09-16T10:00:00Z".to_string()),
            received_at: Some("2026-09-16T10:00:01Z".to_string()),
            availability: TelemetryAvailability::Online,
            axes: vec![],
            controller_health: Some(test_health("2026-09-16T10:00:00Z", 100)),
            actuator: Some(test_actuator("2026-09-16T10:00:00Z")),
            fsm: Some(ObservedFsmState {
                state: "RUNNING".to_string(),
                observed_at: Some("2026-09-16T10:00:00Z".to_string()),
                received_at: Some("2026-09-16T10:00:01Z".to_string()),
                source: TelemetrySource::ControllerSensor,
            }),
            runtime_ready: Some(true),
            actuator_contradictory: false,
            operational_state: OperationalState::unknown(),
        };
        let restarted = AuthoritativeTelemetrySnapshot {
            controller_health: Some(test_health("2026-09-16T10:01:00Z", 3)),
            observed_at: Some("2026-09-16T10:01:00Z".to_string()),
            received_at: Some("2026-09-16T10:01:01Z".to_string()),
            availability: TelemetryAvailability::Online,
            axes: vec![],
            actuator: None,
            fsm: None,
            runtime_ready: None,
            actuator_contradictory: false,
            operational_state: OperationalState::unknown(),
            device_id: "dev-1".to_string(),
        };

        let merged = restarted.merge_into(Some(&first));
        assert!(merged.actuator.is_none());
        assert!(merged.fsm.is_none());
        assert_eq!(merged.runtime_ready, None);
        assert!(!merged.actuator_contradictory);

        let mut classified = merged;
        classified.refresh_operational_state(
            chrono::DateTime::parse_from_rfc3339("2026-09-16T10:01:02Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        );
        assert_eq!(
            classified.operational_state.actuator,
            ActuatorKnowledge::Unknown
        );
        assert_eq!(
            classified.operational_state.readiness,
            RuntimeReadiness::Unknown
        );
    }

    #[test]
    fn partial_merge_preserves_unrelated_known_axis() {
        let first = IncomingSensorPayload {
            ph: Some(6.2),
            ec: Some(1.4),
            time: Some("2026-09-15T10:00:00Z".into()),
            ..Default::default()
        };
        let second = IncomingSensorPayload {
            ph: Some(6.3),
            time: Some("2026-09-15T10:01:00Z".into()),
            ..Default::default()
        };
        let first = AuthoritativeTelemetrySnapshot::from_incoming_payload(
            "dev-1",
            &first,
            Some("2026-09-15T10:00:01Z".to_string()),
        );
        let second = AuthoritativeTelemetrySnapshot::from_incoming_payload(
            "dev-1",
            &second,
            Some("2026-09-15T10:01:01Z".to_string()),
        );
        let merged = second.merge_into(Some(&first));
        assert_eq!(
            merged.axes.iter().find(|a| a.name == "ph").unwrap().value,
            Some(6.3_f32 as f64)
        );
        assert_eq!(
            merged.axes.iter().find(|a| a.name == "ec").unwrap().value,
            Some(1.4_f32 as f64)
        );
    }

    #[test]
    fn out_of_order_observation_does_not_replace_newer_axis() {
        let newer = IncomingSensorPayload {
            ph: Some(6.3),
            time: Some("2026-09-15T10:01:00Z".into()),
            ..Default::default()
        };
        let older = IncomingSensorPayload {
            ph: Some(6.1),
            time: Some("2026-09-15T10:00:00Z".into()),
            ..Default::default()
        };
        let newer = AuthoritativeTelemetrySnapshot::from_incoming_payload(
            "dev-1",
            &newer,
            Some("2026-09-15T10:01:01Z".to_string()),
        );
        let older = AuthoritativeTelemetrySnapshot::from_incoming_payload(
            "dev-1",
            &older,
            Some("2026-09-15T10:02:00Z".to_string()),
        );
        let merged = older.merge_into(Some(&newer));
        let ph = merged.axes.iter().find(|a| a.name == "ph").unwrap();
        assert_eq!(ph.value, Some(6.3_f32 as f64));
        assert_eq!(ph.observed_at.as_deref(), Some("2026-09-15T10:01:00Z"));
    }
}
