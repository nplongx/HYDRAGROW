use crate::backend_client::DiagnosisBackend;
use crate::diagnostic_model::DiagnosticModel;
use crate::orchestrator::run_diagnosis;
use crate::trigger::evaluate_triggers;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Design spec §8.7: bounded concurrency across the fleet. Devices beyond
/// the semaphore's capacity wait for a permit within this tick.
pub async fn run_fleet_tick(
    model: Arc<dyn DiagnosticModel>,
    backend: Arc<dyn DiagnosisBackend>,
    max_concurrent: usize,
    watchdog_breach_lookback_minutes: i64,
) {
    let fleet = match backend.get_fleet_hestia().await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!(error = %e, "failed to fetch fleet hestia, skipping this tick");
            return;
        }
    };

    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut handles = Vec::new();

    for (device_id, hestia) in fleet {
        let has_breach = backend
            .has_recent_watchdog_breach(&device_id, watchdog_breach_lookback_minutes)
            .await
            .unwrap_or(false);

        let Some(trigger) = evaluate_triggers(Some(&hestia), has_breach) else {
            continue;
        };

        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore not closed");
        let model = model.clone();
        let backend = backend.clone();
        let device_id = device_id.clone();
        let crop_target = backend
            .get_crop_target(&device_id)
            .await
            .unwrap_or(serde_json::json!({}));

        handles.push(tokio::spawn(async move {
            let _permit = permit; // held for the task's lifetime
            if let Err(e) = run_diagnosis(
                &device_id,
                trigger,
                model.as_ref(),
                backend.as_ref(),
                hestia,
                crop_target,
            )
            .await
            {
                tracing::error!(device_id = %device_id, error = %e, "diagnosis failed");
            }
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_client::DiagnosisBackend;
    use crate::diagnostic_model::{
        Diagnosis, DiagnosisObservations, DiagnosticContext, DiagnosticModelError,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeModel {
        calls: Arc<Mutex<Vec<String>>>,
    }
    #[async_trait]
    impl DiagnosticModel for FakeModel {
        async fn diagnose(
            &self,
            ctx: DiagnosticContext,
        ) -> Result<Diagnosis, DiagnosticModelError> {
            self.calls.lock().unwrap().push(ctx.device_id.clone());
            Ok(Diagnosis {
                reason_codes: vec![
                    hydragrow_shared::supervisor::SupervisorReasonCode::EcOutOfRange,
                ],
                confidence: 0.5,
                narrative: "...".to_string(),
                observations: DiagnosisObservations {
                    current_value: 0.0,
                    target_value: 0.0,
                    delta: 0.0,
                    window_minutes: 0,
                    corroborating_evidence: vec![],
                },
            })
        }
    }

    #[derive(Default)]
    struct FakeBackend {
        fleet: HashMap<String, serde_json::Value>,
    }
    #[async_trait]
    impl DiagnosisBackend for FakeBackend {
        async fn get_fleet_hestia(&self) -> anyhow::Result<HashMap<String, serde_json::Value>> {
            Ok(self.fleet.clone())
        }
        async fn has_recent_watchdog_breach(&self, _: &str, _: i64) -> anyhow::Result<bool> {
            Ok(false)
        }
        async fn recent_alert_exists(&self, _: &str, _: &str) -> anyhow::Result<bool> {
            Ok(false)
        }
        async fn get_crop_target(&self, _: &str) -> anyhow::Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }
        async fn create_diagnosis_alert(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: Vec<String>,
            _: f32,
            _: serde_json::Value,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn run_fleet_tick_diagnoses_only_triggered_devices() {
        let mut fleet = HashMap::new();
        fleet.insert(
            "triggered-dev".to_string(),
            serde_json::json!({"state": "WARNING", "reasons": ["ec_out_of_range"]}),
        );
        fleet.insert(
            "critical-dev".to_string(),
            serde_json::json!({"state": "CRITICAL", "reasons": ["water_level_critical"]}),
        );
        fleet.insert(
            "recovery-dev".to_string(),
            serde_json::json!({"state": "RECOVERY", "reasons": ["recent_intervention_recovery"]}),
        );
        fleet.insert(
            "comfortable-dev".to_string(),
            serde_json::json!({"state": "COMFORTABLE", "reasons": []}),
        );

        let backend = FakeBackend { fleet };
        let calls = Arc::new(Mutex::new(vec![]));
        let model = FakeModel {
            calls: calls.clone(),
        };

        run_fleet_tick(Arc::new(model), Arc::new(backend), 5, 10).await;

        let mut called = calls.lock().unwrap().clone();
        called.sort();
        assert_eq!(
            called,
            vec!["critical-dev".to_string(), "triggered-dev".to_string()]
        );
    }
}
