use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::Path;

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

pub fn validate_scenario(scenario: &Scenario) -> Result<()> {
    for (i, fault) in scenario.faults.iter().enumerate() {
        match &fault.kind {
            FaultEventKind::PumpStuckOn { pump } | FaultEventKind::PumpStuckOff { pump } => {
                let p = pump.to_ascii_uppercase();
                let valid = matches!(
                    p.as_str(),
                    "PUMP_A"
                        | "PUMP_B"
                        | "PUMP_PH_UP"
                        | "PH_UP"
                        | "PUMP_PH_DOWN"
                        | "PH_DOWN"
                        | "WATER_PUMP_IN"
                        | "WATER_IN"
                        | "WATER_PUMP_OUT"
                        | "WATER_OUT"
                );
                if !valid {
                    bail!(
                        "fault index {} contains unknown pump target name: '{}'",
                        i,
                        pump
                    );
                }
            }
            FaultEventKind::SensorFrozen { sensor } => {
                let s = sensor.to_ascii_uppercase();
                let valid = matches!(s.as_str(), "EC" | "PH" | "TEMP" | "WATER_LEVEL" | "WATER");
                if !valid {
                    bail!(
                        "fault index {} contains unknown sensor target name: '{}'",
                        i,
                        sensor
                    );
                }
            }
        }
    }
    Ok(())
}

pub fn load_scenario(path: &Path) -> Result<Scenario> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario {}", path.display()))?;
    let scenario = serde_json::from_str::<Scenario>(&content)
        .with_context(|| format!("invalid scenario JSON in {}", path.display()))?;
    validate_scenario(&scenario)?;
    Ok(scenario)
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
        assert!(validate_scenario(&scenario).is_ok());
    }

    #[test]
    fn test_validate_scenario_unknown_pump() {
        let json = r#"{
            "initial_tank": { "volume_l": 10.0, "ec": 1.0, "ph": 6.0, "temp": 25.0, "water_level": 50.0 },
            "faults": [
                { "at_ms": 5000, "kind": "PumpStuckOn", "pump": "UNKNOWN_PUMP" }
            ]
        }"#;
        let scenario: Scenario = serde_json::from_str(json).unwrap();
        let res = validate_scenario(&scenario);
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("unknown pump target name")
        );
    }
}
