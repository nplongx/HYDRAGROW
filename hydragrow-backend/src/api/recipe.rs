use actix_web::{HttpResponse, Responder, web};
use chrono::Utc;
use hydragrow_shared::{
    recipe::{CropRecipe, CropStage},
    topics::topic_recipe_set,
};

use rumqttc::QoS;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, Row};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{AppState, api::mqtt_utils::sign_command, db::postgres, db::recipes as db_recipes};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecipeTemplate {
    pub id: String,
    pub name: String,
    pub crop: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    #[sqlx(skip)]
    pub stages: Vec<CropStage>,
}

impl From<db_recipes::RecipeRow> for RecipeTemplate {
    fn from(r: db_recipes::RecipeRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            crop: r.crop,
            description: r.description,
            created_at: r.created_at,
            stages: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRecipeStatus {
    pub device_id: String,
    pub active_recipe: Option<CropRecipe>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRecipeRequest {
    pub name: String,
    pub crop: String,
    pub description: Option<String>,
    pub stages: Vec<CropStage>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyRecipeRequest {
    pub recipe_id: Option<String>,
    pub recipe: Option<CreateRecipeRequest>,
}

#[derive(Debug, Serialize)]
struct RecipeMqttPayload {
    action: &'static str,
    recipe: CropRecipe,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ts: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    nonce: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
}

#[derive(Debug, Serialize)]
struct ClearRecipeMqttPayload {
    action: &'static str,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ts: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    nonce: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
}

fn map_row_to_crop_stage<R: Row>(r: &R) -> CropStage
where
    usize: sqlx::ColumnIndex<R>,
    for<'c> i32: sqlx::Decode<'c, R::Database> + sqlx::Type<R::Database>,
    for<'c> String: sqlx::Decode<'c, R::Database> + sqlx::Type<R::Database>,
    for<'c> f32: sqlx::Decode<'c, R::Database> + sqlx::Type<R::Database>,
    for<'c> &'c str: sqlx::ColumnIndex<R>,
{
    let duration_days: i32 = r.try_get("duration_days").unwrap_or(7);

    CropStage {
        name: r.try_get("name").unwrap_or_default(),
        duration_sec: (duration_days.max(1) as u64) * 86_400,
        ec_target: r.try_get("ec_target").unwrap_or(1.4),
        ec_tolerance: r.try_get("ec_tolerance").unwrap_or(0.1),
        ph_target: r.try_get("ph_target").unwrap_or(6.0),
        ph_tolerance: r.try_get("ph_tolerance").unwrap_or(0.2),
        nutrient_a_ratio: r.try_get("nutrient_a_ratio").unwrap_or(1.0),
        nutrient_b_ratio: r.try_get("nutrient_b_ratio").unwrap_or(1.0),
        water_level_target: r.try_get("water_level_target").unwrap_or(20.0),

        water_change_interval_days: r
            .try_get::<Option<i32>, _>("water_change_interval_days")
            .ok()
            .flatten()
            .map(|v| v.max(0) as u32),

        water_change_drain_cm: r
            .try_get::<Option<f32>, _>("water_change_drain_cm")
            .ok()
            .flatten(),

        auto_dilute_ec_trigger: r
            .try_get::<Option<f32>, _>("auto_dilute_ec_trigger")
            .ok()
            .flatten(),

        misting_on_duration_ms: r.try_get("misting_on_duration_ms").unwrap_or(10_000),

        misting_off_duration_ms: r.try_get("misting_off_duration_ms").unwrap_or(180_000),

        max_dose_per_cycle_ml: r
            .try_get::<Option<f32>, _>("max_dose_per_cycle_ml")
            .ok()
            .flatten(),
    }
}

async fn fetch_stages_for_recipe(pool: &sqlx::PgPool, recipe_id: &str) -> Vec<CropStage> {
    let rows = db_recipes::list_stages_for_recipe(pool, recipe_id)
        .await
        .unwrap_or_default();

    rows.into_iter()
        .map(|r| CropStage {
            name: r.name,
            duration_sec: (r.duration_days.max(1) as u64) * 86_400,
            ec_target: r.ec_target,
            ec_tolerance: r.ec_tolerance,
            ph_target: r.ph_target,
            ph_tolerance: r.ph_tolerance,
            nutrient_a_ratio: r.nutrient_a_ratio,
            nutrient_b_ratio: r.nutrient_b_ratio,
            water_level_target: r.water_level_target,
            water_change_interval_days: None,
            water_change_drain_cm: None,
            auto_dilute_ec_trigger: None,
            misting_on_duration_ms: r.misting_on_duration_ms,
            misting_off_duration_ms: r.misting_off_duration_ms,
            max_dose_per_cycle_ml: None,
        })
        .collect()
}

// CẬP NHẬT RECIPE TEMPLATE TRONG CSDL
pub async fn update_recipe(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    req: web::Json<CreateRecipeRequest>,
) -> impl Responder {
    let recipe_id = path.into_inner();

    if req.name.trim().is_empty() || req.crop.trim().is_empty() || req.stages.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "invalid_recipe",
            "message": "name, crop và stages là bắt buộc"
        }));
    }

    let mut tx = match app_state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Lỗi mở transaction: {:?}", e);

            return HttpResponse::InternalServerError().json(json!({"error": "db_error"}));
        }
    };

    // 1. Cập nhật thông tin chung của Recipe
    if let Err(e) = sqlx::query(
        r#"
        UPDATE crop_recipes
        SET name = $1,
            crop = $2,
            description = $3
        WHERE id = $4
        "#,
    )
    .bind(req.name.trim())
    .bind(req.crop.trim())
    .bind(&req.description)
    .bind(&recipe_id)
    .execute(&mut *tx)
    .await
    {
        return HttpResponse::InternalServerError().json(json!({
            "error": "db_update_failed",
            "details": e.to_string()
        }));
    }

    // 2. Xóa sạch các Stages cũ của Recipe này
    if let Err(e) = sqlx::query("DELETE FROM crop_recipe_stages WHERE recipe_id = $1")
        .bind(&recipe_id)
        .execute(&mut *tx)
        .await
    {
        return HttpResponse::InternalServerError().json(json!({
            "error": "db_delete_stage_failed",
            "details": e.to_string()
        }));
    }

    // 3. Chèn lại các Stages mới bằng một query
    if !req.stages.is_empty() {
        let mut query_builder = sqlx::QueryBuilder::new(
            r#"
            INSERT INTO crop_recipe_stages (
                id,
                recipe_id,
                stage_order,
                name,
                duration_days,
                ec_target,
                ec_tolerance,
                ph_target,
                ph_tolerance,
                nutrient_a_ratio,
                nutrient_b_ratio,
                water_level_target,
                water_change_interval_days,
                water_change_drain_cm,
                auto_dilute_ec_trigger,
                misting_on_duration_ms,
                misting_off_duration_ms,
                max_dose_per_cycle_ml
            )
            "#,
        );

        query_builder.push_values(req.stages.iter().enumerate(), |mut b, (idx, stage)| {
            let stage_id = Uuid::new_v4().to_string();
            let duration_days = (stage.duration_sec / 86_400).max(1) as i32;

            b.push_bind(stage_id)
                .push_bind(&recipe_id)
                .push_bind((idx + 1) as i32)
                .push_bind(&stage.name)
                .push_bind(duration_days)
                .push_bind(stage.ec_target)
                .push_bind(stage.ec_tolerance)
                .push_bind(stage.ph_target)
                .push_bind(stage.ph_tolerance)
                .push_bind(stage.nutrient_a_ratio)
                .push_bind(stage.nutrient_b_ratio)
                .push_bind(stage.water_level_target)
                .push_bind(stage.water_change_interval_days.map(|v| v as i32))
                .push_bind(stage.water_change_drain_cm)
                .push_bind(stage.auto_dilute_ec_trigger)
                .push_bind(stage.misting_on_duration_ms)
                .push_bind(stage.misting_off_duration_ms)
                .push_bind(stage.max_dose_per_cycle_ml);
        });

        if let Err(e) = query_builder.build().execute(&mut *tx).await {
            return HttpResponse::InternalServerError().json(json!({
                "error": "db_insert_stage_failed",
                "details": e.to_string()
            }));
        }
    }

    if let Err(e) = tx.commit().await {
        return HttpResponse::InternalServerError().json(json!({
            "error": "db_commit_failed",
            "details": e.to_string()
        }));
    }

    HttpResponse::Ok().json(json!({
        "status": "success",
        "message": "Đã cập nhật công thức"
    }))
}

pub async fn list_recipes(app_state: web::Data<AppState>) -> impl Responder {
    let recipes_res = db_recipes::list_recipes(&app_state.pg_pool).await;

    let rows = match recipes_res {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Lỗi truy vấn crop_recipes: {:?}", e);

            return HttpResponse::InternalServerError().json(json!({
                "error": "database_error",
                "details": e.to_string()
            }));
        }
    };

    let mut recipes: Vec<RecipeTemplate> = rows.into_iter().map(RecipeTemplate::from).collect();

    if recipes.is_empty() {
        return HttpResponse::Ok().json(json!({
            "status": "success",
            "data": recipes
        }));
    }

    // Lấy tất cả recipe IDs để fetch stages trong một query,
    // tránh N+1 query (mỗi recipe một query riêng).
    let recipe_ids: Vec<String> = recipes.iter().map(|recipe| recipe.id.clone()).collect();

    let stages_rows = sqlx::query(
        r#"
        SELECT
            recipe_id,
            name,
            duration_days,
            ec_target,
            ec_tolerance,
            ph_target,
            ph_tolerance,
            nutrient_a_ratio,
            nutrient_b_ratio,
            water_level_target,
            water_change_interval_days,
            water_change_drain_cm,
            auto_dilute_ec_trigger,
            misting_on_duration_ms,
            misting_off_duration_ms,
            max_dose_per_cycle_ml
        FROM crop_recipe_stages
        WHERE recipe_id = ANY($1)
        ORDER BY recipe_id, stage_order ASC
        "#,
    )
    .bind(&recipe_ids)
    .fetch_all(&app_state.pg_pool)
    .await
    .unwrap_or_default();

    // Group stages theo recipe_id.
    let mut stages_by_recipe: HashMap<String, Vec<CropStage>> = HashMap::new();

    for row in stages_rows {
        let recipe_id: String = row.try_get("recipe_id").unwrap_or_default();

        let stage = map_row_to_crop_stage(&row);

        stages_by_recipe.entry(recipe_id).or_default().push(stage);
    }

    // Gắn stages vào từng recipe.
    for recipe in &mut recipes {
        recipe.stages = stages_by_recipe.remove(&recipe.id).unwrap_or_default();
    }

    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": recipes
    }))
}

pub async fn create_recipe(
    app_state: web::Data<AppState>,
    req: web::Json<CreateRecipeRequest>,
) -> impl Responder {
    if req.name.trim().is_empty() || req.crop.trim().is_empty() || req.stages.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "invalid_recipe",
            "message": "name, crop và stages là bắt buộc"
        }));
    }

    let recipe_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let mut tx = match app_state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Lỗi mở transaction: {:?}", e);

            return HttpResponse::InternalServerError().json(json!({
                "error": "db_error",
                "details": e.to_string()
            }));
        }
    };

    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO crop_recipes (
            id,
            name,
            crop,
            description,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(&recipe_id)
    .bind(req.name.trim())
    .bind(req.crop.trim())
    .bind(&req.description)
    .bind(now)
    .execute(&mut *tx)
    .await
    {
        tracing::error!("Lỗi INSERT crop_recipes: {:?}", e);

        return HttpResponse::InternalServerError().json(json!({
            "error": "db_insert_recipe_failed",
            "details": e.to_string()
        }));
    }

    if !req.stages.is_empty() {
        let mut query_builder = sqlx::QueryBuilder::new(
            r#"
            INSERT INTO crop_recipe_stages (
                id,
                recipe_id,
                stage_order,
                name,
                duration_days,
                ec_target,
                ec_tolerance,
                ph_target,
                ph_tolerance,
                nutrient_a_ratio,
                nutrient_b_ratio,
                water_level_target,
                water_change_interval_days,
                water_change_drain_cm,
                auto_dilute_ec_trigger,
                misting_on_duration_ms,
                misting_off_duration_ms,
                max_dose_per_cycle_ml
            )
            "#,
        );

        query_builder.push_values(req.stages.iter().enumerate(), |mut b, (idx, stage)| {
            let stage_id = Uuid::new_v4().to_string();
            let duration_days = (stage.duration_sec / 86_400).max(1) as i32;

            b.push_bind(stage_id)
                .push_bind(&recipe_id)
                .push_bind((idx + 1) as i32)
                .push_bind(&stage.name)
                .push_bind(duration_days)
                .push_bind(stage.ec_target)
                .push_bind(stage.ec_tolerance)
                .push_bind(stage.ph_target)
                .push_bind(stage.ph_tolerance)
                .push_bind(stage.nutrient_a_ratio)
                .push_bind(stage.nutrient_b_ratio)
                .push_bind(stage.water_level_target)
                .push_bind(stage.water_change_interval_days.map(|v| v as i32))
                .push_bind(stage.water_change_drain_cm)
                .push_bind(stage.auto_dilute_ec_trigger)
                .push_bind(stage.misting_on_duration_ms)
                .push_bind(stage.misting_off_duration_ms)
                .push_bind(stage.max_dose_per_cycle_ml);
        });

        if let Err(e) = query_builder.build().execute(&mut *tx).await {
            tracing::error!("Lỗi INSERT crop_recipe_stages: {:?}", e);

            return HttpResponse::InternalServerError().json(json!({
                "error": "db_insert_stage_failed",
                "details": e.to_string()
            }));
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("Lỗi commit transaction: {:?}", e);

        return HttpResponse::InternalServerError().json(json!({
            "error": "db_commit_failed",
            "details": e.to_string()
        }));
    }

    let created = RecipeTemplate {
        id: recipe_id,
        name: req.name.trim().to_string(),
        crop: req.crop.trim().to_string(),
        description: req.description.clone(),
        stages: req.stages.clone(),
        created_at: now,
    };

    HttpResponse::Created().json(json!({
        "status": "success",
        "data": created
    }))
}

// XÓA RECIPE TEMPLATE TRONG CSDL
pub async fn delete_recipe(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let recipe_id = path.into_inner();

    let res = db_recipes::delete_recipe(&app_state.pg_pool, &recipe_id).await;

    match res {
        Ok(rows) if rows > 0 => HttpResponse::Ok().json(json!({
            "status": "success",
            "message": "Đã xóa recipe template"
        })),

        Ok(_) => HttpResponse::NotFound().json(json!({
            "error": "recipe_not_found"
        })),

        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": "db_delete_failed",
            "details": e.to_string()
        })),
    }
}

pub async fn apply_recipe(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    req: web::Json<ApplyRecipeRequest>,
    http_req: actix_web::HttpRequest,
) -> impl Responder {
    use actix_web::HttpMessage;
    let device_id = path.into_inner();

    // Lấy user_id từ AuthContext để kiểm tra quyền
    let user_id: Option<i64> = http_req
        .extensions()
        .get::<crate::api::middleware::auth::AuthContext>()
        .and_then(|ctx| ctx.user_id.as_ref())
        .and_then(|id| id.parse().ok());

    if let Some(uid) = user_id {
        let owned = crate::db::device_ownership::is_owner(&app_state.pg_pool, uid, &device_id)
            .await
            .unwrap_or(false);
        if !owned {
            return HttpResponse::Forbidden().json(json!({
                "error": "Bạn không có quyền áp dụng recipe cho thiết bị này"
            }));
        }
    }

    let recipe_template = if let Some(recipe_id) = &req.recipe_id {
        let row = db_recipes::get_recipe(&app_state.pg_pool, recipe_id).await;

        match row {
            Ok(Some(r)) => {
                let mut template = RecipeTemplate::from(r);
                template.stages = fetch_stages_for_recipe(&app_state.pg_pool, &template.id).await;
                template
            }

            _ => {
                return HttpResponse::NotFound().json(json!({
                    "error": "recipe_not_found"
                }));
            }
        }
    } else if let Some(inline) = &req.recipe {
        RecipeTemplate {
            id: Uuid::new_v4().to_string(),
            name: inline.name.trim().to_string(),
            crop: inline.crop.trim().to_string(),
            description: inline.description.clone(),
            stages: inline.stages.clone(),
            created_at: Utc::now(),
        }
    } else {
        return HttpResponse::BadRequest().json(json!({
            "error": "missing_recipe",
            "message": "Cần truyền recipe_id hoặc recipe"
        }));
    };

    let season_id = match postgres::get_active_crop_season(&app_state.pg_pool, &device_id).await {
        Ok(Some(season)) => {
            // TỰ ĐỘNG ĐỒNG BỘ GIỐNG CÂY TRỒNG
            // CỦA MÙA VỤ THEO RECIPE
            let _ = sqlx::query(
                "UPDATE crop_seasons
                     SET plant_type = $1
                     WHERE id = $2",
            )
            .bind(&recipe_template.crop)
            .bind(&season.id)
            .execute(&app_state.pg_pool)
            .await;

            season.id
        }

        _ => "default_season".to_string(),
    };

    let now_ts = Utc::now().timestamp() as u64;

    let snapshot = CropRecipe {
        schema_version: 1,
        recipe_id: recipe_template.id.clone(),
        season_id: season_id.clone(),
        device_id: device_id.clone(),
        revision: 1,
        start_time_sec: now_ts,
        current_stage_index: 0,
        stages: recipe_template.stages.clone(),
    };

    let payload = RecipeMqttPayload {
        action: "apply",
        recipe: snapshot.clone(),
        ts: None,
        nonce: None,
        signature: None,
    };

    let signed_payload = match sign_command(&device_id, &payload) {
        Ok(v) => v,

        Err(e) => {
            tracing::error!("Lỗi ký recipe payload: {:?}", e);

            return HttpResponse::InternalServerError().json(json!({
                "error": "signing_failed"
            }));
        }
    };

    let payload_bytes = match serde_json::to_vec(&signed_payload) {
        Ok(b) => b,

        Err(e) => {
            tracing::error!("Lỗi serialize recipe: {:?}", e);

            return HttpResponse::InternalServerError().json(json!({
                "error": "serialization_failed"
            }));
        }
    };

    if let Err(e) = app_state
        .mqtt_client
        .publish(
            topic_recipe_set(&device_id),
            QoS::AtLeastOnce,
            false,
            payload_bytes,
        )
        .await
    {
        tracing::error!("Lỗi publish MQTT: {:?}", e);

        return HttpResponse::InternalServerError().json(json!({
            "error": "mqtt_publish_failed"
        }));
    }

    let active_id = Uuid::new_v4().to_string();

    let _ = sqlx::query(
        r#"
        INSERT INTO device_active_recipes (
            id,
            device_id,
            season_id,
            recipe_id,
            current_stage_id
        )
        VALUES ($1, $2, $3, $4, 'stage_1')
        ON CONFLICT (device_id) DO UPDATE SET
            season_id = EXCLUDED.season_id,
            recipe_id = EXCLUDED.recipe_id,
            current_stage_id = EXCLUDED.current_stage_id
        "#,
    )
    .bind(&active_id)
    .bind(&device_id)
    .bind(&season_id)
    .bind(&recipe_template.id)
    .execute(&app_state.pg_pool)
    .await;

    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": snapshot
    }))
}

pub async fn clear_recipe(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();

    let payload = ClearRecipeMqttPayload {
        action: "clear",
        ts: None,
        nonce: None,
        signature: None,
    };

    let signed_payload = match sign_command(&device_id, &payload) {
        Ok(v) => v,

        Err(e) => {
            tracing::error!("Lỗi ký clear recipe: {:?}", e);

            return HttpResponse::InternalServerError().json(json!({
                "error": "signing_failed"
            }));
        }
    };

    if let Err(e) = app_state
        .mqtt_client
        .publish(
            topic_recipe_set(&device_id),
            QoS::AtLeastOnce,
            false,
            signed_payload.to_string(),
        )
        .await
    {
        tracing::error!("Lỗi MQTT clear recipe: {:?}", e);

        return HttpResponse::InternalServerError().json(json!({
            "error": "mqtt_publish_failed"
        }));
    }

    let _ = sqlx::query(
        "DELETE FROM device_active_recipes
         WHERE device_id = $1",
    )
    .bind(&device_id)
    .execute(&app_state.pg_pool)
    .await;

    HttpResponse::Ok().json(json!({
        "status": "success"
    }))
}

pub async fn recipe_status(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();

    let active_row = sqlx::query(
        "SELECT recipe_id, season_id
         FROM device_active_recipes
         WHERE device_id = $1
         LIMIT 1",
    )
    .bind(&device_id)
    .fetch_optional(&app_state.pg_pool)
    .await
    .unwrap_or(None);

    let Some(row) = active_row else {
        return HttpResponse::Ok().json(json!({
            "status": "success",
            "data": {
                "device_id": device_id,
                "active_recipe": null,
                "updated_at": Utc::now()
            }
        }));
    };

    let recipe_id: String = row.get("recipe_id");
    let season_id: String = row.get("season_id");

    let stages = fetch_stages_for_recipe(&app_state.pg_pool, &recipe_id).await;

    let active_recipe = CropRecipe {
        schema_version: 1,
        recipe_id,
        season_id,
        device_id: device_id.clone(),
        revision: 1,
        start_time_sec: Utc::now().timestamp() as u64,
        current_stage_index: 0,
        stages,
    };

    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": {
            "device_id": device_id,
            "active_recipe": active_recipe,
            "updated_at": Utc::now()
        }
    }))
}

#[derive(Debug, Deserialize)]
pub struct BulkApplyRecipeRequest {
    /// Danh sách device_id muốn áp recipe
    pub device_ids: Vec<String>,
    /// ID của recipe template đã lưu
    pub recipe_id: Option<String>,
    /// Hoặc inline recipe (tương tự apply_recipe đơn)
    pub recipe: Option<CreateRecipeRequest>,
}

#[derive(Debug, Serialize)]
pub struct BulkApplyRecipeResponse {
    pub succeeded: Vec<String>,
    pub failed: Vec<BulkApplyFailure>,
}

#[derive(Debug, Serialize)]
pub struct BulkApplyFailure {
    pub device_id: String,
    pub reason: String,
}

/// POST /api/recipes/bulk-apply — Áp dụng recipe cho nhiều thiết bị cùng lúc.
pub async fn bulk_apply_recipe(
    req: actix_web::HttpRequest,
    app_state: web::Data<AppState>,
    body: web::Json<BulkApplyRecipeRequest>,
) -> impl Responder {
    use crate::api::middleware::auth::AuthContext;
    use crate::db::device_ownership;
    use actix_web::HttpMessage;

    // Lấy user_id
    let user_id: Option<i64> = req
        .extensions()
        .get::<AuthContext>()
        .and_then(|ctx| ctx.user_id.as_ref())
        .and_then(|id| id.parse().ok());

    let Some(user_id) = user_id else {
        return HttpResponse::Unauthorized().json(json!({"error": "Chưa đăng nhập"}));
    };

    if body.device_ids.is_empty() {
        return HttpResponse::BadRequest().json(json!({"error": "device_ids không được rỗng"}));
    }

    // Kiểm tra user sở hữu tất cả devices
    let device_id_refs: Vec<&str> = body.device_ids.iter().map(|s| s.as_str()).collect();
    let all_owned = device_ownership::is_owner_of_all(&app_state.pg_pool, user_id, &device_id_refs)
        .await
        .unwrap_or(false);

    if !all_owned {
        return HttpResponse::Forbidden().json(json!({
            "error": "Bạn không sở hữu một hoặc nhiều thiết bị trong danh sách"
        }));
    }

    // Lấy hoặc build recipe template
    let recipe_template = if let Some(recipe_id) = &body.recipe_id {
        let row = db_recipes::get_recipe(&app_state.pg_pool, recipe_id).await;

        match row {
            Ok(Some(r)) => {
                let mut template = RecipeTemplate::from(r);
                template.stages = fetch_stages_for_recipe(&app_state.pg_pool, &template.id).await;
                template
            }
            _ => return HttpResponse::NotFound().json(json!({"error": "recipe_not_found"})),
        }
    } else if let Some(inline) = &body.recipe {
        RecipeTemplate {
            id: Uuid::new_v4().to_string(),
            name: inline.name.trim().to_string(),
            crop: inline.crop.trim().to_string(),
            description: inline.description.clone(),
            stages: inline.stages.clone(),
            created_at: Utc::now(),
        }
    } else {
        return HttpResponse::BadRequest().json(json!({
            "error": "Cần truyền recipe_id hoặc recipe"
        }));
    };

    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    let now_ts = Utc::now().timestamp() as u64;

    // Apply cho từng device
    for device_id in &body.device_ids {
        let season_id = match crate::db::postgres::get_active_crop_season(
            &app_state.pg_pool,
            device_id,
        )
        .await
        {
            Ok(Some(s)) => {
                // Sync plant_type
                let _ = sqlx::query("UPDATE crop_seasons SET plant_type = $1 WHERE id = $2")
                    .bind(&recipe_template.crop)
                    .bind(&s.id)
                    .execute(&app_state.pg_pool)
                    .await;
                s.id
            }
            _ => "default_season".to_string(),
        };

        let snapshot = CropRecipe {
            schema_version: 1,
            recipe_id: recipe_template.id.clone(),
            season_id: season_id.clone(),
            device_id: device_id.clone(),
            revision: 1,
            start_time_sec: now_ts,
            current_stage_index: 0,
            stages: recipe_template.stages.clone(),
        };

        let payload = RecipeMqttPayload {
            action: "apply",
            recipe: snapshot.clone(),
            ts: None,
            nonce: None,
            signature: None,
        };

        let signed = match crate::api::mqtt_utils::sign_command(device_id, &payload) {
            Ok(v) => v,
            Err(e) => {
                failed.push(BulkApplyFailure {
                    device_id: device_id.clone(),
                    reason: format!("signing_failed: {}", e),
                });
                continue;
            }
        };

        let payload_bytes = match serde_json::to_vec(&signed) {
            Ok(b) => b,
            Err(e) => {
                failed.push(BulkApplyFailure {
                    device_id: device_id.clone(),
                    reason: format!("serialize_failed: {}", e),
                });
                continue;
            }
        };

        if let Err(e) = app_state
            .mqtt_client
            .publish(
                topic_recipe_set(device_id),
                rumqttc::QoS::AtLeastOnce,
                false,
                payload_bytes,
            )
            .await
        {
            failed.push(BulkApplyFailure {
                device_id: device_id.clone(),
                reason: format!("mqtt_failed: {}", e),
            });
            continue;
        }

        // Lưu vào DB
        let _ = sqlx::query(
            r#"INSERT INTO device_active_recipes (id, device_id, season_id, recipe_id, current_stage_id)
               VALUES ($1, $2, $3, $4, 'stage_1')
               ON CONFLICT (device_id) DO UPDATE SET
                   season_id = EXCLUDED.season_id,
                   recipe_id = EXCLUDED.recipe_id,
                   current_stage_id = EXCLUDED.current_stage_id"#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(device_id)
        .bind(&season_id)
        .bind(&recipe_template.id)
        .execute(&app_state.pg_pool)
        .await;

        succeeded.push(device_id.clone());
    }

    HttpResponse::Ok().json(json!({
        "status": if failed.is_empty() { "success" } else { "partial" },
        "data": BulkApplyRecipeResponse { succeeded, failed }
    }))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/recipes", web::get().to(list_recipes))
        .route("/recipes", web::post().to(create_recipe))
        .route("/recipes/bulk-apply", web::post().to(bulk_apply_recipe))
        .route("/recipes/{recipe_id}", web::put().to(update_recipe))
        .route("/recipes/{recipe_id}", web::delete().to(delete_recipe))
        .route(
            "/devices/{device_id}/recipe/apply",
            web::post().to(apply_recipe),
        )
        .route(
            "/devices/{device_id}/recipe/clear",
            web::post().to(clear_recipe),
        )
        .route(
            "/devices/{device_id}/recipe/status",
            web::get().to(recipe_status),
        );
}
