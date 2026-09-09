use hydragrow_supervisor_query::{QueryBackend, QueryError, SupervisorQuery, validate_and_clamp};

pub async fn run(
    query: SupervisorQuery,
    backend: &dyn QueryBackend,
    compact: bool,
) -> Result<String, QueryError> {
    let clamped = validate_and_clamp(query)?;
    let result = backend.execute(clamped).await?;

    let json = if compact {
        serde_json::to_string(&result)
    } else {
        serde_json::to_string_pretty(&result)
    };

    json.map_err(|e| QueryError::InvalidResponse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydragrow_supervisor_query::testing::FakeQueryBackend;
    use hydragrow_supervisor_query::{HealthTopicsQuery, SupervisorQuery};
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[tokio::test]
    async fn run_prints_pretty_json_by_default() {
        let mut responses = HashMap::new();
        responses.insert(
            "health_topics".to_string(),
            serde_json::json!({"controller/status": "2026-09-08T10:00:00Z"}),
        );
        let backend = FakeQueryBackend {
            responses: Mutex::new(responses),
            calls: Mutex::new(vec![]),
        };

        let query = SupervisorQuery::HealthTopics(HealthTopicsQuery {
            device_id: "d1".to_string(),
        });
        let output = run(query, &backend, false).await.unwrap();

        assert!(output.contains('\n')); // pretty-printed, multi-line
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["query_type"], "health_topics");
    }

    #[tokio::test]
    async fn run_prints_compact_json_when_requested() {
        let backend = FakeQueryBackend::default();
        let query = SupervisorQuery::HealthTopics(HealthTopicsQuery {
            device_id: "d1".to_string(),
        });
        let output = run(query, &backend, true).await.unwrap();

        assert!(!output.contains('\n')); // compact, single-line
    }

    #[tokio::test]
    async fn run_clamps_before_executing() {
        let backend = FakeQueryBackend::default();
        let query =
            SupervisorQuery::SensorHistory(hydragrow_supervisor_query::SensorHistoryQuery {
                device_id: "d1".to_string(),
                minutes: 999_999,
            });
        run(query, &backend, true).await.unwrap();

        let calls = backend.calls.lock().unwrap();
        match &calls[0] {
            SupervisorQuery::SensorHistory(q) => {
                assert_eq!(
                    q.minutes,
                    hydragrow_supervisor_query::MAX_SENSOR_HISTORY_MINUTES
                )
            }
            _ => panic!("wrong variant recorded"),
        }
    }

    #[tokio::test]
    async fn run_rejects_empty_device_id_without_calling_backend() {
        let backend = FakeQueryBackend::default();
        let query = SupervisorQuery::HealthTopics(HealthTopicsQuery {
            device_id: String::new(),
        });
        let result = run(query, &backend, true).await;

        assert!(result.is_err());
        assert!(backend.calls.lock().unwrap().is_empty());
    }
}
