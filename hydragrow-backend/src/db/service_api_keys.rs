// hydragrow-backend/src/db/service_api_keys.rs
//! Scoped machine credentials for fleet-wide internal services (not tied to device_id).
//!
//! Only the SHA-256 hash of the raw key is stored; the raw key (`svc_<uuid-simple>`)
//! is shown once at creation time.

use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct ServiceApiKey {
    pub id: Uuid,
    pub label: String,
    pub key_hash: String,
    pub scopes: Vec<String>,
    pub is_active: bool,
}

/// SHA-256 hex digest of the presented raw key.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// Create a new service API key. Returns the DB record plus the raw key (shown once).
pub async fn create_service_api_key(
    pool: &PgPool,
    label: &str,
    scopes: &[String],
) -> Result<(ServiceApiKey, String), sqlx::Error> {
    let raw_key = format!("svc_{}", Uuid::new_v4().simple());
    let key_hash = sha256_hex(&raw_key);
    let record = sqlx::query_as::<_, ServiceApiKey>(
        r#"
        INSERT INTO service_api_keys (label, key_hash, scopes)
        VALUES ($1, $2, $3)
        RETURNING id, label, key_hash, scopes, is_active
        "#,
    )
    .bind(label)
    .bind(&key_hash)
    .bind(scopes)
    .fetch_one(pool)
    .await?;
    Ok((record, raw_key))
}

/// Look up an active key by hash. Returns `None` when unknown, inactive, or on DB error.
/// Best-effort bump of `last_used_at` (failures ignored).
pub async fn find_active_by_key_hash(pool: &PgPool, key_hash: &str) -> Option<ServiceApiKey> {
    let record = sqlx::query_as::<_, ServiceApiKey>(
        r#"
        SELECT id, label, key_hash, scopes, is_active
        FROM service_api_keys
        WHERE key_hash = $1 AND is_active = TRUE
        "#,
    )
    .bind(key_hash)
    .fetch_optional(pool)
    .await
    .ok()??;

    let _ = sqlx::query("UPDATE service_api_keys SET last_used_at = NOW() WHERE id = $1")
        .bind(record.id)
        .execute(pool)
        .await;

    Some(record)
}

/// Deactivate a key by id.
pub async fn deactivate_service_api_key(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE service_api_keys SET is_active = FALSE WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
