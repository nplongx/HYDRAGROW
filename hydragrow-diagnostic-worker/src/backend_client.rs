use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;

#[async_trait]
pub trait DiagnosisBackend: Send + Sync {
    async fn get_fleet_hestia(&self) -> anyhow::Result<HashMap<String, serde_json::Value>>;
    async fn has_recent_watchdog_breach(
        &self,
        device_id: &str,
        lookback_minutes: i64,
    ) -> anyhow::Result<bool>;
    async fn recent_alert_exists(&self, device_id: &str, reason_code: &str)
    -> anyhow::Result<bool>;
    async fn get_crop_target(&self, device_id: &str) -> anyhow::Result<serde_json::Value>;
    #[allow(clippy::too_many_arguments)]
    async fn create_diagnosis_alert(
        &self,
        device_id: &str,
        level: &str,
        title: &str,
        message: &str,
        reason_codes: Vec<String>,
        confidence: f32,
        observations: serde_json::Value,
    ) -> anyhow::Result<()>;
}

pub struct BackendClient {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct CreateEventBody {
    level: String,
    category: String,
    title: String,
    message: String,
    reason_codes: Vec<String>,
    confidence: f32,
    observations: serde_json::Value,
}

#[derive(serde::Deserialize)]
struct ApiEnvelope<T> {
    status: String,
    data: T,
}

impl BackendClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            http,
        }
    }
}

#[async_trait]
impl DiagnosisBackend for BackendClient {
    async fn get_fleet_hestia(&self) -> anyhow::Result<HashMap<String, serde_json::Value>> {
        let url = format!("{}/api/health/hestia", self.base_url);
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("GET /api/health/hestia returned {}", resp.status());
        }
        let envelope: ApiEnvelope<HashMap<String, serde_json::Value>> = resp.json().await?;
        if envelope.status != "success" {
            anyhow::bail!(
                "GET /api/health/hestia returned API status {}",
                envelope.status
            );
        }
        Ok(envelope.data)
    }

    /// Design spec §4.1's WatchdogBreach trigger. `/events` has no
    /// `source` filter (only `category`/`level`/`limit`/timestamps), so
    /// this fetches recent `category=alert` rows and filters client-side —
    /// deliberately not a new backend endpoint, since this data is already
    /// fully readable through the existing, unscoped `/events` GET.
    async fn has_recent_watchdog_breach(
        &self,
        device_id: &str,
        lookback_minutes: i64,
    ) -> anyhow::Result<bool> {
        let url = format!("{}/api/devices/{}/events", self.base_url, device_id);
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .query(&[("category", "alert"), ("limit", "20")])
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("GET .../events returned {}", resp.status());
        }
        let rows: Vec<serde_json::Value> = resp.json().await?;
        let cutoff =
            (chrono::Utc::now() - chrono::Duration::minutes(lookback_minutes)).timestamp_millis();

        Ok(rows.iter().any(|row| {
            row.get("source").and_then(|s| s.as_str()) == Some("watchdog")
                && row.get("primary_reason_code").and_then(|r| r.as_str())
                    == Some("topic_stale_controller_status")
                && row.get("timestamp").and_then(|t| t.as_i64()).unwrap_or(0) >= cutoff
        }))
    }

    async fn recent_alert_exists(
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
        #[derive(serde::Deserialize)]
        struct R {
            recent_alert_exists: bool,
        }
        Ok(resp.json::<R>().await?.recent_alert_exists)
    }

    async fn get_crop_target(&self, device_id: &str) -> anyhow::Result<serde_json::Value> {
        let url = format!(
            "{}/api/devices/{}/config/crop-target",
            self.base_url, device_id
        );
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("GET .../config/crop-target returned {}", resp.status());
        }
        Ok(resp.json().await?)
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_diagnosis_alert(
        &self,
        device_id: &str,
        level: &str,
        title: &str,
        message: &str,
        reason_codes: Vec<String>,
        confidence: f32,
        observations: serde_json::Value,
    ) -> anyhow::Result<()> {
        let url = format!("{}/api/devices/{}/events", self.base_url, device_id);
        let body = CreateEventBody {
            level: level.to_string(),
            category: "alert".to_string(),
            title: title.to_string(),
            message: message.to_string(),
            reason_codes,
            confidence,
            observations,
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
        let _ = resp.bytes().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_fleet_hestia_parses_response() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/health/hestia")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
        "status":"success",
        "data":{
            "dev-1":{
                "score":65.0,
                "state":"WARNING",
                "confidence":0.8,
                "axes":{},
                "reasons":["ec_out_of_range"]
            }
        }
    }"#,
            )
            .create_async()
            .await;

        let client = BackendClient::new(server.url(), "svc_test".to_string());
        let fleet = client.get_fleet_hestia().await.unwrap();
        assert_eq!(fleet["dev-1"]["state"], "WARNING");
    }

    #[tokio::test]
    async fn get_fleet_hestia_rejects_api_error_envelope() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/health/hestia")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status":"error","data":{}}"#)
            .create_async()
            .await;

        let client = BackendClient::new(server.url(), "svc_test".to_string());
        assert!(client.get_fleet_hestia().await.is_err());
    }

    #[tokio::test]
    async fn has_recent_watchdog_breach_true_when_matching_row_within_window() {
        let mut server = mockito::Server::new_async().await;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let _m = server
            .mock("GET", "/api/devices/dev-1/events?category=alert&limit=20")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!([
                    {"source": "watchdog", "primary_reason_code": "topic_stale_controller_status", "timestamp": now_ms}
                ])
                .to_string(),
            )
            .create_async()
            .await;

        let client = BackendClient::new(server.url(), "svc_test".to_string());
        let breach = client
            .has_recent_watchdog_breach("dev-1", 10)
            .await
            .unwrap();
        assert!(breach);
    }

    #[tokio::test]
    async fn has_recent_watchdog_breach_false_when_only_ai_supervisor_rows_present() {
        let mut server = mockito::Server::new_async().await;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let _m = server
            .mock("GET", "/api/devices/dev-1/events?category=alert&limit=20")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                serde_json::json!([
                    {"source": "ai_supervisor", "primary_reason_code": "leak_suspected", "timestamp": now_ms}
                ])
                .to_string(),
            )
            .create_async()
            .await;

        let client = BackendClient::new(server.url(), "svc_test".to_string());
        let breach = client
            .has_recent_watchdog_breach("dev-1", 10)
            .await
            .unwrap();
        assert!(!breach);
    }

    #[tokio::test]
    async fn recent_alert_exists_and_create_alert_work_as_in_watchdog() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock(
                "GET",
                "/api/devices/dev-1/events/recent-check?reason_code=leak_suspected",
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"recent_alert_exists": false}"#)
            .create_async()
            .await;
        server
            .mock("POST", "/api/devices/dev-1/events")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status": "created"}"#)
            .create_async()
            .await;

        let client = BackendClient::new(server.url(), "svc_test".to_string());
        assert!(
            !client
                .recent_alert_exists("dev-1", "leak_suspected")
                .await
                .unwrap()
        );
        client
            .create_diagnosis_alert(
                "dev-1",
                "warning",
                "Leak suspected",
                "EC rose without dosing",
                vec!["leak_suspected".to_string()],
                0.7,
                serde_json::json!({}),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn get_crop_target_calls_expected_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let _m = server
            .mock("GET", "/api/devices/dev-1/config/crop-target")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"ec_target": 1.8, "ec_tolerance": 0.2}"#)
            .create_async()
            .await;

        let client = BackendClient::new(server.url(), "svc_test".to_string());
        let target = client.get_crop_target("dev-1").await.unwrap();
        assert_eq!(target["ec_target"], 1.8);
    }
}
