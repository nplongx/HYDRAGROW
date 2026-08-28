use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Loại script — xác định signature của hàm main(input) trong Rhai
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum ScriptKind {
    /// fn main(input: Map) -> Map?
    /// input fields: ph, ec, temp, water_level, device_id, timestamp_ms
    /// return None/() để không trigger alert, hoặc Map { level, title, message }
    #[serde(rename = "alert")]
    Alert,
    /// fn main(input: Map) -> Map?
    /// input fields: phase, stage_index, ec, ph, elapsed_sec
    /// return None/() để giữ stage hiện tại, hoặc Map { target_stage_index, reason }
    #[serde(rename = "recipe_override")]
    RecipeOverride,
}

impl std::fmt::Display for ScriptKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptKind::Alert => write!(f, "alert"),
            ScriptKind::RecipeOverride => write!(f, "recipe_override"),
        }
    }
}

/// Row từ bảng user_scripts
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserScript {
    pub id: Uuid,
    pub device_id: String,
    pub kind: String, // "alert" | "recipe_override"
    pub name: String,
    pub source: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Kết quả sau khi eval một alert script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertOutput {
    pub level: String, // "info" | "warning" | "error"
    pub title: String,
    pub message: String,
}

/// Kết quả sau khi eval một recipe_override script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOverride {
    pub target_stage_index: i64,
    pub reason: String,
}

/// Request body để tạo/update script
#[derive(Debug, Deserialize)]
pub struct UpsertScriptRequest {
    pub kind: String,
    pub name: String,
    pub source: String,
    pub enabled: Option<bool>,
}

/// Response trả về sau validate script (dry-run)
#[derive(Debug, Serialize)]
pub struct ScriptValidateResponse {
    pub valid: bool,
    pub error: Option<String>,
}

/// Input truyền vào alert script dưới dạng Rhai Map
#[derive(Debug, Clone)]
pub struct ScriptSensorInput {
    pub ph: f32,
    pub ec: f32,
    pub temp: f32,
    pub water_level: f32,
    pub device_id: String,
    pub timestamp_ms: i64,
}

/// Input truyền vào recipe_override script
#[derive(Debug, Clone)]
pub struct ScriptFsmInput {
    pub phase: String,
    pub stage_index: i64,
    pub ec: f32,
    pub ph: f32,
    pub elapsed_sec: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_kind_display() {
        assert_eq!(ScriptKind::Alert.to_string(), "alert");
        assert_eq!(ScriptKind::RecipeOverride.to_string(), "recipe_override");
    }

    #[test]
    fn test_script_kind_serde() {
        let alert = ScriptKind::Alert;
        let serialized = serde_json::to_string(&alert).unwrap();
        assert_eq!(serialized, "\"alert\"");
        let deserialized: ScriptKind = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, ScriptKind::Alert);

        let recipe = ScriptKind::RecipeOverride;
        let serialized_recipe = serde_json::to_string(&recipe).unwrap();
        assert_eq!(serialized_recipe, "\"recipe_override\"");
        let deserialized_recipe: ScriptKind = serde_json::from_str(&serialized_recipe).unwrap();
        assert_eq!(deserialized_recipe, ScriptKind::RecipeOverride);
    }
}
