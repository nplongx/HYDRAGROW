use crate::trigger::SupervisorTrigger;
use async_trait::async_trait;
use hydragrow_shared::supervisor::SupervisorReasonCode;
use serde::Serialize;

/// Design spec §8.3: assembled once per diagnosis, given to the model as
/// task framing before any tool calls happen.
#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticContext {
    pub device_id: String,
    pub trigger: SupervisorTrigger,
    pub hestia_snapshot: serde_json::Value,
    pub crop_target: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosisObservations {
    pub current_value: f64,
    pub target_value: f64,
    pub delta: f64,
    pub window_minutes: u32,
    pub corroborating_evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnosis {
    pub reason_codes: Vec<SupervisorReasonCode>,
    pub confidence: f32,
    pub narrative: String,
    pub observations: DiagnosisObservations,
}

#[derive(Debug, thiserror::Error)]
pub enum DiagnosticModelError {
    #[error("diagnosis timed out")]
    Timeout,
    #[error("provider error: {0}")]
    ProviderError(String),
    #[error("invalid model output: {0}")]
    InvalidOutput(String),
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
}

#[async_trait]
pub trait DiagnosticModel: Send + Sync {
    async fn diagnose(&self, context: DiagnosticContext)
    -> Result<Diagnosis, DiagnosticModelError>;
}

/// Design spec §8.3: validated on receipt, not just requested via prompt.
/// `reason_codes` must be non-empty and every entry must parse against the
/// §9 enum; a model returning anything else is `InvalidOutput`, handled the
/// same as a timeout — fall back, never insert a partially-valid record.
pub fn parse_diagnosis(json: &serde_json::Value) -> Result<Diagnosis, DiagnosticModelError> {
    let raw_codes = json
        .get("reason_codes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| DiagnosticModelError::InvalidOutput("missing reason_codes".to_string()))?;
    if raw_codes.is_empty() {
        return Err(DiagnosticModelError::InvalidOutput(
            "reason_codes must not be empty".to_string(),
        ));
    }
    let reason_codes: Result<Vec<SupervisorReasonCode>, _> = raw_codes
        .iter()
        .map(|v| {
            v.as_str()
                .and_then(SupervisorReasonCode::from_str)
                .ok_or_else(|| {
                    DiagnosticModelError::InvalidOutput(format!("unknown reason_code: {v}"))
                })
        })
        .collect();
    let reason_codes = reason_codes?;

    let confidence = json
        .get("confidence")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| DiagnosticModelError::InvalidOutput("missing confidence".to_string()))?
        .clamp(0.0, 1.0) as f32;

    let narrative = json
        .get("narrative")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DiagnosticModelError::InvalidOutput("missing narrative".to_string()))?
        .to_string();

    let obs = json
        .get("observations")
        .ok_or_else(|| DiagnosticModelError::InvalidOutput("missing observations".to_string()))?;
    let observations = DiagnosisObservations {
        current_value: obs
            .get("current_value")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                DiagnosticModelError::InvalidOutput(
                    "missing observations.current_value".to_string(),
                )
            })?,
        target_value: obs
            .get("target_value")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                DiagnosticModelError::InvalidOutput("missing observations.target_value".to_string())
            })?,
        delta: obs.get("delta").and_then(|v| v.as_f64()).ok_or_else(|| {
            DiagnosticModelError::InvalidOutput("missing observations.delta".to_string())
        })?,
        window_minutes: obs
            .get("window_minutes")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                DiagnosticModelError::InvalidOutput(
                    "missing observations.window_minutes".to_string(),
                )
            })? as u32,
        corroborating_evidence: obs
            .get("corroborating_evidence")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
    };

    Ok(Diagnosis {
        reason_codes,
        confidence,
        narrative,
        observations,
    })
}

/// Design spec §8.1/§8.5: what gets written when the model doesn't reach a
/// valid conclusion within the round-trip, token, or wall-clock budget —
/// never silence, always a citable record of what Hestia itself already
/// knew.
pub fn unexplained_anomaly_fallback(trigger: &SupervisorTrigger) -> Diagnosis {
    let (narrative, evidence) = match trigger {
        SupervisorTrigger::HestiaState { state, reasons } => (
            format!("Hestia reported {state} ({}) but the diagnosis could not be completed within budget.", reasons.join(", ")),
            reasons.clone(),
        ),
        SupervisorTrigger::WatchdogBreach => (
            "A watchdog liveness breach was open but the diagnosis could not be completed within budget.".to_string(),
            vec!["watchdog_breach".to_string()],
        ),
    };
    Diagnosis {
        reason_codes: vec![SupervisorReasonCode::UnexplainedAnomaly],
        confidence: 0.0,
        narrative,
        observations: DiagnosisObservations {
            current_value: 0.0,
            target_value: 0.0,
            delta: 0.0,
            window_minutes: 0,
            corroborating_evidence: evidence,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json() -> serde_json::Value {
        serde_json::json!({
            "reason_codes": ["leak_suspected"],
            "confidence": 0.7,
            "narrative": "EC rose without a dosing event; water level also dropped.",
            "observations": {
                "current_value": 2.74, "target_value": 1.8, "delta": 0.61,
                "window_minutes": 10, "corroborating_evidence": ["no dosing event in window"]
            }
        })
    }

    #[test]
    fn parse_diagnosis_accepts_well_formed_output() {
        let diagnosis = parse_diagnosis(&valid_json()).unwrap();
        assert_eq!(diagnosis.reason_codes.len(), 1);
        assert_eq!(diagnosis.confidence, 0.7);
    }

    #[test]
    fn parse_diagnosis_clamps_out_of_range_confidence() {
        let mut json = valid_json();
        json["confidence"] = serde_json::json!(1.5);
        let diagnosis = parse_diagnosis(&json).unwrap();
        assert_eq!(diagnosis.confidence, 1.0);
    }

    #[test]
    fn parse_diagnosis_rejects_unknown_reason_code() {
        let mut json = valid_json();
        json["reason_codes"] = serde_json::json!(["not_a_real_code"]);
        assert!(parse_diagnosis(&json).is_err());
    }

    #[test]
    fn parse_diagnosis_rejects_empty_reason_codes() {
        let mut json = valid_json();
        json["reason_codes"] = serde_json::json!([]);
        assert!(parse_diagnosis(&json).is_err());
    }

    #[test]
    fn parse_diagnosis_rejects_missing_field() {
        let mut json = valid_json();
        json.as_object_mut().unwrap().remove("narrative");
        assert!(parse_diagnosis(&json).is_err());
    }

    #[test]
    fn unexplained_anomaly_fallback_carries_the_raw_trigger() {
        let trigger = crate::trigger::SupervisorTrigger::HestiaState {
            state: "WARNING".to_string(),
            reasons: vec!["ec_degrading".to_string()],
        };
        let diagnosis = unexplained_anomaly_fallback(&trigger);
        assert_eq!(
            diagnosis.reason_codes,
            vec![hydragrow_shared::supervisor::SupervisorReasonCode::UnexplainedAnomaly]
        );
        assert!(diagnosis.narrative.contains("ec_degrading"));
    }
}
