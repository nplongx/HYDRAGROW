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
use uuid::Uuid;

use crate::{AppState, api::mqtt_utils::sign_command, db::postgres};

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

        misting_on_duration_ms: r
            .try_get("misting_on_duration_ms")
            .unwrap_or(10_000),

        misting_off_duration_ms: r
            .try_get("misting_off_duration_ms")
            .unwrap_or(180_000),

        max_dose_per_cycle_ml: r
            .try_get::<Option<f32>, _>("max_dose_per_cycle_ml")
            .ok()
            .flatten(),
    }
}

async fn fetch_stages_for_recipe(
    pool: &sqlx::PgPool,
    recipe_id: &str,
) -> Vec<CropStage> {
    let rows = sqlx::query(
        r#"
        SELECT
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
        WHERE recipe_id = $1
        ORDER BY stage_order ASC
        "#,
    )
    .bind(recipe_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.iter().map(map_row_to_crop_stage).collect()
}

// CẬP NHẬT RECIPE TEMPLATE TRONG CSDL
pub async fn update_recipe(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    req: web::Json<CreateRecipeRequest>,
) -> impl Responder {
    let recipe_id = path.into_inner();

    if req.name.trim().is_empty()
        || req.crop.trim().is_empty()
        || req.stages.is_empty()
    {
        return HttpResponse::BadRequest().json(json!({
            "error": "invalid_recipe",
            "message": "name, crop và stages là bắt buộc"
        }));
    }

    let mut tx = match app_state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Lỗi mở transaction: {:?}", e);

            return HttpResponse::InternalServerError()
                .json(json!({"error": "db_error"}));
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
        return HttpResponse::InternalServerError()
            .json(json!({
                "error": "db_update_failed",
                "details": e.to_string()
            }));
    }

    // 2. Xóa sạch các Stages cũ của Recipe này
    if let Err(e) = sqlx::query(
        "DELETE FROM crop_recipe_stages WHERE recipe_id = $1",
    )
    .bind(&recipe_id)
    .execute(&mut *tx)
    .await
    {
        return HttpResponse::InternalServerError()
            .json(json!({
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

        query_builder.push_values(
            req.stages.iter().enumerate(),
            |mut b, (idx, stage)| {
                let stage_id = Uuid::new_v4().to_string();
                let duration_days =
                    (stage.duration_sec / 86_400).max(1) as i32;

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
                    .push_bind(
                        stage.water_change_interval_days
                            .map(|v| v as i32),
                    )
                    .push_bind(stage.water_change_drain_cm)
                    .push_bind(stage.auto_dilute_ec_trigger)
                    .push_bind(stage.misting_on_duration_ms)
                    .push_bind(stage.misting_off_duration_ms)
                    .push_bind(stage.max_dose_per_cycle_ml);
            },
        );

        if let Err(e) = query_builder
            .build()
            .execute(&mut *tx)
            .await
        {
            return HttpResponse::InternalServerError()
                .json(json!({
                    "error": "db_insert_stage_failed",
                    "details": e.to_string()
                }));
        }
    }

    if let Err(e) = tx.commit().await {
        return HttpResponse::InternalServerError()
            .json(json!({
                "error": "db_commit_failed",
                "details": e.to_string()
            }));
    }

    HttpResponse::Ok().json(json!({
        "status": "success",
        "message": "Đã cập nhật công thức"
    }))
}

use std::collections::HashMap;

pub async fn list_recipes(
    app_state: web::Data<AppState>,
) -> impl Responder {
    let recipes_res = sqlx::query_as::<_, RecipeTemplate>(
        r#"
        SELECT id, name, crop, description, created_at
        FROM crop_recipes
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(&app_state.pg_pool)
    .await;

    let mut recipes = match recipes_res {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(
                "Lỗi truy vấn crop_recipes: {:?}",
                e
            );

            return HttpResponse::InternalServerError()
                .json(json!({
                    "error": "database_error",
                    "details": e.to_string()
                }));
        }
    };

    if recipes.is_empty() {
        return HttpResponse::Ok().json(json!({
            "status": "success",
            "data": recipes
        }));
    }

    let recipe_ids: Vec<String> =
        recipes.iter().map(|r| r.id.clone()).collect();

    let rows = sqlx::query(
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

    let mut stages_by_recipe: HashMap<String, Vec<CropStage>> =
        HashMap::new();

    for r in rows {
        let recipe_id: String =
            r.try_get("recipe_id").unwrap_or_default();

        let stage = map_row_to_crop_stage(&r);

        stages_by_recipe
            .entry(recipe_id)
            .or_default()
            .push(stage);
    }

    for recipe in &mut recipes {
        recipe.stages = stages_by_recipe
            .remove(&recipe.id)
            .unwrap_or_default();
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
    if req.name.trim().is_empty()
        || req.crop.trim().is_empty()
        || req.stages.is_empty()
    {
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

            return HttpResponse::InternalServerError()
                .json(json!({
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
        tracing::error!(
            "Lỗi INSERT crop_recipes: {:?}",
            e
        );

        return HttpResponse::InternalServerError()
            .json(json!({
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

        query_builder.push_values(
            req.stages.iter().enumerate(),
            |mut b, (idx, stage)| {
                let stage_id = Uuid::new_v4().to_string();
                let duration_days =
                    (stage.duration_sec / 86_400).max(1) as i32;

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
                    .push_bind(
                        stage.water_change_interval_days
                            .map(|v| v as i32),
                    )
                    .push_bind(stage.water_change_drain_cm)
                    .push_bind(stage.auto_dilute_ec_trigger)
                    .push_bind(stage.misting_on_duration_ms)
                    .push_bind(stage.misting_off_duration_ms)
                    .push_bind(stage.max_dose_per_cycle_ml);
            },
        );

        if let Err(e) = query_builder
            .build()
            .execute(&mut *tx)
            .await
        {
            tracing::error!(
                "Lỗi INSERT crop_recipe_stages: {:?}",
                e
            );

            return HttpResponse::InternalServerError()
                .json(json!({
                    "error": "db_insert_stage_failed",
                    "details": e.to_string()
                }));
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(
            "Lỗi commit transaction: {:?}",
            e
        );

        return HttpResponse::InternalServerError()
            .json(json!({
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

    let res = sqlx::query(
        "DELETE FROM crop_recipes WHERE id = $1"
    )
    .bind(&recipe_id)
    .execute(&app_state.pg_pool)
    .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => {
            HttpResponse::Ok().json(json!({
                "status": "success",
                "message": "Đã xóa recipe template"
            }))
        }

        Ok(_) => {
            HttpResponse::NotFound().json(json!({
                "error": "recipe_not_found"
            }))
        }

        Err(e) => {
            HttpResponse::InternalServerError()
                .json(json!({
                    "error": "db_delete_failed",
                    "details": e.to_string()
                }))
        }
    }
}

pub async fn apply_recipe(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    req: web::Json<ApplyRecipeRequest>,
) -> impl Responder {
    let device_id = path.into_inner();

    let recipe_template = if let Some(recipe_id) = &req.recipe_id {
        let row = sqlx::query_as::<_, RecipeTemplate>(
            "SELECT id, name, crop, description, created_at
             FROM crop_recipes
             WHERE id = $1",
        )
        .bind(recipe_id)
        .fetch_optional(&app_state.pg_pool)
        .await;

        match row {
            Ok(Some(mut r)) => {
                r.stages =
                    fetch_stages_for_recipe(
                        &app_state.pg_pool,
                        &r.id,
                    )
                    .await;
                r
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

    let season_id =
        match postgres::get_active_crop_season(
            &app_state.pg_pool,
            &device_id,
        )
        .await
        {
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

    let signed_payload =
        match sign_command(&device_id, &payload) {
            Ok(v) => v,

            Err(e) => {
                tracing::error!(
                    "Lỗi ký recipe payload: {:?}",
                    e
                );

                return HttpResponse::InternalServerError()
                    .json(json!({
                        "error": "signing_failed"
                    }));
            }
        };

    let payload_bytes =
        match serde_json::to_vec(&signed_payload) {
            Ok(b) => b,

            Err(e) => {
                tracing::error!(
                    "Lỗi serialize recipe: {:?}",
                    e
                );

                return HttpResponse::InternalServerError()
                    .json(json!({
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
        tracing::error!(
            "Lỗi publish MQTT: {:?}",
            e
        );

        return HttpResponse::InternalServerError()
            .json(json!({
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

    let signed_payload =
        match sign_command(&device_id, &payload) {
            Ok(v) => v,

            Err(e) => {
                tracing::error!(
                    "Lỗi ký clear recipe: {:?}",
                    e
                );

                return HttpResponse::InternalServerError()
                    .json(json!({
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
        tracing::error!(
            "Lỗi MQTT clear recipe: {:?}",
            e
        );

        return HttpResponse::InternalServerError()
            .json(json!({
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

    let stages =
        fetch_stages_for_recipe(
            &app_state.pg_pool,
            &recipe_id,
        )
        .await;

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

pub fn init_routes(
    cfg: &mut web::ServiceConfig,
) {
    cfg.route(
        "/recipes",
        web::get().to(list_recipes),
    )
    .route(
        "/recipes",
        web::post().to(create_recipe),
    )
    .route(
        "/recipes/{recipe_id}",
        web::put().to(update_recipe),
    )
    .route(
        "/recipes/{recipe_id}",
        web::delete().to(delete_recipe),
    )
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