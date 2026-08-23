use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct DeviceOwnershipRecord {
    pub id: i64,
    pub user_id: i64,
    pub device_id: String,
    pub label: Option<String>,
}

/// Gán thiết bị cho user (upsert: cập nhật label nếu đã tồn tại).
pub async fn claim_device(
    pool: &PgPool,
    user_id: i64,
    device_id: &str,
    label: Option<&str>,
) -> Result<DeviceOwnershipRecord, sqlx::Error> {
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
    .await
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
