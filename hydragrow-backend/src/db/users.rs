// hydragrow-backend/src/db/users.rs
//! Tài khoản đăng nhập được cấp sẵn: ánh xạ Firebase UID -> scope nội bộ.

use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct UserRecord {
    pub id: i64,
    pub firebase_uid: String,
    pub email: String,
    pub display_name: Option<String>,
    pub scopes: Vec<String>,
    pub is_active: bool,
}

/// Tìm user đang hoạt động theo Firebase UID (dùng bởi middleware xác thực mỗi request).
pub async fn find_active_by_firebase_uid(
    pool: &PgPool,
    firebase_uid: &str,
) -> Result<Option<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, firebase_uid, email, display_name, scopes, is_active
        FROM users
        WHERE firebase_uid = $1 AND is_active = TRUE
        "#,
    )
    .bind(firebase_uid)
    .fetch_optional(pool)
    .await
}

/// Tạo mới hoặc cập nhật scope cho một tài khoản (dùng bởi endpoint provisioning của admin).
pub async fn upsert_user(
    pool: &PgPool,
    firebase_uid: &str,
    email: &str,
    display_name: Option<&str>,
    scopes: &[String],
) -> Result<UserRecord, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        INSERT INTO users (firebase_uid, email, display_name, scopes)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (firebase_uid) DO UPDATE SET
            email = EXCLUDED.email,
            display_name = COALESCE(EXCLUDED.display_name, users.display_name),
            scopes = EXCLUDED.scopes,
            updated_at = CURRENT_TIMESTAMP
        RETURNING id, firebase_uid, email, display_name, scopes, is_active
        "#,
    )
    .bind(firebase_uid)
    .bind(email)
    .bind(display_name)
    .bind(scopes)
    .fetch_one(pool)
    .await
}
