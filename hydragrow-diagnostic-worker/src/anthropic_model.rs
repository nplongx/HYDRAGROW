use crate::diagnostic_model::{
    Diagnosis, DiagnosticContext, DiagnosticModel, DiagnosticModelError, parse_diagnosis,
};
use async_trait::async_trait;
use hydragrow_supervisor_query::{
    QueryBackend, SupervisorQuery, validate_and_clamp, validate_device_scope,
};
use std::sync::Arc;
use std::time::Duration;

pub struct AnthropicDiagnosticModel {
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

impl AnthropicDiagnosticModel {
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

    /// Design spec §8.3: the four model-callable tools. `HestiaSnapshot` and
    /// `CropTarget` are deliberately absent — they're pre-fetched into
    /// `DiagnosticContext` before this loop starts, not offered as tools.
    fn tool_definitions() -> serde_json::Value {
        serde_json::json!([
            {"name": "sensor_history", "description": "Raw sensor readings for the flagged axis, most recent first.",
             "input_schema": {"type": "object", "properties": {"device_id": {"type": "string"}, "minutes": {"type": "integer"}}, "required": ["device_id"]}},
            {"name": "dosing_history", "description": "Recent dosing events.",
             "input_schema": {"type": "object", "properties": {"device_id": {"type": "string"}, "minutes": {"type": "integer"}}, "required": ["device_id"]}},
            {"name": "fsm_events", "description": "Recent FSM transitions.",
             "input_schema": {"type": "object", "properties": {"device_id": {"type": "string"}, "limit": {"type": "integer"}}, "required": ["device_id"]}},
            {"name": "health_topics", "description": "Per-topic message freshness for this device.",
             "input_schema": {"type": "object", "properties": {"device_id": {"type": "string"}}, "required": ["device_id"]}},
        ])
    }

    /// Design spec §8.3/§8.6: role, tool contract, output schema, and the
    /// source-of-truth priority order, all stated explicitly rather than
    /// left for the model to infer.
    fn system_prompt() -> &'static str {
        r#"You are a supervising diagnostic assistant for a hydroponic controller.
You can only read data through the tools provided. You cannot control anything —
there is no tool for dosing, pump control, acknowledgment, or reset.

When information conflicts, prioritize in this order:
1. The current device_state snapshot over any historical trend.
2. Hestia's own reasons over your independent read of raw numbers — Hestia
   already accounts for dosing-suppression windows and config tolerances you
   were not given directly.
3. Recent events (dosing, FSM transitions) over older sensor history.
4. Longer sensor history only as trend context, never as the primary basis
   for a reason code.

When you have enough information, respond with ONLY a JSON object (no other
text) matching exactly this shape:
{"reason_codes": [...], "confidence": 0.0-1.0, "narrative": "...",
 "observations": {"current_value": ..., "target_value": ..., "delta": ...,
                   "window_minutes": ..., "corroborating_evidence": [...]}}
reason_codes must be chosen only from the canonical set you'll be told about
in the task message — never invent a new one."#
    }

    async fn call_anthropic(
        &self,
        messages: &serde_json::Value,
    ) -> Result<serde_json::Value, DiagnosticModelError> {
        let resp = self
            .http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "max_tokens": self.max_output_tokens,
                "system": Self::system_prompt(),
                "tools": Self::tool_definitions(),
                "messages": messages,
            }))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DiagnosticModelError::Timeout
                } else {
                    DiagnosticModelError::ProviderError(e.to_string())
                }
            })?;

        if !resp.status().is_success() {
            return Err(DiagnosticModelError::ProviderError(format!(
                "status {}",
                resp.status()
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
}

#[async_trait]
impl DiagnosticModel for AnthropicDiagnosticModel {
    async fn diagnose(
        &self,
        context: DiagnosticContext,
    ) -> Result<Diagnosis, DiagnosticModelError> {
        let started = std::time::Instant::now();
        let task_message = serde_json::json!({
            "role": "user",
            "content": format!(
                "Device: {}\nTrigger: {:?}\nHestia snapshot: {}\nCrop target: {}\nCanonical reason codes: {:?}",
                context.device_id, context.trigger, context.hestia_snapshot, context.crop_target,
                hydragrow_shared::supervisor::SupervisorReasonCode::all_as_str(),
            ),
        });
        let mut messages = vec![task_message];

        for _round in 0..self.max_tool_round_trips {
            if started.elapsed().as_secs() >= self.wall_clock_budget_secs {
                return Err(DiagnosticModelError::BudgetExceeded(
                    "wall_clock".to_string(),
                ));
            }

            // Design spec §8.5: max input tokens per diagnosis. Real
            // tokenization isn't available without a provider round trip,
            // so this uses the same rough heuristic Anthropic's own docs
            // suggest for estimation purposes (~4 chars/token) — good
            // enough to catch a runaway conversation before it happens,
            // not meant to be exact.
            let estimated_tokens = serde_json::to_string(&messages)
                .map(|s| s.len() / 4)
                .unwrap_or(0) as u32;
            if estimated_tokens > self.max_input_tokens {
                return Err(DiagnosticModelError::BudgetExceeded(
                    "max_input_tokens".to_string(),
                ));
            }

            let response = self
                .call_anthropic(&serde_json::Value::Array(messages.clone()))
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
                            "no text or tool_use in response".to_string(),
                        )
                    })?;
                let parsed: serde_json::Value = serde_json::from_str(text)
                    .map_err(|e| DiagnosticModelError::InvalidOutput(e.to_string()))?;
                return parse_diagnosis(&parsed);
            }

            messages.push(serde_json::json!({"role": "assistant", "content": content}));

            let mut tool_results = Vec::new();
            for tool_use in tool_uses {
                let id = tool_use.get("id").and_then(|i| i.as_str()).unwrap_or("");
                let name = tool_use.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let input = tool_use
                    .get("input")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                let result_text = match Self::tool_call_to_query(name, &input) {
                    Some(query) => match validate_device_scope(&query, &context.device_id) {
                        Ok(()) => match validate_and_clamp(query) {
                            Ok(clamped) => match self.query_backend.execute(clamped).await {
                                Ok(result) => serde_json::to_string(&result).unwrap_or_default(),
                                Err(e) => format!("{{\"error\": \"{e}\"}}"),
                            },
                            Err(e) => format!("{{\"error\": \"{e}\"}}"),
                        },
                        Err(e) => format!("{{\"error\": \"{e}\"}}"),
                    },
                    None => format!("{{\"error\": \"unknown or malformed tool call: {name}\"}}"),
                };

                tool_results.push(serde_json::json!({
                    "type": "tool_result", "tool_use_id": id, "content": result_text
                }));
            }
            messages.push(serde_json::json!({"role": "user", "content": tool_results}));
        }

        Err(DiagnosticModelError::BudgetExceeded(
            "max_tool_round_trips".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trigger::SupervisorTrigger;
    use hydragrow_supervisor_query::testing::FakeQueryBackend;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn context() -> DiagnosticContext {
        DiagnosticContext {
            device_id: "d1".to_string(),
            trigger: SupervisorTrigger::HestiaState {
                state: "WARNING".to_string(),
                reasons: vec!["ec_out_of_range".to_string()],
            },
            hestia_snapshot: serde_json::json!({"state": "WARNING"}),
            crop_target: serde_json::json!({"ec_target": 1.8}),
        }
    }

    #[tokio::test]
    async fn diagnose_errors_with_budget_exceeded_when_estimated_input_tokens_too_high() {
        let mut server = mockito::Server::new_async().await;
        // Never actually reached — the budget check must trip before the
        // first call whenever the task message alone already exceeds it.
        let _m = server
            .mock("POST", "/v1/messages")
            .expect(0)
            .create_async()
            .await;

        let model = AnthropicDiagnosticModel::new(
            server.url(),
            "sk-test".to_string(),
            "claude-sonnet-5".to_string(),
            std::sync::Arc::new(FakeQueryBackend::default()),
            5,
            /* max_input_tokens */ 1,
            1024,
            25,
            60,
        );
        let result = model.diagnose(context()).await;
        assert!(matches!(
            result,
            Err(DiagnosticModelError::BudgetExceeded(_))
        ));
    }

    #[tokio::test]
    async fn diagnose_returns_final_text_response_with_no_tool_calls() {
        let mut server = mockito::Server::new_async().await;
        let final_diagnosis = serde_json::json!({
            "reason_codes": ["ec_out_of_range"], "confidence": 0.8,
            "narrative": "EC is above target.",
            "observations": {"current_value": 2.2, "target_value": 1.8, "delta": 0.4, "window_minutes": 10, "corroborating_evidence": []}
        });
        let _m = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "content": [{"type": "text", "text": final_diagnosis.to_string()}]
                })
                .to_string(),
            )
            .create_async()
            .await;

        let model = AnthropicDiagnosticModel::new(
            server.url(),
            "sk-test".to_string(),
            "claude-sonnet-5".to_string(),
            std::sync::Arc::new(FakeQueryBackend::default()),
            5,
            8000,
            1024,
            25,
            60,
        );
        let diagnosis = model.diagnose(context()).await.unwrap();
        assert_eq!(diagnosis.confidence, 0.8);
    }

    #[tokio::test]
    async fn diagnose_dispatches_tool_use_then_returns_final_answer() {
        let mut server = mockito::Server::new_async().await;
        let tool_call_response = serde_json::json!({
            "content": [{"type": "tool_use", "id": "call_1", "name": "sensor_history", "input": {"device_id": "d1", "minutes": 30}}]
        });
        let final_diagnosis = serde_json::json!({
            "reason_codes": ["leak_suspected"], "confidence": 0.6,
            "narrative": "EC rose with no dosing event.",
            "observations": {"current_value": 2.7, "target_value": 1.8, "delta": 0.9, "window_minutes": 30, "corroborating_evidence": ["no dosing event"]}
        });
        let final_response =
            serde_json::json!({"content": [{"type": "text", "text": final_diagnosis.to_string()}]});

        // Second mock for when tool_result is sent back
        let _m2 = server
            .mock("POST", "/v1/messages")
            .match_body(mockito::Matcher::Regex("tool_result".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(final_response.to_string())
            .expect(1)
            .create_async()
            .await;

        // First mock for tool call
        let _m1 = server
            .mock("POST", "/v1/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(tool_call_response.to_string())
            .expect(1)
            .create_async()
            .await;

        let backend = std::sync::Arc::new(FakeQueryBackend {
            responses: Mutex::new(HashMap::from([(
                "sensor_history".to_string(),
                serde_json::json!({"data": []}),
            )])),
            calls: Mutex::new(vec![]),
        });

        let model = AnthropicDiagnosticModel::new(
            server.url(),
            "sk-test".to_string(),
            "claude-sonnet-5".to_string(),
            backend.clone(),
            5,
            8000,
            1024,
            25,
            60,
        );

        let result = model.diagnose(context()).await;
        let diagnosis = result.unwrap();
        assert_eq!(diagnosis.confidence, 0.6);
        assert_eq!(backend.calls.lock().unwrap().len(), 1);
    }
}
