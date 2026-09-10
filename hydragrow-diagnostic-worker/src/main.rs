mod backend_client;
mod config;
mod diagnostic_model;
mod openrouter_model;
mod orchestrator;
mod tick;
mod trigger;

use backend_client::BackendClient;
use config::WorkerConfig;
use hydragrow_supervisor_query::HttpQueryBackend;
use openrouter_model::OpenRouterDiagnosticModel;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = WorkerConfig::from_env()?;
    tracing::info!(
        provider = %config.llm_provider,
        model = %config.llm_model,
        "hydragrow-diagnostic-worker starting"
    );

    let query_backend: Arc<dyn hydragrow_supervisor_query::QueryBackend> = Arc::new(
        HttpQueryBackend::new(config.backend_url.clone(), config.api_key.clone()),
    );

    let model: Arc<dyn diagnostic_model::DiagnosticModel> =
        Arc::new(OpenRouterDiagnosticModel::new(
            "https://openrouter.ai/api".to_string(),
            config.openrouter_api_key.clone(),
            config.llm_model.clone(),
            query_backend,
            config.max_tool_round_trips,
            config.max_input_tokens,
            config.max_output_tokens,
            config.per_call_timeout_secs,
            config.diagnosis_wall_clock_budget_secs,
        ));

    let backend: Arc<dyn backend_client::DiagnosisBackend> = Arc::new(BackendClient::new(
        config.backend_url.clone(),
        config.api_key.clone(),
    ));

    let mut interval =
        tokio::time::interval(std::time::Duration::from_secs(config.poll_interval_secs));

    loop {
        interval.tick().await;
        tick::run_fleet_tick(
            model.clone(),
            backend.clone(),
            config.max_concurrent_diagnoses,
            config.watchdog_breach_lookback_minutes,
        )
        .await;
    }
}
