#[cfg(test)]
mod tests {
    use crate::db::recipes::*;

    fn stage_fixture(recipe_id: &str, order: i32) -> RecipeStageRow {
        RecipeStageRow {
            id: format!("stage-{}-{}", recipe_id, order),
            recipe_id: recipe_id.to_string(),
            stage_order: order,
            name: format!("Stage {}", order),
            duration_days: 7,
            ec_target: 1.4,
            ec_tolerance: 0.1,
            ph_target: 6.0,
            ph_tolerance: 0.2,
            nutrient_a_ratio: 1.0,
            nutrient_b_ratio: 1.0,
            water_level_target: 20.0,
            misting_on_duration_ms: 10000,
            misting_off_duration_ms: 180000,
        }
    }

    #[sqlx::test]
    async fn insert_and_list_recipes(pool: sqlx::PgPool) {
        insert_recipe(&pool, "recipe-001", "Lettuce Basic", "lettuce", None)
            .await
            .unwrap();
        let list = list_recipes(&pool).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Lettuce Basic");
    }

    #[sqlx::test]
    async fn get_recipe_returns_none_for_unknown(pool: sqlx::PgPool) {
        let result = get_recipe(&pool, "nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[sqlx::test]
    async fn delete_recipe_cascades_stages(pool: sqlx::PgPool) {
        insert_recipe(&pool, "recipe-del", "To Delete", "herb", None)
            .await
            .unwrap();
        insert_stage(&pool, &stage_fixture("recipe-del", 1))
            .await
            .unwrap();
        let rows = delete_recipe(&pool, "recipe-del").await.unwrap();
        assert_eq!(rows, 1);
        let stages = list_stages_for_recipe(&pool, "recipe-del").await.unwrap();
        assert!(stages.is_empty());
    }

    #[sqlx::test]
    async fn list_stages_ordered_by_stage_order(pool: sqlx::PgPool) {
        insert_recipe(&pool, "recipe-ord", "Ordered", "tomato", None)
            .await
            .unwrap();
        insert_stage(&pool, &stage_fixture("recipe-ord", 2))
            .await
            .unwrap();
        insert_stage(&pool, &stage_fixture("recipe-ord", 1))
            .await
            .unwrap();

        let stages = list_stages_for_recipe(&pool, "recipe-ord").await.unwrap();
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].stage_order, 1);
        assert_eq!(stages[1].stage_order, 2);
    }

    #[sqlx::test]
    async fn get_active_stage_context_returns_none_when_no_active_recipe(pool: sqlx::PgPool) {
        let ctx = get_active_stage_context(&pool, "device_without_recipe")
            .await
            .expect("query should not error");
        assert!(ctx.is_none());
    }

    #[sqlx::test]
    async fn get_active_stage_context_returns_stage_index_and_elapsed(pool: sqlx::PgPool) {
        sqlx::query("INSERT INTO crop_recipes (id, name, crop) VALUES ('r1', 'Recipe 1', 'lettuce')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO crop_recipe_stages (id, recipe_id, stage_order, name)
             VALUES ('s1', 'r1', 1, 'Seedling'), ('s2', 'r1', 2, 'Vegetative')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO device_active_recipes (id, device_id, season_id, recipe_id, current_stage_id, applied_at)
             VALUES ('a1', 'dev1', 'season1', 'r1', 's2', NOW() - INTERVAL '90 seconds')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let ctx = get_active_stage_context(&pool, "dev1")
            .await
            .expect("query should not error")
            .expect("expected Some(context)");

        assert_eq!(ctx.recipe_id, "r1");
        assert_eq!(ctx.stage_index, 1); // stage_order=2 → 0-based index 1
        assert!(ctx.elapsed_sec >= 90);
    }

    #[sqlx::test]
    async fn advance_active_recipe_stage_updates_current_stage_id(pool: sqlx::PgPool) {
        sqlx::query("INSERT INTO crop_recipes (id, name, crop) VALUES ('r1', 'Recipe 1', 'lettuce')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO crop_recipe_stages (id, recipe_id, stage_order, name)
             VALUES ('s1', 'r1', 1, 'Seedling'), ('s2', 'r1', 2, 'Vegetative')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO device_active_recipes (id, device_id, season_id, recipe_id, current_stage_id)
             VALUES ('a1', 'dev1', 'season1', 'r1', 's1')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let new_stage = advance_active_recipe_stage(&pool, "dev1", 1) // target index 1 → stage_order 2
            .await
            .expect("query should not error")
            .expect("expected Some(stage)");
        assert_eq!(new_stage.id, "s2");
        assert_eq!(new_stage.name, "Vegetative");

        let current_id: String = sqlx::query_scalar(
            "SELECT current_stage_id FROM device_active_recipes WHERE device_id = 'dev1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(current_id, "s2");
    }

    #[sqlx::test]
    async fn advance_active_recipe_stage_returns_none_for_out_of_range_index(pool: sqlx::PgPool) {
        sqlx::query("INSERT INTO crop_recipes (id, name, crop) VALUES ('r1', 'Recipe 1', 'lettuce')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO crop_recipe_stages (id, recipe_id, stage_order, name) VALUES ('s1', 'r1', 1, 'Seedling')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO device_active_recipes (id, device_id, season_id, recipe_id, current_stage_id)
             VALUES ('a1', 'dev1', 'season1', 'r1', 's1')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = advance_active_recipe_stage(&pool, "dev1", 5)
            .await
            .expect("query should not error");
        assert!(result.is_none());
    }
}
