use crate::query::{MAX_DOSING_HISTORY_COUNT, QUERY_TIMEOUT_SECS, QueryError, SupervisorQuery};
use async_trait::async_trait;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    pub query_type: String,
    pub data: serde_json::Value,
    pub truncated: bool,
}

#[async_trait]
pub trait QueryBackend: Send + Sync {
    async fn execute(&self, query: SupervisorQuery) -> Result<QueryResult, QueryError>;
}

/// The one real implementation. Every branch below issues exactly one GET
/// request — there is no code path in this impl that constructs a POST,
/// PUT, PATCH, or DELETE.
pub struct HttpQueryBackend {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

impl HttpQueryBackend {
    pub fn new(base_url: String, api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(QUERY_TIMEOUT_SECS))
            .build()
            .expect("reqwest client build should not fail with these settings");
        Self {
            base_url,
            api_key,
            http,
        }
    }

    async fn get_json(
        &self,
        url: &str,
        params: &[(&str, String)],
    ) -> Result<serde_json::Value, QueryError> {
        let resp = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .query(params)
            .send()
            .await
            .map_err(|e| QueryError::Backend(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(QueryError::Backend(format!("status {}", resp.status())));
        }

        resp.json()
            .await
            .map_err(|e| QueryError::InvalidResponse(e.to_string()))
    }
}

fn truncate_if_needed(data: serde_json::Value) -> (serde_json::Value, bool) {
    let serialized = match serde_json::to_string(&data) {
        Ok(s) => s,
        Err(_) => return (data, false),
    };
    if serialized.len() <= crate::query::MAX_OUTPUT_BYTES {
        return (data, false);
    }

    // Most-recent-first truncation (design spec §8.5): if the payload is an
    // array or an object containing a "data" array (e.g. backend response wrapper),
    // keep dropping from the front (oldest) until it fits. Anything
    // else (a single object, e.g. crop_target) is left as-is — those
    // responses are small and fixed-shape, never the source of an
    // oversized payload.
    let truncate_array =
        |mut arr: Vec<serde_json::Value>,
         wrapper: &dyn Fn(Vec<serde_json::Value>) -> serde_json::Value| {
            let mut low = 0;
            let mut high = arr.len();
            let mut best_idx = arr.len(); // fallback: empty array

            while low < high {
                let mid = (low + high) / 2;
                let candidate = wrapper(arr[mid..].to_vec());
                if serde_json::to_string(&candidate)
                    .map(|s| s.len())
                    .unwrap_or(usize::MAX)
                    <= crate::query::MAX_OUTPUT_BYTES
                {
                    best_idx = mid;
                    high = mid; // Try to keep even more items from the tail (smaller mid)
                } else {
                    low = mid + 1; // Need to drop more items from the head (larger mid)
                }
            }

            (wrapper(arr.drain(best_idx..).collect()), true)
        };

    match data {
        serde_json::Value::Array(arr) => truncate_array(arr, &serde_json::Value::Array),
        serde_json::Value::Object(mut map) => {
            if let Some(serde_json::Value::Array(arr)) = map.remove("data") {
                let wrapper = |sub_arr: Vec<serde_json::Value>| {
                    let mut candidate_map = map.clone();
                    candidate_map.insert("data".to_string(), serde_json::Value::Array(sub_arr));
                    serde_json::Value::Object(candidate_map)
                };
                truncate_array(arr, &wrapper)
            } else {
                (serde_json::Value::Object(map), false)
            }
        }
        other => (other, false),
    }
}

#[async_trait]
impl QueryBackend for HttpQueryBackend {
    async fn execute(&self, query: SupervisorQuery) -> Result<QueryResult, QueryError> {
        let query_type = query.tag().to_string();
        let device_id = query.device_id().to_string();

        let data = match &query {
            SupervisorQuery::SensorHistory(q) => {
                let url = format!(
                    "{}/api/devices/{}/sensors/history",
                    self.base_url, device_id
                );
                self.get_json(&url, &[("range", format!("{}m", q.minutes))])
                    .await?
            }
            SupervisorQuery::DosingHistory(q) => {
                let end = chrono::Utc::now();
                let start = end - chrono::Duration::minutes(q.minutes as i64);
                let url = format!(
                    "{}/api/devices/{}/analytics/dosing-history",
                    self.base_url, device_id
                );
                let raw = self
                    .get_json(
                        &url,
                        &[("start", start.to_rfc3339()), ("end", end.to_rfc3339())],
                    )
                    .await?;

                // Row-count cap enforced here, client-side, since the real
                // endpoint has no limit parameter (found while implementing
                // this plan — it's range-based only). Keep the most recent
                // MAX_DOSING_HISTORY_COUNT entries.
                let capped = raw
                    .get("data")
                    .and_then(|d| d.as_array())
                    .map(|arr| {
                        let mut arr = arr.clone();
                        if arr.len() > MAX_DOSING_HISTORY_COUNT {
                            arr = arr.split_off(arr.len() - MAX_DOSING_HISTORY_COUNT);
                        }
                        arr
                    })
                    .unwrap_or_default();
                serde_json::Value::Array(capped)
            }
            SupervisorQuery::FsmEvents(q) => {
                let url = format!("{}/api/devices/{}/events", self.base_url, device_id);
                self.get_json(
                    &url,
                    &[
                        ("category", "fsm".to_string()),
                        ("limit", q.limit.to_string()),
                    ],
                )
                .await?
            }
            SupervisorQuery::HealthTopics(_) => {
                let url = format!("{}/api/devices/{}/health/topics", self.base_url, device_id);
                self.get_json(&url, &[]).await?
            }
            SupervisorQuery::HestiaSnapshot(_) => {
                let url = format!("{}/api/devices/{}/health/hestia", self.base_url, device_id);
                self.get_json(&url, &[]).await?
            }
            SupervisorQuery::CropTarget(_) => {
                let url = format!(
                    "{}/api/devices/{}/config/crop-target",
                    self.base_url, device_id
                );
                self.get_json(&url, &[]).await?
            }
        };

        let (data, truncated) = truncate_if_needed(data);
        Ok(QueryResult {
            query_type,
            data,
            truncated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::*;

    #[tokio::test]
    async fn execute_sensor_history_calls_expected_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/devices/d1/sensors/history")
            .match_query(mockito::Matcher::UrlEncoded("range".into(), "30m".into()))
            .match_header("x-api-key", "test-key")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "success", "data": []}"#)
            .create_async()
            .await;

        let backend = HttpQueryBackend::new(server.url(), "test-key".to_string());
        let result = backend
            .execute(SupervisorQuery::SensorHistory(SensorHistoryQuery {
                device_id: "d1".to_string(),
                minutes: 30,
            }))
            .await
            .unwrap();

        assert_eq!(result.query_type, "sensor_history");
    }

    #[tokio::test]
    async fn execute_dosing_history_calls_range_endpoint_and_caps_rows() {
        let mut server = mockito::Server::new_async().await;
        let body = serde_json::json!({
            "status": "success",
            "data": (1..=6).map(|i| serde_json::json!({"created_at": i, "pump_a_ml": 1.0})).collect::<Vec<_>>()
        });
        let _m = server
            .mock("GET", "/api/devices/d1/analytics/dosing-history")
            .match_query(mockito::Matcher::Regex("start=.*&end=.*".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let backend = HttpQueryBackend::new(server.url(), "test-key".to_string());
        let result = backend
            .execute(SupervisorQuery::DosingHistory(DosingHistoryQuery {
                device_id: "d1".to_string(),
                minutes: 60,
            }))
            .await
            .unwrap();

        let rows = result.data.as_array().unwrap();
        assert_eq!(rows.len(), MAX_DOSING_HISTORY_COUNT);
    }

    #[tokio::test]
    async fn execute_crop_target_calls_expected_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/devices/d1/config/crop-target")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ec_target": 1.8, "ec_tolerance": 0.2, "ph_target": 6.0, "ph_tolerance": 0.3, "water_level_target": 70.0, "water_level_tolerance": 5.0}"#)
            .create_async()
            .await;

        let backend = HttpQueryBackend::new(server.url(), "test-key".to_string());
        let result = backend
            .execute(SupervisorQuery::CropTarget(CropTargetQuery {
                device_id: "d1".to_string(),
            }))
            .await
            .unwrap();

        assert_eq!(result.data["ec_target"], 1.8);
    }

    #[tokio::test]
    async fn execute_errors_on_non_success_status_without_panicking() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/devices/d1/config/crop-target")
            .with_status(404)
            .create_async()
            .await;

        let backend = HttpQueryBackend::new(server.url(), "test-key".to_string());
        let result = backend
            .execute(SupervisorQuery::CropTarget(CropTargetQuery {
                device_id: "d1".to_string(),
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn every_query_variant_issues_only_get_requests() {
        let mut server = mockito::Server::new_async().await;
        let device = "d1";
        for (method, path) in [
            ("GET", "/api/devices/d1/sensors/history"),
            ("GET", "/api/devices/d1/analytics/dosing-history"),
            ("GET", "/api/devices/d1/events"),
            ("GET", "/api/devices/d1/health/topics"),
            ("GET", "/api/devices/d1/health/hestia"),
            ("GET", "/api/devices/d1/config/crop-target"),
        ] {
            server
                .mock(method, path)
                .match_query(mockito::Matcher::Any)
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(r#"{"status": "success", "data": []}"#)
                .create_async()
                .await;
        }

        let backend = HttpQueryBackend::new(server.url(), "test-key".to_string());
        let queries = vec![
            SupervisorQuery::SensorHistory(SensorHistoryQuery {
                device_id: device.into(),
                minutes: 30,
            }),
            SupervisorQuery::DosingHistory(DosingHistoryQuery {
                device_id: device.into(),
                minutes: 60,
            }),
            SupervisorQuery::FsmEvents(FsmEventsQuery {
                device_id: device.into(),
                limit: 5,
            }),
            SupervisorQuery::HealthTopics(HealthTopicsQuery {
                device_id: device.into(),
            }),
            SupervisorQuery::HestiaSnapshot(HestiaSnapshotQuery {
                device_id: device.into(),
            }),
            SupervisorQuery::CropTarget(CropTargetQuery {
                device_id: device.into(),
            }),
        ];
        for query in queries {
            let result = backend.execute(query.clone()).await;
            assert!(result.is_ok(), "{:?} failed: {:?}", query, result.err());
        }
    }

    #[tokio::test]
    async fn execute_truncates_oversized_response() {
        let mut server = mockito::Server::new_async().await;
        let huge_data: Vec<_> = (0..5000)
            .map(|i| serde_json::json!({"time": i, "ec": 1.8, "ph": 6.0, "temp": 24.0, "water_level": 70.0}))
            .collect();
        let body = serde_json::json!({"status": "success", "data": huge_data});
        let _m = server
            .mock("GET", "/api/devices/d1/sensors/history")
            .match_query(mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let backend = HttpQueryBackend::new(server.url(), "test-key".to_string());
        let result = backend
            .execute(SupervisorQuery::SensorHistory(SensorHistoryQuery {
                device_id: "d1".to_string(),
                minutes: 60,
            }))
            .await
            .unwrap();

        let serialized = serde_json::to_string(&result.data).unwrap();
        assert!(serialized.len() <= MAX_OUTPUT_BYTES);
        assert!(result.truncated);
    }
}
