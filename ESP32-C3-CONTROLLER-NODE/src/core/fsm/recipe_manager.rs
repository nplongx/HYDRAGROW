//! Recipe Manager — tính stage cây trồng và phát delta/event khi stage thay đổi.

use hydragrow_shared::recipe::{CropRecipe, CropStage};
use hydragrow_shared::ControllerConfig;

use crate::core::fsm::context::SystemContext;
use crate::core::fsm::events::OrchestratorEvent;
use crate::core::fsm::tick_result::TickResult;

pub const MIN_VALID_UNIX_SEC: u64 = 1_700_000_000;
pub const RECIPE_CHECK_INTERVAL_SEC: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeStageResult {
    NotStarted,
    Active { stage_index: usize },
    Completed,
}

pub fn calculate_stage_index(recipe: &CropRecipe, now_sec: u64) -> RecipeStageResult {
    if now_sec < recipe.start_time_sec || recipe.stages.is_empty() {
        return RecipeStageResult::NotStarted;
    }

    let elapsed_sec = now_sec.saturating_sub(recipe.start_time_sec);
    let mut accum_sec = 0_u64;

    for (idx, stage) in recipe.stages.iter().enumerate() {
        accum_sec = accum_sec.saturating_add(stage.duration_sec);
        if elapsed_sec < accum_sec {
            return RecipeStageResult::Active { stage_index: idx };
        }
    }

    RecipeStageResult::Completed
}

pub fn tick_recipe_engine(
    config: &mut ControllerConfig,
    ctx: &SystemContext,
    now_sec: u64,
) -> TickResult {
    let mut result = TickResult::default();

    if now_sec < MIN_VALID_UNIX_SEC {
        return result;
    }

    if ctx.last_recipe_check_sec != 0
        && now_sec.saturating_sub(ctx.last_recipe_check_sec) < RECIPE_CHECK_INTERVAL_SEC
    {
        return result;
    }

    result.delta.last_recipe_check_sec = Some(now_sec);

    let Some(recipe) = config.active_recipe.clone() else {
        return result;
    };

    let stage_result = calculate_stage_index(&recipe, now_sec);
    match stage_result {
        RecipeStageResult::NotStarted => {}
        RecipeStageResult::Active { stage_index } => {
            if ctx.current_stage_index != Some(stage_index) || ctx.recipe_completed {
                if let Some(stage) = recipe.stages.get(stage_index) {
                    apply_stage_override(config, stage);
                }
                result.delta.current_stage_index = Some(Some(stage_index));
                result.delta.recipe_completed = Some(false);
                emit_stage_event(&mut result, &recipe, Some(stage_index), false, now_sec);
                result
                    .events
                    .push(OrchestratorEvent::SaveCurrentStageIndex {
                        stage_index: Some(stage_index),
                    });
            }
        }
        RecipeStageResult::Completed => {
            if !ctx.recipe_completed {
                result.delta.recipe_completed = Some(true);
                result.delta.current_stage_index = Some(None);
                emit_stage_event(&mut result, &recipe, None, true, now_sec);
                result
                    .events
                    .push(OrchestratorEvent::SaveCurrentStageIndex { stage_index: None });
            }
        }
    }

    result
}

pub fn apply_stage_override(config: &mut ControllerConfig, stage: &CropStage) {
    config.ec_target = stage.ec_target;
    config.ec_tolerance = stage.ec_tolerance;
    config.ph_target = stage.ph_target;
    config.ph_tolerance = stage.ph_tolerance;
    config.water_level_target = stage.water_level_target;

    // Ghi đè tỷ lệ A:B
    config.nutrient_a_ratio = stage.nutrient_a_ratio;
    config.nutrient_b_ratio = stage.nutrient_b_ratio;

    // Ghi đè lịch thay nước theo ngày của stage
    config.water_change_interval_days = stage.water_change_interval_days;
    if let Some(drain_cm) = stage.water_change_drain_cm {
        config.scheduled_drain_amount_cm = drain_cm;
    }

    // Ghi đè giới hạn châm nếu stage có quy định
    if let Some(max_dose) = stage.max_dose_per_cycle_ml {
        config.max_dose_per_cycle = max_dose;
    }

    config.misting_on_duration_ms = stage.misting_on_duration_ms;
    config.misting_off_duration_ms = stage.misting_off_duration_ms;
}

fn emit_stage_event(
    result: &mut TickResult,
    recipe: &CropRecipe,
    stage_index: Option<usize>,
    completed: bool,
    now_sec: u64,
) {
    let stage_name = stage_index
        .and_then(|idx| recipe.stages.get(idx))
        .map(|stage| stage.name.clone());

    let payload = serde_json::json!({
        "type": if completed { "recipe_completed" } else { "recipe_stage_changed" },
        "schema_version": recipe.schema_version,
        "recipe_id": recipe.recipe_id,
        "season_id": recipe.season_id,
        "device_id": recipe.device_id,
        "revision": recipe.revision,
        "stage_index": stage_index,
        "stage_name": stage_name,
        "completed": completed,
        "timestamp_sec": now_sec,
    });

    result
        .events
        .push(OrchestratorEvent::PublishRecipeStageChanged {
            payload_json: payload.to_string(),
        });
}