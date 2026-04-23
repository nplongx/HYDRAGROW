use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CropSeason {
    pub id: String,
    pub device_id: String,
    pub name: String,
    pub plant_type: Option<String>,
    pub start_time: String,
    pub end_time: Option<String>,
    pub status: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCropSeasonRequest {
    pub name: String,
    pub plant_type: Option<String>,
    pub description: Option<String>,
}
