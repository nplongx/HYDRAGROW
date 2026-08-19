//! Recipe Manager — tính stage cây trồng và phát delta/event khi stage thay đổi.

use hydragrow_shared::{ControllerConfig, CropRecipe};

use crate::core::fsm::context::SystemContext;
use crate::core::fsm::events::OrchestratorEvent;
use crate::core::fsm::tick_result::{ContextDelta, TickResult};

pub const MIN_VALID_UNIX_SEC: u64 = 1_700_000_000;
pub const RECIPE_CHECK_INTERVAL_SEC: u64 = 60;
const SECONDS_PER_DAY: u64 = 86_400;

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

    let elapsed_days = now_sec.saturating_sub(recipe.start_time_sec) / SECONDS_PER_DAY;
    if recipe
        .end_day
        .map(|end_day| elapsed_days >= u64::from(end_day))
        .unwrap_or(false)
    {
        return RecipeStageResult::Completed;
    }
    let mut active_index = None;

    for (idx, stage) in recipe.stages.iter().enumerate() {
        if elapsed_days >= u64::from(stage.start_day) {
            active_index = Some(idx);
        } else {
            break;
        }
    }

    match active_index {
        Some(idx) => RecipeStageResult::Active { stage_index: idx },
        None => RecipeStageResult::NotStarted,
    }
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
                apply_stage_override(config, &recipe, stage_index);
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

fn apply_stage_override(config: &mut ControllerConfig, recipe: &CropRecipe, stage_index: usize) {
    if let Some(stage) = recipe.stages.get(stage_index) {
        if let Some(ec) = stage.ec_target {
            config.ec_target = ec;
        }
        if let Some(ph) = stage.ph_target {
            config.ph_target = ph;
        }
        if let Some(water_level) = stage.water_level_target {
            config.water_level_target = water_level;
        }
    }
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
        "recipe_id": recipe.id,
        "recipe_name": recipe.name,
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
