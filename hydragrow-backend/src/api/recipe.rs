use std::sync::{LazyLock, Mutex};

use actix_web::{HttpResponse, Responder, web};
use chrono::{DateTime, Utc};
use hydragrow_shared::{
    recipe::{CropRecipe, CropStage},
    topics::topic_recipe_set,
};
use rumqttc::QoS;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{AppState, api::mqtt_utils::sign_command, db::postgres};

static RECIPES: LazyLock<Mutex<Vec<RecipeTemplate>>> =
    LazyLock::new(|| Mutex::new(default_recipes()));
static DEVICE_RECIPES: LazyLock<Mutex<Vec<DeviceRecipeStatus>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeTemplate {
    pub id: String,
    pub name: String,
    pub crop: String,
    pub description: Option<String>,
    pub stages: Vec<CropStage>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRecipeStatus {
    pub device_id: String,
    pub active_recipe: Option<CropRecipe>,
    pub updated_at: DateTime<Utc>,
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

fn default_recipes() -> Vec<RecipeTemplate> {
    vec![RecipeTemplate {
        id: "lettuce-default".to_string(),
        name: "Default Lettuce".to_string(),
        crop: "lettuce".to_string(),
        description: Some("Baseline hydroponic lettuce recipe".to_string()),
        stages: vec![CropStage {
            name: "grow".to_string(),
            duration_sec: 30 * 86_400, // 30 ngày quy đổi sang giây
            ec_target: 1.4,
            ec_tolerance: 0.1,
            ph_target: 6.0,
            ph_tolerance: 0.2,
            nutrient_a_ratio: 1.0,
            nutrient_b_ratio: 1.0,
            water_level_target: 20.0,
            water_change_interval_days: Some(14),
            water_change_drain_cm: Some(5.0),
            auto_dilute_ec_trigger: None,
            misting_on_duration_ms: 10_000,
            misting_off_duration_ms: 180_000,
            max_dose_per_cycle_ml: None,
        }],
        created_at: Utc::now(),
    }]
}

fn build_crop_recipe_snapshot(device_id: &str, season_id: &str, recipe: RecipeTemplate) -> CropRecipe {
    CropRecipe {
        schema_version: 1,
        recipe_id: recipe.id,
        season_id: season_id.to_string(),
        device_id: device_id.to_string(),
        revision: 1,
        start_time_sec: Utc::now().timestamp() as u64,
        current_stage_index: 0,
        stages: recipe.stages,
    }
}

fn set_device_recipe(device_id: &str, active_recipe: Option<CropRecipe>) {
    let mut statuses = DEVICE_RECIPES.lock().unwrap();
    if let Some(status) = statuses
        .iter_mut()
        .find(|status| status.device_id == device_id)
    {
        status.active_recipe = active_recipe;
        status.updated_at = Utc::now();
    } else {
        statuses.push(DeviceRecipeStatus {
            device_id: device_id.to_string(),
            active_recipe,
            updated_at: Utc::now(),
        });
    }
}

pub async fn list_recipes() -> impl Responder {
    let recipes = RECIPES.lock().unwrap().clone();
    HttpResponse::Ok().json(json!({ "status": "success", "data": recipes }))
}

pub async fn create_recipe(req: web::Json<CreateRecipeRequest>) -> impl Responder {
    if req.name.trim().is_empty() || req.crop.trim().is_empty() || req.stages.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "error": "invalid_recipe",
            "message": "name, crop và stages là bắt buộc"
        }));
    }

    let recipe = RecipeTemplate {
        id: Uuid::new_v4().to_string(),
        name: req.name.trim().to_string(),
        crop: req.crop.trim().to_string(),
        description: req.description.clone(),
        stages: req.stages.clone(),
        created_at: Utc::now(),
    };
    RECIPES.lock().unwrap().push(recipe.clone());

    HttpResponse::Created().json(json!({ "status": "success", "data": recipe }))
}

pub async fn apply_recipe(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    req: web::Json<ApplyRecipeRequest>,
) -> impl Responder {
    let device_id = path.into_inner();
    let recipe_template = if let Some(recipe_id) = &req.recipe_id {
        let recipes = RECIPES.lock().unwrap();
        match recipes.iter().find(|recipe| &recipe.id == recipe_id) {
            Some(recipe) => recipe.clone(),
            None => return HttpResponse::NotFound().json(json!({ "error": "recipe_not_found" })),
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

    // Lấy season_id hiện tại từ DB nếu có, ngược lại dùng fallback
    let season_id = match postgres::get_active_crop_season(&app_state.pg_pool, &device_id).await {
        Ok(Some(season)) => season.id,
        _ => "default_season".to_string(),
    };

    let snapshot = build_crop_recipe_snapshot(&device_id, &season_id, recipe_template);
    let payload = RecipeMqttPayload {
        action: "apply",
        recipe: snapshot.clone(),
        ts: None,
        nonce: None,
        signature: None,
    };
    let signed_payload = match sign_command(&device_id, &payload) {
        Ok(value) => value,
        Err(e) => {
            tracing::error!("Lỗi ký recipe payload: {:?}", e);
            return HttpResponse::InternalServerError().json(json!({ "error": "signing_failed" }));
        }
    };

    let payload_bytes = match serde_json::to_vec(&signed_payload) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("Lỗi serialize recipe payload: {:?}", e);
            return HttpResponse::InternalServerError()
                .json(json!({ "error": "serialization_failed" }));
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
        tracing::error!("Lỗi publish recipe qua MQTT: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({ "error": "mqtt_publish_failed" }));
    }

    set_device_recipe(&device_id, Some(snapshot.clone()));
    HttpResponse::Ok().json(json!({ "status": "success", "data": snapshot }))
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
        Ok(value) => value,
        Err(e) => {
            tracing::error!("Lỗi ký clear recipe payload: {:?}", e);
            return HttpResponse::InternalServerError().json(json!({ "error": "signing_failed" }));
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
        tracing::error!("Lỗi publish clear recipe qua MQTT: {:?}", e);
        return HttpResponse::InternalServerError().json(json!({ "error": "mqtt_publish_failed" }));
    }

    set_device_recipe(&device_id, None);
    HttpResponse::Ok().json(json!({ "status": "success" }))
}

pub async fn recipe_status(path: web::Path<String>) -> impl Responder {
    let device_id = path.into_inner();
    let status = DEVICE_RECIPES
        .lock()
        .unwrap()
        .iter()
        .find(|status| status.device_id == device_id)
        .cloned()
        .unwrap_or(DeviceRecipeStatus {
            device_id,
            active_recipe: None,
            updated_at: Utc::now(),
        });

    HttpResponse::Ok().json(json!({ "status": "success", "data": status }))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/recipes", web::get().to(list_recipes))
        .route("/recipes", web::post().to(create_recipe))
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