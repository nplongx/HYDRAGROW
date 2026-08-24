use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng as ArgonOsRng};
use argon2::Argon2;
use base64::Engine;
use rand::RngCore;
use rand::rngs::OsRng;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct DeviceOwnershipRecord {
    pub id: i64,
    pub user_id: i64,
    pub device_id: String,
    pub label: Option<String>,
}

/// Returned only once, right after claim — the caller must show this to the
/// user immediately and never store the plaintext password server-side.
pub struct ClaimedMqttCredentials {
    pub mqtt_username: String,
    pub mqtt_password: String,
}

fn generate_mqtt_credentials(device_id: &str) -> (String, String) {
    let mqtt_username = format!("device_{}", device_id);
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let mqtt_password = base64::engine::general_purpose::STANDARD.encode(bytes);
    (mqtt_username, mqtt_password)
}

fn hash_mqtt_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut ArgonOsRng);
    let argon2 = Argon2::default();
    Ok(argon2.hash_password(password.as_bytes(), &salt)?.to_string())
}

/// Gán thiết bị cho user (upsert: cập nhật label nếu đã tồn tại).
/// Sinh credential MQTT riêng cho thiết bị nếu chưa có (không ghi đè nếu đã claim trước đó).
pub async fn claim_device(
    pool: &PgPool,
    user_id: i64,
    device_id: &str,
    label: Option<&str>,
) -> Result<(DeviceOwnershipRecord, Option<ClaimedMqttCredentials>), sqlx::Error> {
    let existing_username: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT mqtt_username FROM device_ownership WHERE device_id = $1 LIMIT 1",
    )
    .bind(device_id)
    .fetch_optional(pool)
    .await?;

    let already_has_credentials = existing_username
        .map(|(u,)| u.is_some())
        .unwrap_or(false);

    let new_credentials = if already_has_credentials {
        None
    } else {
        let (mqtt_username, mqtt_password) = generate_mqtt_credentials(device_id);
        let password_hash = hash_mqtt_password(&mqtt_password)
            .expect("argon2 hashing should not fail for a freshly generated password");
        Some((mqtt_username, mqtt_password, password_hash))
    };

    let record = if let Some((ref mqtt_username, _, ref password_hash)) = new_credentials {
        sqlx::query_as::<_, DeviceOwnershipRecord>(
            r#"
            INSERT INTO device_ownership (user_id, device_id, label, mqtt_username, mqtt_password_hash)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id, device_id) DO UPDATE SET label = EXCLUDED.label
            RETURNING id, user_id, device_id, label
            "#,
        )
        .bind(user_id)
        .bind(device_id)
        .bind(label)
        .bind(mqtt_username)
        .bind(password_hash)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_as::<_, DeviceOwnershipRecord>(
            r#"
            INSERT INTO device_ownership (user_id, device_id, label)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, device_id) DO UPDATE SET label = EXCLUDED.label
            RETURNING id, user_id, device_id, label
            "#,
        )
        .bind(user_id)
        .bind(device_id)
        .bind(label)
        .fetch_one(pool)
        .await?
    };

    let returned_credentials = new_credentials.map(|(mqtt_username, mqtt_password, _)| {
        ClaimedMqttCredentials {
            mqtt_username,
            mqtt_password,
        }
    });

    Ok((record, returned_credentials))
}

/// Xoá liên kết thiết bị - user.
pub async fn unclaim_device(
    pool: &PgPool,
    user_id: i64,
    device_id: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM device_ownership WHERE user_id = $1 AND device_id = $2",
    )
    .bind(user_id)
    .bind(device_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Liệt kê tất cả thiết bị của user.
pub async fn list_devices_for_user(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<DeviceOwnershipRecord>, sqlx::Error> {
    sqlx::query_as::<_, DeviceOwnershipRecord>(
        "SELECT id, user_id, device_id, label FROM device_ownership WHERE user_id = $1 ORDER BY claimed_at",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Kiểm tra user có sở hữu device_id không.
pub async fn is_owner(
    pool: &PgPool,
    user_id: i64,
    device_id: &str,
) -> Result<bool, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM device_ownership WHERE user_id = $1 AND device_id = $2",
    )
    .bind(user_id)
    .bind(device_id)
    .fetch_one(pool)
    .await?;
    Ok(count.0 > 0)
}
