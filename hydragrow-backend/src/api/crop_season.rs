use crate::db::postgres;
use crate::models::crop_season::CreateCropSeasonRequest;
use crate::{AppState, db::postgres::end_active_crop_season};
use actix_web::{HttpResponse, Responder, web};
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct UpdateCropSeasonRequest {
    pub name: String,
    pub plant_type: Option<String>,
    pub description: Option<String>,
}

async fn get_active_season(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    match postgres::get_active_crop_season(&app_state.pg_pool, &device_id).await {
        Ok(season) => HttpResponse::Ok().json(json!({ "status": "success", "data": season })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn get_seasons_history(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    match postgres::get_crop_seasons_history(&app_state.pg_pool, &device_id).await {
        Ok(seasons) => HttpResponse::Ok().json(json!({ "status": "success", "data": seasons })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn create_season(
    path: web::Path<String>,
    req: web::Json<CreateCropSeasonRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    match postgres::create_crop_season(&app_state.pg_pool, &device_id, req.into_inner()).await {
        Ok(season) => HttpResponse::Ok().json(json!({ "status": "success", "data": season })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn update_season(
    path: web::Path<String>,
    req: web::Json<UpdateCropSeasonRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let data = req.into_inner();

    match postgres::update_active_crop_season(
        &app_state.pg_pool,
        &device_id,
        &data.name,
        data.plant_type.as_deref(),
        data.description.as_deref(),
    )
    .await
    {
        Ok(season) => HttpResponse::Ok().json(json!({ "status": "success", "data": season })),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().json(
            json!({ "status": "error", "message": "Không tìm thấy mùa vụ đang chạy để sửa" }),
        ),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn end_season(path: web::Path<String>, app_state: web::Data<AppState>) -> impl Responder {
    let device_id = path.into_inner();
    match end_active_crop_season(&app_state.pg_pool, &device_id).await {
        Ok(_) => {
            HttpResponse::Ok().json(json!({ "status": "success", "message": "Đã kết thúc mùa vụ" }))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": format!("{}", e) })),
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/seasons", web::get().to(get_seasons_history))
        .route("/seasons", web::post().to(create_season))
        .route("/seasons/active", web::get().to(get_active_season))
        .route("/seasons/active", web::put().to(update_season))
        .route("/seasons/active/end", web::put().to(end_season));
}
