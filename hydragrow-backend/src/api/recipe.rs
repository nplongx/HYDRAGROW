use std::sync::{LazyLock, Mutex};

use actix_web::{HttpResponse, Responder, web};
use chrono::{DateTime, Utc};
use hydragrow_shared::topics::topic_recipe_set;
use rumqttc::QoS;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{AppState, api::mqtt_utils::sign_command};

static RECIPES: LazyLock<Mutex<Vec<Recipe>>> = LazyLock::new(|| Mutex::new(default_recipes()));
static DEVICE_RECIPES: LazyLock<Mutex<Vec<DeviceRecipeStatus>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStage {
    pub name: String,
    pub duration_days: u32,
    pub ec_target: f32,
    pub ph_target: f32,
    pub water_level_target: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub crop: String,
    pub description: Option<String>,
    pub stages: Vec<RecipeStage>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropRecipe {
    pub recipe_id: String,
    pub name: String,
    pub crop: String,
    pub stages: Vec<RecipeStage>,
    pub applied_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceRecipeStatus {
    device_id: String,
    active_recipe: Option<CropRecipe>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRecipeRequest {
    pub name: String,
    pub crop: String,
    pub description: Option<String>,
    pub stages: Vec<RecipeStage>,
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

fn default_recipes() -> Vec<Recipe> {
    vec![Recipe {
        id: "lettuce-default".to_string(),
        name: "Default Lettuce".to_string(),
        crop: "lettuce".to_string(),
        description: Some("Baseline hydroponic lettuce recipe".to_string()),
        stages: vec![RecipeStage {
            name: "grow".to_string(),
            duration_days: 30,
            ec_target: 1.4,
            ph_target: 6.0,
            water_level_target: 20.0,
        }],
        created_at: Utc::now(),
    }]
}

fn recipe_to_snapshot(recipe: Recipe) -> CropRecipe {
    CropRecipe {
        recipe_id: recipe.id,
        name: recipe.name,
        crop: recipe.crop,
        stages: recipe.stages,
        applied_at: Utc::now(),
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

    let recipe = Recipe {
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
    let recipe = if let Some(recipe_id) = &req.recipe_id {
        let recipes = RECIPES.lock().unwrap();
        match recipes.iter().find(|recipe| &recipe.id == recipe_id) {
            Some(recipe) => recipe.clone(),
            None => return HttpResponse::NotFound().json(json!({ "error": "recipe_not_found" })),
        }
    } else if let Some(inline) = &req.recipe {
        Recipe {
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

    let snapshot = recipe_to_snapshot(recipe);
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
