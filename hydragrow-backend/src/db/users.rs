// hydragrow-backend/src/db/users.rs
//! Tài khoản đăng nhập được cấp sẵn: ánh xạ Firebase UID -> scope nội bộ.

use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow, serde::Serialize)]
pub struct UserRecord {
    pub id: i64,
    pub firebase_uid: String,
    pub email: String,
    pub display_name: Option<String>,
    /// 'admin' | 'operator' | 'viewer' — mapped từ scopes khi tạo.
    #[sqlx(default)]
    pub role: Option<String>,
    #[sqlx(default)]
    pub preferences: Option<serde_json::Value>,
    pub scopes: Vec<String>,
    pub is_active: bool,
}

impl UserRecord {
    pub fn role_or_viewer(&self) -> &str {
        self.role.as_deref().unwrap_or("viewer")
    }
}

/// Tìm user đang hoạt động theo Firebase UID (dùng bởi middleware xác thực mỗi request).
pub async fn find_active_by_firebase_uid(
    pool: &PgPool,
    firebase_uid: &str,
) -> Result<Option<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, firebase_uid, email, display_name, role, preferences, scopes, is_active
        FROM users
        WHERE firebase_uid = $1 AND is_active = TRUE
        "#,
    )
    .bind(firebase_uid)
    .fetch_optional(pool)
    .await
}

/// Danh sách mọi user (không lọc theo is_active) — dùng bởi GET /api/admin/users.
pub async fn list_users(pool: &PgPool) -> Result<Vec<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, firebase_uid, email, display_name, role, preferences, scopes, is_active
        FROM users
        ORDER BY created_at, id
        "#,
    )
    .fetch_all(pool)
    .await
}

/// Cập nhật role / scopes / is_active cho 1 user; trả về row sau khi update
/// (None nếu không tồn tại). Chỉ field được truyền Some() là được đổi.
pub async fn update_user(
    pool: &PgPool,
    id: i64,
    role: Option<&str>,
    scopes: Option<&[String]>,
    is_active: Option<bool>,
) -> Result<Option<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        UPDATE users SET
            role = COALESCE($2, role),
            scopes = COALESCE($3, scopes),
            is_active = COALESCE($4, is_active),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $1
        RETURNING id, firebase_uid, email, display_name, role, preferences, scopes, is_active
        "#,
    )
    .bind(id)
    .bind(role)
    .bind(scopes)
    .bind(is_active)
    .fetch_optional(pool)
    .await
}

/// Cập nhật preferences JSONB cho chính user đang đăng nhập.
pub async fn update_user_preferences(
    pool: &PgPool,
    firebase_uid: &str,
    preferences: &serde_json::Value,
) -> Result<Option<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        UPDATE users SET
            preferences = $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE firebase_uid = $1
        RETURNING id, firebase_uid, email, display_name, role, preferences, scopes, is_active
        "#,
    )
    .bind(firebase_uid)
    .bind(preferences)
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
        RETURNING id, firebase_uid, email, display_name, role, preferences, scopes, is_active
        "#,
    )
    .bind(firebase_uid)
    .bind(email)
    .bind(display_name)
    .bind(scopes)
    .fetch_one(pool)
    .await
}

/// Tự cấp tài khoản lần đầu cho user Firebase mới (self-registration).
/// CHỈ tạo khi chưa tồn tại; không bao giờ ghi đè scope/display_name của
/// tài khoản đã có (khác `upsert_user` dành cho provisioning của admin).
/// Scope mặc định chỉ gồm `read:telemetry`.
pub async fn provision_default_user(
    pool: &PgPool,
    firebase_uid: &str,
    email: &str,
) -> Result<UserRecord, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        INSERT INTO users (firebase_uid, email, display_name, scopes)
        VALUES ($1, $2, NULL, $3)
        ON CONFLICT (firebase_uid) DO UPDATE SET firebase_uid = EXCLUDED.firebase_uid
        RETURNING id, firebase_uid, email, display_name, role, preferences, scopes, is_active
        "#,
    )
    .bind(firebase_uid)
    .bind(email)
    .bind(vec!["read:telemetry".to_string()])
    .fetch_one(pool)
    .await
}
