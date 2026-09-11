use crate::diagnostic_model::{parse_diagnosis, Diagnosis, DiagnosticContext, DiagnosticModel, DiagnosticModelError};
use async_trait::async_trait;
use hydragrow_supervisor_query::{validate_and_clamp, validate_device_scope, QueryBackend, SupervisorQuery};
use std::sync::Arc;
use std::time::Duration;

pub struct LocalLlamaDiagnosticModel {
    base_url: String,
    model: String,
    query_backend: Arc<dyn QueryBackend>,
    max_tool_round_trips: u32,
    max_input_tokens: u32,
    max_output_tokens: u32,
    wall_clock_budget_secs: u64,
    http: reqwest::Client,
}

impl LocalLlamaDiagnosticModel {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_url: String,
        model: String,
        query_backend: Arc<dyn QueryBackend>,
        max_tool_round_trips: u32,
        max_input_tokens: u32,
        max_output_tokens: u32,
        per_call_timeout_secs: u64,
        wall_clock_budget_secs: u64,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(per_call_timeout_secs))
            .build()
            .expect("reqwest client build should not fail with these settings");
        Self {
            base_url,
            model,
            query_backend,
            max_tool_round_trips,
            max_input_tokens,
            max_output_tokens,
            wall_clock_budget_secs,
            http,
        }
    }

    fn tool_definitions() -> serde_json::Value {
        serde_json::json!([
            {"type":"function","function":{"name":"sensor_history","description":"Raw sensor readings for the flagged axis, most recent first.","parameters":{"type":"object","properties":{"device_id":{"type":"string"},"minutes":{"type":"integer"}},"required":["device_id"]}}},
            {"type":"function","function":{"name":"dosing_history","description":"Recent dosing events.","parameters":{"type":"object","properties":{"device_id":{"type":"string"},"minutes":{"type":"integer"}},"required":["device_id"]}}},
            {"type":"function","function":{"name":"fsm_events","description":"Recent FSM transitions.","parameters":{"type":"object","properties":{"device_id":{"type":"string"},"limit":{"type":"integer"}},"required":["device_id"]}}},
            {"type":"function","function":{"name":"health_topics","description":"Per-topic message freshness for this device.","parameters":{"type":"object","properties":{"device_id":{"type":"string"}},"required":["device_id"]}}}
        ])
    }

    fn system_prompt() -> &'static str {
        r#"You are a supervising diagnostic assistant for a hydroponic controller.
You can only read data through the tools provided. You cannot control anything.
Prioritize current device state, Hestia reasons, recent dosing/FSM events, then longer sensor history.
When enough information is available, respond with ONLY a JSON object matching:
{"reason_codes":[...],"confidence":0.0,"narrative":"...","observations":{"current_value":...,"target_value":...,"delta":...,"window_minutes":...,"corroborating_evidence":[...]}}
Use only canonical reason codes supplied in the task. Never invent a reason code.
Do not return Markdown or code fences."#
    }

    async fn call_llama(&self, messages: &[serde_json::Value]) -> Result<serde_json::Value, DiagnosticModelError> {
        let resp = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url.trim_end_matches('/')))
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "messages": messages,
                "tools": Self::tool_definitions(),
                "tool_choice": "auto",
                "max_tokens": self.max_output_tokens,
                "temperature": 0.1
            }))
            .send()
            .await
            .map_err(|e| if e.is_timeout() { DiagnosticModelError::Timeout } else { DiagnosticModelError::ProviderError(e.to_string()) })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DiagnosticModelError::ProviderError(format!("Local llama HTTP {}: {}", status, body)));
        }
        resp.json().await.map_err(|e| DiagnosticModelError::InvalidOutput(e.to_string()))
    }

    fn tool_call_to_query(name: &str, arguments: &str) -> Option<SupervisorQuery> {
        let mut input: serde_json::Value = serde_json::from_str(arguments).ok()?;
        input["query_type"] = serde_json::Value::String(name.to_string());
        serde_json::from_value(input).ok()
    }

    fn error_json(error: impl std::fmt::Display) -> String {
        serde_json::json!({"error": error.to_string()}).to_string()
    }
}

#[async_trait]
impl DiagnosticModel for LocalLlamaDiagnosticModel {
    async fn diagnose(&self, context: DiagnosticContext) -> Result<Diagnosis, DiagnosticModelError> {
        let started = std::time::Instant::now();
        let mut messages = vec![
            serde_json::json!({"role":"system","content":Self::system_prompt()}),
            serde_json::json!({"role":"user","content":format!(
                "Device: {}\nTrigger: {:?}\nHestia snapshot: {}\nCrop target: {}\nCanonical reason codes: {:?}",
                context.device_id, context.trigger, context.hestia_snapshot, context.crop_target,
                hydragrow_shared::supervisor::SupervisorReasonCode::all_as_str()
            )}),
        ];

        for _round in 0..self.max_tool_round_trips {
            if started.elapsed().as_secs() >= self.wall_clock_budget_secs {
                return Err(DiagnosticModelError::BudgetExceeded("wall_clock".to_string()));
            }
            let estimated_tokens = serde_json::to_string(&messages).map(|s| s.len() / 4).unwrap_or(0) as u32;
            if estimated_tokens > self.max_input_tokens {
                return Err(DiagnosticModelError::BudgetExceeded("max_input_tokens".to_string()));
            }

            let response = self.call_llama(&messages).await?;
            let message = response.get("choices").and_then(|c| c.get(0)).and_then(|c| c.get("message"))
                .ok_or_else(|| DiagnosticModelError::InvalidOutput("missing choices[0].message".to_string()))?;
            let tool_calls = message.get("tool_calls").and_then(|v| v.as_array()).cloned().unwrap_or_default();

            if tool_calls.is_empty() {
                let text = message.get("content").and_then(|v| v.as_str()).ok_or_else(|| DiagnosticModelError::InvalidOutput("missing assistant content".to_string()))?;
                let parsed: serde_json::Value = serde_json::from_str(text.trim()).map_err(|e| DiagnosticModelError::InvalidOutput(e.to_string()))?;
                return parse_diagnosis(&parsed);
            }

            messages.push(message.clone());
            for tool_call in tool_calls {
                let id = tool_call.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let function = tool_call.get("function").cloned().unwrap_or_else(|| serde_json::json!({}));
                let name = function.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let arguments = function.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}");
                let result_text = match Self::tool_call_to_query(name, arguments) {
                    Some(query) => match validate_device_scope(&query, &context.device_id) {
                        Ok(()) => match validate_and_clamp(query) {
                            Ok(clamped) => match self.query_backend.execute(clamped).await {
                                Ok(result) => serde_json::to_string(&result).unwrap_or_else(Self::error_json),
                                Err(e) => Self::error_json(e),
                            },
                            Err(e) => Self::error_json(e),
                        },
                        Err(e) => Self::error_json(e),
                    },
                    None => Self::error_json(format!("unknown or malformed tool call: {name}")),
                };
                messages.push(serde_json::json!({"role":"tool","tool_call_id":id,"content":result_text}));
            }
        }

        Err(DiagnosticModelError::BudgetExceeded("max_tool_round_trips".to_string()))
    }
}
