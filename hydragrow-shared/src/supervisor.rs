//! Canonical supervisor reason codes.
//!
//! The set is deliberately closed: only these 13 codes may be emitted.
//! Dedup keys off `(device_id, source, reason_code)`.

use serde::{Deserialize, Serialize};

/// Canonical reason a supervisor suggestion was raised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisorReasonCode {
    EcOutOfRange,
    EcDegrading,
    PhOutOfRange,
    PhDegrading,
    WaterLevelOutOfRange,
    WaterLevelDegrading,
    TempOutOfRange,
    TempDegrading,
    LeakSuspected,
    SensorFaultSuspected,
    DosingIneffective,
    UnexplainedAnomaly,
    TopicStaleControllerStatus,
}

impl SupervisorReasonCode {
    /// Snake-case wire representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EcOutOfRange => "ec_out_of_range",
            Self::EcDegrading => "ec_degrading",
            Self::PhOutOfRange => "ph_out_of_range",
            Self::PhDegrading => "ph_degrading",
            Self::WaterLevelOutOfRange => "water_level_out_of_range",
            Self::WaterLevelDegrading => "water_level_degrading",
            Self::TempOutOfRange => "temp_out_of_range",
            Self::TempDegrading => "temp_degrading",
            Self::LeakSuspected => "leak_suspected",
            Self::SensorFaultSuspected => "sensor_fault_suspected",
            Self::DosingIneffective => "dosing_ineffective",
            Self::UnexplainedAnomaly => "unexplained_anomaly",
            Self::TopicStaleControllerStatus => "topic_stale_controller_status",
        }
    }

    /// Inverse of [`Self::as_str`]; `None` for unknown strings.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ec_out_of_range" => Some(Self::EcOutOfRange),
            "ec_degrading" => Some(Self::EcDegrading),
            "ph_out_of_range" => Some(Self::PhOutOfRange),
            "ph_degrading" => Some(Self::PhDegrading),
            "water_level_out_of_range" => Some(Self::WaterLevelOutOfRange),
            "water_level_degrading" => Some(Self::WaterLevelDegrading),
            "temp_out_of_range" => Some(Self::TempOutOfRange),
            "temp_degrading" => Some(Self::TempDegrading),
            "leak_suspected" => Some(Self::LeakSuspected),
            "sensor_fault_suspected" => Some(Self::SensorFaultSuspected),
            "dosing_ineffective" => Some(Self::DosingIneffective),
            "unexplained_anomaly" => Some(Self::UnexplainedAnomaly),
            "topic_stale_controller_status" => Some(Self::TopicStaleControllerStatus),
            _ => None,
        }
    }

    /// All wire strings in canonical order.
    pub fn all_as_str() -> &'static [&'static str] {
        &[
            "ec_out_of_range",
            "ec_degrading",
            "ph_out_of_range",
            "ph_degrading",
            "water_level_out_of_range",
            "water_level_degrading",
            "temp_out_of_range",
            "temp_degrading",
            "leak_suspected",
            "sensor_fault_suspected",
            "dosing_ineffective",
            "unexplained_anomaly",
            "topic_stale_controller_status",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [SupervisorReasonCode; 13] = [
        SupervisorReasonCode::EcOutOfRange,
        SupervisorReasonCode::EcDegrading,
        SupervisorReasonCode::PhOutOfRange,
        SupervisorReasonCode::PhDegrading,
        SupervisorReasonCode::WaterLevelOutOfRange,
        SupervisorReasonCode::WaterLevelDegrading,
        SupervisorReasonCode::TempOutOfRange,
        SupervisorReasonCode::TempDegrading,
        SupervisorReasonCode::LeakSuspected,
        SupervisorReasonCode::SensorFaultSuspected,
        SupervisorReasonCode::DosingIneffective,
        SupervisorReasonCode::UnexplainedAnomaly,
        SupervisorReasonCode::TopicStaleControllerStatus,
    ];

    #[test]
    fn roundtrip_every_variant() {
        for code in ALL {
            assert_eq!(SupervisorReasonCode::from_str(code.as_str()), Some(code));
        }
        assert_eq!(SupervisorReasonCode::all_as_str().len(), ALL.len());
        for (code, s) in ALL.iter().zip(SupervisorReasonCode::all_as_str().iter()) {
            assert_eq!(code.as_str(), *s);
        }
    }

    #[test]
    fn from_str_none_for_unknown() {
        assert_eq!(SupervisorReasonCode::from_str("nope"), None);
        assert_eq!(SupervisorReasonCode::from_str(""), None);
        assert_eq!(SupervisorReasonCode::from_str("EC_OUT_OF_RANGE"), None);
    }

    #[test]
    fn snake_case_spot_checks() {
        assert_eq!(
            SupervisorReasonCode::WaterLevelOutOfRange.as_str(),
            "water_level_out_of_range"
        );
        assert_eq!(
            SupervisorReasonCode::TopicStaleControllerStatus.as_str(),
            "topic_stale_controller_status"
        );
        assert_eq!(
            SupervisorReasonCode::SensorFaultSuspected.as_str(),
            "sensor_fault_suspected"
        );
    }
}
