use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialTank {
    pub volume_l: f32,
    pub ec: f32,
    pub ph: f32,
    pub temp: f32,
    pub water_level: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FaultEventKind {
    PumpStuckOn { pump: String },
    PumpStuckOff { pump: String },
    SensorFrozen { sensor: String },
    // more as needed mapping to fsm faults
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultEvent {
    pub at_ms: u64,
    #[serde(flatten)]
    pub kind: FaultEventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub initial_tank: InitialTank,
    pub faults: Vec<FaultEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_scenario() {
        let json = r#"{
            "initial_tank": { "volume_l": 10.0, "ec": 1.0, "ph": 6.0, "temp": 25.0, "water_level": 50.0 },
            "faults": [
                { "at_ms": 5000, "kind": "PumpStuckOn", "pump": "PUMP_A" }
            ]
        }"#;
        let scenario: Scenario = serde_json::from_str(json).unwrap();
        assert_eq!(scenario.faults.len(), 1);
    }
}
