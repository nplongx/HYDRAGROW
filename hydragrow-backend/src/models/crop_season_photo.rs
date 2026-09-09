use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CropSeasonPhoto {
    pub id: String,
    pub season_id: String,
    pub device_id: String,
    pub cloudinary_public_id: String,
    pub cloudinary_url: String,
    pub day_offset: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCropSeasonPhotoRequest {
    pub cloudinary_public_id: String,
    pub cloudinary_url: String,
    pub day_offset: i32,
}
