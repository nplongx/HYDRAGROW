use crate::diagnostic_model::{
    parse_diagnosis, Diagnosis, DiagnosticContext, DiagnosticModel, DiagnosticModelError,
};
use async_trait::async_trait;
use hydragrow_supervisor_query::{
    validate_and_clamp, validate_device_scope, QueryBackend, SupervisorQuery,
};
use std::sync::Arc;
use std::time::Duration;

pub struct OpenRouterDiagnosticModel {
    base_url: String,
    api_key: String,
    model: String,
    query_backend: Arc<dyn QueryBackend>,
    max_tool_round_trips: u32,
    max_input_tokens: u32,
    max_output_tokens: u32,
    wall_clock_budget_secs: u64,
    http: reqwest::Client,
}

impl OpenRouterDiagnosticModel {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_url: String,
        api_key: String,
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
            api_key,
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
            {"name":"sensor_history","description":"Raw sensor readings for the flagged axis, most recent first.","input_schema":{"type":"object","properties":{"device_id":{"type":"string"},"minutes":{"type":"integer"}},"required":["device_id"]}},
            {"name":"dosing_history","description":"Recent dosing events.","input_schema":{"type":"object","properties":{"device_id":{"type":"string"},"minutes":{"type":"integer"}},"required":["device_id"]}},
            {"name":"fsm_events","description":"Recent FSM transitions.","input_schema":{"type":"object","properties":{"device_id":{"type":"string"},"limit":{"type":"integer"}},"required":["device_id"]}},
            {"name":"health_topics","description":"Per-topic message freshness for this device.","input_schema":{"type":"object","properties":{"device_id":{"type":"string"}},"required":["device_id"]}}
        ])
    }

    fn system_prompt() -> &'static str {
        r#"You are a supervising diagnostic assistant for a hydroponic controller.
You can only read data through the tools provided. You cannot control anything.

Prioritize evidence in this order:
1. Current device_state snapshot over historical trend.
2. Hestia's own reasons over independent raw numbers.
3. Recent dosing/FSM events over older sensor history.
4. Longer sensor history only as trend context, never as the primary basis for a reason code.

When enough information is available, respond with ONLY a JSON object matching:
{"reason_codes":[...],"confidence":0.0,"narrative":"...","observations":{"current_value":...,"target_value":...,"delta":...,"window_minutes":...,"corroborating_evidence":[...]}}

Use only canonical reason codes supplied in the task. Never invent a reason code.
Do not return Markdown or code fences."#
    }

    async fn call_openrouter(
        &self,
        messages: &serde_json::Value,
    ) -> Result<serde_json::Value, DiagnosticModelError> {
        let resp = self
            .http
            .post(format!("{}/v1/messages", self.base_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("HTTP-Referer", "https://github.com/nplongx/HYDRAGROW")
            .header("X-Title", "HYDRAGROW Diagnostic Worker")
            .json(&serde_json::json!({
                "model": self.model,
                "max_tokens": self.max_output_tokens,
                "system": Self::system_prompt(),
                "tools": Self::tool_definitions(),
                "messages": messages
            }))
            .send()
            .await
            .map_err(|e| if e.is_timeout() {
                DiagnosticModelError::Timeout
            } else {
                DiagnosticModelError::ProviderError(e.to_string())
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DiagnosticModelError::ProviderError(format!(
                "OpenRouter HTTP {}: {}",
                status, body
            )));
        }

        resp.json()
            .await
            .map_err(|e| DiagnosticModelError::InvalidOutput(e.to_string()))
    }

    fn tool_call_to_query(name: &str, input: &serde_json::Value) -> Option<SupervisorQuery> {
        let mut tagged = input.clone();
        tagged["query_type"] = serde_json::Value::String(name.to_string());
        serde_json::from_value(tagged).ok()
    }

    fn error_json(error: impl std::fmt::Display) -> String {
        serde_json::json!({"error": error.to_string()}).to_string()
    }
}

#[async_trait]
impl DiagnosticModel for OpenRouterDiagnosticModel {
    async fn diagnose(
        &self,
        context: DiagnosticContext,
    ) -> Result<Diagnosis, DiagnosticModelError> {
        let started = std::time::Instant::now();
        let task_message = serde_json::json!({
            "role": "user",
            "content": format!(
                "Device: {}\nTrigger: {:?}\nHestia snapshot: {}\nCrop target: {}\nCanonical reason codes: {:?}",
                context.device_id,
                context.trigger,
                context.hestia_snapshot,
                context.crop_target,
                hydragrow_shared::supervisor::SupervisorReasonCode::all_as_str(),
            ),
        });
        let mut messages = vec![task_message];

        for _round in 0..self.max_tool_round_trips {
            if started.elapsed().as_secs() >= self.wall_clock_budget_secs {
                return Err(DiagnosticModelError::BudgetExceeded("wall_clock".to_string()));
            }

            let estimated_tokens = serde_json::to_string(&messages)
                .map(|s| s.len() / 4)
                .unwrap_or(0) as u32;
            if estimated_tokens > self.max_input_tokens {
                return Err(DiagnosticModelError::BudgetExceeded(
                    "max_input_tokens".to_string(),
                ));
            }

            let response = self
                .call_openrouter(&serde_json::Value::Array(messages.clone()))
                .await?;
            let content = response
                .get("content")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();

            let tool_uses: Vec<&serde_json::Value> = content
                .iter()
                .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
                .collect();

            if tool_uses.is_empty() {
                let text = content
                    .iter()
                    .find(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                    .and_then(|b| b.get("text"))
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| {
                        DiagnosticModelError::InvalidOutput(
                            "no text or tool_use in OpenRouter response".to_string(),
                        )
                    })?;
                let parsed: serde_json::Value = serde_json::from_str(text)
                    .map_err(|e| DiagnosticModelError::InvalidOutput(e.to_string()))?;
                return parse_diagnosis(&parsed);
            }

            messages.push(serde_json::json!({"role":"assistant","content":content}));

            let mut tool_results = Vec::new();
            for tool_use in tool_uses {
                let id = tool_use.get("id").and_then(|i| i.as_str()).unwrap_or("");
                let name = tool_use.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let input = tool_use.get("input").cloned().unwrap_or_else(|| serde_json::json!({}));

                let result_text = match Self::tool_call_to_query(name, &input) {
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

                tool_results.push(serde_json::json!({
                    "type":"tool_result",
                    "tool_use_id":id,
                    "content":result_text
                }));
            }

            messages.push(serde_json::json!({"role":"user","content":tool_results}));
        }

        Err(DiagnosticModelError::BudgetExceeded(
            "max_tool_round_trips".to_string(),
        ))
    }
}
