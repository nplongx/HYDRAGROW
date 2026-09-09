mod backend_client;
mod config;
mod staleness;
mod tick;

use backend_client::BackendClient;
use config::WatchdogConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = WatchdogConfig::from_env()?;
    tracing::info!(
        poll_interval_secs = config.poll_interval_secs,
        stale_threshold_secs = config.stale_threshold_secs,
        "hydragrow-watchdog starting"
    );

    let client = BackendClient::new(config.backend_url.clone(), config.api_key.clone());
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(config.poll_interval_secs));

    loop {
        interval.tick().await;
        if let Err(e) = tick::run_tick(&client, config.stale_threshold_secs).await {
            // A failed tick (e.g. backend temporarily unreachable) must never
            // crash the process — design spec §8.1's isolation principle
            // applies here too: the watchdog keeps running and simply tries
            // again next interval.
            tracing::error!(error = %e, "tick failed, will retry next interval");
        }
    }
}
