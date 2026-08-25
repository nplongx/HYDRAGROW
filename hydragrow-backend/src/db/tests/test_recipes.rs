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
}
