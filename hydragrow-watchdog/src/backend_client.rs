use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Matches the JSON shape `TopicStatus` in
/// hydragrow-backend/src/api/health_topics.rs serializes — see the "Note on
/// types" at the top of this plan for why this is a separate, matching
/// struct rather than a shared one.
#[derive(Debug, Clone, Deserialize)]
pub struct TopicStatus {
    pub topic_category: String,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct FleetTopicsResponse {
    data: HashMap<String, Vec<TopicStatus>>,
}

#[derive(Deserialize)]
struct DedupCheckResponse {
    recent_alert_exists: bool,
}

#[derive(Serialize)]
struct CreateEventRequest {
    level: String,
    category: String,
    title: String,
    message: String,
    reason_codes: Vec<String>,
}

pub struct BackendClient {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

impl BackendClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            http: reqwest::Client::new(),
        }
    }

    /// Design spec §6.1, fleet-wide view — one call per tick regardless of
    /// fleet size, used for trigger evaluation.
    pub async fn get_fleet_topics(&self) -> anyhow::Result<HashMap<String, Vec<TopicStatus>>> {
        let url = format!("{}/api/health/topics", self.base_url);
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("GET /api/health/topics returned {}", resp.status());
        }

        let parsed: FleetTopicsResponse = resp.json().await?;
        Ok(parsed.data)
    }

    /// Design spec §5: called before creating an alert so a still-cooling-
    /// down breach doesn't produce a second row for the same condition.
    pub async fn recent_alert_exists(
        &self,
        device_id: &str,
        reason_code: &str,
    ) -> anyhow::Result<bool> {
        let url = format!(
            "{}/api/devices/{}/events/recent-check",
            self.base_url, device_id
        );
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .query(&[("reason_code", reason_code)])
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("GET .../events/recent-check returned {}", resp.status());
        }

        let parsed: DedupCheckResponse = resp.json().await?;
        Ok(parsed.recent_alert_exists)
    }

    pub async fn create_stale_controller_alert(
        &self,
        device_id: &str,
        seconds_stale: i64,
    ) -> anyhow::Result<()> {
        let url = format!("{}/api/devices/{}/events", self.base_url, device_id);
        let reason_code =
            hydragrow_shared::supervisor::SupervisorReasonCode::TopicStaleControllerStatus
                .as_str()
                .to_string();

        let body = CreateEventRequest {
            level: "warning".to_string(),
            category: "alert".to_string(),
            title: "Controller status feed is stale".to_string(),
            message: format!("No controller/status message received in {seconds_stale} seconds"),
            reason_codes: vec![reason_code],
        };

        let resp = self
            .http
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("POST .../events returned {}", resp.status());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_fleet_topics_parses_grouped_response() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/health/topics")
            .match_header("x-api-key", "svc_test123")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"status": "success", "data": {
"dev-1": [{"topic_category": "controller/status", "last_seen_at": "2026-09-08T10:00:00Z"}],
"dev-2": [{"topic_category": "controller/status", "last_seen_at": "2026-09-08T09:58:00Z"}]
}}"#,
            )
            .create_async()
            .await;

        let client = BackendClient::new(server.url(), "svc_test123".to_string());
        let fleet = client.get_fleet_topics().await.unwrap();

        assert_eq!(fleet.len(), 2);
        assert_eq!(fleet["dev-1"][0].topic_category, "controller/status");
    }

    #[tokio::test]
    async fn get_fleet_topics_errors_on_401() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/health/topics")
            .with_status(401)
            .create_async()
            .await;

        let client = BackendClient::new(server.url(), "svc_bad_key".to_string());
        assert!(client.get_fleet_topics().await.is_err());
    }

    #[tokio::test]
    async fn recent_alert_exists_parses_true() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/devices/dev-1/events/recent-check")
            .match_query(mockito::Matcher::UrlEncoded(
                "reason_code".into(),
                "topic_stale_controller_status".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"recent_alert_exists": true}"#)
            .create_async()
            .await;

        let client = BackendClient::new(server.url(), "svc_test123".to_string());
        let exists = client
            .recent_alert_exists("dev-1", "topic_stale_controller_status")
            .await
            .unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn create_alert_posts_expected_body() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("POST", "/api/devices/dev-1/events")
            .match_header("x-api-key", "svc_test123")
            .match_body(mockito::Matcher::PartialJson(serde_json::json!({
                "level": "warning",
                "category": "alert",
                "reason_codes": ["topic_stale_controller_status"]
            })))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "created", "timestamp": 1234}"#)
            .create_async()
            .await;

        let client = BackendClient::new(server.url(), "svc_test123".to_string());
        client
            .create_stale_controller_alert("dev-1", 90)
            .await
            .unwrap();
    }
}
