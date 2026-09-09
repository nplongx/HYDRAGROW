use crate::AppState;
use crate::db::postgres;
use crate::models::crop_season_photo::CreateCropSeasonPhotoRequest;
use actix_web::{HttpResponse, Responder, web};
use serde_json::json;

async fn sign_photo_upload(
    path: web::Path<(String, String)>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let (device_id, season_id) = path.into_inner();
    let Some(cloudinary) = app_state.cloudinary.as_ref() else {
        return HttpResponse::ServiceUnavailable().json(json!({
            "status": "error",
            "message": "Nhật ký ảnh chưa được cấu hình"
        }));
    };

    let folder = format!("hydragrow/{}/{}", device_id, season_id);
    let timestamp = chrono::Utc::now().timestamp();
    let signature = cloudinary.sign_upload(&folder, timestamp);

    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": {
            "cloud_name": cloudinary.cloud_name,
            "api_key": cloudinary.api_key,
            "timestamp": timestamp,
            "signature": signature,
            "folder": folder,
            "signature_algorithm": "sha256",
        }
    }))
}

async fn create_photo(
    path: web::Path<(String, String)>,
    req: web::Json<CreateCropSeasonPhotoRequest>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let (device_id, season_id) = path.into_inner();
    if app_state.cloudinary.is_none() {
        return HttpResponse::ServiceUnavailable().json(json!({
            "status": "error",
            "message": "Nhật ký ảnh chưa được cấu hình"
        }));
    }
    match postgres::create_crop_season_photo(
        &app_state.pg_pool,
        &device_id,
        &season_id,
        req.into_inner(),
    )
    .await
    {
        Ok(photo) => HttpResponse::Ok().json(json!({ "status": "success", "data": photo })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn list_photos(
    path: web::Path<(String, String)>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let (_device_id, season_id) = path.into_inner();
    match postgres::list_crop_season_photos(&app_state.pg_pool, &season_id).await {
        Ok(photos) => HttpResponse::Ok().json(json!({ "status": "success", "data": photos })),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({ "status": "error", "message": e.to_string() })),
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/seasons/{season_id}/photos/sign",
        web::post().to(sign_photo_upload),
    )
    .route("/seasons/{season_id}/photos", web::post().to(create_photo))
    .route("/seasons/{season_id}/photos", web::get().to(list_photos));
}
