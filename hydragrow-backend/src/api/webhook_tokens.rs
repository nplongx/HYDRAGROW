use crate::AppState;
use actix_web::{HttpResponse, Scope, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct WebhookToken {
    pub id: Uuid,
    pub device_id: String,
    pub label: String,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub label: String,
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

pub async fn find_by_token_hash(pool: &PgPool, hash: &str) -> Option<WebhookToken> {
    sqlx::query_as::<_, WebhookToken>(
        "SELECT id, device_id, label, token_hash, created_at, last_used_at, is_active FROM webhook_tokens WHERE token_hash = $1",
    ).bind(hash)
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
}

pub async fn create_token(
    pool: &PgPool,
    device_id: &str,
    label: &str,
) -> Result<(WebhookToken, String), sqlx::Error> {
    let raw_token = format!("wh_{}", Uuid::new_v4().simple());
    let token_hash = sha256_hex(&raw_token);

    let token = sqlx::query_as::<_, WebhookToken>(
        "INSERT INTO webhook_tokens (device_id, label, token_hash) VALUES ($1, $2, $3) RETURNING id, device_id, label, token_hash, created_at, last_used_at, is_active",
    ).bind(device_id).bind(label).bind(token_hash)
    .fetch_one(pool)
    .await?;

    Ok((token, raw_token))
}

pub async fn revoke_token(pool: &PgPool, id: Uuid, device_id: &str) -> Result<bool, sqlx::Error> {
    let result: sqlx::postgres::PgQueryResult =
        sqlx::query("UPDATE webhook_tokens SET is_active = false WHERE id = $1 AND device_id = $2")
            .bind(id)
            .bind(device_id)
            .execute(pool)
            .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn list_tokens(pool: &PgPool, device_id: &str) -> Result<Vec<WebhookToken>, sqlx::Error> {
    sqlx::query_as::<_, WebhookToken>(
        "SELECT id, device_id, label, token_hash, created_at, last_used_at, is_active FROM webhook_tokens WHERE device_id = $1 ORDER BY created_at DESC",
    ).bind(device_id)
    .fetch_all(pool)
    .await
}

async fn handle_list_tokens(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let device_id = path.into_inner();
    match list_tokens(&app_state.pg_pool, &device_id).await {
        Ok(tokens) => HttpResponse::Ok().json(tokens),
        Err(e) => {
            tracing::error!("Failed to list tokens: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Internal server error" }))
        }
    }
}

async fn handle_create_token(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<CreateTokenRequest>,
) -> HttpResponse {
    let device_id = path.into_inner();
    match create_token(&app_state.pg_pool, &device_id, &req.label).await {
        Ok((token, raw_token)) => HttpResponse::Ok().json(serde_json::json!({
            "token": token,
            "raw_token": raw_token
        })),
        Err(e) => {
            tracing::error!("Failed to create token: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Internal server error" }))
        }
    }
}

async fn handle_revoke_token(
    app_state: web::Data<AppState>,
    path: web::Path<(String, Uuid)>,
) -> HttpResponse {
    let (device_id, token_id) = path.into_inner();
    match revoke_token(&app_state.pg_pool, token_id, &device_id).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({ "success": true })),
        Ok(false) => {
            HttpResponse::NotFound().json(serde_json::json!({ "error": "Token not found" }))
        }
        Err(e) => {
            tracing::error!("Failed to revoke token: {}", e);
            HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Internal server error" }))
        }
    }
}

pub fn routes() -> Scope {
    web::scope("/devices/{device_id}/webhook-tokens")
        .route("", web::get().to(handle_list_tokens))
        .route("", web::post().to(handle_create_token))
        .route("/{token_id}", web::delete().to(handle_revoke_token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_is_deterministic() {
        assert_eq!(sha256_hex("hello"), sha256_hex("hello"));
        assert_ne!(sha256_hex("hello"), sha256_hex("world"));
    }
}
