use crate::backend_client::DiagnosisBackend;
use crate::diagnostic_model::{DiagnosticContext, DiagnosticModel, unexplained_anomaly_fallback};
use crate::trigger::SupervisorTrigger;

pub async fn run_diagnosis(
    device_id: &str,
    trigger: SupervisorTrigger,
    model: &dyn DiagnosticModel,
    backend: &dyn DiagnosisBackend,
    hestia_snapshot: serde_json::Value,
    crop_target: serde_json::Value,
) -> anyhow::Result<()> {
    // Design spec §5: pre-check before spending anything on the LLM.
    let reason_code_for_dedup = match &trigger {
        SupervisorTrigger::HestiaState { reasons, .. } => reasons
            .first()
            .cloned()
            .unwrap_or_else(|| "unexplained_anomaly".to_string()),
        SupervisorTrigger::WatchdogBreach => "topic_stale_controller_status".to_string(),
    };
    if backend
        .recent_alert_exists(device_id, &reason_code_for_dedup)
        .await?
    {
        tracing::debug!(device_id, "recent alert exists, skipping diagnosis");
        return Ok(());
    }

    let context = DiagnosticContext {
        device_id: device_id.to_string(),
        trigger: trigger.clone(),
        hestia_snapshot,
        crop_target,
    };

    let started = std::time::Instant::now();
    let diagnosis = match model.diagnose(context).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(device_id, error = %e, "diagnosis failed, falling back to unexplained_anomaly");
            unexplained_anomaly_fallback(&trigger)
        }
    };

    let level = if diagnosis.confidence >= 0.5 {
        "warning"
    } else {
        "info"
    };
    let title = diagnosis
        .reason_codes
        .first()
        .map(|c| c.as_str())
        .unwrap_or("unexplained_anomaly");
    let reason_codes: Vec<String> = diagnosis
        .reason_codes
        .iter()
        .map(|c| c.as_str().to_string())
        .collect();
    let observations = serde_json::json!({
        "current_value": diagnosis.observations.current_value,
        "target_value": diagnosis.observations.target_value,
        "delta": diagnosis.observations.delta,
        "window_minutes": diagnosis.observations.window_minutes,
        "corroborating_evidence": diagnosis.observations.corroborating_evidence,
    });

    backend
        .create_diagnosis_alert(
            device_id,
            level,
            title,
            &diagnosis.narrative,
            reason_codes,
            diagnosis.confidence,
            observations,
        )
        .await?;

    tracing::info!(
        device_id,
        latency_ms = started.elapsed().as_millis(),
        confidence = diagnosis.confidence,
        "diagnosis complete, alert written"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend_client::DiagnosisBackend;
    use crate::diagnostic_model::{Diagnosis, DiagnosisObservations, DiagnosticModelError};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeModel {
        result: Mutex<Option<Result<Diagnosis, DiagnosticModelError>>>,
        calls: Mutex<u32>,
    }
    #[async_trait]
    impl DiagnosticModel for FakeModel {
        async fn diagnose(
            &self,
            _ctx: DiagnosticContext,
        ) -> Result<Diagnosis, DiagnosticModelError> {
            *self.calls.lock().unwrap() += 1;
            self.result.lock().unwrap().take().unwrap()
        }
    }

    #[derive(Default)]
    struct FakeBackend {
        recent_alert_exists: Mutex<bool>,
        created: Mutex<Vec<String>>,
    }
    #[async_trait]
    impl DiagnosisBackend for FakeBackend {
        async fn get_fleet_hestia(&self) -> anyhow::Result<HashMap<String, serde_json::Value>> {
            Ok(HashMap::new())
        }
        async fn has_recent_watchdog_breach(&self, _: &str, _: i64) -> anyhow::Result<bool> {
            Ok(false)
        }
        async fn recent_alert_exists(&self, _: &str, _: &str) -> anyhow::Result<bool> {
            Ok(*self.recent_alert_exists.lock().unwrap())
        }
        async fn get_crop_target(&self, _: &str) -> anyhow::Result<serde_json::Value> {
            Ok(serde_json::json!({}))
        }
        async fn create_diagnosis_alert(
            &self,
            device_id: &str,
            _: &str,
            _: &str,
            _: &str,
            _: Vec<String>,
            _: f32,
            _: serde_json::Value,
        ) -> anyhow::Result<()> {
            self.created.lock().unwrap().push(device_id.to_string());
            Ok(())
        }
    }

    fn sample_diagnosis() -> Diagnosis {
        Diagnosis {
            reason_codes: vec![hydragrow_shared::supervisor::SupervisorReasonCode::LeakSuspected],
            confidence: 0.6,
            narrative: "EC rose without dosing.".to_string(),
            observations: DiagnosisObservations {
                current_value: 2.7,
                target_value: 1.8,
                delta: 0.9,
                window_minutes: 10,
                corroborating_evidence: vec![],
            },
        }
    }

    #[tokio::test]
    async fn skips_llm_entirely_when_dedup_check_says_recent_alert_exists() {
        let model = FakeModel {
            result: Mutex::new(Some(Ok(sample_diagnosis()))),
            calls: Mutex::new(0),
        };
        let backend = FakeBackend {
            recent_alert_exists: Mutex::new(true),
            created: Mutex::new(vec![]),
        };
        let trigger = crate::trigger::SupervisorTrigger::WatchdogBreach;

        run_diagnosis(
            "d1",
            trigger,
            &model,
            &backend,
            serde_json::json!({}),
            serde_json::json!({}),
        )
        .await
        .unwrap();

        assert_eq!(*model.calls.lock().unwrap(), 0);
        assert!(backend.created.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn calls_llm_and_writes_alert_when_not_deduped() {
        let model = FakeModel {
            result: Mutex::new(Some(Ok(sample_diagnosis()))),
            calls: Mutex::new(0),
        };
        let backend = FakeBackend::default();
        let trigger = crate::trigger::SupervisorTrigger::HestiaState {
            state: "WARNING".to_string(),
            reasons: vec!["ec_out_of_range".to_string()],
        };

        run_diagnosis(
            "d1",
            trigger,
            &model,
            &backend,
            serde_json::json!({}),
            serde_json::json!({}),
        )
        .await
        .unwrap();

        assert_eq!(*model.calls.lock().unwrap(), 1);
        assert_eq!(*backend.created.lock().unwrap(), vec!["d1".to_string()]);
    }

    #[tokio::test]
    async fn writes_unexplained_anomaly_fallback_when_model_errors() {
        let model = FakeModel {
            result: Mutex::new(Some(Err(DiagnosticModelError::Timeout))),
            calls: Mutex::new(0),
        };
        let backend = FakeBackend::default();
        let trigger = crate::trigger::SupervisorTrigger::HestiaState {
            state: "CRITICAL".to_string(),
            reasons: vec!["ec_out_of_range".to_string()],
        };

        run_diagnosis(
            "d1",
            trigger,
            &model,
            &backend,
            serde_json::json!({}),
            serde_json::json!({}),
        )
        .await
        .unwrap();

        assert_eq!(*backend.created.lock().unwrap(), vec!["d1".to_string()]);
    }
}
