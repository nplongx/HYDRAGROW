use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use std::str::FromStr;
use std::time::Duration;
use tracing::{error, info, warn};

pub fn compute_next_run(
    expression: &str,
    from: DateTime<Tz>,
) -> Result<DateTime<Tz>, String> {
    let schedule = cron::Schedule::from_str(expression)
        .map_err(|e| format!("Cron expression không hợp lệ: {}", e))?;
    schedule
        .after(&from)
        .next()
        .ok_or_else(|| "Không tính được lần chạy tiếp theo".to_string())
}

/// Chạy nền, quét mỗi 30s các Flow có cron_next_run_at đã tới hạn.
pub async fn run_cron_loop(app_state: std::sync::Arc<crate::AppState>) {
    let mut ticker = tokio::time::interval(Duration::from_secs(30));
    loop {
        ticker.tick().await;
        if let Err(e) = tick_once(&app_state).await {
            error!(error = %e, "cron_scheduler tick thất bại");
        }
    }
}

async fn tick_once(app_state: &crate::AppState) -> Result<(), sqlx::Error> {
    let due: Vec<crate::models::script::UserScript> = sqlx::query_as(
        "SELECT * FROM user_scripts \
         WHERE cron_next_run_at IS NOT NULL AND cron_next_run_at <= now() AND enabled = TRUE",
    )
    .fetch_all(&app_state.pg_pool)
    .await?;

    let engine = std::sync::Arc::new(crate::services::script_engine::ScriptEngine::new());

    for script in due {
        info!(script_id = %script.id, "cron trigger fired");

        // Dispatch chain execution if AST exists
        if let Ok(ast) = engine.compile(&script.source) {
            let node = crate::mqtt::handlers::script_eval::WebhookChainNode {
                id: script.id,
                kind: match script.kind.as_str() {
                    "alert" => crate::models::script::ScriptKind::Alert,
                    "action_command" => crate::models::script::ScriptKind::ActionCommand,
                    _ => crate::models::script::ScriptKind::Alert,
                },
                next_flow_ids: script.next_flow_ids.clone(),
                ast,
            };

            let payload = serde_json::Map::new();
            let all_nodes = vec![node];
            let results = crate::mqtt::handlers::script_eval::eval_webhook_chain(
                &engine,
                &all_nodes,
                &payload,
            );

            for (_id, res) in results {
                if let crate::mqtt::handlers::script_eval::ChainFireResult::ActionCommand(cmd) = res {
                    if let Ok(cfg) = crate::db::postgres::get_safety_config(&app_state.pg_pool, &script.device_id).await {
                        let limits = hydragrow_shared::safety::DoseSafetyLimits {
                            max_dose_per_cycle_ml: cfg.max_dose_per_cycle,
                            max_dose_per_hour_ml: cfg.max_dose_per_hour,
                            cooldown_sec: cfg.cooldown_sec as u64,
                        };
                        let calibration = crate::db::postgres::fetch_dosing_calibration(&app_state.pg_pool, &script.device_id).await.unwrap_or(None);
                        let hourly = crate::db::postgres::get_dosing_history_last_hour(&app_state.pg_pool, &script.device_id).await.unwrap_or_default();
                        let last_dose = crate::db::postgres::get_last_dose_at(&app_state.pg_pool, &script.device_id).await.unwrap_or(None);
                        let now_sec = (chrono::Utc::now().timestamp_millis() / 1000) as u64;

                        let _ = crate::services::action_dispatch::dispatch_action_command(
                            app_state,
                            &script.device_id,
                            cmd,
                            &limits,
                            &hourly,
                            now_sec,
                            last_dose,
                            calibration.as_ref(),
                        ).await;
                    }
                }
            }
        }

        if let Some(ir) = &script.ir_json {
            let expr = ir.pointer("/trigger/cronExpression").and_then(|v| v.as_str());
            let tz_name = ir.pointer("/trigger/timezone").and_then(|v| v.as_str()).unwrap_or("Asia/Ho_Chi_Minh");
            if let (Some(expr), Ok(tz)) = (expr, tz_name.parse::<Tz>()) {
                let now = Utc::now().with_timezone(&tz);
                match compute_next_run(expr, now) {
                    Ok(next) => {
                        let _ = sqlx::query("UPDATE user_scripts SET cron_next_run_at = $1 WHERE id = $2")
                            .bind(next.with_timezone(&Utc)).bind(script.id)
                            .execute(&app_state.pg_pool).await;
                    }
                    Err(e) => warn!(script_id = %script.id, error = %e, "không tính được next_run, tắt cron cho Flow này"),
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn computes_next_run_for_daily_7am_expression() {
        let now = chrono_tz::Asia::Ho_Chi_Minh
            .with_ymd_and_hms(2026, 9, 4, 8, 0, 0).unwrap();
        let next = compute_next_run("0 0 7 * * * *", now).unwrap();
        assert_eq!(next.format("%Y-%m-%d %H:%M").to_string(), "2026-09-05 07:00");
    }

    #[test]
    fn invalid_expression_returns_error() {
        assert!(compute_next_run("not a cron", chrono::Utc::now().with_timezone(&chrono_tz::Asia::Ho_Chi_Minh)).is_err());
    }
}
