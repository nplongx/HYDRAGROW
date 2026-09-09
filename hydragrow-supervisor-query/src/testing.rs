use crate::backend::{QueryBackend, QueryResult};
use crate::query::{QueryError, SupervisorQuery};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

/// Returns a fixed, caller-supplied response for each query tag, recording
/// every query it was asked to execute for later assertions.
#[derive(Default)]
pub struct FakeQueryBackend {
    pub responses: Mutex<HashMap<String, serde_json::Value>>,
    pub calls: Mutex<Vec<SupervisorQuery>>,
}

#[async_trait]
impl QueryBackend for FakeQueryBackend {
    async fn execute(&self, query: SupervisorQuery) -> Result<QueryResult, QueryError> {
        let tag = query.tag().to_string();
        self.calls.lock().unwrap().push(query);
        let data = self
            .responses
            .lock()
            .unwrap()
            .get(&tag)
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        Ok(QueryResult {
            query_type: tag,
            data,
            truncated: false,
        })
    }
}
