use hydragrow_controller_core::core::fsm::context::SystemContext;
use hydragrow_controller_core::core::fsm::recipe_manager::{
    ControllerRuntimeState, RecipeStageResult, calculate_stage_index, tick_recipe_engine,
};
use hydragrow_shared::ControllerConfig;
use hydragrow_shared::recipe::{CropRecipe, CropStage};

fn test_recipe() -> CropRecipe {
    CropRecipe {
        schema_version: 1,
        recipe_id: "recipe_test".to_string(),
        season_id: "season_test".to_string(),
        device_id: "dev_test".to_string(),
        revision: 1,
        start_time_sec: 1_700_000_000,
        current_stage_index: 0,
        stages: vec![
            CropStage {
                name: "Stage 1".to_string(),
                duration_sec: 3600, // 1 hour
                ec_target: 2.2,
                ec_tolerance: 0.08,
                ph_target: 5.8,
                ph_tolerance: 0.15,
                nutrient_a_ratio: 1.5,
                nutrient_b_ratio: 0.8,
                water_level_target: 22.0,
                water_change_interval_days: Some(5),
                water_change_drain_cm: Some(7.0),
                auto_dilute_ec_trigger: None,
                misting_on_duration_ms: 8000,
                misting_off_duration_ms: 120000,
                max_dose_per_cycle_ml: Some(15.0),
            },
            CropStage {
                name: "Stage 2".to_string(),
                duration_sec: 3600, // 1 hour
                ec_target: 2.8,
                ec_tolerance: 0.1,
                ph_target: 6.2,
                ph_tolerance: 0.2,
                nutrient_a_ratio: 2.0,
                nutrient_b_ratio: 1.0,
                water_level_target: 25.0,
                water_change_interval_days: Some(10),
                water_change_drain_cm: Some(8.0),
                auto_dilute_ec_trigger: None,
                misting_on_duration_ms: 10000,
                misting_off_duration_ms: 90000,
                max_dose_per_cycle_ml: Some(20.0),
            },
        ],
    }
}

#[test]
fn active_stage_overrides_all_fields_consistently() {
    let recipe = test_recipe();
    let base = ControllerConfig {
        ec_target: 1.0,
        ec_tolerance: 0.05,
        ph_target: 6.5,
        ph_tolerance: 0.1,
        nutrient_a_ratio: 1.0,
        nutrient_b_ratio: 1.0,
        water_level_target: 18.0,
        water_change_interval_days: None,
        scheduled_drain_amount_cm: 3.0,
        max_dose_per_cycle: 5.0,
        misting_on_duration_ms: 5000,
        misting_off_duration_ms: 180000,
        active_recipe: Some(recipe.clone()),
        ..ControllerConfig::default()
    };

    let mut state = ControllerRuntimeState::new(base);
    let mut ctx = SystemContext::default();

    // Tick at start_time_sec: stage 0 should become active
    let tick = tick_recipe_engine(&mut state.effective_config, &ctx, 1_700_000_000);
    state.apply_recipe_tick_result(&tick);
    ctx.apply_delta(&mut { tick.delta });

    let stage0 = &recipe.stages[0];
    assert_eq!(state.effective_config.ec_target, stage0.ec_target);
    assert_eq!(state.effective_config.ec_tolerance, stage0.ec_tolerance);
    assert_eq!(state.effective_config.ph_target, stage0.ph_target);
    assert_eq!(state.effective_config.ph_tolerance, stage0.ph_tolerance);
    assert_eq!(
        state.effective_config.nutrient_a_ratio,
        stage0.nutrient_a_ratio
    );
    assert_eq!(
        state.effective_config.nutrient_b_ratio,
        stage0.nutrient_b_ratio
    );
    assert_eq!(
        state.effective_config.water_level_target,
        stage0.water_level_target
    );
    assert_eq!(
        state.effective_config.water_change_interval_days,
        stage0.water_change_interval_days
    );
    assert_eq!(
        state.effective_config.scheduled_drain_amount_cm,
        stage0.water_change_drain_cm.unwrap()
    );
    assert_eq!(
        state.effective_config.max_dose_per_cycle,
        stage0.max_dose_per_cycle_ml.unwrap()
    );
    assert_eq!(
        state.effective_config.misting_on_duration_ms,
        stage0.misting_on_duration_ms
    );
    assert_eq!(
        state.effective_config.misting_off_duration_ms,
        stage0.misting_off_duration_ms
    );
}

#[test]
fn base_config_update_during_active_stage_preserves_stage_overrides() {
    let recipe = test_recipe();
    let base = ControllerConfig {
        active_recipe: Some(recipe.clone()),
        ..ControllerConfig::default()
    };

    let mut state = ControllerRuntimeState::new(base);
    let mut ctx = SystemContext::default();

    // Activate stage 0
    let tick = tick_recipe_engine(&mut state.effective_config, &ctx, 1_700_000_000);
    state.apply_recipe_tick_result(&tick);
    ctx.apply_delta(&mut { tick.delta });

    let stage0 = &recipe.stages[0];
    assert_eq!(state.effective_config.ec_target, stage0.ec_target);

    // Update base config while stage 0 is active
    let mut new_base = state.base_config.clone();
    new_base.cooldown_sec = 999;
    new_base.ec_target = 0.5; // attempted base override of EC
    new_base.ph_target = 4.5; // attempted base override of pH
    new_base.water_level_target = 10.0;
    state.set_base_config(new_base);

    // Non-overridden fields take effect from new base
    assert_eq!(state.effective_config.cooldown_sec, 999);

    // Overridden fields MUST retain the stage overrides
    assert_eq!(state.effective_config.ec_target, stage0.ec_target);
    assert_eq!(state.effective_config.ec_tolerance, stage0.ec_tolerance);
    assert_eq!(state.effective_config.ph_target, stage0.ph_target);
    assert_eq!(state.effective_config.ph_tolerance, stage0.ph_tolerance);
    assert_eq!(
        state.effective_config.nutrient_a_ratio,
        stage0.nutrient_a_ratio
    );
    assert_eq!(
        state.effective_config.nutrient_b_ratio,
        stage0.nutrient_b_ratio
    );
    assert_eq!(
        state.effective_config.water_level_target,
        stage0.water_level_target
    );
    assert_eq!(
        state.effective_config.misting_on_duration_ms,
        stage0.misting_on_duration_ms
    );
    assert_eq!(
        state.effective_config.misting_off_duration_ms,
        stage0.misting_off_duration_ms
    );
}

#[test]
fn recipe_completion_clears_active_stage_and_restores_entire_base_config() {
    let recipe = test_recipe();
    let base = ControllerConfig {
        ec_target: 1.1,
        ph_target: 6.8,
        water_level_target: 16.0,
        nutrient_a_ratio: 1.0,
        nutrient_b_ratio: 1.0,
        active_recipe: Some(recipe),
        ..ControllerConfig::default()
    };

    let mut state = ControllerRuntimeState::new(base.clone());
    let mut ctx = SystemContext::default();

    // Activate stage 0
    let tick1 = tick_recipe_engine(&mut state.effective_config, &ctx, 1_700_000_000);
    state.apply_recipe_tick_result(&tick1);
    ctx.apply_delta(&mut { tick1.delta });
    assert!(state.active_recipe.is_some());

    // Advance beyond recipe completion (total duration = 7200s)
    let tick2 = tick_recipe_engine(&mut state.effective_config, &ctx, 1_700_000_000 + 7201);
    assert_eq!(tick2.delta.recipe_completed, Some(true));
    state.apply_recipe_tick_result(&tick2);
    ctx.apply_delta(&mut { tick2.delta });

    // Active stage must be None
    assert!(state.active_recipe.is_none());

    // Effective config must be fully restored to base config
    assert_eq!(state.effective_config, state.base_config);
}

#[test]
fn calculated_stage_wins_over_stale_persisted_stage_index() {
    let recipe = test_recipe();
    // At start_time + 4000s, stage 1 is active (stage 0 was 3600s)
    let res = calculate_stage_index(&recipe, 1_700_000_000 + 4000);
    assert_eq!(res, RecipeStageResult::Active { stage_index: 1 });

    // Even if ctx has persisted stage_index = 0, tick_recipe_engine updates to stage 1
    let ctx = SystemContext {
        current_stage_index: Some(0),
        ..SystemContext::default()
    };

    let mut config = ControllerConfig {
        active_recipe: Some(recipe),
        ..ControllerConfig::default()
    };

    let tick = tick_recipe_engine(&mut config, &ctx, 1_700_000_000 + 4000);
    assert_eq!(tick.delta.current_stage_index, Some(Some(1)));
}

#[test]
fn recipe_boot_activates_correct_stage_on_startup() {
    let recipe = test_recipe();
    // Device reboots at start_time + 4000s (during Stage 2, which starts at +3600s)
    let base = ControllerConfig {
        ec_target: 1.0,
        ph_target: 6.5,
        active_recipe: Some(recipe.clone()),
        ..ControllerConfig::default()
    };

    let mut state = ControllerRuntimeState::new(base);
    let mut ctx = SystemContext::default();
    assert_eq!(ctx.current_stage_index, None);

    // Initial tick on boot
    let tick = tick_recipe_engine(&mut state.effective_config, &ctx, 1_700_000_000 + 4000);
    state.apply_recipe_tick_result(&tick);
    ctx.apply_delta(&mut { tick.delta });

    assert_eq!(ctx.current_stage_index, Some(1));
    let stage1 = &recipe.stages[1];
    assert_eq!(state.active_recipe.as_ref().unwrap().name, stage1.name);
    assert_eq!(state.effective_config.ec_target, stage1.ec_target);
    assert_eq!(state.effective_config.ph_target, stage1.ph_target);
    assert_eq!(
        state.effective_config.water_level_target,
        stage1.water_level_target
    );
}

#[test]
fn factory_reset_simulation_restores_default_context_and_clears_recipe() {
    let recipe = test_recipe();
    let base = ControllerConfig {
        active_recipe: Some(recipe),
        ..ControllerConfig::default()
    };

    let mut state = ControllerRuntimeState::new(base);
    let mut ctx = SystemContext::default();

    // Activate stage 0
    let tick = tick_recipe_engine(&mut state.effective_config, &ctx, 1_700_000_000);
    state.apply_recipe_tick_result(&tick);
    ctx.apply_delta(&mut { tick.delta });
    assert_eq!(ctx.current_stage_index, Some(0));

    // Simulate factory reset:
    // 1. Clear active_recipe from base and runtime
    let mut reset_base = state.base_config.clone();
    reset_base.active_recipe = None;
    state.set_base_config(reset_base);
    state.set_active_recipe(None);

    // 2. Clear context stage and reset safety/actor state
    let mut reset_delta = hydragrow_controller_core::core::fsm::ContextDelta {
        phase: Some(hydragrow_shared::fsm::SystemPhase::Fault(
            hydragrow_shared::fsm::FaultCode::EmergencyStop,
        )),
        current_stage_index: Some(None),
        reset_safety_budget: true,
        ..Default::default()
    };
    ctx.apply_delta(&mut reset_delta);

    // Verify all recipe state and overrides are wiped clean
    assert!(state.active_recipe.is_none());
    assert_eq!(ctx.current_stage_index, None);
    assert_eq!(
        ctx.phase,
        hydragrow_shared::fsm::SystemPhase::Fault(hydragrow_shared::fsm::FaultCode::EmergencyStop)
    );
    assert_eq!(state.effective_config, state.base_config);
}
