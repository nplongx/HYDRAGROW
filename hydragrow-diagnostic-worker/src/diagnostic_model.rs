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

/// Extracts a JSON value from LLM output, resiliently handling markdown code fences
/// (```json ... ```), surrounding whitespace, and conversational framing.
pub fn extract_json(text: &str) -> Result<serde_json::Value, DiagnosticModelError> {
    let trimmed = text.trim();
    if let Ok(val) = serde_json::from_str(trimmed) {
        return Ok(val);
    }

    if let Some(first) = first_markdown_fence(trimmed)
        && let Ok(val) = serde_json::from_str(first)
    {
        return Ok(val);
    }

    if let Some(stripped) = strip_markdown_fences(trimmed)
        && let Ok(val) = serde_json::from_str(stripped)
    {
        return Ok(val);
    }

    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}'))
        && start < end
        && let Ok(val) = serde_json::from_str(&trimmed[start..=end])
    {
        return Ok(val);
    }

    // Defensive pass: small LLMs sometimes emit literal `[...]` placeholder from prompt schemas
    let sanitized = trimmed.replace("[...]", "[]");
    if let Ok(val) = serde_json::from_str(&sanitized) {
        return Ok(val);
    }
    if let Some(stripped) = strip_markdown_fences(&sanitized)
        && let Ok(val) = serde_json::from_str(stripped)
    {
        return Ok(val);
    }
    if let (Some(start), Some(end)) = (sanitized.find('{'), sanitized.rfind('}'))
        && start < end
        && let Ok(val) = serde_json::from_str(&sanitized[start..=end])
    {
        return Ok(val);
    }

    // Defensive pass: small LLMs sometimes emit unquoted key-value pairs inside corroborating_evidence
    let sanitized_evidence = sanitize_corroborating_evidence(&sanitized);
    if let Ok(val) = serde_json::from_str(&sanitized_evidence) {
        return Ok(val);
    }
    if let Some(first) = first_markdown_fence(&sanitized_evidence)
        && let Ok(val) = serde_json::from_str(first)
    {
        return Ok(val);
    }
    if let Some(stripped) = strip_markdown_fences(&sanitized_evidence)
        && let Ok(val) = serde_json::from_str(stripped)
    {
        return Ok(val);
    }
    if let (Some(start), Some(end)) = (sanitized_evidence.find('{'), sanitized_evidence.rfind('}'))
        && start < end
        && let Ok(val) = serde_json::from_str(&sanitized_evidence[start..=end])
    {
        return Ok(val);
    }

    Err(DiagnosticModelError::InvalidOutput(format!(
        "could not parse JSON from model output: {text}"
    )))
}

fn sanitize_corroborating_evidence(s: &str) -> String {
    if let Some(start) = s.find("\"corroborating_evidence\"")
        && let Some(bracket_start) = s[start..].find('[')
    {
        let abs_bracket_start = start + bracket_start;
        if let Some(bracket_end) = s[abs_bracket_start..].find(']') {
            let abs_bracket_end = abs_bracket_start + bracket_end;
            let inner = &s[abs_bracket_start + 1..abs_bracket_end];
            if inner.contains(':') {
                return format!("{}[]{}", &s[..abs_bracket_start], &s[abs_bracket_end + 1..]);
            }
        }
    }
    s.to_string()
}

fn first_markdown_fence(s: &str) -> Option<&str> {
    let s = s.trim();
    if !s.starts_with("```") {
        return None;
    }
    let after_opening = s.find('\n').map(|idx| &s[idx + 1..])?;
    let end_idx = after_opening.find("```")?;
    Some(after_opening[..end_idx].trim())
}

fn strip_markdown_fences(s: &str) -> Option<&str> {
    let s = s.trim();
    if !s.starts_with("```") {
        return None;
    }
    let after_opening = s.find('\n').map(|idx| &s[idx + 1..])?;
    let content = if let Some(idx) = after_opening.rfind("```") {
        &after_opening[..idx]
    } else {
        after_opening
    };
    Some(content.trim())
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
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
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

    let parse_f64 = |val: Option<&serde_json::Value>,
                     field: &'static str|
     -> Result<f64, DiagnosticModelError> {
        val.and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .ok_or_else(|| DiagnosticModelError::InvalidOutput(format!("missing observations.{field}")))
    };

    let parse_u32 = |val: Option<&serde_json::Value>,
                     field: &'static str|
     -> Result<u32, DiagnosticModelError> {
        val.and_then(|v| {
            v.as_u64()
                .map(|n| n as u32)
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .ok_or_else(|| DiagnosticModelError::InvalidOutput(format!("missing observations.{field}")))
    };

    let observations = DiagnosisObservations {
        current_value: parse_f64(obs.get("current_value"), "current_value")?,
        target_value: parse_f64(obs.get("target_value"), "target_value")?,
        delta: parse_f64(obs.get("delta"), "delta")?,
        window_minutes: parse_u32(obs.get("window_minutes"), "window_minutes")?,
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

    #[test]
    fn extract_json_parses_plain_json() {
        let json_str = r#"{"key": "value", "num": 42}"#;
        let val = extract_json(json_str).unwrap();
        assert_eq!(val["key"], "value");
        assert_eq!(val["num"], 42);
    }

    #[test]
    fn extract_json_parses_markdown_code_fences() {
        let fenced = "```json\n{\"reason_codes\": [\"ec_out_of_range\"], \"confidence\": 0.8}\n```";
        let val = extract_json(fenced).unwrap();
        assert_eq!(val["reason_codes"][0], "ec_out_of_range");
        assert_eq!(val["confidence"], 0.8);
    }

    #[test]
    fn extract_json_parses_first_of_repeated_markdown_fences() {
        let repeated = "```json\n{\"reason_codes\": [\"ec_out_of_range\"], \"confidence\": 0.8}\n```\n\n```json\n{\"reason_codes\": [\"other\"]}\n```";
        let val = extract_json(repeated).unwrap();
        assert_eq!(val["reason_codes"][0], "ec_out_of_range");
        assert_eq!(val["confidence"], 0.8);
    }

    #[test]
    fn extract_json_parses_preamble_and_fences() {
        let text = "Here is the diagnosis result:\n```json\n{\"reason_codes\": [\"leak_suspected\"], \"confidence\": 0.9}\n```\nLet me know if you need more details.";
        let val = extract_json(text).unwrap();
        assert_eq!(val["reason_codes"][0], "leak_suspected");
    }

    #[test]
    fn extract_json_rejects_invalid_json() {
        let text = "This is just text without any json at all.";
        assert!(extract_json(text).is_err());
    }

    #[test]
    fn extract_json_sanitizes_literal_ellipsis_array_placeholder() {
        let text = "```json\n{\n  \"corroborating_evidence\": [...]\n}\n```";
        let val = extract_json(text).unwrap();
        assert_eq!(val["corroborating_evidence"], serde_json::json!([]));
    }

    #[test]
    fn extract_json_sanitizes_unquoted_key_values_in_corroborating_evidence() {
        let text = "```json\n{\n  \"reason_codes\": [\"ec_out_of_range\"],\n  \"confidence\": 0.8,\n  \"narrative\": \"summary\",\n  \"observations\": {\n    \"current_value\": 2.4,\n    \"target_value\": 1.8,\n    \"delta\": 0.6,\n    \"window_minutes\": 10,\n    \"corroborating_evidence\": [\"ec\": 2.45, \"ph\": 6.05]\n  }\n}\n```";
        let val = extract_json(text).unwrap();
        assert_eq!(val["reason_codes"][0], "ec_out_of_range");
        assert_eq!(
            val["observations"]["corroborating_evidence"],
            serde_json::json!([])
        );
    }

    #[test]
    fn parse_diagnosis_accepts_string_encoded_numbers() {
        let json = serde_json::json!({
            "reason_codes": ["ec_out_of_range"],
            "confidence": "0.85",
            "narrative": "EC is elevated.",
            "observations": {
                "current_value": "2.4",
                "target_value": "1.8",
                "delta": "0.6",
                "window_minutes": "15",
                "corroborating_evidence": []
            }
        });
        let diagnosis = parse_diagnosis(&json).unwrap();
        assert_eq!(diagnosis.confidence, 0.85);
        assert_eq!(diagnosis.observations.current_value, 2.4);
        assert_eq!(diagnosis.observations.target_value, 1.8);
        assert_eq!(diagnosis.observations.delta, 0.6);
        assert_eq!(diagnosis.observations.window_minutes, 15);
    }
}
