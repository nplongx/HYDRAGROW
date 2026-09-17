use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use rumqttc::QoS;
use serde_json::Value;
use tracing::{error, info};

use crate::{AppState, db::config_sync};
use hydragrow_shared::topics::{topic_controller_config, topic_sensors_config};

const BATCH_SIZE: i64 = 32;
const RETRY_LIMIT: i32 = 5;

fn target_payload<'a>(row: &'a config_sync::ConfigurationSync, target: &str) -> &'a Value {
    match target {
        "controller" => &row.desired_controller_config,
        _ => &row.desired_sensor_config,
    }
}

async fn publish_target(
    app: &AppState,
    row: &config_sync::ConfigurationSync,
    target: &str,
) -> Result<(), String> {
    let topic = match target {
        "controller" => topic_controller_config(&row.device_id),
        "sensor" => topic_sensors_config(&row.device_id),
        _ => return Err("invalid target".to_string()),
    };
    let payload = serde_json::to_vec(target_payload(row, target))
        .map_err(|e| format!("serialize config: {e}"))?;
    app.mqtt_client
        .publish(&topic, QoS::AtLeastOnce, true, payload)
        .await
        .map_err(|e| format!("MQTT publish: {e}"))
}

async fn reconcile_once(app: &AppState) -> Result<usize, anyhow::Error> {
    if !app.mqtt_connected.load(Ordering::Relaxed) {
        return Ok(0);
    }

    let rows = config_sync::list_pending(&app.pg_pool, BATCH_SIZE).await?;
    crate::metrics::SYNC_PENDING
        .with_label_values(&["configuration"])
        .set(rows.len() as i64);
    let mut work = 0;
    for row in rows {
        for target in ["controller", "sensor"] {
            let state = if target == "controller" {
                &row.controller_state
            } else {
                &row.sensor_state
            };
            if state != config_sync::PENDING {
                continue;
            }
            let attempts = if target == "controller" {
                row.controller_attempts
            } else {
                row.sensor_attempts
            };
            if attempts >= RETRY_LIMIT {
                continue;
            }
            work += 1;
            match publish_target(app, &row, target).await {
                Ok(()) => {
                    config_sync::mark_published(
                        &app.pg_pool,
                        &row.device_id,
                        row.config_version,
                        target,
                    )
                    .await?;
                    crate::metrics::SYNC_ATTEMPTS_TOTAL
                        .with_label_values(&["configuration", "published"])
                        .inc();
                    crate::metrics::MQTT_DELIVERY_TOTAL
                        .with_label_values(&["configuration", "success"])
                        .inc();
                }
                Err(error_message) => {
                    let reason = "publish";
                    config_sync::mark_publish_failed(
                        &app.pg_pool,
                        &row.device_id,
                        row.config_version,
                        target,
                        &error_message,
                    )
                    .await?;
                    crate::metrics::SYNC_ATTEMPTS_TOTAL
                        .with_label_values(&["configuration", "failure"])
                        .inc();
                    crate::metrics::SYNC_FAILURES_TOTAL
                        .with_label_values(&["configuration", reason])
                        .inc();
                    crate::metrics::MQTT_DELIVERY_FAILURES_TOTAL
                        .with_label_values(&["configuration", reason])
                        .inc();
                    error!(device_id = %row.device_id, config_version = row.config_version, target, error = %error_message, "Configuration synchronization publish failed");
                }
            }
        }
    }
    Ok(work)
}

struct WorkerGuard(Arc<AtomicBool>);
impl Drop for WorkerGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

pub fn spawn(app_state: Arc<AppState>, readiness: Arc<AtomicBool>) {
    tokio::spawn(async move {
        readiness.store(true, Ordering::Relaxed);
        let _guard = WorkerGuard(readiness);
        if let Err(e) = reconcile_once(&app_state).await {
            error!(error = %e, "Initial configuration synchronization failed");
        }
        loop {
            match reconcile_once(&app_state).await {
                Ok(work) if work > 0 => {
                    info!(count = work, "Configuration synchronization pass completed")
                }
                Ok(_) => {}
                Err(e) => error!(error = %e, "Configuration synchronization pass failed"),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_guard_clears_readiness() {
        let ready = Arc::new(AtomicBool::new(true));
        {
            let _guard = WorkerGuard(ready.clone());
            assert!(ready.load(Ordering::Relaxed));
        }
        assert!(!ready.load(Ordering::Relaxed));
    }
}
