use crate::backend_client::{BackendClient, TopicStatus};
use crate::staleness::check_topic_staleness;
use std::collections::HashMap;

/// Abstracts over `BackendClient` so `run_tick`'s orchestration logic (which
/// devices get alerted) is testable without a real or mocked HTTP server —
/// Task 2 and Task 4 already cover `BackendClient`'s own HTTP behavior.
#[async_trait::async_trait]
pub trait TickClient {
    async fn get_fleet_topics(&self) -> anyhow::Result<HashMap<String, Vec<TopicStatus>>>;
    async fn recent_alert_exists(&self, device_id: &str, reason_code: &str)
    -> anyhow::Result<bool>;
    async fn create_stale_controller_alert(
        &self,
        device_id: &str,
        seconds_stale: i64,
    ) -> anyhow::Result<()>;
    async fn create_stale_sensor_alert(
        &self,
        device_id: &str,
        seconds_stale: i64,
    ) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl TickClient for BackendClient {
    async fn get_fleet_topics(&self) -> anyhow::Result<HashMap<String, Vec<TopicStatus>>> {
        BackendClient::get_fleet_topics(self).await
    }
    async fn recent_alert_exists(
        &self,
        device_id: &str,
        reason_code: &str,
    ) -> anyhow::Result<bool> {
        BackendClient::recent_alert_exists(self, device_id, reason_code).await
    }
    async fn create_stale_controller_alert(
        &self,
        device_id: &str,
        seconds_stale: i64,
    ) -> anyhow::Result<()> {
        BackendClient::create_stale_controller_alert(self, device_id, seconds_stale).await
    }
    async fn create_stale_sensor_alert(
        &self,
        device_id: &str,
        seconds_stale: i64,
    ) -> anyhow::Result<()> {
        BackendClient::create_stale_sensor_alert(self, device_id, seconds_stale).await
    }
}

/// One fleet-wide tick: design spec §12 requires logging which trigger
/// fired for every tick that fires one, so each outcome below is logged
/// individually rather than only summarized at the end.
pub async fn run_tick(client: &impl TickClient, threshold_secs: u64) -> anyhow::Result<()> {
    let started = std::time::Instant::now();
    let fleet = client.get_fleet_topics().await?;

    for (device_id, topics) in &fleet {
        let now = chrono::Utc::now();

        if let Some(breach) =
            check_topic_staleness(topics, "controller/status", now, threshold_secs)
        {
            tracing::info!(
                device_id = %device_id,
                seconds_stale = breach.seconds_stale,
                "controller/status staleness breach detected"
            );

            let reason_code =
                hydragrow_shared::supervisor::SupervisorReasonCode::TopicStaleControllerStatus
                    .as_str();

            match client.recent_alert_exists(device_id, reason_code).await {
                Ok(true) => {
                    tracing::debug!(device_id = %device_id, "breach already alerted within cooldown, skipping");
                }
                Ok(false) => {
                    match client
                        .create_stale_controller_alert(device_id, breach.seconds_stale)
                        .await
                    {
                        Ok(()) => {
                            tracing::info!(device_id = %device_id, "stale-controller alert created")
                        }
                        Err(e) => {
                            tracing::error!(device_id = %device_id, error = %e, "failed to create alert")
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(device_id = %device_id, error = %e, "dedup check failed, skipping this tick");
                }
            }
        }

        if let Some(breach) = check_topic_staleness(topics, "sensor/status", now, threshold_secs) {
            tracing::info!(
                device_id = %device_id,
                seconds_stale = breach.seconds_stale,
                "sensor/status staleness breach detected"
            );

            let reason_code =
                hydragrow_shared::supervisor::SupervisorReasonCode::SensorFaultSuspected.as_str();

            match client.recent_alert_exists(device_id, reason_code).await {
                Ok(true) => {
                    tracing::debug!(device_id = %device_id, "sensor breach already alerted within cooldown, skipping");
                }
                Ok(false) => {
                    match client
                        .create_stale_sensor_alert(device_id, breach.seconds_stale)
                        .await
                    {
                        Ok(()) => {
                            tracing::info!(device_id = %device_id, "stale-sensor alert created")
                        }
                        Err(e) => {
                            tracing::error!(device_id = %device_id, error = %e, "failed to create sensor alert")
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(device_id = %device_id, error = %e, "sensor dedup check failed, skipping this tick");
                }
            }
        }
    }

    tracing::debug!(
        devices_checked = fleet.len(),
        latency_ms = started.elapsed().as_millis(),
        "tick complete"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_client::TopicStatus;
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeClient {
        recent_alert_exists: Mutex<HashMap<String, bool>>,
        created_alerts: Mutex<Vec<String>>,
        created_sensor_alerts: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl TickClient for FakeClient {
        async fn get_fleet_topics(&self) -> anyhow::Result<HashMap<String, Vec<TopicStatus>>> {
            let now = Utc::now();
            Ok(HashMap::from([
                (
                    "stale-dev".to_string(),
                    vec![TopicStatus {
                        topic_category: "controller/status".to_string(),
                        last_seen_at: now - Duration::seconds(90),
                    }],
                ),
                (
                    "fresh-dev".to_string(),
                    vec![TopicStatus {
                        topic_category: "controller/status".to_string(),
                        last_seen_at: now - Duration::seconds(5),
                    }],
                ),
                (
                    "already-alerted-dev".to_string(),
                    vec![TopicStatus {
                        topic_category: "controller/status".to_string(),
                        last_seen_at: now - Duration::seconds(90),
                    }],
                ),
            ]))
        }

        async fn recent_alert_exists(
            &self,
            device_id: &str,
            _reason_code: &str,
        ) -> anyhow::Result<bool> {
            Ok(*self
                .recent_alert_exists
                .lock()
                .unwrap()
                .get(device_id)
                .unwrap_or(&false))
        }

        async fn create_stale_controller_alert(
            &self,
            device_id: &str,
            _seconds_stale: i64,
        ) -> anyhow::Result<()> {
            self.created_alerts
                .lock()
                .unwrap()
                .push(device_id.to_string());
            Ok(())
        }

        async fn create_stale_sensor_alert(
            &self,
            device_id: &str,
            _seconds_stale: i64,
        ) -> anyhow::Result<()> {
            self.created_sensor_alerts
                .lock()
                .unwrap()
                .push(device_id.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn run_tick_alerts_only_stale_and_not_recently_alerted_devices() {
        let client = FakeClient::default();
        client
            .recent_alert_exists
            .lock()
            .unwrap()
            .insert("already-alerted-dev".to_string(), true);

        run_tick(&client, 60).await.unwrap();
        let created = client.created_alerts.lock().unwrap();
        assert_eq!(*created, vec!["stale-dev".to_string()]);
        // No sensor/status topics in this fixture, so no sensor alerts fire.
        assert!(client.created_sensor_alerts.lock().unwrap().is_empty());
    }

    #[derive(Default)]
    struct SensorFakeClient {
        created_sensor_alerts: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl TickClient for SensorFakeClient {
        async fn get_fleet_topics(&self) -> anyhow::Result<HashMap<String, Vec<TopicStatus>>> {
            let now = Utc::now();
            Ok(HashMap::from([
                (
                    "sensor-stale-dev".to_string(),
                    vec![TopicStatus {
                        topic_category: "sensor/status".to_string(),
                        last_seen_at: now - Duration::seconds(120),
                    }],
                ),
                (
                    "sensor-fresh-dev".to_string(),
                    vec![TopicStatus {
                        topic_category: "sensor/status".to_string(),
                        last_seen_at: now - Duration::seconds(5),
                    }],
                ),
            ]))
        }

        async fn recent_alert_exists(
            &self,
            _device_id: &str,
            _reason_code: &str,
        ) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn create_stale_controller_alert(
            &self,
            _device_id: &str,
            _seconds_stale: i64,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn create_stale_sensor_alert(
            &self,
            device_id: &str,
            _seconds_stale: i64,
        ) -> anyhow::Result<()> {
            self.created_sensor_alerts
                .lock()
                .unwrap()
                .push(device_id.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn run_tick_alerts_stale_sensor_devices() {
        let client = SensorFakeClient::default();
        run_tick(&client, 60).await.unwrap();
        let created = client.created_sensor_alerts.lock().unwrap();
        assert_eq!(*created, vec!["sensor-stale-dev".to_string()]);
    }
}
