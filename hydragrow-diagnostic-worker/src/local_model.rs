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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trigger::SupervisorTrigger;
    use hydragrow_shared::supervisor::SupervisorReasonCode;
    use hydragrow_supervisor_query::testing::FakeQueryBackend;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[test]
    fn local_model_tool_definitions_are_strictly_read_only() {
        let tools = LocalLlamaDiagnosticModel::tool_definitions();
        let tools_array = tools
            .as_array()
            .expect("tool_definitions must return a JSON array");

        let expected = [
            "sensor_history",
            "dosing_history",
            "fsm_events",
            "health_topics",
        ];

        let actual_names: Vec<String> = tools_array
            .iter()
            .map(|t| {
                t.get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    .expect("each tool must have function.name")
                    .to_string()
            })
            .collect();

        let mut sorted_actual = actual_names.clone();
        sorted_actual.sort();
        let mut sorted_expected = expected.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        sorted_expected.sort();
        assert_eq!(sorted_actual, sorted_expected);

        let forbidden = [
            "pump", "dose", "refill", "drain", "misting", "mix", "control", "command", "actuator",
        ];

        for name in &actual_names {
            for word in forbidden {
                assert!(
                    !name.to_lowercase().contains(word),
                    "tool name '{}' contains forbidden actuator/control substring '{}'",
                    name,
                    word
                );
            }
        }
    }

    fn test_context() -> DiagnosticContext {
        DiagnosticContext {
            device_id: "dev-1".to_string(),
            trigger: SupervisorTrigger::HestiaState {
                state: "WARNING".to_string(),
                reasons: vec!["ec_out_of_range".to_string()],
            },
            hestia_snapshot: serde_json::json!({"state": "WARNING", "reasons": ["ec_out_of_range"]}),
            crop_target: serde_json::json!({"ec_target": 1.8}),
        }
    }

    fn test_model(
        base_url: String,
        query_backend: Arc<dyn QueryBackend>,
        max_tool_round_trips: u32,
        max_input_tokens: u32,
        wall_clock_budget_secs: u64,
    ) -> LocalLlamaDiagnosticModel {
        LocalLlamaDiagnosticModel::new(
            base_url,
            "qwen3-0.6b".to_string(),
            query_backend,
            max_tool_round_trips,
            max_input_tokens,
            1024,
            25,
            wall_clock_budget_secs,
        )
    }

    fn final_diagnosis_body() -> serde_json::Value {
        serde_json::json!({
            "reason_codes": ["ec_out_of_range"],
            "confidence": 0.8,
            "narrative": "EC is outside the target range.",
            "observations": {
                "current_value": 2.4,
                "target_value": 1.8,
                "delta": 0.6,
                "window_minutes": 10,
                "corroborating_evidence": ["recent sensor history"]
            }
        })
    }

    #[tokio::test]
    async fn diagnose_returns_mocked_final_answer() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!({
                    "choices": [{"message": {"content": final_diagnosis_body().to_string()}}]
                })
                .to_string(),
            )
            .expect(1)
            .create_async()
            .await;

        let backend = Arc::new(FakeQueryBackend::default());
        let model = test_model(server.url(), backend, 5, 8000, 60);
        let diagnosis = model.diagnose(test_context()).await.unwrap();

        assert_eq!(
            diagnosis.reason_codes,
            vec![SupervisorReasonCode::EcOutOfRange]
        );
        assert_eq!(diagnosis.confidence, 0.8);
        assert_eq!(diagnosis.narrative, "EC is outside the target range.");
        assert_eq!(diagnosis.observations.current_value, 2.4);
        assert_eq!(diagnosis.observations.target_value, 1.8);
        assert_eq!(diagnosis.observations.delta, 0.6);
        assert_eq!(diagnosis.observations.window_minutes, 10);
        assert_eq!(
            diagnosis.observations.corroborating_evidence,
            vec!["recent sensor history".to_string()]
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn diagnose_executes_sensor_history_tool_call_then_returns_final_answer() {
        let mut server = mockito::Server::new_async().await;
        let tool_call_response = serde_json::json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call-1",
                        "type": "function",
                        "function": {
                            "name": "sensor_history",
                            "arguments": "{\"device_id\":\"dev-1\",\"minutes\":10}"
                        }
                    }]
                }
            }]
        });
        let final_response = serde_json::json!({
            "choices": [{"message": {"content": final_diagnosis_body().to_string()}}]
        });

        // Created first so it wins for the follow-up request carrying the tool result.
        let mock_final = server
            .mock("POST", "/v1/chat/completions")
            .match_body(mockito::Matcher::Regex("tool_call_id".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(final_response.to_string())
            .expect(1)
            .create_async()
            .await;

        let mock_tool_call = server
            .mock("POST", "/v1/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(tool_call_response.to_string())
            .expect(1)
            .create_async()
            .await;

        let backend = Arc::new(FakeQueryBackend {
            responses: Mutex::new(HashMap::from([(
                "sensor_history".to_string(),
                serde_json::json!({"readings": [{"ec": 2.4, "minutes_ago": 1}]}),
            )])),
            calls: Mutex::new(vec![]),
        });

        let model = test_model(server.url(), backend.clone(), 5, 8000, 60);
        let diagnosis = model.diagnose(test_context()).await.unwrap();

        assert_eq!(
            diagnosis.reason_codes,
            vec![SupervisorReasonCode::EcOutOfRange]
        );
        assert_eq!(diagnosis.confidence, 0.8);
        assert_eq!(diagnosis.narrative, "EC is outside the target range.");

        let calls = backend.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tag(), "sensor_history");
        assert_eq!(calls[0].device_id(), "dev-1");

        mock_tool_call.assert_async().await;
        mock_final.assert_async().await;
    }

    #[tokio::test]
    async fn diagnose_returns_budget_exceeded_on_zero_wall_clock_budget() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .expect(0)
            .create_async()
            .await;

        let backend = Arc::new(FakeQueryBackend::default());
        let model = test_model(server.url(), backend, 5, 8000, 0);
        assert!(matches!(
            model.diagnose(test_context()).await,
            Err(DiagnosticModelError::BudgetExceeded(_))
        ));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn diagnose_returns_budget_exceeded_when_tool_round_trips_exhausted() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .expect(0)
            .create_async()
            .await;

        let backend = Arc::new(FakeQueryBackend::default());
        let model = test_model(server.url(), backend, 0, 8000, 60);
        assert!(matches!(
            model.diagnose(test_context()).await,
            Err(DiagnosticModelError::BudgetExceeded(_))
        ));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn diagnose_returns_budget_exceeded_when_input_token_budget_too_small() {
        let mut server = mockito::Server::new_async().await;
        // Never reached: the task message alone already exceeds one token.
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .expect(0)
            .create_async()
            .await;

        let backend = Arc::new(FakeQueryBackend::default());
        let model = test_model(server.url(), backend, 5, 1, 60);
        assert!(matches!(
            model.diagnose(test_context()).await,
            Err(DiagnosticModelError::BudgetExceeded(_))
        ));
        mock.assert_async().await;
    }
}
