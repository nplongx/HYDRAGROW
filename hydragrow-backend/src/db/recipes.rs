//! DB helpers for crop_recipes và crop_recipe_stages.

use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RecipeRow {
    pub id: String,
    pub name: String,
    pub crop: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RecipeStageRow {
    pub id: String,
    pub recipe_id: String,
    pub stage_order: i32,
    pub name: String,
    pub duration_days: i32,
    pub ec_target: f32,
    pub ec_tolerance: f32,
    pub ph_target: f32,
    pub ph_tolerance: f32,
    pub nutrient_a_ratio: f32,
    pub nutrient_b_ratio: f32,
    pub water_level_target: f32,
    pub misting_on_duration_ms: i32,
    pub misting_off_duration_ms: i32,
}

pub async fn list_recipes(pool: &PgPool) -> Result<Vec<RecipeRow>, sqlx::Error> {
    sqlx::query_as::<_, RecipeRow>(
        "SELECT id, name, crop, description, created_at FROM crop_recipes ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_recipe(pool: &PgPool, id: &str) -> Result<Option<RecipeRow>, sqlx::Error> {
    sqlx::query_as::<_, RecipeRow>(
        "SELECT id, name, crop, description, created_at FROM crop_recipes WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn insert_recipe(
    pool: &PgPool,
    id: &str,
    name: &str,
    crop: &str,
    description: Option<&str>,
) -> Result<RecipeRow, sqlx::Error> {
    sqlx::query_as::<_, RecipeRow>(
        r#"
        INSERT INTO crop_recipes (id, name, crop, description)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, crop, description, created_at
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(crop)
    .bind(description)
    .fetch_one(pool)
    .await
}

/// Context cần thiết để build `ScriptFsmInput` cho recipe_override scripts.
/// `stage_index` là 0-based (stage_order - 1) — giả định `stage_order` liên tục
/// bắt đầu từ 1 theo đúng convention "ORDER BY stage_order" của module-rules/backend.md.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ActiveStageContext {
    pub recipe_id: String,
    pub season_id: String,
    pub stage_index: i64,
    pub elapsed_sec: i64,
}

pub async fn get_active_stage_context(
    pool: &PgPool,
    device_id: &str,
) -> Result<Option<ActiveStageContext>, sqlx::Error> {
    sqlx::query_as::<_, ActiveStageContext>(
        r#"
        SELECT
            dar.recipe_id AS recipe_id,
            dar.season_id AS season_id,
            (s.stage_order - 1)::BIGINT AS stage_index,
            EXTRACT(EPOCH FROM (NOW() - dar.applied_at))::BIGINT AS elapsed_sec
        FROM device_active_recipes dar
        JOIN crop_recipe_stages s
            ON s.id = dar.current_stage_id AND s.recipe_id = dar.recipe_id
        WHERE dar.device_id = $1
        "#,
    )
    .bind(device_id)
    .fetch_optional(pool)
    .await
}

/// Áp dụng kết quả `StageOverride` từ recipe_override script: tìm stage có
/// `stage_order = target_stage_index + 1` trong recipe đang active của device, và
/// cập nhật `device_active_recipes.current_stage_id`. Trả `None` (không lỗi) nếu
/// index nằm ngoài phạm vi — script lỗi logic không được phép làm crash pipeline
/// FSM, chỉ log cảnh báo ở call site (xem Task 3).
pub async fn advance_active_recipe_stage(
    pool: &PgPool,
    device_id: &str,
    target_stage_index: i64,
) -> Result<Option<RecipeStageRow>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let recipe_id: Option<String> =
        sqlx::query_scalar("SELECT recipe_id FROM device_active_recipes WHERE device_id = $1")
            .bind(device_id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some(recipe_id) = recipe_id else {
        return Ok(None);
    };

    let target_stage = sqlx::query_as::<_, RecipeStageRow>(
        r#"
        SELECT id, recipe_id, stage_order, name, duration_days, ec_target, ec_tolerance,
               ph_target, ph_tolerance, nutrient_a_ratio, nutrient_b_ratio,
               water_level_target, misting_on_duration_ms, misting_off_duration_ms
        FROM crop_recipe_stages
        WHERE recipe_id = $1 AND stage_order = $2
        "#,
    )
    .bind(&recipe_id)
    .bind((target_stage_index + 1) as i32)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(stage) = target_stage else {
        return Ok(None);
    };

    sqlx::query("UPDATE device_active_recipes SET current_stage_id = $1 WHERE device_id = $2")
        .bind(&stage.id)
        .bind(device_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Some(stage))
}

pub async fn delete_recipe(pool: &PgPool, id: &str) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query("DELETE FROM crop_recipes WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected())
}

pub async fn list_stages_for_recipe(
    pool: &PgPool,
    recipe_id: &str,
) -> Result<Vec<RecipeStageRow>, sqlx::Error> {
    sqlx::query_as::<_, RecipeStageRow>(
        r#"
        SELECT id, recipe_id, stage_order, name, duration_days,
               ec_target, ec_tolerance, ph_target, ph_tolerance,
               nutrient_a_ratio, nutrient_b_ratio, water_level_target,
               misting_on_duration_ms, misting_off_duration_ms
        FROM crop_recipe_stages
        WHERE recipe_id = $1
        ORDER BY stage_order
        "#,
    )
    .bind(recipe_id)
    .fetch_all(pool)
    .await
}

pub async fn insert_stage(
    pool: &PgPool,
    stage: &RecipeStageRow,
) -> Result<RecipeStageRow, sqlx::Error> {
    sqlx::query_as::<_, RecipeStageRow>(
        r#"
        INSERT INTO crop_recipe_stages (
            id, recipe_id, stage_order, name, duration_days,
            ec_target, ec_tolerance, ph_target, ph_tolerance,
            nutrient_a_ratio, nutrient_b_ratio, water_level_target,
            misting_on_duration_ms, misting_off_duration_ms
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
        RETURNING id, recipe_id, stage_order, name, duration_days,
                  ec_target, ec_tolerance, ph_target, ph_tolerance,
                  nutrient_a_ratio, nutrient_b_ratio, water_level_target,
                  misting_on_duration_ms, misting_off_duration_ms
        "#,
    )
    .bind(&stage.id)
    .bind(&stage.recipe_id)
    .bind(stage.stage_order)
    .bind(&stage.name)
    .bind(stage.duration_days)
    .bind(stage.ec_target)
    .bind(stage.ec_tolerance)
    .bind(stage.ph_target)
    .bind(stage.ph_tolerance)
    .bind(stage.nutrient_a_ratio)
    .bind(stage.nutrient_b_ratio)
    .bind(stage.water_level_target)
    .bind(stage.misting_on_duration_ms)
    .bind(stage.misting_off_duration_ms)
    .fetch_one(pool)
    .await
}
