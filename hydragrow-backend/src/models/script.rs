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
    /// fn main(input: Map) -> Map?
    /// input fields: ph, ec, temp, water_level, phase, device_id, timestamp_ms
    /// return None/() để không phát lệnh, hoặc Map { action, pump?, dose_ml?, duration_sec? }.
    /// MỌI kết quả với action="dose" bắt buộc đi qua `hydragrow_shared::safety::check_dose`
    /// trước khi publish MQTT — xem Task 6. Script KHÔNG thể bỏ qua bước này.
    #[serde(rename = "action_command")]
    ActionCommand,
}

impl std::fmt::Display for ScriptKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptKind::Alert => write!(f, "alert"),
            ScriptKind::RecipeOverride => write!(f, "recipe_override"),
            ScriptKind::ActionCommand => write!(f, "action_command"),
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
    /// Automation IR (JSON) nếu script này được build bằng Blockly/React Flow.
    /// NULL với script viết tay trực tiếp bằng Rhai.
    pub ir_json: Option<serde_json::Value>,
    /// Danh sách script IDs sẽ được kích hoạt sau khi script này thực thi thành công.
    /// Lưu dưới dạng JSON array text trong SQLite; Vec<String> sau khi parse.
    /// Vắng / `[]` = Flow độc lập (hành vi cũ).
    #[sqlx(json)]
    pub next_flow_ids: Vec<String>,
    pub cron_next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Kết quả sau khi eval một alert script
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertOutput {
    pub level: String, // "info" | "warning" | "error"
    pub title: String,
    pub message: String,
}

/// Kết quả sau khi eval một recipe_override script
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageOverride {
    pub target_stage_index: i64,
    pub reason: String,
}

/// Kết quả eval một recipe_override script — 2 hành động được hỗ trợ. Discriminator
/// là key "action" trong Map Rhai trả về; vắng key này (script viết trước Phase 3)
/// mặc định là AdvanceStage — không phải lỗi, giữ nguyên hành vi cũ.
#[derive(Debug, Clone, PartialEq)]
pub enum RecipeOverrideOutput {
    AdvanceStage(StageOverride),
    EndSeason { reason: String },
}

/// Request body để tạo/update script
#[derive(Debug, Deserialize)]
pub struct UpsertScriptRequest {
    pub id: Option<Uuid>,
    pub kind: String,
    pub name: String,
    pub source: String,
    pub enabled: Option<bool>,
    /// Optional — chỉ set khi request đến từ visual builder.
    pub ir_json: Option<serde_json::Value>,
    /// Optional — chỉ set khi người dùng configure Flow chain trên UI.
    pub next_flow_ids: Option<Vec<String>>,
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

/// Dữ liệu snapshot cảm biến + phase FSM hợp nhất dùng cho eval_flow_chain
#[derive(Debug, Clone)]
pub struct SensorSnapshot {
    pub ph: f32,
    pub ec: f32,
    pub temp: f32,
    pub water_level: f32,
    pub phase: String,
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

/// Input truyền vào action_command script — hợp nhất sensor + FSM phase vì
/// action block có thể cần điều kiện dựa trên cả hai (VD: chỉ dose khi phase=Monitoring).
#[derive(Debug, Clone)]
pub struct ScriptActionInput {
    pub ph: f32,
    pub ec: f32,
    pub temp: f32,
    pub water_level: f32,
    pub phase: String,
    pub device_id: String,
    pub timestamp_ms: i64,
}

/// Kết quả sau khi eval một action_command script.
/// schema_version không bắt buộc ở đây vì đây là kiểu nội bộ backend (không serialize
/// qua MQTT/DB trực tiếp — CommandPayload trong services/command.rs mới là kiểu wire).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionCommandOutput {
    pub action: String,
    pub pump: Option<String>,
    pub dose_ml: Option<f32>,
    /// Chỉ có ý nghĩa khi action="dose" — % công suất bơm, dùng cùng
    /// hydragrow_shared::dosing để quy đổi dose_ml → duration_sec.
    pub pwm: Option<u32>,
    pub duration_sec: Option<u64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionTraceEntry {
    pub description: String,
    pub passed: bool,
    pub actual_value: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum SampleValue {
    Series(Vec<f64>),
    Value(f64),
}

impl SampleValue {
    pub fn resolve(&self, mode: &str) -> f64 {
        match self {
            SampleValue::Value(v) => *v,
            SampleValue::Series(s) if s.is_empty() => 0.0,
            SampleValue::Series(s) => match mode {
                "mean" => s.iter().sum::<f64>() / s.len() as f64,
                "min" => s.iter().cloned().fold(f64::INFINITY, f64::min),
                "max" => s.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                _ => *s.last().unwrap(), // instant = giá trị mới nhất trong chuỗi
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TestScriptRequest {
    pub ir_json: serde_json::Value,
    pub sample: std::collections::HashMap<String, SampleValue>,
}

#[derive(Debug, Serialize)]
#[derive(Deserialize)]
pub struct TestScriptResponse {
    pub will_fire: bool,
    pub trace: Vec<ConditionTraceEntry>,
    pub actions_preview: Vec<serde_json::Value>,
}
