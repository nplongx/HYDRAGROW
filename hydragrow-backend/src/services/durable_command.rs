use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use hydragrow_shared::{CommandLifecycle, CommandLifecycleEvent};
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::AppState;

pub const MAX_ATTEMPTS: i32 = 5;
pub const RETRY_BASE_SECS: i64 = 2;
pub const RETRY_MAX_SECS: i64 = 60;
pub const ACK_TIMEOUT_SECS: i64 = 30;
pub const CONFIRM_TIMEOUT_SECS: i64 = 90;

#[derive(Debug, Clone)]
pub struct NewCommand {
    pub idempotency_key: Option<String>,
    pub principal_kind: String,
    pub principal_id: Option<String>,
    pub service_key_label: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<i64>,
    pub device_id: String,
    pub action: String,
    pub request_payload: Value,
    pub requested_state: Option<bool>,
    pub requested_pwm: Option<u32>,
    pub pump_id: Option<String>,
    /// Whether a transport retry is safe for this command class.
    pub retry_safe: bool,
}

#[derive(Debug, Clone)]
pub struct DurableCommand {
    pub command_id: String,
    pub device_id: String,
    pub action: String,
    pub pump_id: Option<String>,
    pub requested_state: Option<bool>,
    pub requested_pwm: Option<u32>,
    pub lifecycle: CommandLifecycle,
    pub created_at: DateTime<Utc>,
    pub authorized_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub terminal_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub attempt_count: i32,
    pub last_error: Option<String>,
    pub last_observed_at: Option<DateTime<Utc>>,
    pub version: i64,
    pub retry_safe: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionResult {
    Applied,
    Duplicate,
}

fn lifecycle_name(lifecycle: CommandLifecycle) -> &'static str {
    match lifecycle {
        CommandLifecycle::Requested => "REQUESTED",
        CommandLifecycle::Sent => "SENT",
        CommandLifecycle::Acknowledged => "ACKNOWLEDGED",
        CommandLifecycle::Confirmed => "CONFIRMED",
        CommandLifecycle::Rejected => "REJECTED",
        CommandLifecycle::Failed => "FAILED",
        CommandLifecycle::Timeout => "TIMEOUT",
        CommandLifecycle::Unknown => "UNKNOWN",
    }
}

fn parse_lifecycle(value: &str) -> Result<CommandLifecycle> {
    match value {
        "REQUESTED" => Ok(CommandLifecycle::Requested),
        "SENT" => Ok(CommandLifecycle::Sent),
        "ACKNOWLEDGED" => Ok(CommandLifecycle::Acknowledged),
        "CONFIRMED" => Ok(CommandLifecycle::Confirmed),
        "REJECTED" => Ok(CommandLifecycle::Rejected),
        "FAILED" => Ok(CommandLifecycle::Failed),
        "TIMEOUT" => Ok(CommandLifecycle::Timeout),
        "UNKNOWN" => Ok(CommandLifecycle::Unknown),
        other => Err(anyhow!("invalid durable lifecycle {other}")),
    }
}

fn idempotency_scope(command: &NewCommand) -> String {
    let action_class = if matches!(
        command.action.as_str(),
        "force_on" | "reset_fault" | "set_pwm" | "emergency_stop"
    ) {
        "dangerous"
    } else {
        "normal"
    };
    format!(
        "{}:{}:{}:{}:{}",
        command.principal_kind,
        command.principal_id.as_deref().unwrap_or(""),
        command.service_key_label.as_deref().unwrap_or(""),
        command.device_id,
        action_class
    )
}

fn safe_payload(payload: &Value) -> Value {
    let Some(object) = payload.as_object() else {
        return json!({});
    };
    let allowed = ["action", "pump_id", "duration_sec", "pwm", "state"];
    object
        .iter()
        .filter(|(key, _)| allowed.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

pub async fn create_command(pool: &PgPool, command: NewCommand) -> Result<(DurableCommand, bool)> {
    let scope = idempotency_scope(&command);
    let mut tx = pool.begin().await?;

    if let Some(key) = command.idempotency_key.as_deref()
        && let Some(row) = sqlx::query(
            "SELECT command_id, request_payload, action, device_id FROM commands WHERE idempotency_scope = $1 AND idempotency_key = $2 FOR UPDATE",
        )
        .bind(&scope)
        .bind(key)
        .fetch_optional(&mut *tx)
        .await?
    {
        let existing_payload: Value = row.try_get("request_payload")?;
        let existing_action: String = row.try_get("action")?;
        let existing_device: String = row.try_get("device_id")?;
        if existing_action != command.action
            || existing_device != command.device_id
            || existing_payload != safe_payload(&command.request_payload)
        {
            return Err(anyhow!("idempotency key conflicts with existing command"));
        }
        let id: String = row.try_get("command_id")?;
        let existing = get_command_tx(&mut tx, &id).await?;
        tx.commit().await?;
        return Ok((existing, true));
    }

    let command_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let payload = safe_payload(&command.request_payload);
    let insert_result = sqlx::query(
        r#"INSERT INTO commands
        (command_id,idempotency_key,idempotency_scope,principal_kind,principal_id,service_key_label,
         session_id,user_id,device_id,action,request_payload,requested_state,requested_pwm,pump_id,
         lifecycle,created_at,authorized_at,next_retry_at,retry_safe)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,'REQUESTED',$15,$15,$15,$16)"#,
    )
    .bind(&command_id)
    .bind(&command.idempotency_key)
    .bind(&scope)
    .bind(&command.principal_kind)
    .bind(&command.principal_id)
    .bind(&command.service_key_label)
    .bind(&command.session_id)
    .bind(command.user_id)
    .bind(&command.device_id)
    .bind(&command.action)
    .bind(&payload)
    .bind(command.requested_state)
    .bind(command.requested_pwm.map(|v| v as i32))
    .bind(&command.pump_id)
    .bind(now)
    .bind(command.retry_safe)
    .execute(&mut *tx)
    .await;
    if let Err(error) = insert_result {
        if error
            .as_database_error()
            .and_then(|db| db.code())
            .is_some_and(|code| code == "23505")
        {
            tx.rollback().await?;
            if let Some(key) = command.idempotency_key.as_deref()
                && let Some(row) = sqlx::query("SELECT command_id,request_payload,action,device_id FROM commands WHERE idempotency_scope=$1 AND idempotency_key=$2")
                    .bind(&scope).bind(key).fetch_optional(pool).await?
            {
                let existing_payload: Value = row.try_get("request_payload")?;
                let existing_action: String = row.try_get("action")?;
                let existing_device: String = row.try_get("device_id")?;
                if existing_action != command.action || existing_device != command.device_id || existing_payload != payload {
                    return Err(anyhow!("idempotency key conflicts with existing command"));
                }
                let id: String = row.try_get("command_id")?;
                return Ok((get_command(pool, &id).await?, true));
            }
        }
        return Err(error.into());
    }

    insert_event_tx(
        &mut tx,
        &command_id,
        1,
        None,
        CommandLifecycle::Requested,
        &command.device_id,
        None,
        "create",
        0,
        now,
        json!({}),
    )
    .await?;
    tx.commit().await?;
    let created = get_command(pool, &command_id).await?;
    Ok((created, false))
}

pub async fn create_command_with_state(
    app_state: &AppState,
    command: NewCommand,
) -> Result<(DurableCommand, bool)> {
    let device_id = command.device_id.clone();
    let (created, existing) = create_command(&app_state.pg_pool, command).await?;
    if !existing {
        crate::metrics::COMMAND_LIFECYCLE_TOTAL
            .with_label_values(&["create", "REQUESTED"])
            .inc();
        let _ = app_state
            .event_bus
            .send(hydragrow_shared::events::AppEvent::CommandLifecycle(
                CommandLifecycleEvent {
                    command_id: created.command_id.clone(),
                    device_id,
                    lifecycle: CommandLifecycle::Requested,
                    reason: None,
                    timestamp_ms: created.created_at.timestamp_millis(),
                },
            ));
    }
    Ok((created, existing))
}

async fn get_command_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command_id: &str,
) -> Result<DurableCommand> {
    let row = sqlx::query("SELECT * FROM commands WHERE command_id = $1")
        .bind(command_id)
        .fetch_one(&mut **tx)
        .await?;
    row_to_command(&row)
}

pub async fn get_command(pool: &PgPool, command_id: &str) -> Result<DurableCommand> {
    let row = sqlx::query("SELECT * FROM commands WHERE command_id = $1")
        .bind(command_id)
        .fetch_one(pool)
        .await?;
    row_to_command(&row)
}

pub async fn list_lifecycle_events(pool: &PgPool, command_id: &str) -> Result<Vec<Value>> {
    let rows = sqlx::query("SELECT sequence_no,from_lifecycle,lifecycle,device_id,reason,source,attempt_no,occurred_at,metadata FROM command_lifecycle_events WHERE command_id=$1 ORDER BY sequence_no")
        .bind(command_id).fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| {
            Ok(json!({
                "sequence_no": row.try_get::<i32,_>("sequence_no")?,
                "from_lifecycle": row.try_get::<Option<String>,_>("from_lifecycle")?,
                "lifecycle": row.try_get::<String,_>("lifecycle")?,
                "device_id": row.try_get::<String,_>("device_id")?,
                "reason": row.try_get::<Option<String>,_>("reason")?,
                "source": row.try_get::<String,_>("source")?,
                "attempt_no": row.try_get::<i32,_>("attempt_no")?,
                "occurred_at": row.try_get::<DateTime<Utc>,_>("occurred_at")?,
                "metadata": row.try_get::<Value,_>("metadata")?,
            }))
        })
        .collect()
}

fn row_to_command(row: &sqlx::postgres::PgRow) -> Result<DurableCommand> {
    Ok(DurableCommand {
        command_id: row.try_get("command_id")?,
        device_id: row.try_get("device_id")?,
        action: row.try_get("action")?,
        pump_id: row.try_get("pump_id")?,
        retry_safe: row.try_get("retry_safe")?,
        requested_state: row.try_get("requested_state")?,
        requested_pwm: row
            .try_get::<Option<i32>, _>("requested_pwm")?
            .map(|v| v as u32),
        lifecycle: parse_lifecycle(&row.try_get::<String, _>("lifecycle")?)?,
        created_at: row.try_get("created_at")?,
        authorized_at: row.try_get("authorized_at")?,
        sent_at: row.try_get("sent_at")?,
        acknowledged_at: row.try_get("acknowledged_at")?,
        confirmed_at: row.try_get("confirmed_at")?,
        terminal_at: row.try_get("terminal_at")?,
        next_retry_at: row.try_get("next_retry_at")?,
        attempt_count: row.try_get("attempt_count")?,
        last_error: row.try_get("last_error")?,
        last_observed_at: row.try_get("last_observed_at")?,
        version: row.try_get("version")?,
    })
}

pub async fn mark_publish_attempt(pool: &PgPool, command_id: &str) -> Result<i32> {
    let now = Utc::now();
    let row = sqlx::query(
        r#"UPDATE commands SET attempt_count = attempt_count + 1,
             next_retry_at = NULL, last_error = NULL, version = version + 1
           WHERE command_id = $1 AND lifecycle IN ('REQUESTED','SENT')
           RETURNING attempt_count"#,
    )
    .bind(command_id)
    .fetch_one(pool)
    .await?;
    let _ = now;
    Ok(row.try_get("attempt_count")?)
}

pub async fn schedule_publish_retry(pool: &PgPool, command_id: &str, error: &str) -> Result<()> {
    let command = get_command(pool, command_id).await?;
    if command.lifecycle.is_terminal() {
        return Ok(());
    }
    if !command.retry_safe {
        transition(
            pool,
            command_id,
            CommandLifecycle::Unknown,
            Some("dangerous command publish outcome is ambiguous; automatic retry disabled".into()),
            "publish",
            json!({"automatic_retry":false}),
        )
        .await?;
        return Ok(());
    }
    let delay = (RETRY_BASE_SECS * 2_i64.pow(command.attempt_count.saturating_sub(1) as u32))
        .min(RETRY_MAX_SECS);
    let next = Utc::now() + Duration::seconds(delay);
    if command.attempt_count >= MAX_ATTEMPTS {
        transition(
            pool,
            command_id,
            CommandLifecycle::Failed,
            Some("MQTT publish retry budget exhausted".into()),
            "retry",
            json!({"attempt_count": command.attempt_count}),
        )
        .await?;
        return Ok(());
    }
    sqlx::query("UPDATE commands SET next_retry_at=$2,last_error=$3,version=version+1 WHERE command_id=$1 AND NOT lifecycle IN ('CONFIRMED','REJECTED','FAILED','TIMEOUT','UNKNOWN')")
        .bind(command_id).bind(next).bind(error).execute(pool).await?;
    Ok(())
}

pub async fn schedule_publish_retry_with_state(
    app_state: &AppState,
    command_id: &str,
    error: &str,
) -> Result<()> {
    let command = get_command(&app_state.pg_pool, command_id).await?;
    if command.lifecycle.is_terminal() {
        return Ok(());
    }
    if !command.retry_safe {
        transition_with_state(
            app_state,
            command_id,
            &command.device_id,
            CommandLifecycle::Unknown,
            Some("command publish outcome is ambiguous; automatic retry disabled".into()),
            "publish",
            json!({"automatic_retry":false}),
        )
        .await?;
        return Ok(());
    }
    if command.attempt_count >= MAX_ATTEMPTS {
        transition_with_state(
            app_state,
            command_id,
            &command.device_id,
            CommandLifecycle::Failed,
            Some("MQTT publish retry budget exhausted".into()),
            "retry",
            json!({"attempt_count": command.attempt_count}),
        )
        .await?;
        return Ok(());
    }
    let delay = (RETRY_BASE_SECS * 2_i64.pow(command.attempt_count.saturating_sub(1) as u32))
        .min(RETRY_MAX_SECS);
    let next = Utc::now() + Duration::seconds(delay);
    sqlx::query("UPDATE commands SET next_retry_at=$2,last_error=$3,version=version+1 WHERE command_id=$1 AND lifecycle='REQUESTED'")
        .bind(command_id).bind(next).bind(error).execute(&app_state.pg_pool).await?;
    Ok(())
}

pub async fn transition(
    pool: &PgPool,
    command_id: &str,
    next: CommandLifecycle,
    reason: Option<String>,
    source: &str,
    metadata: Value,
) -> Result<TransitionResult> {
    let now = Utc::now();
    let mut tx = pool.begin().await?;
    let row = sqlx::query("SELECT * FROM commands WHERE command_id=$1 FOR UPDATE")
        .bind(command_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("unknown command_id"))?;
    let current = parse_lifecycle(&row.try_get::<String, _>("lifecycle")?)?;
    let device_id: String = row.try_get("device_id")?;
    let attempt_no: i32 = row.try_get("attempt_count")?;
    let phase_started_at: Option<DateTime<Utc>> = match current {
        CommandLifecycle::Requested => row.try_get("created_at")?,
        CommandLifecycle::Sent => row.try_get("sent_at")?,
        CommandLifecycle::Acknowledged => row.try_get("acknowledged_at")?,
        CommandLifecycle::Confirmed
        | CommandLifecycle::Rejected
        | CommandLifecycle::Failed
        | CommandLifecycle::Timeout
        | CommandLifecycle::Unknown => row.try_get("last_observed_at")?,
    };
    if current == next {
        tx.commit().await?;
        return Ok(TransitionResult::Duplicate);
    }
    current
        .transition_to(next)
        .map_err(|e| anyhow!("invalid command transition: {e:?}"))?;
    let sequence: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sequence_no),0)+1 FROM command_lifecycle_events WHERE command_id=$1",
    )
    .bind(command_id)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        r#"UPDATE commands SET lifecycle=$2,
          sent_at=CASE WHEN $2='SENT' THEN COALESCE(sent_at,$3) ELSE sent_at END,
          acknowledged_at=CASE WHEN $2='ACKNOWLEDGED' THEN COALESCE(acknowledged_at,$3) ELSE acknowledged_at END,
          confirmed_at=CASE WHEN $2='CONFIRMED' THEN COALESCE(confirmed_at,$3) ELSE confirmed_at END,
          terminal_at=CASE WHEN $2 IN ('CONFIRMED','REJECTED','FAILED','TIMEOUT','UNKNOWN') THEN COALESCE(terminal_at,$3) ELSE terminal_at END,
          last_error=CASE WHEN $2 IN ('FAILED','REJECTED','TIMEOUT','UNKNOWN') THEN $4 ELSE last_error END,
          last_observed_at=CASE WHEN $2 IN ('ACKNOWLEDGED','CONFIRMED') THEN $3 ELSE last_observed_at END,
          next_retry_at=CASE WHEN $2 <> 'REQUESTED' THEN NULL ELSE next_retry_at END,
          version=version+1 WHERE command_id=$1"#,
    )
    .bind(command_id).bind(lifecycle_name(next)).bind(now).bind(&reason)
    .execute(&mut *tx).await?;
    insert_event_tx(
        &mut tx,
        command_id,
        sequence,
        Some(current),
        next,
        &device_id,
        reason.as_deref(),
        source,
        attempt_no,
        now,
        metadata,
    )
    .await?;
    tx.commit().await?;
    if let Some(started_at) = phase_started_at
        && let Ok(duration) = (now - started_at).to_std()
    {
        let phase = match (current, next) {
            (CommandLifecycle::Requested, CommandLifecycle::Sent) => "requested_to_sent",
            (CommandLifecycle::Sent, CommandLifecycle::Acknowledged) => "sent_to_acknowledged",
            (CommandLifecycle::Acknowledged, CommandLifecycle::Confirmed) => {
                "acknowledged_to_confirmed"
            }
            _ => "other",
        };
        crate::metrics::SYNC_PHASE_DURATION_SECONDS
            .with_label_values(&["command", phase])
            .observe(duration.as_secs_f64());
    }
    Ok(TransitionResult::Applied)
}

#[allow(clippy::too_many_arguments)]
async fn insert_event_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command_id: &str,
    sequence_no: i32,
    from: Option<CommandLifecycle>,
    lifecycle: CommandLifecycle,
    device_id: &str,
    reason: Option<&str>,
    source: &str,
    attempt_no: i32,
    occurred_at: DateTime<Utc>,
    metadata: Value,
) -> Result<()> {
    sqlx::query("INSERT INTO command_lifecycle_events (command_id,sequence_no,from_lifecycle,lifecycle,device_id,reason,source,attempt_no,occurred_at,metadata) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
        .bind(command_id).bind(sequence_no).bind(from.map(lifecycle_name)).bind(lifecycle_name(lifecycle))
        .bind(device_id).bind(reason).bind(source).bind(attempt_no).bind(occurred_at).bind(metadata)
        .execute(&mut **tx).await?;
    Ok(())
}

pub async fn transition_with_state(
    app_state: &AppState,
    command_id: &str,
    device_id: &str,
    next: CommandLifecycle,
    reason: Option<String>,
    source: &str,
    metadata: Value,
) -> Result<TransitionResult> {
    let result = transition(
        &app_state.pg_pool,
        command_id,
        next,
        reason.clone(),
        source,
        metadata,
    )
    .await?;
    if result == TransitionResult::Applied {
        match next {
            CommandLifecycle::Unknown => crate::metrics::COMMAND_UNKNOWN_TOTAL.inc(),
            CommandLifecycle::Timeout => crate::metrics::COMMAND_TIMEOUT_TOTAL.inc(),
            _ => {}
        }
        crate::metrics::COMMAND_LIFECYCLE_TOTAL
            .with_label_values(&[source, lifecycle_name(next)])
            .inc();
        let level = match next {
            CommandLifecycle::Confirmed => "success",
            CommandLifecycle::Rejected
            | CommandLifecycle::Failed
            | CommandLifecycle::Timeout
            | CommandLifecycle::Unknown => "warning",
            _ => "info",
        };
        let _ = crate::db::postgres::insert_system_event(
            &app_state.pg_pool,
            &crate::db::postgres::NewSystemEventRecord {
                device_id: device_id.to_string(),
                level: level.to_string(),
                category: "user_action".to_string(),
                title: "Command Lifecycle".to_string(),
                message: format!("command_id={} lifecycle={}", command_id, lifecycle_name(next)),
                reason: reason.clone(),
                metadata: Some(json!({"event_type":"command_lifecycle","command_id":command_id,"lifecycle":lifecycle_name(next),"source":source})),
                timestamp: Utc::now().timestamp_millis(),
                source: "command_lifecycle".to_string(),
                primary_reason_code: None,
            },
        )
        .await;
        let event = CommandLifecycleEvent {
            command_id: command_id.to_string(),
            device_id: device_id.to_string(),
            lifecycle: next,
            reason,
            timestamp_ms: Utc::now().timestamp_millis(),
        };
        let _ = app_state
            .event_bus
            .send(hydragrow_shared::events::AppEvent::CommandLifecycle(event));
    } else {
        crate::metrics::COMMAND_LIFECYCLE_TOTAL
            .with_label_values(&["duplicate", lifecycle_name(next)])
            .inc();
    }
    Ok(result)
}

pub async fn list_commands(
    pool: &PgPool,
    device_id: &str,
    limit: i64,
) -> Result<Vec<DurableCommand>> {
    let rows =
        sqlx::query("SELECT * FROM commands WHERE device_id=$1 ORDER BY created_at DESC LIMIT $2")
            .bind(device_id)
            .bind(limit.clamp(1, 100))
            .fetch_all(pool)
            .await?;
    rows.iter().map(row_to_command).collect()
}

pub async fn reconcile(pool: &PgPool) -> Result<u64> {
    let now = Utc::now();
    let mut count = 0;
    let rows = sqlx::query("SELECT command_id,lifecycle,attempt_count,next_retry_at,sent_at,acknowledged_at FROM commands WHERE lifecycle IN ('REQUESTED','SENT','ACKNOWLEDGED')")
        .fetch_all(pool).await?;
    for row in rows {
        let id: String = row.try_get("command_id")?;
        let lifecycle = parse_lifecycle(&row.try_get::<String, _>("lifecycle")?)?;
        let attempts: i32 = row.try_get("attempt_count")?;
        let sent_at: Option<DateTime<Utc>> = row.try_get("sent_at")?;
        let ack_at: Option<DateTime<Utc>> = row.try_get("acknowledged_at")?;
        let next_retry: Option<DateTime<Utc>> = row.try_get("next_retry_at")?;
        let last_error: Option<String> =
            sqlx::query_scalar("SELECT last_error FROM commands WHERE command_id=$1")
                .bind(&id)
                .fetch_optional(pool)
                .await?
                .flatten();
        if lifecycle == CommandLifecycle::Requested && attempts > 0 && last_error.is_none() {
            transition(
                pool,
                &id,
                CommandLifecycle::Unknown,
                Some(
                    "backend recovery found an outbound attempt with ambiguous publication outcome"
                        .into(),
                ),
                "recovery",
                json!({"ambiguous_publish":true}),
            )
            .await?;
            count += 1;
        } else if lifecycle == CommandLifecycle::Requested && attempts >= MAX_ATTEMPTS {
            transition(
                pool,
                &id,
                CommandLifecycle::Failed,
                Some("publish retry budget exhausted during recovery".into()),
                "recovery",
                json!({}),
            )
            .await?;
            count += 1;
        } else if lifecycle == CommandLifecycle::Sent
            && sent_at.is_some_and(|t| t + Duration::seconds(ACK_TIMEOUT_SECS) <= now)
        {
            transition(
                pool,
                &id,
                CommandLifecycle::Timeout,
                Some("ACK deadline elapsed".into()),
                "timeout",
                json!({"deadline_seconds": ACK_TIMEOUT_SECS}),
            )
            .await?;
            count += 1;
        } else if lifecycle == CommandLifecycle::Acknowledged
            && ack_at.is_some_and(|t| t + Duration::seconds(CONFIRM_TIMEOUT_SECS) <= now)
        {
            transition(
                pool,
                &id,
                CommandLifecycle::Timeout,
                Some("confirmation deadline elapsed".into()),
                "timeout",
                json!({"deadline_seconds": CONFIRM_TIMEOUT_SECS}),
            )
            .await?;
            count += 1;
        } else if lifecycle == CommandLifecycle::Requested && next_retry.is_none() {
            sqlx::query("UPDATE commands SET next_retry_at=$2,version=version+1 WHERE command_id=$1 AND lifecycle='REQUESTED'").bind(&id).bind(now).execute(pool).await?;
        }
    }
    Ok(count)
}

/// Reconcile durable commands and publish lifecycle notifications only after
/// each transition commits. The DB remains the authority; the event bus is
/// delivery/WS fan-out only.
pub async fn reconcile_with_state(app_state: &AppState) -> Result<u64> {
    let now = Utc::now();
    let rows = sqlx::query(
        "SELECT command_id,lifecycle,attempt_count,next_retry_at,sent_at,acknowledged_at,last_error,device_id
         FROM commands WHERE lifecycle IN ('REQUESTED','SENT','ACKNOWLEDGED')",
    )
    .fetch_all(&app_state.pg_pool)
    .await?;
    let mut count = 0;

    for row in rows {
        let id: String = row.try_get("command_id")?;
        let device_id: String = row.try_get("device_id")?;
        let lifecycle = parse_lifecycle(&row.try_get::<String, _>("lifecycle")?)?;
        let attempts: i32 = row.try_get("attempt_count")?;
        let sent_at: Option<DateTime<Utc>> = row.try_get("sent_at")?;
        let ack_at: Option<DateTime<Utc>> = row.try_get("acknowledged_at")?;
        let next_retry: Option<DateTime<Utc>> = row.try_get("next_retry_at")?;
        let last_error: Option<String> = row.try_get("last_error")?;

        let transition =
            if lifecycle == CommandLifecycle::Requested && attempts > 0 && last_error.is_none() {
                Some((
                CommandLifecycle::Unknown,
                Some(
                    "backend recovery found an outbound attempt with ambiguous publication outcome"
                        .to_string(),
                ),
                "recovery",
                json!({"ambiguous_publish":true}),
            ))
            } else if lifecycle == CommandLifecycle::Requested && attempts >= MAX_ATTEMPTS {
                Some((
                    CommandLifecycle::Failed,
                    Some("publish retry budget exhausted during recovery".to_string()),
                    "recovery",
                    json!({}),
                ))
            } else if lifecycle == CommandLifecycle::Sent
                && sent_at.is_some_and(|t| t + Duration::seconds(ACK_TIMEOUT_SECS) <= now)
            {
                Some((
                    CommandLifecycle::Timeout,
                    Some("ACK deadline elapsed".to_string()),
                    "timeout",
                    json!({"deadline_seconds": ACK_TIMEOUT_SECS}),
                ))
            } else if lifecycle == CommandLifecycle::Acknowledged
                && ack_at.is_some_and(|t| t + Duration::seconds(CONFIRM_TIMEOUT_SECS) <= now)
            {
                Some((
                    CommandLifecycle::Timeout,
                    Some("confirmation deadline elapsed".to_string()),
                    "timeout",
                    json!({"deadline_seconds": CONFIRM_TIMEOUT_SECS}),
                ))
            } else {
                None
            };

        if let Some((next, reason, source, metadata)) = transition {
            if transition_with_state(app_state, &id, &device_id, next, reason, source, metadata)
                .await?
                == TransitionResult::Applied
            {
                count += 1;
            }
        } else if lifecycle == CommandLifecycle::Requested && next_retry.is_none() {
            sqlx::query(
                "UPDATE commands SET next_retry_at=$2,version=version+1 WHERE command_id=$1 AND lifecycle='REQUESTED'",
            )
            .bind(&id)
            .bind(now)
            .execute(&app_state.pg_pool)
            .await?;
        }
    }
    Ok(count)
}

pub async fn retry_due(app_state: &AppState) -> Result<u64> {
    // Claim due commands atomically. Clearing next_retry_at in the same UPDATE
    // makes concurrent backend instances mutually exclude the same retry attempt.
    let rows = sqlx::query(
        "UPDATE commands SET attempt_count = attempt_count + 1, next_retry_at = NULL, version = version + 1
         WHERE command_id IN (
             SELECT command_id FROM commands
             WHERE lifecycle='REQUESTED' AND next_retry_at <= NOW() AND attempt_count < $1
             ORDER BY next_retry_at
             FOR UPDATE SKIP LOCKED
             LIMIT 20
         )
         RETURNING command_id,device_id,action,request_payload,requested_state,requested_pwm,pump_id,attempt_count",
    )
    .bind(MAX_ATTEMPTS)
    .fetch_all(&app_state.pg_pool)
    .await?;
    let mut count = 0;
    for row in rows {
        let id: String = row.try_get("command_id")?;
        let device_id: String = row.try_get("device_id")?;
        let action: String = row.try_get("action")?;
        let payload: Value = row.try_get("request_payload")?;
        let pump_id: Option<String> = row.try_get("pump_id")?;
        let requested_state: Option<bool> = row.try_get("requested_state")?;
        let requested_pwm: Option<i32> = row.try_get("requested_pwm")?;
        let attempt: i32 = row.try_get("attempt_count")?;
        let mqtt_action = match action.as_str() {
            "on" => {
                if requested_pwm.is_some() {
                    "set_pwm"
                } else {
                    "pump_on"
                }
            }
            "off" => "pump_off",
            "reset_fault" => "reset_fault",
            "set_pwm" => "set_pwm",
            "force_on" => "force_on",
            "emergency_stop" => "emergency_stop",
            _ => {
                schedule_publish_retry_with_state(
                    app_state,
                    &id,
                    "unsupported durable command action",
                )
                .await?;
                continue;
            }
        };
        let command = hydragrow_shared::MqttCommandOut {
            target: "all".to_string(),
            action: mqtt_action.to_string(),
            params: Some(hydragrow_shared::MqttCommandParams {
                pump_id,
                duration_sec: payload.get("duration_sec").and_then(Value::as_u64),
                pwm: requested_pwm.map(|v| v as u32),
                state: payload
                    .get("state")
                    .and_then(Value::as_bool)
                    .or(requested_state),
                ota_url: None,
                candidates: None,
                ota_provision: None,
            }),
            ts: None,
            nonce: None,
            signature: None,
            metadata: Some(hydragrow_shared::CommandMetadata {
                command_id: Some(id.clone()),
            }),
        };
        match crate::api::mqtt_utils::publish_command(app_state, &device_id, &command).await {
            Ok(()) => {
                crate::metrics::MQTT_DELIVERY_TOTAL
                    .with_label_values(["command", "success"].as_ref())
                    .inc();
                transition_with_state(
                    app_state,
                    &id,
                    &device_id,
                    CommandLifecycle::Sent,
                    None,
                    "retry",
                    json!({"attempt_no":attempt}),
                )
                .await?;
            }
            Err(error) => {
                crate::metrics::MQTT_DELIVERY_TOTAL
                    .with_label_values(["command", "failure"].as_ref())
                    .inc();
                crate::metrics::MQTT_DELIVERY_FAILURES_TOTAL
                    .with_label_values(["command", "publish"].as_ref())
                    .inc();
                schedule_publish_retry_with_state(app_state, &id, &error.to_string()).await?;
            }
        }
        count += 1;
    }
    Ok(count)
}

struct ReconciliationWorkerGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Drop for ReconciliationWorkerGuard {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

pub fn spawn_recovery(
    app_state: std::sync::Arc<AppState>,
    readiness: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    tokio::spawn(async move {
        readiness.store(true, std::sync::atomic::Ordering::Relaxed);
        let _worker_guard = ReconciliationWorkerGuard(readiness);
        if let Err(error) = reconcile_with_state(&app_state).await {
            tracing::error!(?error, "Durable command recovery failed");
        }
        loop {
            if let Err(error) = reconcile_with_state(&app_state).await {
                tracing::error!(?error, "Durable command reconciliation failed");
            }
            if let Err(error) = retry_due(&app_state).await {
                tracing::error!(?error, "Durable command retry failed");
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciliation_worker_guard_clears_readiness_when_worker_exits() {
        let readiness = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        {
            let _guard = ReconciliationWorkerGuard(readiness.clone());
            assert!(readiness.load(std::sync::atomic::Ordering::Relaxed));
        }
        assert!(!readiness.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn safe_payload_keeps_only_retryable_command_fields() {
        let payload = json!({
            "action": "on",
            "pump_id": "A",
            "duration_sec": 10,
            "pwm": 80,
            "state": true,
            "api_key": "secret",
            "firebase_token": "secret",
            "X-Privileged-Token": "secret"
        });
        let safe = safe_payload(&payload);
        assert_eq!(safe.get("action").and_then(Value::as_str), Some("on"));
        assert!(safe.get("api_key").is_none());
        assert!(safe.get("firebase_token").is_none());
        assert!(safe.get("X-Privileged-Token").is_none());
    }

    #[test]
    fn lifecycle_name_round_trips_all_states() {
        for lifecycle in [
            CommandLifecycle::Requested,
            CommandLifecycle::Sent,
            CommandLifecycle::Acknowledged,
            CommandLifecycle::Confirmed,
            CommandLifecycle::Rejected,
            CommandLifecycle::Failed,
            CommandLifecycle::Timeout,
            CommandLifecycle::Unknown,
        ] {
            assert_eq!(
                parse_lifecycle(lifecycle_name(lifecycle)).unwrap(),
                lifecycle
            );
        }
    }

    #[test]
    fn retry_delay_is_bounded() {
        let delay = (RETRY_BASE_SECS * 2_i64.pow(20)).min(RETRY_MAX_SECS);
        assert_eq!(delay, RETRY_MAX_SECS);
    }

    fn new_test_command(key: Option<&str>) -> NewCommand {
        NewCommand {
            idempotency_key: key.map(str::to_string),
            principal_kind: "user".to_string(),
            principal_id: Some("test-user".to_string()),
            service_key_label: None,
            session_id: Some("test-session".to_string()),
            user_id: None,
            device_id: format!("p1-2-test-{}", Uuid::new_v4()),
            action: "on".to_string(),
            request_payload: json!({"action":"on","pump_id":"A","state":true}),
            requested_state: Some(true),
            requested_pwm: None,
            pump_id: Some("A".to_string()),
            retry_safe: true,
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn durable_create_persists_requested_and_initial_event(pool: PgPool) {
        let command = new_test_command(None);
        let id = command.device_id.clone();
        let (created, existing) = create_command(&pool, command).await.unwrap();
        assert!(!existing);
        assert_eq!(created.lifecycle, CommandLifecycle::Requested);
        let events = list_lifecycle_events(&pool, &created.command_id)
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["lifecycle"], "REQUESTED");
        assert_eq!(events[0]["device_id"], id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn idempotent_create_returns_same_command_and_conflict_is_rejected(pool: PgPool) {
        let command = new_test_command(Some("same-key"));
        let (first, first_existing) = create_command(&pool, command.clone()).await.unwrap();
        let (second, second_existing) = create_command(&pool, command.clone()).await.unwrap();
        assert!(!first_existing);
        assert!(second_existing);
        assert_eq!(first.command_id, second.command_id);

        let mut conflict = command;
        conflict.requested_state = Some(false);
        conflict.request_payload = json!({"action":"on","pump_id":"A","state":false});
        let error = create_command(&pool, conflict).await.unwrap_err();
        assert!(error.to_string().contains("idempotency key conflicts"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn transition_is_durable_and_terminal_is_immutable(pool: PgPool) {
        let (created, _) = create_command(&pool, new_test_command(None)).await.unwrap();
        assert_eq!(
            transition(
                &pool,
                &created.command_id,
                CommandLifecycle::Sent,
                None,
                "publish",
                json!({})
            )
            .await
            .unwrap(),
            TransitionResult::Applied
        );
        assert_eq!(
            transition(
                &pool,
                &created.command_id,
                CommandLifecycle::Acknowledged,
                None,
                "controller",
                json!({})
            )
            .await
            .unwrap(),
            TransitionResult::Applied
        );
        assert_eq!(
            transition(
                &pool,
                &created.command_id,
                CommandLifecycle::Confirmed,
                None,
                "runtime",
                json!({})
            )
            .await
            .unwrap(),
            TransitionResult::Applied
        );
        assert_eq!(
            transition(
                &pool,
                &created.command_id,
                CommandLifecycle::Confirmed,
                None,
                "runtime",
                json!({})
            )
            .await
            .unwrap(),
            TransitionResult::Duplicate
        );
        assert!(
            transition(
                &pool,
                &created.command_id,
                CommandLifecycle::Failed,
                None,
                "test",
                json!({})
            )
            .await
            .is_err()
        );
        let events = list_lifecycle_events(&pool, &created.command_id)
            .await
            .unwrap();
        assert_eq!(events.len(), 4);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn unknown_command_lifecycle_event_does_not_create_record(pool: PgPool) {
        let result = transition(
            &pool,
            "does-not-exist",
            CommandLifecycle::Acknowledged,
            None,
            "controller",
            json!({}),
        )
        .await;
        assert!(result.is_err());
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM commands")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn concurrent_same_transition_has_one_effective_writer(pool: PgPool) {
        let (created, _) = create_command(&pool, new_test_command(None)).await.unwrap();
        let a = transition(
            &pool,
            &created.command_id,
            CommandLifecycle::Sent,
            None,
            "worker-a",
            json!({}),
        );
        let b = transition(
            &pool,
            &created.command_id,
            CommandLifecycle::Sent,
            None,
            "worker-b",
            json!({}),
        );
        let (a, b) = tokio::join!(a, b);
        let results = [&a, &b];
        let applied = results
            .iter()
            .filter(|r| matches!(r, Ok(TransitionResult::Applied)))
            .count();
        let duplicate = results
            .iter()
            .filter(|r| matches!(r, Ok(TransitionResult::Duplicate)))
            .count();
        assert_eq!(applied, 1);
        assert_eq!(duplicate, 1);
        assert_eq!(
            list_lifecycle_events(&pool, &created.command_id)
                .await
                .unwrap()
                .len(),
            2
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn publish_failure_persists_bounded_retry_state(pool: PgPool) {
        let (created, _) = create_command(&pool, new_test_command(None)).await.unwrap();
        assert_eq!(
            mark_publish_attempt(&pool, &created.command_id)
                .await
                .unwrap(),
            1
        );
        schedule_publish_retry(&pool, &created.command_id, "broker unavailable")
            .await
            .unwrap();
        let current = get_command(&pool, &created.command_id).await.unwrap();
        assert_eq!(current.lifecycle, CommandLifecycle::Requested);
        assert!(current.next_retry_at.is_some());
        assert_eq!(current.last_error.as_deref(), Some("broker unavailable"));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reconcile_times_out_sent_and_acknowledged_commands(pool: PgPool) {
        let (sent, _) = create_command(&pool, new_test_command(None)).await.unwrap();
        transition(
            &pool,
            &sent.command_id,
            CommandLifecycle::Sent,
            None,
            "publish",
            json!({}),
        )
        .await
        .unwrap();
        sqlx::query("UPDATE commands SET sent_at=NOW()-INTERVAL '31 seconds' WHERE command_id=$1")
            .bind(&sent.command_id)
            .execute(&pool)
            .await
            .unwrap();

        let (ack, _) = create_command(&pool, new_test_command(None)).await.unwrap();
        transition(
            &pool,
            &ack.command_id,
            CommandLifecycle::Sent,
            None,
            "publish",
            json!({}),
        )
        .await
        .unwrap();
        transition(
            &pool,
            &ack.command_id,
            CommandLifecycle::Acknowledged,
            None,
            "controller",
            json!({}),
        )
        .await
        .unwrap();
        sqlx::query(
            "UPDATE commands SET acknowledged_at=NOW()-INTERVAL '91 seconds' WHERE command_id=$1",
        )
        .bind(&ack.command_id)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(reconcile(&pool).await.unwrap(), 2);
        assert_eq!(
            get_command(&pool, &sent.command_id)
                .await
                .unwrap()
                .lifecycle,
            CommandLifecycle::Timeout
        );
        assert_eq!(
            get_command(&pool, &ack.command_id).await.unwrap().lifecycle,
            CommandLifecycle::Timeout
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reconcile_marks_ambiguous_publish_and_exhausted_retry_terminal(pool: PgPool) {
        let (ambiguous, _) = create_command(&pool, new_test_command(None)).await.unwrap();
        sqlx::query("UPDATE commands SET attempt_count=1,last_error=NULL WHERE command_id=$1")
            .bind(&ambiguous.command_id)
            .execute(&pool)
            .await
            .unwrap();

        let (exhausted, _) = create_command(&pool, new_test_command(None)).await.unwrap();
        sqlx::query("UPDATE commands SET attempt_count=$2,last_error='broker unavailable' WHERE command_id=$1")
            .bind(&exhausted.command_id)
            .bind(MAX_ATTEMPTS)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(reconcile(&pool).await.unwrap(), 2);
        assert_eq!(
            get_command(&pool, &ambiguous.command_id)
                .await
                .unwrap()
                .lifecycle,
            CommandLifecycle::Unknown
        );
        assert_eq!(
            get_command(&pool, &exhausted.command_id)
                .await
                .unwrap()
                .lifecycle,
            CommandLifecycle::Failed
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn reconcile_does_not_reopen_terminal_commands(pool: PgPool) {
        let (created, _) = create_command(&pool, new_test_command(None)).await.unwrap();
        transition(
            &pool,
            &created.command_id,
            CommandLifecycle::Sent,
            None,
            "publish",
            json!({}),
        )
        .await
        .unwrap();
        transition(
            &pool,
            &created.command_id,
            CommandLifecycle::Acknowledged,
            None,
            "controller",
            json!({}),
        )
        .await
        .unwrap();
        transition(
            &pool,
            &created.command_id,
            CommandLifecycle::Confirmed,
            None,
            "runtime",
            json!({}),
        )
        .await
        .unwrap();
        assert_eq!(reconcile(&pool).await.unwrap(), 0);
        assert_eq!(
            get_command(&pool, &created.command_id)
                .await
                .unwrap()
                .lifecycle,
            CommandLifecycle::Confirmed
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn retry_safety_is_durable_and_survives_reload(pool: PgPool) {
        let mut command = new_test_command(None);
        command.retry_safe = false;
        let (created, _) = create_command(&pool, command).await.unwrap();
        let reloaded = get_command(&pool, &created.command_id).await.unwrap();
        assert!(!reloaded.retry_safe);
        mark_publish_attempt(&pool, &created.command_id)
            .await
            .unwrap();
        schedule_publish_retry(&pool, &created.command_id, "broker unavailable")
            .await
            .unwrap();
        assert_eq!(
            get_command(&pool, &created.command_id)
                .await
                .unwrap()
                .lifecycle,
            CommandLifecycle::Unknown
        );
    }

    #[test]
    fn normal_commands_are_retry_safe_by_default_in_test_fixture() {
        assert!(new_test_command(None).retry_safe);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn dangerous_publish_failure_becomes_unknown_without_retry(pool: PgPool) {
        let mut command = new_test_command(None);
        command.action = "emergency_stop".to_string();
        command.request_payload = json!({"action":"emergency_stop","state":true});
        command.retry_safe = false;
        let (created, _) = create_command(&pool, command).await.unwrap();
        mark_publish_attempt(&pool, &created.command_id)
            .await
            .unwrap();
        schedule_publish_retry(&pool, &created.command_id, "broker unavailable")
            .await
            .unwrap();
        let current = get_command(&pool, &created.command_id).await.unwrap();
        assert_eq!(current.lifecycle, CommandLifecycle::Unknown);
        assert!(current.next_retry_at.is_none());
    }
}
