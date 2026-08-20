use serde::{Deserialize, Serialize};

fn default_ratio_one() -> f32 {
    1.0
}

/// One growth-stage target block inside a crop recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CropStage {
    pub name: String,
    pub duration_sec: u64,
    pub ec_target: f32,
    pub ec_tolerance: f32,
    pub ph_target: f32,
    pub ph_tolerance: f32,
    // --- 2. Tỷ lệ dinh dưỡng A:B ---
    #[serde(default = "default_ratio_one")]
    pub nutrient_a_ratio: f32,
    #[serde(default = "default_ratio_one")]
    pub nutrient_b_ratio: f32,

    // --- 3. Mực nước & Lịch thay nước theo giai đoạn ---
    pub water_level_target: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub water_change_interval_days: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub water_change_drain_cm: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_dilute_ec_trigger: Option<f32>,

    // --- 4. Vi khí hậu & Phun sương ---
    pub misting_on_duration_ms: i32,
    pub misting_off_duration_ms: i32,

    // --- 5. Ràng buộc an toàn & Nhiệt độ (Đề xuất thêm) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_dose_per_cycle_ml: Option<f32>,
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
