use serde::{Deserialize, Serialize};

/// One growth-stage target block inside a crop recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CropStage {
    pub name: String,
    pub duration_sec: u64,
    pub ec_target: f32,
    pub ec_tolerance: f32,
    pub ph_target: f32,
    pub ph_tolerance: f32,
    pub water_level_target: f32,
    pub light_hours_per_day: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature_target_c: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub humidity_target_percent: Option<f32>,
}

/// Versioned recipe snapshot shared by firmware, backend, and frontend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CropRecipe {
    pub schema_version: u16,
    pub recipe_id: String,
    pub season_id: String,
    pub device_id: String,
    pub revision: u64,
    pub start_time_sec: u64,
    pub current_stage_index: usize,
    pub stages: Vec<CropStage>,
}

/// Event emitted when a device/season advances from one recipe stage to another.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStageChangedEvent {
    pub schema_version: u16,
    pub recipe_id: String,
    pub season_id: String,
    pub device_id: String,
    pub revision: u64,
    pub start_time_sec: u64,
    pub previous_stage_index: Option<usize>,
    pub current_stage_index: usize,
    pub changed_at_sec: u64,
    pub stages: Vec<CropStage>,
}
